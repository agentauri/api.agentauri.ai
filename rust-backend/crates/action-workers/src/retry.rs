//! Retry logic for action workers
//!
//! Provides exponential backoff retry policy with configurable parameters.

use std::time::Duration;

use crate::error::WorkerError;
use crate::metrics;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries (doubles each attempt)
    pub base_delay: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    /// Default policy: 3 attempts with delays of 1s, 2s, 4s
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(4),
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_attempts: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts,
            base_delay,
            max_delay,
        }
    }

    /// Calculate delay for given attempt (1-indexed)
    ///
    /// Uses exponential backoff: base_delay * 2^(attempt-1)
    /// Capped at max_delay
    ///
    /// # Arguments
    ///
    /// * `attempt` - Current attempt number (1-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let multiplier = 2u32.saturating_pow(attempt.saturating_sub(1));
        let delay = self.base_delay.saturating_mul(multiplier);
        std::cmp::min(delay, self.max_delay)
    }

    /// Check if another retry should be attempted
    ///
    /// # Arguments
    ///
    /// * `attempt` - Current attempt number (1-indexed)
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Execute an async operation with retry logic
///
/// # Arguments
///
/// * `policy` - Retry policy to use
/// * `action_type` - Action type for metrics labeling
/// * `operation` - Async operation to execute
///
/// # Returns
///
/// Result of the operation, or the last error if all retries failed
pub async fn execute_with_retry<F, Fut, T>(
    policy: &RetryPolicy,
    action_type: &str,
    mut operation: F,
) -> Result<T, WorkerError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, WorkerError>>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if error is retryable and we have attempts left
                if e.is_retryable() && policy.should_retry(attempt) {
                    let delay = policy.delay_for_attempt(attempt);

                    tracing::warn!(
                        attempt = attempt,
                        max_attempts = policy.max_attempts,
                        delay_ms = delay.as_millis(),
                        error = %e,
                        action_type = action_type,
                        "Retrying after error"
                    );

                    metrics::record_retry(action_type, attempt);
                    tokio::time::sleep(delay).await;
                } else {
                    // Not retryable or no retries left
                    if !e.is_retryable() {
                        tracing::debug!(
                            error = %e,
                            action_type = action_type,
                            "Error is not retryable, failing immediately"
                        );
                    } else {
                        tracing::warn!(
                            attempt = attempt,
                            max_attempts = policy.max_attempts,
                            error = %e,
                            action_type = action_type,
                            "Max retries exceeded"
                        );
                    }
                    return Err(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_default_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(4));
    }

    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy::default();

        // Attempt 1: 1s * 2^0 = 1s
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1));

        // Attempt 2: 1s * 2^1 = 2s
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2));

        // Attempt 3: 1s * 2^2 = 4s (capped at max_delay)
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(4));

        // Attempt 4: would be 8s but capped at 4s
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(4));
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::default();

        assert!(policy.should_retry(1)); // Can retry after attempt 1
        assert!(policy.should_retry(2)); // Can retry after attempt 2
        assert!(!policy.should_retry(3)); // Cannot retry after attempt 3 (max)
        assert!(!policy.should_retry(4)); // Cannot retry after attempt 4
    }

    #[tokio::test]
    async fn test_execute_with_retry_success_first_try() {
        let policy = RetryPolicy::default();
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result = execute_with_retry(&policy, "test", || {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<_, WorkerError>(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_execute_with_retry_success_after_retries() {
        let policy = RetryPolicy::new(
            3,
            Duration::from_millis(10), // Short delay for testing
            Duration::from_millis(40),
        );
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result = execute_with_retry(&policy, "test", || {
            let count = count.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(WorkerError::queue("temporary error"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_with_retry_non_retryable_error() {
        let policy = RetryPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40));
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32, WorkerError> = execute_with_retry(&policy, "test", || {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err(WorkerError::invalid_config("bad config"))
            }
        })
        .await;

        assert!(result.is_err());
        // Should fail immediately without retries
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_execute_with_retry_exhausted() {
        let policy = RetryPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40));
        let call_count = Arc::new(AtomicU32::new(0));
        let count = call_count.clone();

        let result: Result<i32, WorkerError> = execute_with_retry(&policy, "test", || {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err(WorkerError::queue("always fails"))
            }
        })
        .await;

        assert!(result.is_err());
        // Should try max_attempts times
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }
}
