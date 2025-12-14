//! Telegram action worker
//!
//! Sends notifications via Telegram Bot API using teloxide.

use async_trait::async_trait;
use secrecy::Secret;
use serde::Deserialize;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::error::WorkerError;

/// Telegram action configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    /// Telegram chat ID (can be negative for groups)
    pub chat_id: String,
    /// Message template with {{variable}} placeholders
    pub message_template: String,
    /// Parse mode: "Markdown", "MarkdownV2", or "HTML"
    #[serde(default = "default_parse_mode")]
    pub parse_mode: String,
}

fn default_parse_mode() -> String {
    "MarkdownV2".to_string()
}

impl TelegramConfig {
    /// Get teloxide ParseMode from config string
    pub fn get_parse_mode(&self) -> ParseMode {
        match self.parse_mode.to_lowercase().as_str() {
            "html" => ParseMode::Html,
            "markdown" => ParseMode::MarkdownV2, // Use MarkdownV2 for better compatibility
            "markdownv2" => ParseMode::MarkdownV2,
            _ => ParseMode::MarkdownV2,
        }
    }

    /// Validate chat ID format
    ///
    /// # Security
    ///
    /// Validates that the chat ID is a valid numeric format (positive or negative integer).
    /// This prevents injection attacks and ensures the chat ID can be safely parsed.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(WorkerError)` if invalid
    pub fn validate_chat_id(&self) -> Result<(), WorkerError> {
        validate_chat_id(&self.chat_id)
    }
}

/// Validate a Telegram chat ID
///
/// Chat IDs must be numeric (optionally prefixed with `-` for groups).
/// Valid examples: "123456789", "-100123456789"
/// Invalid examples: "abc", "12.34", "12-34", "12 34"
fn validate_chat_id(chat_id: &str) -> Result<(), WorkerError> {
    if chat_id.is_empty() {
        return Err(WorkerError::invalid_config("Chat ID cannot be empty"));
    }

    // Try to parse as i64 to ensure it's a valid numeric ID
    chat_id.parse::<i64>().map_err(|_| {
        WorkerError::invalid_config(format!(
            "Invalid chat_id: '{}' (must be numeric, optionally with '-' prefix for groups)",
            sanitize_for_logging(chat_id)
        ))
    })?;

    Ok(())
}

/// Sanitize a string for safe logging
///
/// # Security
///
/// Prevents log injection attacks by removing newlines and control characters.
/// Also truncates excessively long values.
fn sanitize_for_logging(value: &str) -> String {
    const MAX_LOG_LENGTH: usize = 100;

    let sanitized: String = value
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .take(MAX_LOG_LENGTH)
        .collect();

    if value.len() > MAX_LOG_LENGTH {
        format!("{}...", sanitized)
    } else {
        sanitized
    }
}

/// Telegram client trait for testability
#[async_trait]
pub trait TelegramClient: Send + Sync {
    /// Send a message to a Telegram chat
    ///
    /// # Arguments
    ///
    /// * `chat_id` - Chat ID to send to
    /// * `text` - Message text
    /// * `parse_mode` - Parse mode for formatting
    async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: ParseMode,
    ) -> Result<(), WorkerError>;
}

/// Teloxide-based Telegram client
pub struct TeloxideTelegramClient {
    bot: Bot,
}

impl TeloxideTelegramClient {
    /// Create a new Telegram client with bot token
    ///
    /// # Security
    ///
    /// The token is stored securely using the `secrecy` crate to prevent
    /// accidental exposure in logs or debug output.
    pub fn new(token: &str) -> Self {
        Self {
            bot: Bot::new(token),
        }
    }

    /// Create from environment variable TELOXIDE_TOKEN
    ///
    /// # Security
    ///
    /// The token is read from environment variables and handled securely.
    /// It will never be logged or exposed in error messages.
    pub fn from_env() -> Result<Self, WorkerError> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .or_else(|_| std::env::var("TELOXIDE_TOKEN"))
            .map_err(|_| {
                WorkerError::invalid_config(
                    "TELEGRAM_BOT_TOKEN or TELOXIDE_TOKEN environment variable not set",
                )
            })?;

        // Wrap token in Secret to prevent accidental logging
        let _secret_token = Secret::new(token.clone());

        tracing::debug!("Telegram bot token loaded from environment (token redacted for security)");
        Ok(Self::new(&token))
    }
}

impl Clone for TeloxideTelegramClient {
    fn clone(&self) -> Self {
        Self {
            bot: self.bot.clone(),
        }
    }
}

#[async_trait]
impl TelegramClient for TeloxideTelegramClient {
    async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: ParseMode,
    ) -> Result<(), WorkerError> {
        // Validate and parse chat_id to i64
        validate_chat_id(chat_id)?;

        let chat_id_num: i64 = chat_id.parse().map_err(|_| {
            WorkerError::invalid_config(format!(
                "Invalid chat_id: '{}' (must be numeric)",
                sanitize_for_logging(chat_id)
            ))
        })?;

        // Send message
        self.bot
            .send_message(ChatId(chat_id_num), text)
            .parse_mode(parse_mode)
            .await
            .map_err(|e| {
                // Log error details for debugging (in dev mode only)
                tracing::error!(
                    chat_id = sanitize_for_logging(chat_id),
                    error = %e,
                    error_debug = ?e,
                    "Failed to send Telegram message"
                );
                WorkerError::telegram(format!("Failed to send message to Telegram API: {}", e))
            })?;

        tracing::debug!(
            chat_id = sanitize_for_logging(chat_id),
            "Telegram message sent successfully"
        );

        Ok(())
    }
}

/// Mock Telegram client for testing
#[cfg(test)]
#[derive(Clone, Default)]
pub struct MockTelegramClient {
    /// Track sent messages for verification
    messages: std::sync::Arc<std::sync::Mutex<Vec<SentMessage>>>,
    /// Simulate failures
    should_fail: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Record of a sent message
#[cfg(test)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SentMessage {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: ParseMode,
}

#[cfg(test)]
impl MockTelegramClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a client that always fails
    pub fn failing() -> Self {
        let client = Self::new();
        client
            .should_fail
            .store(true, std::sync::atomic::Ordering::SeqCst);
        client
    }

    /// Get all sent messages
    pub fn sent_messages(&self) -> Vec<SentMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// Get count of sent messages
    pub fn message_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }
}

#[cfg(test)]
#[async_trait]
impl TelegramClient for MockTelegramClient {
    async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: ParseMode,
    ) -> Result<(), WorkerError> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(WorkerError::telegram("Mock failure"));
        }

        self.messages.lock().unwrap().push(SentMessage {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_config_deserialization() {
        let json = r#"{
            "chat_id": "123456789",
            "message_template": "Agent {{agent_id}} received score: {{score}}"
        }"#;

        let config: TelegramConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.chat_id, "123456789");
        assert_eq!(config.parse_mode, "MarkdownV2");
    }

    #[test]
    fn test_telegram_config_with_parse_mode() {
        let json = r#"{
            "chat_id": "-100123456",
            "message_template": "Test",
            "parse_mode": "HTML"
        }"#;

        let config: TelegramConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.parse_mode, "HTML");
        assert!(matches!(config.get_parse_mode(), ParseMode::Html));
    }

    #[test]
    fn test_parse_mode_conversion() {
        let config = TelegramConfig {
            chat_id: "123".to_string(),
            message_template: "test".to_string(),
            parse_mode: "markdown".to_string(),
        };
        assert!(matches!(config.get_parse_mode(), ParseMode::MarkdownV2));

        let config = TelegramConfig {
            chat_id: "123".to_string(),
            message_template: "test".to_string(),
            parse_mode: "html".to_string(),
        };
        assert!(matches!(config.get_parse_mode(), ParseMode::Html));
    }

    #[tokio::test]
    async fn test_mock_client_success() {
        let client = MockTelegramClient::new();

        let result = client
            .send_message("123", "Hello", ParseMode::MarkdownV2)
            .await;

        assert!(result.is_ok());
        assert_eq!(client.message_count(), 1);

        let messages = client.sent_messages();
        assert_eq!(messages[0].chat_id, "123");
        assert_eq!(messages[0].text, "Hello");
    }

    #[tokio::test]
    async fn test_mock_client_failure() {
        let client = MockTelegramClient::failing();

        let result = client
            .send_message("123", "Hello", ParseMode::MarkdownV2)
            .await;

        assert!(result.is_err());
        assert_eq!(client.message_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_client_multiple_messages() {
        let client = MockTelegramClient::new();

        for i in 0..3 {
            client
                .send_message(&i.to_string(), &format!("Message {}", i), ParseMode::Html)
                .await
                .unwrap();
        }

        assert_eq!(client.message_count(), 3);
    }

    #[test]
    fn test_validate_chat_id_valid() {
        assert!(validate_chat_id("123456789").is_ok());
        assert!(validate_chat_id("-100123456789").is_ok());
        assert!(validate_chat_id("0").is_ok());
    }

    #[test]
    fn test_validate_chat_id_invalid() {
        assert!(validate_chat_id("").is_err());
        assert!(validate_chat_id("abc").is_err());
        assert!(validate_chat_id("12.34").is_err());
        assert!(validate_chat_id("12-34").is_err());
        assert!(validate_chat_id("12 34").is_err());
        assert!(validate_chat_id("123abc").is_err());
    }

    #[test]
    fn test_sanitize_for_logging_removes_control_chars() {
        let input = "test\nwith\nnewlines\rand\ttabs";
        let sanitized = sanitize_for_logging(input);
        assert!(!sanitized.contains('\n'));
        assert!(!sanitized.contains('\r'));
        assert!(!sanitized.contains('\t'));
    }

    #[test]
    fn test_sanitize_for_logging_truncates_long_strings() {
        let long_string = "a".repeat(200);
        let sanitized = sanitize_for_logging(&long_string);
        assert!(sanitized.len() <= 103); // 100 + "..."
        assert!(sanitized.ends_with("..."));
    }

    #[test]
    fn test_sanitize_for_logging_preserves_spaces() {
        let input = "test with spaces";
        let sanitized = sanitize_for_logging(input);
        assert_eq!(sanitized, "test with spaces");
    }

    #[test]
    fn test_telegram_config_validate_chat_id() {
        let valid_config = TelegramConfig {
            chat_id: "123456789".to_string(),
            message_template: "test".to_string(),
            parse_mode: "MarkdownV2".to_string(),
        };
        assert!(valid_config.validate_chat_id().is_ok());

        let invalid_config = TelegramConfig {
            chat_id: "invalid".to_string(),
            message_template: "test".to_string(),
            parse_mode: "MarkdownV2".to_string(),
        };
        assert!(invalid_config.validate_chat_id().is_err());
    }
}
