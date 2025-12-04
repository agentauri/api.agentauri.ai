//! AWS Secrets Manager integration for secure credential management.
//!
//! This module provides integration with AWS Secrets Manager for production
//! secret storage with enterprise features:
//!
//! - Automatic encryption at rest (AES-256 with AWS KMS)
//! - Automatic rotation support via Lambda functions
//! - Audit logging via AWS CloudTrail
//! - IAM-based access control
//! - Regional replication
//! - In-memory caching with configurable TTL
//!
//! # Prerequisites
//!
//! - AWS credentials configured (IAM role or access keys)
//! - AWS region set (default: us-east-1)
//! - Required IAM permissions:
//!   - `secretsmanager:GetSecretValue`
//!   - `kms:Decrypt` (if using custom KMS key)
//!
//! # Configuration
//!
//! Environment variables:
//! - `AWS_REGION`: AWS region (default: us-east-1)
//! - `AWS_ACCESS_KEY_ID`: Access key (optional, use IAM role if possible)
//! - `AWS_SECRET_ACCESS_KEY`: Secret key (optional)
//! - `SECRETS_CACHE_TTL_SECONDS`: Cache TTL in seconds (default: 3600)
//!
//! # Secret Naming Convention
//!
//! All secrets are prefixed with `agentauri/` for organization:
//! - `agentauri/database_url`
//! - `agentauri/redis_url`
//! - `agentauri/jwt_secret`
//! - etc.
//!
//! # Usage
//!
//! ```no_run
//! use shared::secrets::aws::SecretsManager;
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
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// Mock AWS SDK types for compilation (real implementation requires aws-sdk-secretsmanager)
#[cfg(not(feature = "aws-secrets"))]
mod mock_aws {
    #[allow(dead_code)]
    pub struct Client;
    #[allow(dead_code)]
    pub struct Error;

    impl Client {
        #[allow(dead_code)]
        pub async fn get_secret_value(&self) -> GetSecretValueBuilder {
            GetSecretValueBuilder
        }
    }

    #[allow(dead_code)]
    pub struct GetSecretValueBuilder;

    impl GetSecretValueBuilder {
        #[allow(dead_code)]
        pub fn secret_id(self, _: &str) -> Self {
            self
        }

        #[allow(dead_code)]
        pub async fn send(self) -> Result<GetSecretValueOutput, Error> {
            Err(Error)
        }
    }

    #[allow(dead_code)]
    pub struct GetSecretValueOutput {
        pub secret_string: Option<String>,
    }

    impl GetSecretValueOutput {
        #[allow(dead_code)]
        pub fn secret_string(&self) -> Option<&str> {
            self.secret_string.as_deref()
        }
    }
}

#[cfg(not(feature = "aws-secrets"))]
use mock_aws::Client;

// Real AWS SDK types when feature is enabled
#[cfg(feature = "aws-secrets")]
use aws_sdk_secretsmanager::{
    error::SdkError, operation::get_secret_value::GetSecretValueError, Client,
};
#[cfg(feature = "aws-secrets")]
#[allow(dead_code)]
type AwsError = SdkError<GetSecretValueError>;

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

/// AWS Secrets Manager client wrapper
pub struct SecretsManager {
    #[allow(dead_code)]
    client: Client,
    cache: Arc<RwLock<HashMap<String, CachedSecret>>>,
    #[allow(dead_code)]
    cache_ttl: Duration,
}

impl SecretsManager {
    /// Create new secrets manager client
    ///
    /// # Errors
    ///
    /// Returns an error if AWS SDK initialization fails.
    pub async fn new() -> Result<Self, SecretsError> {
        Self::with_cache_ttl(Duration::from_secs(3600)).await
    }

    /// Create new secrets manager client with custom cache TTL
    ///
    /// # Arguments
    ///
    /// * `ttl` - Cache time-to-live duration
    ///
    /// # Errors
    ///
    /// Returns an error if AWS SDK initialization fails.
    pub async fn with_cache_ttl(ttl: Duration) -> Result<Self, SecretsError> {
        #[cfg(feature = "aws-secrets")]
        let client = {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .load()
                .await;
            aws_sdk_secretsmanager::Client::new(&config)
        };

        #[cfg(not(feature = "aws-secrets"))]
        let client = Client;

        Ok(Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: ttl,
        })
    }

    /// Get secret value by name with caching
    ///
    /// # Arguments
    ///
    /// * `secret_name` - Name of the secret in AWS Secrets Manager
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Secret does not exist
    /// - AWS SDK call fails
    /// - IAM permissions are insufficient
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

        // Fetch from AWS
        tracing::debug!("Fetching secret from AWS: {}", secret_name);

        #[cfg(feature = "aws-secrets")]
        let response = self
            .client
            .get_secret_value()
            .secret_id(secret_name)
            .send()
            .await
            .map_err(|e| {
                SecretsError::Aws(format!("Failed to get secret {}: {}", secret_name, e))
            })?;

        #[cfg(not(feature = "aws-secrets"))]
        {
            tracing::warn!(
                "AWS Secrets Manager feature not enabled - returning mock value for {}",
                secret_name
            );
            Err(SecretsError::Config(
                "AWS Secrets Manager feature not enabled. Add 'aws-secrets' feature to Cargo.toml"
                    .to_string(),
            ))
        }

        #[cfg(feature = "aws-secrets")]
        let secret_value = response
            .secret_string()
            .ok_or_else(|| SecretsError::NotFound(secret_name.to_string()))?
            .to_string();

        // Update cache
        #[cfg(feature = "aws-secrets")]
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                secret_name.to_string(),
                CachedSecret::new(secret_value.clone(), self.cache_ttl),
            );
        }

        #[cfg(feature = "aws-secrets")]
        Ok(secret_value)
    }

    /// Get all application secrets
    ///
    /// This method fetches all required secrets for the application from
    /// AWS Secrets Manager using the `agentauri/` prefix naming convention.
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
            self.get_secret("agentauri/database_url"),
            self.get_secret("agentauri/redis_url"),
            self.get_secret("agentauri/jwt_secret"),
            self.get_secret("agentauri/stripe_secret_key"),
            self.get_secret("agentauri/stripe_webhook_secret"),
            self.get_secret("agentauri/ethereum_sepolia_rpc_url"),
            self.get_secret("agentauri/base_sepolia_rpc_url"),
            self.get_secret_optional("agentauri/linea_sepolia_rpc_url"),
            self.get_secret("agentauri/api_encryption_key"),
            self.get_secret_optional("agentauri/telegram_bot_token"),
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
    /// Forces the next get_secret() call to fetch from AWS.
    pub async fn invalidate_secret(&self, secret_name: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(secret_name);
        tracing::debug!("Invalidated cache for secret: {}", secret_name);
    }

    /// Invalidate entire cache
    ///
    /// Forces all subsequent get_secret() calls to fetch from AWS.
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
        // Create manager with 1 second TTL
        let ttl = Duration::from_secs(1);
        let cached = CachedSecret::new("test_value".to_string(), ttl);

        assert!(!cached.is_expired());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        assert!(cached.is_expired());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let manager = SecretsManager::new().await.unwrap();

        // Manually insert a cached value
        {
            let mut cache = manager.cache.write().await;
            cache.insert(
                "test_secret".to_string(),
                CachedSecret::new("test_value".to_string(), Duration::from_secs(3600)),
            );
        }

        // Verify cached
        {
            let stats = manager.cache_stats().await;
            assert_eq!(stats.total_entries, 1);
        }

        // Invalidate
        manager.invalidate_secret("test_secret").await;

        // Verify removed
        {
            let stats = manager.cache_stats().await;
            assert_eq!(stats.total_entries, 0);
        }
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let manager = SecretsManager::new().await.unwrap();

        // Add some entries (one expired, one valid)
        {
            let mut cache = manager.cache.write().await;
            cache.insert(
                "valid".to_string(),
                CachedSecret::new("value1".to_string(), Duration::from_secs(3600)),
            );
            cache.insert(
                "expired".to_string(),
                CachedSecret::new("value2".to_string(), Duration::from_secs(0)),
            );
        }

        // Wait a moment to ensure expiration
        tokio::time::sleep(Duration::from_millis(10)).await;

        let stats = manager.cache_stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.expired_entries, 1);
        assert_eq!(stats.valid_entries, 1);
    }
}
