//! Generic Redis caching layer for entities
//!
//! Provides a write-through caching pattern for frequently accessed entities
//! like users, organizations, and triggers.
//!
//! # Performance Characteristics
//!
//! - **Cache hit**: ~0.1-0.5ms (8-100x faster than PostgreSQL)
//! - **Cache miss**: Transparent fallback to database
//! - **Expected hit rate**: 80-95% for auth/org lookups
//!
//! # Cache Strategy
//!
//! - **Write-through**: Updates written to both PostgreSQL and Redis
//! - **TTL**: Configurable per entity type (default 5 minutes)
//! - **Graceful degradation**: Falls back to PostgreSQL if Redis unavailable
//!
//! # Key Prefixes
//!
//! - `user:id:{user_id}` - User by ID
//! - `user:email:{email}` - User by email
//! - `org:id:{org_id}` - Organization by ID
//! - `org:member:{org_id}:{user_id}` - Membership role
//! - `trigger:id:{trigger_id}` - Trigger by ID

use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Default cache TTL in seconds (5 minutes)
const DEFAULT_TTL_SECS: u64 = 300;

/// Entity cache manager for Redis
///
/// Generic caching layer that can cache any serializable entity.
#[derive(Clone)]
pub struct EntityCache {
    redis: ConnectionManager,
    ttl: Duration,
    enabled: bool,
}

impl EntityCache {
    /// Create a new entity cache
    ///
    /// # Arguments
    ///
    /// * `redis` - Redis connection manager
    /// * `ttl_secs` - Cache TTL in seconds (None for default 300s)
    pub fn new(redis: ConnectionManager, ttl_secs: Option<u64>) -> Self {
        let enabled = std::env::var("ENTITY_CACHE_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let ttl = Duration::from_secs(ttl_secs.unwrap_or(DEFAULT_TTL_SECS));

        debug!(
            ttl_secs = ttl.as_secs(),
            enabled = enabled,
            "Initializing EntityCache"
        );

        Self {
            redis,
            ttl,
            enabled,
        }
    }

    /// Get an entity from cache
    ///
    /// Returns None if not found or on Redis error (graceful degradation)
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        if !self.enabled {
            return None;
        }

        let mut conn = self.redis.clone();

        match conn.get::<_, Option<String>>(key).await {
            Ok(Some(json_str)) => match serde_json::from_str(&json_str) {
                Ok(entity) => {
                    debug!(key = key, "Cache HIT");
                    Some(entity)
                }
                Err(e) => {
                    warn!(key = key, error = %e, "Failed to deserialize cached entity");
                    None
                }
            },
            Ok(None) => {
                debug!(key = key, "Cache MISS");
                None
            }
            Err(e) => {
                warn!(key = key, error = %e, "Redis cache read failed");
                None
            }
        }
    }

    /// Store an entity in cache
    ///
    /// Errors are logged but don't fail the operation (graceful degradation)
    pub async fn set<T: Serialize>(&self, key: &str, entity: &T) {
        if !self.enabled {
            return;
        }

        let mut conn = self.redis.clone();

        match serde_json::to_string(entity) {
            Ok(json_str) => {
                if let Err(e) = conn
                    .set_ex::<_, _, ()>(key, json_str, self.ttl.as_secs())
                    .await
                {
                    warn!(key = key, error = %e, "Redis cache write failed");
                }
            }
            Err(e) => {
                warn!(key = key, error = %e, "Failed to serialize entity for cache");
            }
        }
    }

    /// Delete an entity from cache
    ///
    /// Errors are logged but don't fail the operation
    pub async fn delete(&self, key: &str) {
        if !self.enabled {
            return;
        }

        let mut conn = self.redis.clone();

        if let Err(e) = conn.del::<_, ()>(key).await {
            warn!(key = key, error = %e, "Redis cache delete failed");
        }
    }

    /// Delete multiple entities matching a pattern
    ///
    /// Uses SCAN to avoid blocking Redis (safe for production)
    pub async fn delete_pattern(&self, pattern: &str) {
        if !self.enabled {
            return;
        }

        let mut conn = self.redis.clone();

        // Use SCAN to find matching keys (non-blocking)
        let keys: Result<Vec<String>, _> = redis::cmd("SCAN")
            .arg(0)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await;

        match keys {
            Ok(keys) if !keys.is_empty() => {
                if let Err(e) = conn.del::<_, ()>(&keys).await {
                    warn!(pattern = pattern, error = %e, "Redis pattern delete failed");
                }
            }
            Err(e) => {
                warn!(pattern = pattern, error = %e, "Redis SCAN failed");
            }
            _ => {}
        }
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get cache TTL
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

// ============================================================================
// Key Builders
// ============================================================================

/// Build cache key for user by ID
pub fn user_key_by_id(user_id: &str) -> String {
    format!("user:id:{}", user_id)
}

/// Build cache key for user by email
pub fn user_key_by_email(email: &str) -> String {
    format!("user:email:{}", email.to_lowercase())
}

/// Build cache key for user by username
pub fn user_key_by_username(username: &str) -> String {
    format!("user:username:{}", username.to_lowercase())
}

/// Build cache key for organization by ID
pub fn org_key_by_id(org_id: &str) -> String {
    format!("org:id:{}", org_id)
}

/// Build cache key for organization membership role
pub fn membership_key(org_id: &str, user_id: &str) -> String {
    format!("org:member:{}:{}", org_id, user_id)
}

/// Build cache key for trigger by ID
pub fn trigger_key_by_id(trigger_id: &str) -> String {
    format!("trigger:id:{}", trigger_id)
}

/// Build cache key pattern for all user keys
pub fn user_keys_pattern(user_id: &str) -> String {
    format!("user:*:{}*", user_id)
}

/// Build cache key pattern for all org keys
pub fn org_keys_pattern(org_id: &str) -> String {
    format!("org:*:{}*", org_id)
}

// ============================================================================
// Cached Repository Helpers
// ============================================================================

/// Get from cache or execute database fallback
///
/// This is a free function that provides cache-aside pattern:
/// 1. Check cache first
/// 2. On miss, fetch from database
/// 3. Cache the result for future reads
pub async fn get_or_fetch<T, F, Fut>(cache: &EntityCache, key: &str, fetch: F) -> Result<Option<T>>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync,
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<Option<T>>> + Send,
{
    // Try cache first
    if let Some(cached) = cache.get::<T>(key).await {
        return Ok(Some(cached));
    }

    // Fetch from database
    let result = fetch().await?;

    // Cache the result if found
    if let Some(ref entity) = result {
        cache.set(key, entity).await;
    }

    Ok(result)
}

/// Marker trait for cache-aware repositories (optional, for documentation)
pub trait CacheAware {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_key_by_id() {
        assert_eq!(user_key_by_id("user_123"), "user:id:user_123");
    }

    #[test]
    fn test_user_key_by_email() {
        assert_eq!(
            user_key_by_email("Test@Example.COM"),
            "user:email:test@example.com"
        );
    }

    #[test]
    fn test_user_key_by_username() {
        assert_eq!(user_key_by_username("TestUser"), "user:username:testuser");
    }

    #[test]
    fn test_org_key_by_id() {
        assert_eq!(org_key_by_id("org_456"), "org:id:org_456");
    }

    #[test]
    fn test_membership_key() {
        assert_eq!(
            membership_key("org_456", "user_123"),
            "org:member:org_456:user_123"
        );
    }

    #[test]
    fn test_trigger_key_by_id() {
        assert_eq!(trigger_key_by_id("trigger_789"), "trigger:id:trigger_789");
    }
}
