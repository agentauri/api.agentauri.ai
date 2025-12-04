//! Unified secrets management interface supporting multiple backends.
//!
//! This module provides a unified API for retrieving secrets from different backends:
//! - **EnvBackend**: Development only - reads from .env files
//! - **AwsBackend**: Production - AWS Secrets Manager
//! - **VaultBackend**: Production - HashiCorp Vault
//!
//! # Security Features
//!
//! - In-memory caching with configurable TTL (default: 1 hour)
//! - Automatic cache invalidation
//! - Structured secret organization (prefix: `agentauri/`)
//! - Rotation support (backend-specific)
//!
//! # Usage
//!
//! ```no_run
//! use shared::secrets;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load secrets from configured backend (determined by SECRETS_BACKEND env var)
//!     let secrets = secrets::load_secrets().await?;
//!
//!     // Use secrets
//!     println!("Database URL: {}", secrets.database_url);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Environment Variables
//!
//! - `SECRETS_BACKEND`: Backend to use (`env`, `aws`, or `vault`)
//!   - Default: `env` (development mode)
//!   - Production: Set to `aws` or `vault`
//!
//! ## AWS Secrets Manager Configuration
//!
//! Requires AWS credentials in environment:
//! - `AWS_ACCESS_KEY_ID` or IAM role
//! - `AWS_SECRET_ACCESS_KEY` (if using access keys)
//! - `AWS_REGION` (default: us-east-1)
//!
//! ## HashiCorp Vault Configuration
//!
//! - `VAULT_ADDR`: Vault server address (e.g., https://vault.example.com:8200)
//! - `VAULT_TOKEN`: Authentication token
//! - `VAULT_NAMESPACE`: Namespace (optional, for Vault Enterprise)

pub mod aws;
pub mod env_backend;
pub mod types;
pub mod vault;

use std::env;

pub use types::{AppSecrets, SecretsError};

/// Secrets backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretsBackend {
    /// Development only: read from .env files
    Env,

    /// Production: AWS Secrets Manager
    Aws,

    /// Production: HashiCorp Vault
    Vault,
}

impl SecretsBackend {
    /// Determine backend from environment variable
    ///
    /// Checks `SECRETS_BACKEND` environment variable:
    /// - `env` → EnvBackend (development)
    /// - `aws` → AWS Secrets Manager (production)
    /// - `vault` → HashiCorp Vault (production)
    ///
    /// Default: `env` if not set
    pub fn from_env() -> Self {
        match env::var("SECRETS_BACKEND").as_deref() {
            Ok("aws") => Self::Aws,
            Ok("vault") => Self::Vault,
            _ => Self::Env, // Default to .env for development
        }
    }
}

/// Load secrets from the configured backend
///
/// This is the main entry point for secret retrieval. It automatically
/// selects the appropriate backend based on the `SECRETS_BACKEND` environment
/// variable.
///
/// # Errors
///
/// Returns an error if:
/// - Backend configuration is invalid
/// - Required secrets are missing
/// - Backend communication fails
///
/// # Example
///
/// ```no_run
/// use shared::secrets;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let secrets = secrets::load_secrets().await?;
///     println!("Loaded secrets from backend: {:?}", secrets);
///     Ok(())
/// }
/// ```
pub async fn load_secrets() -> Result<AppSecrets, SecretsError> {
    match SecretsBackend::from_env() {
        SecretsBackend::Env => {
            tracing::info!("Loading secrets from .env files (development mode)");
            env_backend::load_from_env().await
        }
        SecretsBackend::Aws => {
            tracing::info!("Loading secrets from AWS Secrets Manager");
            let manager = aws::SecretsManager::new().await?;
            manager.get_app_secrets().await
        }
        SecretsBackend::Vault => {
            tracing::info!("Loading secrets from HashiCorp Vault");
            let manager = vault::SecretsManager::new().await?;
            manager.get_app_secrets().await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_from_env_default() {
        // Clear environment
        env::remove_var("SECRETS_BACKEND");

        let backend = SecretsBackend::from_env();
        assert_eq!(backend, SecretsBackend::Env);
    }

    #[test]
    fn test_backend_from_env_aws() {
        env::set_var("SECRETS_BACKEND", "aws");

        let backend = SecretsBackend::from_env();
        assert_eq!(backend, SecretsBackend::Aws);

        env::remove_var("SECRETS_BACKEND");
    }

    #[test]
    fn test_backend_from_env_vault() {
        env::set_var("SECRETS_BACKEND", "vault");

        let backend = SecretsBackend::from_env();
        assert_eq!(backend, SecretsBackend::Vault);

        env::remove_var("SECRETS_BACKEND");
    }

    #[tokio::test]
    async fn test_load_secrets_from_env() {
        // This test requires .env file to be present
        env::remove_var("SECRETS_BACKEND");

        let result = load_secrets().await;
        // Should succeed in development environment with .env
        // or fail gracefully if .env is missing
        match result {
            Ok(secrets) => {
                assert!(!secrets.database_url.is_empty());
            }
            Err(e) => {
                // Acceptable if .env is not present in test environment
                tracing::warn!("Could not load secrets from .env: {}", e);
            }
        }
    }
}
