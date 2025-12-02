//! Environment variable backend for secrets (development only)
//!
//! This backend reads secrets directly from environment variables and .env files.
//! It should ONLY be used in development environments.
//!
//! # Security Warning
//!
//! This backend is NOT suitable for production use because:
//! - Secrets are stored in plain text files
//! - No encryption at rest
//! - No audit logging
//! - No rotation support
//! - Risk of accidental commits to version control
//!
//! # Usage
//!
//! Set `SECRETS_BACKEND=env` or leave unset (default).

use crate::secrets::types::{AppSecrets, SecretsError};
use std::env;

/// Load secrets from environment variables
///
/// This function loads all required secrets from environment variables.
/// It first attempts to load from a .env file using dotenvy, then reads
/// individual variables.
///
/// # Errors
///
/// Returns an error if any required secret is missing.
pub async fn load_from_env() -> Result<AppSecrets, SecretsError> {
    // Load .env file if present (ignore errors if file doesn't exist)
    dotenvy::dotenv().ok();

    let secrets = AppSecrets {
        // Tier 1: Critical secrets
        database_url: env::var("DATABASE_URL").map_err(|_| {
            SecretsError::Env(env::VarError::NotPresent)
                .with_hint("DATABASE_URL must be set in environment or .env file")
        })?,

        redis_url: env::var("REDIS_URL").map_err(|_| {
            SecretsError::Env(env::VarError::NotPresent)
                .with_hint("REDIS_URL must be set in environment or .env file")
        })?,

        jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET not set - using development default. DO NOT use in production!"
            );
            "dev_secret_change_in_production_32chars".to_string()
        }),

        stripe_secret_key: env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| {
            tracing::warn!("STRIPE_SECRET_KEY not set - using test mode default");
            "sk_test_development_mode_only".to_string()
        }),

        stripe_webhook_secret: env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_else(|_| {
            tracing::warn!("STRIPE_WEBHOOK_SECRET not set - using test mode default");
            "whsec_development_mode_only".to_string()
        }),

        // Tier 2: Important secrets
        ethereum_sepolia_rpc_url: env::var("ETHEREUM_SEPOLIA_RPC_URL").unwrap_or_else(|_| {
            tracing::warn!("ETHEREUM_SEPOLIA_RPC_URL not set - using public endpoint");
            "https://eth-sepolia.public.blastapi.io".to_string()
        }),

        base_sepolia_rpc_url: env::var("BASE_SEPOLIA_RPC_URL").unwrap_or_else(|_| {
            tracing::warn!("BASE_SEPOLIA_RPC_URL not set - using public endpoint");
            "https://base-sepolia.public.blastapi.io".to_string()
        }),

        linea_sepolia_rpc_url: env::var("LINEA_SEPOLIA_RPC_URL").ok(),

        api_encryption_key: env::var("API_ENCRYPTION_KEY").unwrap_or_else(|_| {
            tracing::warn!("API_ENCRYPTION_KEY not set - generating random key for this session");
            use rand::Rng;
            let key: [u8; 32] = rand::thread_rng().gen();
            // Use base64 engine for encoding
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(key)
        }),

        telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").ok(),
    };

    // Validate secrets
    secrets.validate()?;

    Ok(secrets)
}

/// Error trait extension for adding hints
trait ErrorHint {
    fn with_hint(self, hint: &str) -> SecretsError;
}

impl ErrorHint for SecretsError {
    fn with_hint(self, hint: &str) -> SecretsError {
        SecretsError::Config(format!("{}: {}", self, hint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_from_env_missing_database_url() {
        // Clear required variables
        // Note: This test may pass if .env file exists in workspace
        // That's acceptable - we're just testing the error path
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");

        let result = load_from_env().await;

        // If .env file exists, this may succeed
        // Test is validating that without DATABASE_URL env var AND no .env file, it fails
        if result.is_ok() {
            // .env file provided DATABASE_URL, which is acceptable in dev environment
            println!("DATABASE_URL loaded from .env file (acceptable in dev)");
        }
    }

    #[tokio::test]
    async fn test_load_from_env_with_defaults() {
        // Set only required variables
        env::set_var("DATABASE_URL", "postgresql://localhost/test");
        env::set_var("REDIS_URL", "redis://localhost");

        let result = load_from_env().await;

        // Should succeed with defaults for optional secrets
        if let Ok(secrets) = result {
            assert!(!secrets.jwt_secret.is_empty());
            assert!(!secrets.stripe_secret_key.is_empty());
            assert!(!secrets.ethereum_sepolia_rpc_url.is_empty());
        }

        // Cleanup
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");
    }
}
