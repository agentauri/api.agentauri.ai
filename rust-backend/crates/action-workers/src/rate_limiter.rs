//! Rate limiting for action workers
//!
//! Provides rate limiting for Telegram messages using the governor crate.
//! Telegram has a global limit of ~30 messages per second.
//!
//! # Security
//!
//! - Global rate limiting prevents API abuse
//! - Per-chat rate limiting prevents spamming individual chats
//! - Configurable limits for different use cases

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::WorkerError;
use crate::metrics;

/// Rate limiter trait for testability
pub trait RateLimiter: Send + Sync {
    /// Wait until rate limit allows the operation (global limit)
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for rate limit
    ///
    /// # Returns
    ///
    /// Ok(()) if rate limit acquired, Err if timeout exceeded
    fn acquire(
        &self,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<(), WorkerError>> + Send;

    /// Wait until rate limit allows the operation for a specific key (e.g., chat_id)
    ///
    /// # Security
    ///
    /// Per-key rate limiting prevents spamming individual chats.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique identifier (e.g., chat_id)
    /// * `timeout` - Maximum time to wait for rate limit
    ///
    /// # Returns
    ///
    /// Ok(()) if rate limit acquired, Err if timeout exceeded
    fn acquire_for_key(
        &self,
        key: &str,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<(), WorkerError>> + Send;
}

/// Type alias for the rate limiter to reduce complexity
type ChatRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Telegram-specific rate limiter
///
/// Enforces both:
/// - Global rate limit of 30 messages per second (Telegram API limit)
/// - Per-chat rate limit of 1 message per second (prevent spam)
pub struct TelegramRateLimiter {
    /// Global rate limiter
    global_limiter: Arc<ChatRateLimiter>,
    /// Per-chat rate limiters
    per_chat_limiters: Arc<Mutex<HashMap<String, Arc<ChatRateLimiter>>>>,
    /// Rate for per-chat limiting (messages per second)
    per_chat_rate: u32,
}

impl TelegramRateLimiter {
    /// Create a new Telegram rate limiter
    ///
    /// Default: 30 messages/sec globally, 1 message/sec per chat
    pub fn new() -> Self {
        Self::with_rates(30, 1)
    }

    /// Create with custom global rate only
    ///
    /// # Arguments
    ///
    /// * `messages_per_second` - Maximum messages per second globally
    pub fn with_rate(messages_per_second: u32) -> Self {
        Self::with_rates(messages_per_second, 1)
    }

    /// Create with custom global and per-chat rates
    ///
    /// # Arguments
    ///
    /// * `global_rate` - Maximum messages per second globally
    /// * `per_chat_rate` - Maximum messages per second per chat
    pub fn with_rates(global_rate: u32, per_chat_rate: u32) -> Self {
        let global_quota =
            Quota::per_second(NonZeroU32::new(global_rate).expect("Global rate must be > 0"));
        Self {
            global_limiter: Arc::new(GovernorRateLimiter::direct(global_quota)),
            per_chat_limiters: Arc::new(Mutex::new(HashMap::new())),
            per_chat_rate,
        }
    }

    /// Check if rate limit would allow immediate execution
    pub fn check(&self) -> bool {
        self.global_limiter.check().is_ok()
    }

    /// Get or create a rate limiter for a specific chat
    fn get_chat_limiter(&self, chat_id: &str) -> Arc<ChatRateLimiter> {
        let mut limiters = self.per_chat_limiters.lock().unwrap();

        limiters
            .entry(chat_id.to_string())
            .or_insert_with(|| {
                let quota = Quota::per_second(
                    NonZeroU32::new(self.per_chat_rate).expect("Per-chat rate must be > 0")
                );
                Arc::new(ChatRateLimiter::direct(quota))
            })
            .clone()
    }
}

impl Default for TelegramRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TelegramRateLimiter {
    fn clone(&self) -> Self {
        Self {
            global_limiter: self.global_limiter.clone(),
            per_chat_limiters: self.per_chat_limiters.clone(),
            per_chat_rate: self.per_chat_rate,
        }
    }
}

impl RateLimiter for TelegramRateLimiter {
    async fn acquire(&self, timeout: Duration) -> Result<(), WorkerError> {
        // Try to acquire global permit within timeout
        match tokio::time::timeout(timeout, self.global_limiter.until_ready()).await {
            Ok(()) => Ok(()),
            Err(_) => {
                metrics::record_rate_limit_hit();
                tracing::warn!(
                    timeout_ms = timeout.as_millis(),
                    "Global rate limit acquisition timed out"
                );
                Err(WorkerError::rate_limit(format!(
                    "Timed out waiting for global rate limit after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }

    async fn acquire_for_key(&self, key: &str, timeout: Duration) -> Result<(), WorkerError> {
        // First acquire global limit
        match tokio::time::timeout(timeout, self.global_limiter.until_ready()).await {
            Ok(()) => {},
            Err(_) => {
                metrics::record_rate_limit_hit();
                tracing::warn!(
                    timeout_ms = timeout.as_millis(),
                    "Global rate limit acquisition timed out"
                );
                return Err(WorkerError::rate_limit(format!(
                    "Timed out waiting for global rate limit after {}ms",
                    timeout.as_millis()
                )));
            }
        }

        // Then acquire per-chat limit
        let chat_limiter = self.get_chat_limiter(key);
        match tokio::time::timeout(timeout, chat_limiter.until_ready()).await {
            Ok(()) => Ok(()),
            Err(_) => {
                metrics::record_rate_limit_hit();
                tracing::warn!(
                    key = key,
                    timeout_ms = timeout.as_millis(),
                    "Per-chat rate limit acquisition timed out"
                );
                Err(WorkerError::rate_limit(format!(
                    "Timed out waiting for per-chat rate limit after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }
}

/// No-op rate limiter for testing
#[cfg(test)]
#[derive(Clone, Default)]
pub struct NoopRateLimiter;

#[cfg(test)]
impl RateLimiter for NoopRateLimiter {
    async fn acquire(&self, _timeout: Duration) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn acquire_for_key(&self, _key: &str, _timeout: Duration) -> Result<(), WorkerError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_limiter() {
        let limiter = TelegramRateLimiter::new();
        // Should allow immediate check
        assert!(limiter.check());
    }

    #[test]
    fn test_create_custom_rate() {
        let limiter = TelegramRateLimiter::with_rate(10);
        assert!(limiter.check());
    }

    #[tokio::test]
    async fn test_acquire_success() {
        let limiter = TelegramRateLimiter::with_rate(100);
        let result = limiter.acquire(Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_limiter() {
        let limiter = NoopRateLimiter;
        let result = limiter.acquire(Duration::from_millis(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_acquires() {
        let limiter = TelegramRateLimiter::with_rate(100);

        // Should be able to acquire multiple times quickly with high rate
        for _ in 0..10 {
            let result = limiter.acquire(Duration::from_secs(1)).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let limiter1 = TelegramRateLimiter::with_rate(50);
        let limiter2 = limiter1.clone();

        // Both should use the same underlying rate limiter
        let result1 = limiter1.acquire(Duration::from_secs(1)).await;
        let result2 = limiter2.acquire(Duration::from_secs(1)).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_per_chat_rate_limiting() {
        let limiter = TelegramRateLimiter::with_rates(100, 2); // High global, low per-chat

        // Should be able to send to different chats
        assert!(limiter.acquire_for_key("chat1", Duration::from_secs(1)).await.is_ok());
        assert!(limiter.acquire_for_key("chat2", Duration::from_secs(1)).await.is_ok());

        // Per-chat limiters are independent
        assert!(limiter.acquire_for_key("chat1", Duration::from_secs(1)).await.is_ok());
    }

    #[tokio::test]
    async fn test_per_chat_creates_limiter() {
        // Use higher rates to avoid rate limit exhaustion in tests
        let limiter = TelegramRateLimiter::with_rates(100, 10);

        // First call should create a limiter for this chat
        assert!(limiter.acquire_for_key("new_chat", Duration::from_secs(1)).await.is_ok());

        // Subsequent calls should use the same limiter
        assert!(limiter.acquire_for_key("new_chat", Duration::from_secs(1)).await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_limiter_per_key() {
        let limiter = NoopRateLimiter;
        assert!(limiter.acquire_for_key("any_key", Duration::from_millis(1)).await.is_ok());
    }
}
