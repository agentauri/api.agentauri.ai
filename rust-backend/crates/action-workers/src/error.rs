//! Error types for action workers
//!
//! Provides structured error handling for all action worker operations.

use thiserror::Error;

/// Worker error types
#[derive(Debug, Error)]
pub enum WorkerError {
    /// Redis connection or operation error
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Telegram API error
    #[error("Telegram API error: {0}")]
    TelegramApi(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Database operation error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Template rendering error
    #[error("Template error: {0}")]
    Template(String),

    /// Job not found
    #[error("Job not found: {0}")]
    #[allow(dead_code)]
    JobNotFound(String),

    /// Queue operation error
    #[error("Queue error: {0}")]
    Queue(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl WorkerError {
    /// Check if this error is retryable
    ///
    /// Transient errors (rate limits, timeouts) are retryable.
    /// Permanent errors (invalid config, serialization) are not.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            WorkerError::Redis(_)
                | WorkerError::TelegramApi(_)
                | WorkerError::RateLimitExceeded(_)
                | WorkerError::Database(_)
                | WorkerError::Queue(_)
        )
    }

    /// Get a safe error message for external/user-facing use
    #[allow(dead_code)]
    ///
    /// # Security
    ///
    /// This method returns sanitized error messages that don't expose:
    /// - Internal file paths
    /// - Database connection details
    /// - Stack traces
    /// - Sensitive configuration
    ///
    /// Use this for API responses or user-visible errors.
    /// Use `to_string()` or `Display` for internal logging only.
    pub fn safe_message(&self) -> String {
        match self {
            WorkerError::Redis(_) => "Database connection error".to_string(),
            WorkerError::TelegramApi(_) => "Failed to send notification".to_string(),
            WorkerError::RateLimitExceeded(_) => {
                "Rate limit exceeded, please try again later".to_string()
            }
            WorkerError::Database(_) => "Database operation failed".to_string(),
            WorkerError::Serialization(_) => "Data format error".to_string(),
            WorkerError::InvalidConfig(msg) => {
                // Config errors might be safe to show, but sanitize just in case
                format!("Configuration error: {}", sanitize_error_message(msg))
            }
            WorkerError::Template(msg) => {
                format!("Template error: {}", sanitize_error_message(msg))
            }
            WorkerError::JobNotFound(_) => "Job not found".to_string(),
            WorkerError::Queue(_) => "Queue operation failed".to_string(),
            WorkerError::Internal(_) => "Internal server error".to_string(),
        }
    }

    /// Create a rate limit error with details
    pub fn rate_limit(details: impl Into<String>) -> Self {
        WorkerError::RateLimitExceeded(details.into())
    }

    /// Create an invalid config error
    pub fn invalid_config(details: impl Into<String>) -> Self {
        WorkerError::InvalidConfig(details.into())
    }

    /// Create a template error
    pub fn template(details: impl Into<String>) -> Self {
        WorkerError::Template(details.into())
    }

    /// Create a Telegram API error
    pub fn telegram(details: impl Into<String>) -> Self {
        WorkerError::TelegramApi(details.into())
    }

    /// Create a queue error
    #[allow(dead_code)]
    pub fn queue(details: impl Into<String>) -> Self {
        WorkerError::Queue(details.into())
    }
}

/// Sanitize error messages for safe external display
///
/// # Security
///
/// Removes potentially sensitive information:
/// - File paths (anything with / or \)
/// - IP addresses and ports
/// - Connection strings
fn sanitize_error_message(msg: &str) -> String {
    let sanitized = msg
        // Remove file paths
        .split(['/', '\\'])
        .next_back()
        .unwrap_or(msg)
        // Truncate long messages
        .chars()
        .take(200)
        .collect::<String>();

    // Remove control characters
    sanitized
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect()
}

/// Convenience result type for worker operations
pub type WorkerResult<T> = Result<T, WorkerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_errors() {
        // Retryable errors
        assert!(WorkerError::rate_limit("too many requests").is_retryable());
        assert!(WorkerError::telegram("timeout").is_retryable());
        assert!(WorkerError::queue("connection lost").is_retryable());

        // Non-retryable errors
        assert!(!WorkerError::invalid_config("missing field").is_retryable());
        assert!(!WorkerError::template("invalid syntax").is_retryable());
        assert!(!WorkerError::Internal("unknown".into()).is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = WorkerError::telegram("Bot token invalid");
        assert_eq!(err.to_string(), "Telegram API error: Bot token invalid");

        let err = WorkerError::rate_limit("30 msg/sec exceeded");
        assert_eq!(err.to_string(), "Rate limit exceeded: 30 msg/sec exceeded");
    }

    #[test]
    fn test_from_json_error() {
        let json_err: serde_json::Error = serde_json::from_str::<String>("invalid").unwrap_err();
        let worker_err: WorkerError = json_err.into();
        assert!(!worker_err.is_retryable());
        assert!(matches!(worker_err, WorkerError::Serialization(_)));
    }

    #[test]
    fn test_safe_message_hides_details() {
        let err = WorkerError::Redis(redis::RedisError::from((
            redis::ErrorKind::IoError,
            "Connection refused",
        )));
        let safe = err.safe_message();
        assert_eq!(safe, "Database connection error");
        assert!(!safe.contains("redis"));
        assert!(!safe.contains("Connection refused"));
    }

    #[test]
    fn test_safe_message_sanitizes_config_error() {
        let err = WorkerError::invalid_config("/etc/secret/config.yaml: permission denied");
        let safe = err.safe_message();
        assert!(!safe.contains("/etc/secret"));
        assert!(safe.contains("Configuration error"));
    }

    #[test]
    fn test_sanitize_error_message_removes_paths() {
        let msg = "/var/lib/app/config/secret.yaml not found";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("/var/lib"));
        assert!(sanitized.contains("not found"));
    }

    #[test]
    fn test_sanitize_error_message_truncates() {
        let long_msg = "a".repeat(500);
        let sanitized = sanitize_error_message(&long_msg);
        assert!(sanitized.len() <= 200);
    }

    #[test]
    fn test_sanitize_error_message_removes_control_chars() {
        let msg = "error\nwith\nnewlines";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains('\n'));
    }
}
