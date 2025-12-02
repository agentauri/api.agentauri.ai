//! HashiCorp Vault integration for secure credential management.
//!
//! This module provides integration with HashiCorp Vault for production
//! secret storage with enterprise features:
//!
//! - Dynamic secrets generation
//! - Automatic secret rotation
//! - Encryption as a service
//! - Detailed audit logging
//! - Fine-grained access control policies
//! - Multi-cloud and hybrid support
//! - In-memory caching with configurable TTL
//!
//! # Prerequisites
//!
//! - Vault server running and accessible
//! - Valid authentication token
//! - KV secrets engine v2 mounted at `secret/`
//! - Policies configured for read access
//!
//! # Configuration
//!
//! Environment variables:
//! - `VAULT_ADDR`: Vault server address (e.g., https://vault.example.com:8200)
//! - `VAULT_TOKEN`: Authentication token
//! - `VAULT_NAMESPACE`: Namespace (optional, Vault Enterprise only)
//! - `SECRETS_CACHE_TTL_SECONDS`: Cache TTL in seconds (default: 3600)
//!
//! # Secret Path Convention
//!
//! All secrets are stored under `secret/data/erc8004/` path:
//! - `secret/data/erc8004/database_url`
//! - `secret/data/erc8004/redis_url`
//! - `secret/data/erc8004/jwt_secret`
//! - etc.
//!
//! # Usage
//!
//! ```no_run
//! use shared::secrets::vault::SecretsManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = SecretsManager::new().await?;
//!     let secrets = manager.get_app_secrets().await?;
//!     println!("Database URL: {}", secrets.database_url);
//!     Ok(())
//! }
//! ```

use crate::secrets::types::{AppSecrets, SecretsError};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// Mock Vault client for compilation (real implementation requires vaultrs crate)
#[cfg(not(feature = "vault-secrets"))]
mod mock_vault {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[allow(dead_code)]
    pub struct VaultClient;

    impl VaultClient {
        #[allow(dead_code)]
        pub async fn new(_addr: &str, _token: &str) -> Result<Self, String> {
            Ok(Self)
        }

        #[allow(dead_code)]
        pub async fn get_secret(&self, _path: &str) -> Result<SecretData, String> {
            Err("Vault feature not enabled".to_string())
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecretData {
        pub data: HashMap<String, String>,
    }
}

#[cfg(not(feature = "vault-secrets"))]
use mock_vault::VaultClient;

// Real Vault client when feature is enabled
#[cfg(feature = "vault-secrets")]
use vaultrs::{client::VaultClient, kv2};

/// Cached secret entry with expiration
#[derive(Debug, Clone)]
struct CachedSecret {
    value: String,
    expires_at: Instant,
}

impl CachedSecret {
    #[allow(dead_code)]
    fn new(value: String, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// HashiCorp Vault secrets manager
pub struct SecretsManager {
    #[allow(dead_code)]
    client: VaultClient,
    cache: Arc<RwLock<HashMap<String, CachedSecret>>>,
    #[allow(dead_code)]
    cache_ttl: Duration,
    #[allow(dead_code)]
    mount_path: String,
}

impl SecretsManager {
    /// Create new Vault secrets manager client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - VAULT_ADDR is not set
    /// - VAULT_TOKEN is not set
    /// - Connection to Vault fails
    pub async fn new() -> Result<Self, SecretsError> {
        Self::with_cache_ttl(Duration::from_secs(3600)).await
    }

    /// Create new Vault secrets manager with custom cache TTL
    ///
    /// # Arguments
    ///
    /// * `ttl` - Cache time-to-live duration
    ///
    /// # Errors
    ///
    /// Returns an error if Vault configuration is invalid or connection fails.
    pub async fn with_cache_ttl(ttl: Duration) -> Result<Self, SecretsError> {
        let vault_addr = env::var("VAULT_ADDR")
            .map_err(|_| SecretsError::Config("VAULT_ADDR must be set".to_string()))?;

        let vault_token = env::var("VAULT_TOKEN")
            .map_err(|_| SecretsError::Config("VAULT_TOKEN must be set".to_string()))?;

        #[cfg(feature = "vault-secrets")]
        let client = {
            // Build settings with optional namespace
            let settings = if let Ok(namespace) = env::var("VAULT_NAMESPACE") {
                vaultrs::client::VaultClientSettingsBuilder::default()
                    .address(vault_addr)
                    .token(vault_token)
                    .namespace(Some(namespace))
                    .build()
            } else {
                vaultrs::client::VaultClientSettingsBuilder::default()
                    .address(vault_addr)
                    .token(vault_token)
                    .build()
            }
            .map_err(|e| SecretsError::Config(format!("Failed to build Vault client: {}", e)))?;

            VaultClient::new(settings)
                .map_err(|e| SecretsError::Vault(format!("Failed to create Vault client: {}", e)))?
        };

        #[cfg(not(feature = "vault-secrets"))]
        let client = VaultClient::new(&vault_addr, &vault_token)
            .await
            .map_err(SecretsError::Vault)?;

        Ok(Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: ttl,
            mount_path: "secret".to_string(), // Default KV v2 mount
        })
    }

    /// Get secret value from Vault with caching
    ///
    /// # Arguments
    ///
    /// * `secret_name` - Name of the secret (e.g., "database_url")
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - Vault API call fails
    /// - Token permissions are insufficient
    pub async fn get_secret(&self, secret_name: &str) -> Result<String, SecretsError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(secret_name) {
                if !cached.is_expired() {
                    tracing::debug!("Cache hit for secret: {}", secret_name);
                    return Ok(cached.value.clone());
                }
            }
        }

        // Fetch from Vault
        tracing::debug!("Fetching secret from Vault: {}", secret_name);

        let _path = format!("erc8004/{}", secret_name);

        #[cfg(feature = "vault-secrets")]
        let secret_data: HashMap<String, String> =
            kv2::read(&self.client, &self.mount_path, &_path)
                .await
                .map_err(|e| {
                    SecretsError::Vault(format!("Failed to read secret {}: {}", _path, e))
                })?;

        #[cfg(not(feature = "vault-secrets"))]
        {
            tracing::warn!(
                "Vault secrets feature not enabled - returning error for {}",
                secret_name
            );
            Err(SecretsError::Config(
                "Vault secrets feature not enabled. Add 'vault-secrets' feature to Cargo.toml"
                    .to_string(),
            ))
        }

        #[cfg(feature = "vault-secrets")]
        let secret_value = secret_data
            .get("value")
            .ok_or_else(|| {
                SecretsError::NotFound(format!(
                    "Secret {} does not contain 'value' field",
                    secret_name
                ))
            })?
            .clone();

        // Update cache
        #[cfg(feature = "vault-secrets")]
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                secret_name.to_string(),
                CachedSecret::new(secret_value.clone(), self.cache_ttl),
            );
        }

        #[cfg(feature = "vault-secrets")]
        Ok(secret_value)
    }

    /// Get all application secrets
    ///
    /// This method fetches all required secrets for the application from
    /// HashiCorp Vault using the `secret/data/erc8004/` path convention.
    ///
    /// # Errors
    ///
    /// Returns an error if any required secret is missing or inaccessible.
    pub async fn get_app_secrets(&self) -> Result<AppSecrets, SecretsError> {
        // Fetch all secrets in parallel for performance
        let (
            database_url,
            redis_url,
            jwt_secret,
            stripe_secret_key,
            stripe_webhook_secret,
            ethereum_sepolia_rpc_url,
            base_sepolia_rpc_url,
            linea_sepolia_rpc_url,
            api_encryption_key,
            telegram_bot_token,
        ) = tokio::try_join!(
            self.get_secret("database_url"),
            self.get_secret("redis_url"),
            self.get_secret("jwt_secret"),
            self.get_secret("stripe_secret_key"),
            self.get_secret("stripe_webhook_secret"),
            self.get_secret("ethereum_sepolia_rpc_url"),
            self.get_secret("base_sepolia_rpc_url"),
            self.get_secret_optional("linea_sepolia_rpc_url"),
            self.get_secret("api_encryption_key"),
            self.get_secret_optional("telegram_bot_token"),
        )?;

        let secrets = AppSecrets {
            database_url,
            redis_url,
            jwt_secret,
            stripe_secret_key,
            stripe_webhook_secret,
            ethereum_sepolia_rpc_url,
            base_sepolia_rpc_url,
            linea_sepolia_rpc_url,
            api_encryption_key,
            telegram_bot_token,
        };

        // Validate all secrets
        secrets.validate()?;

        Ok(secrets)
    }

    /// Get optional secret (returns None if not found instead of error)
    async fn get_secret_optional(&self, secret_name: &str) -> Result<Option<String>, SecretsError> {
        match self.get_secret(secret_name).await {
            Ok(value) => Ok(Some(value)),
            Err(SecretsError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Invalidate cache for a specific secret
    ///
    /// Forces the next get_secret() call to fetch from Vault.
    pub async fn invalidate_secret(&self, secret_name: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(secret_name);
        tracing::debug!("Invalidated cache for secret: {}", secret_name);
    }

    /// Invalidate entire cache
    ///
    /// Forces all subsequent get_secret() calls to fetch from Vault.
    pub async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::debug!("Invalidated all cached secrets");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|v| v.is_expired()).count();

        CacheStats {
            total_entries: total,
            expired_entries: expired,
            valid_entries: total - expired,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_expiration() {
        // Create cached entry with 1 second TTL
        let ttl = Duration::from_secs(1);
        let cached = CachedSecret::new("test_value".to_string(), ttl);

        assert!(!cached.is_expired());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        assert!(cached.is_expired());
    }

    #[test]
    fn test_vault_config_missing() {
        // Clear environment
        env::remove_var("VAULT_ADDR");
        env::remove_var("VAULT_TOKEN");

        // Should fail without VAULT_ADDR
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(SecretsManager::new());

        assert!(result.is_err());
    }
}
