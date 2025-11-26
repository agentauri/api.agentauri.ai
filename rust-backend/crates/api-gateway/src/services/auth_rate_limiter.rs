//! Authentication Rate Limiter
//!
//! Provides rate limiting for API key authentication to prevent brute-force attacks.
//!
//! # Security Features
//!
//! - Per-IP rate limiting (default: 20 auth attempts per minute)
//! - Global rate limiting (default: 1000 auth attempts per minute)
//! - In-memory storage with automatic cleanup
//! - Thread-safe for concurrent use
//!
//! # Future Enhancements
//!
//! - Redis-backed storage for distributed deployments
//! - Dynamic rate limits based on threat level
//! - IP reputation scoring

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

/// Default rate limit: 20 auth attempts per minute per IP
const DEFAULT_PER_IP_RATE: u32 = 20;

/// Default global rate limit: 1000 auth attempts per minute
const DEFAULT_GLOBAL_RATE: u32 = 1000;

/// Rate limit error
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub message: String,
    pub retry_after_ms: Option<u64>,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RateLimitError {}

/// Type alias for the rate limiter
type IpRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Authentication rate limiter
///
/// Enforces both:
/// - Per-IP rate limiting (prevent individual attackers)
/// - Global rate limiting (prevent distributed attacks)
#[derive(Clone)]
pub struct AuthRateLimiter {
    /// Global rate limiter
    global_limiter: Arc<IpRateLimiter>,
    /// Per-IP rate limiters
    per_ip_limiters: Arc<Mutex<HashMap<String, Arc<IpRateLimiter>>>>,
    /// Rate for per-IP limiting (attempts per minute)
    per_ip_rate: u32,
}

impl AuthRateLimiter {
    /// Create a new authentication rate limiter with default settings
    ///
    /// Default: 20 auth attempts per minute per IP, 1000 globally
    pub fn new() -> Self {
        Self::with_rates(DEFAULT_GLOBAL_RATE, DEFAULT_PER_IP_RATE)
    }

    /// Create with custom global and per-IP rates
    ///
    /// # Arguments
    ///
    /// * `global_rate` - Maximum auth attempts per minute globally
    /// * `per_ip_rate` - Maximum auth attempts per minute per IP
    pub fn with_rates(global_rate: u32, per_ip_rate: u32) -> Self {
        // Convert per-minute rates to per-second for governor
        // governor uses per-second quotas, so we need to handle this
        let global_quota = Quota::per_minute(
            NonZeroU32::new(global_rate).expect("Global rate must be > 0"),
        );

        Self {
            global_limiter: Arc::new(GovernorRateLimiter::direct(global_quota)),
            per_ip_limiters: Arc::new(Mutex::new(HashMap::new())),
            per_ip_rate,
        }
    }

    /// Check if an authentication attempt is allowed for the given IP
    ///
    /// # Arguments
    ///
    /// * `ip_address` - Client IP address (use "unknown" if unavailable)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Auth attempt is allowed
    /// * `Err(RateLimitError)` - Rate limit exceeded
    pub fn check(&self, ip_address: &str) -> Result<(), RateLimitError> {
        // Check global limit first
        if self.global_limiter.check().is_err() {
            tracing::warn!(
                ip = ip_address,
                "Global auth rate limit exceeded"
            );
            return Err(RateLimitError {
                message: "Too many authentication attempts. Please try again later.".to_string(),
                retry_after_ms: Some(1000), // 1 second
            });
        }

        // Check per-IP limit
        let ip_limiter = self.get_ip_limiter(ip_address);
        if ip_limiter.check().is_err() {
            tracing::warn!(
                ip = ip_address,
                "Per-IP auth rate limit exceeded"
            );
            return Err(RateLimitError {
                message: "Too many authentication attempts from your IP. Please try again later.".to_string(),
                retry_after_ms: Some(60000), // 1 minute
            });
        }

        Ok(())
    }

    /// Get or create a rate limiter for a specific IP address
    fn get_ip_limiter(&self, ip_address: &str) -> Arc<IpRateLimiter> {
        let mut limiters = self.per_ip_limiters.lock().unwrap();

        limiters
            .entry(ip_address.to_string())
            .or_insert_with(|| {
                let quota = Quota::per_minute(
                    NonZeroU32::new(self.per_ip_rate).expect("Per-IP rate must be > 0"),
                );
                Arc::new(IpRateLimiter::direct(quota))
            })
            .clone()
    }

    /// Clean up old rate limiters for IPs that haven't been seen recently
    ///
    /// This should be called periodically to prevent memory leaks.
    /// Note: With governor's default implementation, stale limiters
    /// naturally reset after the quota period.
    #[allow(dead_code)]
    pub fn cleanup_stale_limiters(&self, max_entries: usize) {
        let mut limiters = self.per_ip_limiters.lock().unwrap();

        // If we have too many entries, remove some
        // In production, you'd want a more sophisticated eviction strategy
        if limiters.len() > max_entries {
            let to_remove = limiters.len() - max_entries;
            let keys: Vec<String> = limiters.keys().take(to_remove).cloned().collect();
            for key in keys {
                limiters.remove(&key);
            }
            tracing::info!(
                removed = to_remove,
                remaining = limiters.len(),
                "Cleaned up stale IP rate limiters"
            );
        }
    }
}

impl Default for AuthRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_limiter() {
        let limiter = AuthRateLimiter::new();
        // Should allow initial check
        assert!(limiter.check("192.168.1.1").is_ok());
    }

    #[test]
    fn test_create_custom_rates() {
        let limiter = AuthRateLimiter::with_rates(100, 10);
        assert!(limiter.check("192.168.1.1").is_ok());
    }

    #[test]
    fn test_per_ip_rate_limiting() {
        // Create a limiter with very low per-IP rate
        let limiter = AuthRateLimiter::with_rates(1000, 2);

        // First two checks should pass
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());

        // Third check from same IP should fail
        let result = limiter.check("192.168.1.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("your IP"));
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = AuthRateLimiter::with_rates(1000, 2);

        // Use up limit for first IP
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_err());

        // Second IP should still work
        assert!(limiter.check("192.168.1.2").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());
    }

    #[test]
    fn test_global_rate_limiting() {
        // Create a limiter with very low global rate
        let limiter = AuthRateLimiter::with_rates(2, 100);

        // First two checks should pass (from different IPs to avoid per-IP limit)
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());

        // Third check should fail due to global limit
        let result = limiter.check("192.168.1.3");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Too many authentication attempts"));
    }

    #[test]
    fn test_rate_limit_error_display() {
        let error = RateLimitError {
            message: "Test error".to_string(),
            retry_after_ms: Some(1000),
        };
        assert_eq!(format!("{}", error), "Test error");
    }

    #[test]
    fn test_cleanup_stale_limiters() {
        let limiter = AuthRateLimiter::with_rates(1000, 10);

        // Create entries for many IPs
        for i in 0..100 {
            let _ = limiter.check(&format!("192.168.1.{}", i));
        }

        // Cleanup to keep only 50
        limiter.cleanup_stale_limiters(50);

        // Verify cleanup happened
        let limiters = limiter.per_ip_limiters.lock().unwrap();
        assert!(limiters.len() <= 50);
    }

    #[test]
    fn test_clone_shares_state() {
        let limiter1 = AuthRateLimiter::with_rates(1000, 2);
        let limiter2 = limiter1.clone();

        // Use up limit on first clone
        assert!(limiter1.check("192.168.1.1").is_ok());
        assert!(limiter1.check("192.168.1.1").is_ok());

        // Second clone should see the exhausted limit
        assert!(limiter2.check("192.168.1.1").is_err());
    }
}
