//! Redis-based Rate Limiter with Sliding Window Algorithm
//!
//! This module implements a production-ready rate limiter using Redis Lua scripts
//! for atomic operations. It supports multiple scopes (IP, Organization, Agent) and
//! query tier cost multipliers.
//!
//! # Architecture
//!
//! - **Sliding Window**: 1-hour window with 1-minute granularity (60 buckets)
//! - **Atomic Operations**: Redis Lua script for check-and-increment
//! - **Cost Multipliers**: Different query tiers consume different amounts (1x-10x)
//! - **Graceful Degradation**: Falls back to in-memory rate limiting when Redis unavailable
//!
//! # Fallback Mechanism
//!
//! When Redis is unavailable, the rate limiter automatically falls back to an
//! in-memory DashMap-based limiter with conservative limits:
//! - Uses a 1-minute window for simplicity
//! - Applies a more restrictive fallback limit (default: 10 req/min)
//! - Automatically switches back to Redis when it becomes available
//!
//! # Example
//!
//! ```ignore
//! use shared::redis::{RateLimiter, RateLimitScope};
//!
//! async fn example(redis_manager: redis::aio::ConnectionManager) -> Result<(), Box<dyn std::error::Error>> {
//!     let limiter = RateLimiter::new(redis_manager).await?;
//!
//!     // Check rate limit for an organization
//!     let result = limiter.check(
//!         RateLimitScope::Organization("org_123".to_string()),
//!         100,  // limit
//!         2,    // cost (Tier 1 query)
//!     ).await?;
//!
//!     if result.allowed {
//!         println!("Request allowed. Remaining: {}", result.remaining);
//!     } else {
//!         println!("Rate limited. Retry after: {}", result.retry_after);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Rust guideline compliant 2025-01-28

use crate::error::{Error, Result};
use dashmap::DashMap;
use redis::{aio::ConnectionManager, AsyncCommands, Script};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Rate limit scope (determines Redis key prefix)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitScope {
    /// IP-based rate limiting (Layer 0 - Anonymous)
    Ip(String),
    /// Organization-based rate limiting (Layer 1 - API Key, Layer 2 - Wallet)
    Organization(String),
    /// Agent-based rate limiting (Layer 2 - Agent operations)
    Agent(i64),
}

impl RateLimitScope {
    /// Get the Redis key prefix for this scope
    pub fn key_prefix(&self) -> String {
        match self {
            RateLimitScope::Ip(ip) => format!("rl:ip:{}", ip),
            RateLimitScope::Organization(org_id) => format!("rl:org:{}", org_id),
            RateLimitScope::Agent(agent_id) => format!("rl:agent:{}", agent_id),
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            RateLimitScope::Ip(ip) => format!("IP {}", ip),
            RateLimitScope::Organization(org_id) => format!("Organization {}", org_id),
            RateLimitScope::Agent(agent_id) => format!("Agent {}", agent_id),
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Current usage in the window (after this request if allowed)
    pub current_usage: i64,
    /// The configured limit
    pub limit: i64,
    /// Unix timestamp when the rate limit resets
    pub reset_at: i64,
    /// Seconds until the rate limit resets (convenience field)
    pub retry_after: i64,
    /// Remaining quota (limit - current_usage)
    pub remaining: i64,
}

impl RateLimitResult {
    /// Create a result from Lua script response
    fn from_lua_response(response: Vec<i64>, current_time: i64) -> Self {
        let allowed = response[0] == 1;
        let current_usage = response[1];
        let limit = response[2];
        let reset_at = response[3];
        let retry_after = (reset_at - current_time).max(0);
        let remaining = (limit - current_usage).max(0);

        Self {
            allowed,
            current_usage,
            limit,
            reset_at,
            retry_after,
            remaining,
        }
    }

    /// Create a "fail-open" result (allows request when Redis is down)
    fn fail_open(limit: i64) -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            allowed: true,
            current_usage: 0,
            limit,
            reset_at: current_time + 3600,
            retry_after: 0,
            remaining: limit,
        }
    }
}

/// Entry in the fallback rate limiter
#[derive(Debug, Clone)]
struct FallbackEntry {
    /// Current request count in the window
    count: i64,
    /// When this window started
    window_start: Instant,
}

/// Redis-based rate limiter with in-memory fallback
///
/// Uses a Lua script for atomic check-and-increment operations.
/// Falls back to in-memory rate limiting when Redis is unavailable.
#[derive(Clone)]
pub struct RateLimiter {
    /// Redis connection manager
    redis: ConnectionManager,
    /// Lua script for rate limiting
    script: Script,
    /// Window size in seconds (default: 3600 = 1 hour)
    window_seconds: i64,
    /// Whether to fail open (allow requests) when Redis is unavailable
    fail_open: bool,
    /// In-memory fallback rate limiter (DashMap for thread-safe concurrent access)
    /// Key: scope key prefix, Value: (count, window_start)
    fallback_limiter: Arc<DashMap<String, FallbackEntry>>,
    /// Conservative limit for fallback mode (requests per minute)
    fallback_limit: i64,
    /// Fallback window duration
    fallback_window: Duration,
}

impl RateLimiter {
    /// Default window size (1 hour)
    pub const DEFAULT_WINDOW: i64 = 3600;

    /// Default fallback limit (10 requests per minute - conservative)
    pub const DEFAULT_FALLBACK_LIMIT: i64 = 10;

    /// Default fallback window (1 minute)
    pub const DEFAULT_FALLBACK_WINDOW: Duration = Duration::from_secs(60);

    /// Lua script source (embedded at compile time)
    const LUA_SCRIPT: &'static str = include_str!("rate_limit.lua");

    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `redis` - Redis connection manager
    ///
    /// # Returns
    ///
    /// A configured rate limiter with default settings (1-hour window, fail-open enabled)
    pub async fn new(redis: ConnectionManager) -> Result<Self> {
        Self::with_config(redis, Self::DEFAULT_WINDOW, true).await
    }

    /// Create a rate limiter with custom configuration
    ///
    /// # Arguments
    ///
    /// * `redis` - Redis connection manager
    /// * `window_seconds` - Sliding window size in seconds
    /// * `fail_open` - Whether to allow requests when Redis is unavailable
    pub async fn with_config(
        redis: ConnectionManager,
        window_seconds: i64,
        fail_open: bool,
    ) -> Result<Self> {
        let script = Script::new(Self::LUA_SCRIPT);

        debug!(
            window_seconds = window_seconds,
            fail_open = fail_open,
            "Rate limiter initialized with in-memory fallback"
        );

        Ok(Self {
            redis,
            script,
            window_seconds,
            fail_open,
            fallback_limiter: Arc::new(DashMap::new()),
            fallback_limit: Self::DEFAULT_FALLBACK_LIMIT,
            fallback_window: Self::DEFAULT_FALLBACK_WINDOW,
        })
    }

    /// Check rate limit using the in-memory fallback limiter
    ///
    /// This is used when Redis is unavailable. It provides a simpler,
    /// more conservative rate limiting mechanism.
    ///
    /// # Arguments
    ///
    /// * `scope` - The rate limit scope
    /// * `cost` - Cost of this request
    ///
    /// # Returns
    ///
    /// `RateLimitResult` with fallback limits applied
    fn check_fallback(&self, scope: &RateLimitScope, cost: i64) -> RateLimitResult {
        let key = scope.key_prefix();
        let now = Instant::now();
        let limit = self.fallback_limit;

        let mut entry = self.fallback_limiter.entry(key.clone()).or_insert_with(|| {
            FallbackEntry {
                count: 0,
                window_start: now,
            }
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) >= self.fallback_window {
            // Reset window
            entry.count = 0;
            entry.window_start = now;
        }

        // Check if we're over limit
        let new_count = entry.count + cost;
        let allowed = new_count <= limit;

        if allowed {
            entry.count = new_count;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let reset_at = current_time + self.fallback_window.as_secs() as i64;
        let retry_after = if allowed { 0 } else { self.fallback_window.as_secs() as i64 };
        let remaining = (limit - new_count).max(0);

        RateLimitResult {
            allowed,
            current_usage: new_count,
            limit,
            reset_at,
            retry_after,
            remaining,
        }
    }

    /// Clean up expired entries from the fallback limiter
    ///
    /// Should be called periodically to prevent memory growth.
    /// Entries older than 2x the fallback window are removed.
    pub fn cleanup_fallback(&self) {
        let now = Instant::now();
        let expiry = self.fallback_window * 2;

        self.fallback_limiter.retain(|_, entry| {
            now.duration_since(entry.window_start) < expiry
        });

        debug!(
            entries_remaining = self.fallback_limiter.len(),
            "Cleaned up expired fallback rate limit entries"
        );
    }

    /// Check rate limit and increment if allowed
    ///
    /// This method performs an atomic check-and-increment operation using a Lua script.
    /// If the limit is not exceeded, it increments the counter and returns success.
    ///
    /// # Arguments
    ///
    /// * `scope` - The rate limit scope (IP, Organization, or Agent)
    /// * `limit` - Maximum requests allowed in the window
    /// * `cost` - Cost of this request (1-10 based on query tier)
    ///
    /// # Returns
    ///
    /// `RateLimitResult` containing:
    /// - `allowed`: Whether the request should be allowed
    /// - `current_usage`: Total usage in the window
    /// - `remaining`: Remaining quota
    /// - `reset_at`: When the limit resets (Unix timestamp)
    ///
    /// # Errors
    ///
    /// Returns an error if Redis communication fails and `fail_open` is false.
    /// If `fail_open` is true, returns a success result with a warning log.
    pub async fn check(
        &self,
        scope: RateLimitScope,
        limit: i64,
        cost: i64,
    ) -> Result<RateLimitResult> {
        let key_prefix = scope.key_prefix();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("System time error: {}", e)))?
            .as_secs() as i64;

        debug!(
            scope = %scope.description(),
            limit = limit,
            cost = cost,
            "Checking rate limit"
        );

        // Execute Lua script
        let mut conn = self.redis.clone();
        let result = self
            .script
            .key(&key_prefix)
            .arg(limit)
            .arg(self.window_seconds)
            .arg(cost)
            .arg(current_time)
            .invoke_async::<Vec<i64>>(&mut conn)
            .await;

        match result {
            Ok(response) => {
                let result = RateLimitResult::from_lua_response(response, current_time);

                if result.allowed {
                    debug!(
                        scope = %scope.description(),
                        current_usage = result.current_usage,
                        remaining = result.remaining,
                        "Rate limit check: ALLOWED"
                    );
                } else {
                    warn!(
                        scope = %scope.description(),
                        current_usage = result.current_usage,
                        limit = limit,
                        retry_after = result.retry_after,
                        "Rate limit check: REJECTED"
                    );
                }

                Ok(result)
            }
            Err(e) => {
                error!(
                    scope = %scope.description(),
                    error = %e,
                    "Redis error during rate limit check"
                );

                if self.fail_open {
                    // Use fallback rate limiter instead of just failing open
                    info!(
                        scope = %scope.description(),
                        fallback_limit = self.fallback_limit,
                        "Redis unavailable, using in-memory fallback rate limiter"
                    );
                    let result = self.check_fallback(&scope, cost);

                    if result.allowed {
                        debug!(
                            scope = %scope.description(),
                            current_usage = result.current_usage,
                            remaining = result.remaining,
                            "Fallback rate limit check: ALLOWED"
                        );
                    } else {
                        warn!(
                            scope = %scope.description(),
                            current_usage = result.current_usage,
                            limit = result.limit,
                            "Fallback rate limit check: REJECTED"
                        );
                    }

                    Ok(result)
                } else {
                    Err(Error::internal(format!("Rate limiter unavailable: {}", e)))
                }
            }
        }
    }

    /// Get current usage without incrementing
    ///
    /// This is useful for displaying current rate limit status without consuming quota.
    ///
    /// # Arguments
    ///
    /// * `scope` - The rate limit scope
    /// * `limit` - The configured limit (for calculating remaining)
    ///
    /// # Returns
    ///
    /// `RateLimitResult` with `allowed = true` and current usage information
    pub async fn get_current_usage(
        &self,
        scope: RateLimitScope,
        limit: i64,
    ) -> Result<RateLimitResult> {
        // Check with cost = 0 (won't increment, just reads)
        let key_prefix = scope.key_prefix();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("System time error: {}", e)))?
            .as_secs() as i64;

        // Calculate minute boundaries (same as Lua script)
        let minute_seconds = 60;
        let buckets_per_hour = self.window_seconds / minute_seconds;
        let current_minute = (current_time / minute_seconds) * minute_seconds;

        // Sum all buckets
        let mut total_usage = 0i64;
        let mut redis = self.redis.clone();

        for i in 0..buckets_per_hour {
            let bucket_time = current_minute - (i * minute_seconds);
            let bucket_key = format!("{}:{}", key_prefix, bucket_time);

            match redis.get::<_, Option<i64>>(&bucket_key).await {
                Ok(Some(count)) => total_usage += count,
                Ok(None) => {} // Bucket doesn't exist yet
                Err(e) => {
                    error!(error = %e, "Failed to read bucket during usage check");
                    if self.fail_open {
                        return Ok(RateLimitResult::fail_open(limit));
                    } else {
                        return Err(Error::internal(format!(
                            "Failed to get current usage: {}",
                            e
                        )));
                    }
                }
            }
        }

        let reset_at = current_minute + self.window_seconds;
        let retry_after = (reset_at - current_time).max(0);
        let remaining = (limit - total_usage).max(0);

        Ok(RateLimitResult {
            allowed: true, // Not actually checking limit, just reading
            current_usage: total_usage,
            limit,
            reset_at,
            retry_after,
            remaining,
        })
    }

    /// Reset rate limit for a scope (for testing or admin operations)
    ///
    /// WARNING: This deletes all buckets for the given scope. Use with caution.
    ///
    /// # Arguments
    ///
    /// * `scope` - The rate limit scope to reset
    #[cfg(test)]
    pub async fn reset(&self, scope: RateLimitScope) -> Result<()> {
        let key_pattern = format!("{}:*", scope.key_prefix());
        let mut redis = self.redis.clone();

        // In production, you'd use SCAN for safety, but for tests this is fine
        let keys: Vec<String> = redis
            .keys(&key_pattern)
            .await
            .map_err(|e| Error::internal(format!("Failed to find keys: {}", e)))?;

        if !keys.is_empty() {
            redis
                .del::<_, ()>(&keys)
                .await
                .map_err(|e| Error::internal(format!("Failed to delete keys: {}", e)))?;

            debug!(scope = %scope.description(), keys_deleted = keys.len(), "Rate limit reset");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_scope_key_prefix() {
        assert_eq!(
            RateLimitScope::Ip("192.168.1.1".to_string()).key_prefix(),
            "rl:ip:192.168.1.1"
        );
        assert_eq!(
            RateLimitScope::Organization("org_123".to_string()).key_prefix(),
            "rl:org:org_123"
        );
        assert_eq!(RateLimitScope::Agent(42).key_prefix(), "rl:agent:42");
    }

    #[test]
    fn test_rate_limit_result_remaining() {
        let result = RateLimitResult::from_lua_response(vec![1, 25, 100, 1732804200], 1732800600);

        assert!(result.allowed);
        assert_eq!(result.current_usage, 25);
        assert_eq!(result.limit, 100);
        assert_eq!(result.remaining, 75);
        assert_eq!(result.retry_after, 3600);
    }

    #[test]
    fn test_rate_limit_result_exceeded() {
        let result = RateLimitResult::from_lua_response(vec![0, 105, 100, 1732804200], 1732800600);

        assert!(!result.allowed);
        assert_eq!(result.current_usage, 105);
        assert_eq!(result.remaining, 0); // Clamped to 0
    }

    #[test]
    fn test_fail_open_result() {
        let result = RateLimitResult::fail_open(100);

        assert!(result.allowed);
        assert_eq!(result.current_usage, 0);
        assert_eq!(result.remaining, 100);
    }

    #[test]
    fn test_fallback_entry_creation() {
        let fallback_limiter: DashMap<String, FallbackEntry> = DashMap::new();
        let now = Instant::now();

        fallback_limiter.insert(
            "test_key".to_string(),
            FallbackEntry {
                count: 5,
                window_start: now,
            },
        );

        let entry = fallback_limiter.get("test_key").unwrap();
        assert_eq!(entry.count, 5);
    }

    #[test]
    fn test_fallback_limiter_basic() {
        // Create a minimal fallback limiter for testing
        let fallback_limiter: Arc<DashMap<String, FallbackEntry>> = Arc::new(DashMap::new());
        let fallback_limit: i64 = 10;
        let fallback_window = Duration::from_secs(60);

        let scope = RateLimitScope::Ip("192.168.1.1".to_string());
        let key = scope.key_prefix();
        let now = Instant::now();
        let cost: i64 = 1;

        // First request should be allowed
        let mut entry = fallback_limiter.entry(key.clone()).or_insert_with(|| FallbackEntry {
            count: 0,
            window_start: now,
        });

        let new_count = entry.count + cost;
        let allowed = new_count <= fallback_limit;

        assert!(allowed);
        assert_eq!(new_count, 1);

        entry.count = new_count;
        drop(entry);

        // Simulate 9 more requests (should all be allowed)
        for i in 2..=10 {
            let mut entry = fallback_limiter.get_mut(&key).unwrap();
            let new_count = entry.count + cost;
            let allowed = new_count <= fallback_limit;
            assert!(allowed, "Request {} should be allowed", i);
            entry.count = new_count;
        }

        // 11th request should be rejected
        let entry = fallback_limiter.get(&key).unwrap();
        let new_count = entry.count + cost;
        let allowed = new_count <= fallback_limit;
        assert!(!allowed, "11th request should be rejected");
    }

    #[test]
    fn test_fallback_limiter_window_reset() {
        let fallback_limiter: Arc<DashMap<String, FallbackEntry>> = Arc::new(DashMap::new());
        let key = "test_key".to_string();
        let old_start = Instant::now() - Duration::from_secs(120); // 2 minutes ago

        // Insert an old entry
        fallback_limiter.insert(
            key.clone(),
            FallbackEntry {
                count: 100, // Over limit
                window_start: old_start,
            },
        );

        // Check that we can detect window expiry
        let now = Instant::now();
        let fallback_window = Duration::from_secs(60);

        let entry = fallback_limiter.get(&key).unwrap();
        let window_expired = now.duration_since(entry.window_start) >= fallback_window;

        assert!(window_expired, "Window should be expired");
    }

    #[test]
    fn test_fallback_cleanup() {
        let fallback_limiter: Arc<DashMap<String, FallbackEntry>> = Arc::new(DashMap::new());
        let fallback_window = Duration::from_secs(60);
        let expiry = fallback_window * 2;

        // Insert a fresh entry
        let now = Instant::now();
        fallback_limiter.insert(
            "fresh".to_string(),
            FallbackEntry {
                count: 5,
                window_start: now,
            },
        );

        // Insert an old entry (should be cleaned up)
        fallback_limiter.insert(
            "old".to_string(),
            FallbackEntry {
                count: 10,
                window_start: now - Duration::from_secs(300), // 5 minutes ago
            },
        );

        // Cleanup
        fallback_limiter.retain(|_, entry| {
            now.duration_since(entry.window_start) < expiry
        });

        // Fresh entry should remain
        assert!(fallback_limiter.contains_key("fresh"));
        // Old entry should be removed
        assert!(!fallback_limiter.contains_key("old"));
    }
}
