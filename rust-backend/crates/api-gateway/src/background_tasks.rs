//! Background Tasks for API Gateway
//!
//! This module provides background tasks that run periodically for maintenance:
//!
//! - **Nonce Cleanup**: Removes expired wallet authentication nonces
//! - **OAuth Token Cleanup**: Removes expired OAuth access and refresh tokens
//! - **Session Cleanup**: (Future) Clean up expired sessions
//! - **Rate Limit Cleanup**: (Future) Clean up old rate limit counters
//!
//! # Usage
//!
//! ```ignore
//! use api_gateway::background_tasks::BackgroundTaskRunner;
//!
//! let runner = BackgroundTaskRunner::new(db_pool);
//! let shutdown_token = runner.start();
//!
//! // When shutting down:
//! shutdown_token.cancel();
//! ```
//!
//! # Configuration
//!
//! Environment variables:
//! - `NONCE_CLEANUP_INTERVAL_SECS`: Interval between nonce cleanups (default: 3600 = 1 hour)
//! - `OAUTH_TOKEN_CLEANUP_INTERVAL_SECS`: Interval between OAuth token cleanups (default: 3600 = 1 hour)
//!
//! Rust guideline compliant 2025-01-28

use shared::DbPool;
use std::env;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::repositories::oauth::OAuthTokenRepository;
use crate::repositories::wallet::NonceRepository;

/// Default interval for nonce cleanup (1 hour)
const DEFAULT_NONCE_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Minimum interval for nonce cleanup (5 minutes)
const MIN_NONCE_CLEANUP_INTERVAL_SECS: u64 = 300;

/// Default interval for OAuth token cleanup (1 hour)
const DEFAULT_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Minimum interval for OAuth token cleanup (5 minutes)
const MIN_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS: u64 = 300;

/// Background task configuration
#[derive(Debug, Clone)]
pub struct BackgroundTaskConfig {
    /// Interval between nonce cleanups
    pub nonce_cleanup_interval: Duration,
    /// Interval between OAuth token cleanups
    pub oauth_token_cleanup_interval: Duration,
}

impl Default for BackgroundTaskConfig {
    fn default() -> Self {
        let nonce_interval_secs = env::var("NONCE_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_NONCE_CLEANUP_INTERVAL_SECS)
            .max(MIN_NONCE_CLEANUP_INTERVAL_SECS);

        let oauth_token_interval_secs = env::var("OAUTH_TOKEN_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS)
            .max(MIN_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS);

        Self {
            nonce_cleanup_interval: Duration::from_secs(nonce_interval_secs),
            oauth_token_cleanup_interval: Duration::from_secs(oauth_token_interval_secs),
        }
    }
}

/// Background task runner
///
/// Manages periodic background tasks for the API Gateway.
pub struct BackgroundTaskRunner {
    pool: DbPool,
    config: BackgroundTaskConfig,
}

impl BackgroundTaskRunner {
    /// Create a new background task runner
    pub fn new(pool: DbPool) -> Self {
        Self::with_config(pool, BackgroundTaskConfig::default())
    }

    /// Create a new background task runner with custom configuration
    pub fn with_config(pool: DbPool, config: BackgroundTaskConfig) -> Self {
        Self { pool, config }
    }

    /// Start all background tasks
    ///
    /// Returns a cancellation token that can be used to stop all tasks.
    pub fn start(self) -> CancellationToken {
        let cancel_token = CancellationToken::new();

        // Start nonce cleanup task
        let nonce_token = cancel_token.clone();
        let nonce_pool = self.pool.clone();
        let nonce_interval = self.config.nonce_cleanup_interval;

        tokio::spawn(async move {
            run_nonce_cleanup(nonce_pool, nonce_interval, nonce_token).await;
        });

        // Start OAuth token cleanup task
        let oauth_token = cancel_token.clone();
        let oauth_pool = self.pool.clone();
        let oauth_interval = self.config.oauth_token_cleanup_interval;

        tokio::spawn(async move {
            run_oauth_token_cleanup(oauth_pool, oauth_interval, oauth_token).await;
        });

        info!(
            nonce_cleanup_interval_secs = ?self.config.nonce_cleanup_interval.as_secs(),
            oauth_token_cleanup_interval_secs = ?self.config.oauth_token_cleanup_interval.as_secs(),
            "Background tasks started"
        );

        cancel_token
    }
}

/// Run the nonce cleanup task
async fn run_nonce_cleanup(
    pool: DbPool,
    cleanup_interval: Duration,
    cancel_token: CancellationToken,
) {
    let mut interval = interval(cleanup_interval);

    // Skip the first tick (which fires immediately)
    interval.tick().await;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Nonce cleanup task stopping due to shutdown");
                break;
            }
            _ = interval.tick() => {
                cleanup_expired_nonces(&pool).await;
            }
        }
    }
}

/// Perform the actual nonce cleanup
async fn cleanup_expired_nonces(pool: &DbPool) {
    debug!("Starting nonce cleanup");

    match NonceRepository::cleanup_expired_nonces(pool).await {
        Ok(count) => {
            if count > 0 {
                info!(deleted_count = count, "Cleaned up expired nonces");
            } else {
                debug!("No expired nonces to clean up");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to cleanup expired nonces");
        }
    }
}

/// Standalone function to run nonce cleanup once
///
/// Useful for manual cleanup or testing.
#[allow(dead_code)] // Used in tests and manual cleanup scripts
pub async fn cleanup_nonces_once(pool: &DbPool) -> Result<u64, anyhow::Error> {
    NonceRepository::cleanup_expired_nonces(pool).await
}

/// Run the OAuth token cleanup task
async fn run_oauth_token_cleanup(
    pool: DbPool,
    cleanup_interval: Duration,
    cancel_token: CancellationToken,
) {
    let mut interval = interval(cleanup_interval);

    // Skip the first tick (which fires immediately)
    interval.tick().await;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("OAuth token cleanup task stopping due to shutdown");
                break;
            }
            _ = interval.tick() => {
                cleanup_expired_oauth_tokens(&pool).await;
            }
        }
    }
}

/// Perform the actual OAuth token cleanup
async fn cleanup_expired_oauth_tokens(pool: &DbPool) {
    debug!("Starting OAuth token cleanup");

    match OAuthTokenRepository::cleanup_expired_tokens(pool).await {
        Ok(count) => {
            if count > 0 {
                info!(deleted_count = count, "Cleaned up expired OAuth tokens");
            } else {
                debug!("No expired OAuth tokens to clean up");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to cleanup expired OAuth tokens");
        }
    }
}

/// Standalone function to run OAuth token cleanup once
///
/// Useful for manual cleanup or testing.
#[allow(dead_code)] // Used in tests and manual cleanup scripts
pub async fn cleanup_oauth_tokens_once(pool: &DbPool) -> Result<u64, anyhow::Error> {
    OAuthTokenRepository::cleanup_expired_tokens(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BackgroundTaskConfig::default();
        assert!(
            config.nonce_cleanup_interval >= Duration::from_secs(MIN_NONCE_CLEANUP_INTERVAL_SECS)
        );
    }

    #[test]
    fn test_config_respects_minimum() {
        // Even if env var is set to something lower, it should use minimum
        // The constant MIN_NONCE_CLEANUP_INTERVAL_SECS is enforced in BackgroundTaskConfig::default()
        // This test verifies that the default config uses at least the minimum
        let config = BackgroundTaskConfig::default();
        assert!(
            config.nonce_cleanup_interval >= Duration::from_secs(MIN_NONCE_CLEANUP_INTERVAL_SECS)
        );
    }

    #[test]
    fn test_cancellation_token() {
        // Verify cancellation token can be created and used
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }
}
