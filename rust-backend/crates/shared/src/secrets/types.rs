//! Common types for secrets management

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Secrets management errors
#[derive(Debug, Error)]
pub enum SecretsError {
    /// AWS Secrets Manager error
    #[error("AWS Secrets Manager error: {0}")]
    Aws(String),

    /// HashiCorp Vault error
    #[error("HashiCorp Vault error: {0}")]
    Vault(String),

    /// Secret not found
    #[error("Secret not found: {0}")]
    NotFound(String),

    /// Invalid secret value
    #[error("Invalid secret value: {0}")]
    InvalidValue(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Application secrets structure
///
/// Contains all sensitive credentials required by the application.
/// Secrets are organized into three tiers based on rotation frequency:
///
/// - **Tier 1 (Critical)**: Rotate quarterly
///   - Database credentials
///   - Redis credentials
///   - JWT signing key
///   - Payment processing keys
///
/// - **Tier 2 (Important)**: Rotate annually
///   - RPC endpoints
///   - API encryption keys
///   - Third-party service tokens
///
/// - **Tier 3 (Configuration)**: Non-secret, can stay in .env
///   - Public URLs
///   - Feature flags
///   - Domain names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSecrets {
    // Tier 1: Critical (rotate quarterly)
    /// PostgreSQL connection string
    ///
    /// Format: `postgresql://user:password@host:port/database`
    pub database_url: String,

    /// Redis connection string
    ///
    /// Format: `redis://[:password@]host:port`
    pub redis_url: String,

    /// JWT signing secret (minimum 256 bits)
    ///
    /// Used for signing and verifying JWT authentication tokens.
    /// Must be at least 32 characters for production use.
    pub jwt_secret: String,

    /// Stripe secret key for payment processing
    ///
    /// Format: `sk_live_xxx` (production) or `sk_test_xxx` (test)
    pub stripe_secret_key: String,

    /// Stripe webhook secret for signature verification
    ///
    /// Format: `whsec_xxx`
    pub stripe_webhook_secret: String,

    // Tier 2: Important (rotate annually)
    /// Ethereum Sepolia RPC URL
    ///
    /// Used for blockchain data retrieval and transaction submission.
    pub ethereum_sepolia_rpc_url: String,

    /// Base Sepolia RPC URL
    pub base_sepolia_rpc_url: String,

    /// Linea Sepolia RPC URL (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linea_sepolia_rpc_url: Option<String>,

    /// API key encryption key (Argon2id)
    ///
    /// Used for hashing API keys before storage.
    /// Base64-encoded 32-byte key.
    pub api_encryption_key: String,

    /// Telegram bot token (optional)
    ///
    /// Format: `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,
}

impl AppSecrets {
    /// Validate all secrets meet minimum security requirements
    ///
    /// # Errors
    ///
    /// Returns an error if any secret fails validation:
    /// - Empty values
    /// - JWT secret too short (<32 chars in production)
    /// - Invalid URL formats
    /// - Invalid key formats
    pub fn validate(&self) -> Result<(), SecretsError> {
        // Validate non-empty critical secrets
        if self.database_url.is_empty() {
            return Err(SecretsError::InvalidValue(
                "database_url cannot be empty".to_string(),
            ));
        }
        if self.redis_url.is_empty() {
            return Err(SecretsError::InvalidValue(
                "redis_url cannot be empty".to_string(),
            ));
        }
        if self.stripe_secret_key.is_empty() {
            return Err(SecretsError::InvalidValue(
                "stripe_secret_key cannot be empty".to_string(),
            ));
        }
        if self.stripe_webhook_secret.is_empty() {
            return Err(SecretsError::InvalidValue(
                "stripe_webhook_secret cannot be empty".to_string(),
            ));
        }

        // Validate JWT secret length (256 bits minimum)
        if !cfg!(debug_assertions) && self.jwt_secret.len() < 32 {
            return Err(SecretsError::InvalidValue(format!(
                "jwt_secret must be at least 32 characters (current: {})",
                self.jwt_secret.len()
            )));
        }

        // Validate URL formats
        if !self.database_url.starts_with("postgresql://")
            && !self.database_url.starts_with("postgres://")
        {
            return Err(SecretsError::InvalidValue(
                "database_url must start with postgresql:// or postgres://".to_string(),
            ));
        }
        if !self.redis_url.starts_with("redis://") && !self.redis_url.starts_with("rediss://") {
            return Err(SecretsError::InvalidValue(
                "redis_url must start with redis:// or rediss://".to_string(),
            ));
        }

        // Validate Stripe key formats
        if !self.stripe_secret_key.starts_with("sk_") {
            return Err(SecretsError::InvalidValue(
                "stripe_secret_key must start with sk_".to_string(),
            ));
        }
        if !self.stripe_webhook_secret.starts_with("whsec_") {
            return Err(SecretsError::InvalidValue(
                "stripe_webhook_secret must start with whsec_".to_string(),
            ));
        }

        Ok(())
    }

    /// Redact sensitive values for logging
    ///
    /// Returns a version of the secrets struct safe for logging,
    /// with all sensitive values masked.
    pub fn redacted(&self) -> RedactedSecrets {
        RedactedSecrets {
            database_url: Self::mask_connection_string(&self.database_url),
            redis_url: Self::mask_connection_string(&self.redis_url),
            jwt_secret: Self::mask_secret(&self.jwt_secret),
            stripe_secret_key: Self::mask_secret(&self.stripe_secret_key),
            stripe_webhook_secret: Self::mask_secret(&self.stripe_webhook_secret),
            ethereum_sepolia_rpc_url: self.ethereum_sepolia_rpc_url.clone(),
            base_sepolia_rpc_url: self.base_sepolia_rpc_url.clone(),
            linea_sepolia_rpc_url: self.linea_sepolia_rpc_url.clone(),
            api_encryption_key: Self::mask_secret(&self.api_encryption_key),
            telegram_bot_token: self
                .telegram_bot_token
                .as_ref()
                .map(|s| Self::mask_secret(s)),
        }
    }

    /// Mask a connection string by hiding the password
    fn mask_connection_string(url: &str) -> String {
        // Pattern: protocol://user:password@host:port/database
        if let Some(at_pos) = url.rfind('@') {
            if let Some(colon_pos) = url[..at_pos].rfind(':') {
                // Found password section
                let prefix = &url[..colon_pos + 1];
                let suffix = &url[at_pos..];
                return format!("{}****{}", prefix, suffix);
            }
        }
        url.to_string()
    }

    /// Mask a secret by showing only first 4 and last 4 characters
    fn mask_secret(secret: &str) -> String {
        if secret.len() <= 8 {
            return "****".to_string();
        }
        format!("{}****{}", &secret[..4], &secret[secret.len() - 4..])
    }
}

/// Redacted version of AppSecrets safe for logging
#[derive(Debug, Clone, Serialize)]
pub struct RedactedSecrets {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub ethereum_sepolia_rpc_url: String,
    pub base_sepolia_rpc_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linea_sepolia_rpc_url: Option<String>,
    pub api_encryption_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_database_url() {
        let mut secrets = create_valid_secrets();
        secrets.database_url = String::new();

        let result = secrets.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("database_url cannot be empty"));
    }

    #[test]
    fn test_validate_short_jwt_secret() {
        // Only enforced in release mode
        if !cfg!(debug_assertions) {
            let mut secrets = create_valid_secrets();
            secrets.jwt_secret = "short".to_string();

            let result = secrets.validate();
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_validate_invalid_database_url() {
        let mut secrets = create_valid_secrets();
        secrets.database_url = "http://wrong".to_string();

        let result = secrets.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_stripe_key() {
        let mut secrets = create_valid_secrets();
        secrets.stripe_secret_key = "invalid_key".to_string();

        let result = secrets.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_redact_secrets() {
        let secrets = create_valid_secrets();
        let redacted = secrets.redacted();

        // Database URL should mask password
        assert!(redacted.database_url.contains("****"));
        assert!(!redacted.database_url.contains("mypassword"));

        // JWT secret should be masked
        assert!(redacted.jwt_secret.contains("****"));
        assert_ne!(redacted.jwt_secret, secrets.jwt_secret);
    }

    #[test]
    fn test_mask_connection_string() {
        let url = "postgresql://user:mypassword@localhost:5432/db";
        let masked = AppSecrets::mask_connection_string(url);
        assert_eq!(masked, "postgresql://user:****@localhost:5432/db");
    }

    #[test]
    fn test_mask_secret() {
        let secret = "sk_live_1234567890abcdef";
        let masked = AppSecrets::mask_secret(secret);
        assert_eq!(masked, "sk_l****cdef");
    }

    fn create_valid_secrets() -> AppSecrets {
        AppSecrets {
            database_url: "postgresql://user:mypassword@localhost:5432/db".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            jwt_secret: "a_very_long_and_secure_jwt_secret_key_with_32_plus_chars".to_string(),
            stripe_secret_key: "sk_test_1234567890".to_string(),
            stripe_webhook_secret: "whsec_1234567890".to_string(),
            ethereum_sepolia_rpc_url: "https://eth-sepolia.example.com".to_string(),
            base_sepolia_rpc_url: "https://base-sepolia.example.com".to_string(),
            linea_sepolia_rpc_url: Some("https://linea-sepolia.example.com".to_string()),
            api_encryption_key: "base64encodedkey12345678901234567890".to_string(),
            telegram_bot_token: Some("123456:ABC-DEF1234ghIkl".to_string()),
        }
    }
}
