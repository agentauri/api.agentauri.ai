//! Background Tasks for API Gateway
//!
//! This module provides background tasks that run periodically for maintenance:
//!
//! - **Nonce Cleanup**: Removes expired wallet authentication nonces
//! - **OAuth Token Cleanup**: Removes expired OAuth access and refresh tokens
//! - **OAuth Temp Codes Cleanup**: Removes expired/used OAuth authorization codes
//! - **Payment Nonce Cleanup**: Removes expired payment idempotency keys
//! - **Auth Failures Cleanup**: Removes old authentication failure records (30 days retention)
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
//! - `AUTH_FAILURES_RETENTION_DAYS`: Days to retain auth failure records (default: 30)
//!
//! Rust guideline compliant 2025-12-02

use shared::DbPool;
use std::env;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::repositories::oauth::OAuthTokenRepository;
use crate::repositories::wallet::NonceRepository;
use crate::repositories::OAuthTempCodeRepository;

/// Default interval for nonce cleanup (1 hour)
const DEFAULT_NONCE_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Minimum interval for nonce cleanup (5 minutes)
const MIN_NONCE_CLEANUP_INTERVAL_SECS: u64 = 300;

/// Default interval for OAuth token cleanup (1 hour)
const DEFAULT_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Minimum interval for OAuth token cleanup (5 minutes)
const MIN_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS: u64 = 300;

/// Default retention for auth failures (30 days)
const DEFAULT_AUTH_FAILURES_RETENTION_DAYS: i32 = 30;

/// Background task configuration
#[derive(Debug, Clone)]
pub struct BackgroundTaskConfig {
    /// Interval between nonce cleanups
    pub nonce_cleanup_interval: Duration,
    /// Interval between OAuth token cleanups
    pub oauth_token_cleanup_interval: Duration,
    /// Retention period for auth failures (in days)
    pub auth_failures_retention_days: i32,
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

        let auth_failures_retention_days = env::var("AUTH_FAILURES_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_AUTH_FAILURES_RETENTION_DAYS)
            .max(1); // At least 1 day retention

        Self {
            nonce_cleanup_interval: Duration::from_secs(nonce_interval_secs),
            oauth_token_cleanup_interval: Duration::from_secs(oauth_token_interval_secs),
            auth_failures_retention_days,
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

        // Start nonce cleanup task (used_nonces table)
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

        // Start payment nonces cleanup task
        let payment_nonce_token = cancel_token.clone();
        let payment_nonce_pool = self.pool.clone();
        let payment_nonce_interval = self.config.nonce_cleanup_interval; // Same interval as nonces

        tokio::spawn(async move {
            run_payment_nonce_cleanup(
                payment_nonce_pool,
                payment_nonce_interval,
                payment_nonce_token,
            )
            .await;
        });

        // Start auth failures cleanup task
        let auth_failures_token = cancel_token.clone();
        let auth_failures_pool = self.pool.clone();
        let auth_failures_interval = self.config.nonce_cleanup_interval; // Same interval, different retention
        let auth_failures_retention_days = self.config.auth_failures_retention_days;

        tokio::spawn(async move {
            run_auth_failures_cleanup(
                auth_failures_pool,
                auth_failures_interval,
                auth_failures_retention_days,
                auth_failures_token,
            )
            .await;
        });

        // Start OAuth temp codes cleanup task
        let oauth_temp_codes_token = cancel_token.clone();
        let oauth_temp_codes_pool = self.pool.clone();
        let oauth_temp_codes_interval = self.config.nonce_cleanup_interval; // Same interval as nonces

        tokio::spawn(async move {
            run_oauth_temp_codes_cleanup(
                oauth_temp_codes_pool,
                oauth_temp_codes_interval,
                oauth_temp_codes_token,
            )
            .await;
        });

        info!(
            nonce_cleanup_interval_secs = ?self.config.nonce_cleanup_interval.as_secs(),
            oauth_token_cleanup_interval_secs = ?self.config.oauth_token_cleanup_interval.as_secs(),
            auth_failures_retention_days = self.config.auth_failures_retention_days,
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

/// Run the payment nonce cleanup task
async fn run_payment_nonce_cleanup(
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
                info!("Payment nonce cleanup task stopping due to shutdown");
                break;
            }
            _ = interval.tick() => {
                cleanup_expired_payment_nonces(&pool).await;
            }
        }
    }
}

/// Perform the actual payment nonce cleanup
async fn cleanup_expired_payment_nonces(pool: &DbPool) {
    debug!("Starting payment nonce cleanup");

    // Delete payment nonces that expired more than 24 hours ago
    // Keep a 24h buffer to allow for late payment confirmations
    let result = sqlx::query(
        r#"
        DELETE FROM payment_nonces
        WHERE expires_at < NOW() - INTERVAL '24 hours'
        "#,
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            let count = result.rows_affected();
            if count > 0 {
                info!(deleted_count = count, "Cleaned up expired payment nonces");
            } else {
                debug!("No expired payment nonces to clean up");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to cleanup expired payment nonces");
        }
    }
}

/// Run the auth failures cleanup task
async fn run_auth_failures_cleanup(
    pool: DbPool,
    cleanup_interval: Duration,
    retention_days: i32,
    cancel_token: CancellationToken,
) {
    let mut interval = interval(cleanup_interval);

    // Skip the first tick (which fires immediately)
    interval.tick().await;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Auth failures cleanup task stopping due to shutdown");
                break;
            }
            _ = interval.tick() => {
                cleanup_old_auth_failures(&pool, retention_days).await;
            }
        }
    }
}

/// Perform the actual auth failures cleanup
async fn cleanup_old_auth_failures(pool: &DbPool, retention_days: i32) {
    debug!("Starting auth failures cleanup");

    // Delete auth failures older than the retention period
    let result = sqlx::query(
        r#"
        DELETE FROM auth_failures
        WHERE created_at < NOW() - ($1 || ' days')::INTERVAL
        "#,
    )
    .bind(retention_days.to_string())
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            let count = result.rows_affected();
            if count > 0 {
                info!(
                    deleted_count = count,
                    retention_days = retention_days,
                    "Cleaned up old auth failures"
                );
            } else {
                debug!("No old auth failures to clean up");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to cleanup old auth failures");
        }
    }
}

/// Run the OAuth temp codes cleanup task
async fn run_oauth_temp_codes_cleanup(
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
                info!("OAuth temp codes cleanup task stopping due to shutdown");
                break;
            }
            _ = interval.tick() => {
                cleanup_expired_oauth_temp_codes(&pool).await;
            }
        }
    }
}

/// Perform the actual OAuth temp codes cleanup
async fn cleanup_expired_oauth_temp_codes(pool: &DbPool) {
    debug!("Starting OAuth temp codes cleanup");

    match OAuthTempCodeRepository::cleanup_expired(pool).await {
        Ok(count) => {
            if count > 0 {
                info!(deleted_count = count, "Cleaned up expired OAuth temp codes");
            } else {
                debug!("No expired OAuth temp codes to clean up");
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to cleanup expired OAuth temp codes");
        }
    }
}

/// Standalone function to run payment nonce cleanup once
///
/// Useful for manual cleanup or testing.
#[allow(dead_code)] // Used in tests and manual cleanup scripts
pub async fn cleanup_payment_nonces_once(pool: &DbPool) -> Result<u64, anyhow::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM payment_nonces
        WHERE expires_at < NOW() - INTERVAL '24 hours'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Standalone function to run auth failures cleanup once
///
/// Useful for manual cleanup or testing.
#[allow(dead_code)] // Used in tests and manual cleanup scripts
pub async fn cleanup_auth_failures_once(
    pool: &DbPool,
    retention_days: i32,
) -> Result<u64, anyhow::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM auth_failures
        WHERE created_at < NOW() - ($1 || ' days')::INTERVAL
        "#,
    )
    .bind(retention_days.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
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
        assert!(
            config.oauth_token_cleanup_interval
                >= Duration::from_secs(MIN_OAUTH_TOKEN_CLEANUP_INTERVAL_SECS)
        );
        assert!(config.auth_failures_retention_days >= 1);
        assert_eq!(
            config.auth_failures_retention_days,
            DEFAULT_AUTH_FAILURES_RETENTION_DAYS
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

    #[test]
    fn test_auth_failures_retention_minimum() {
        // Auth failures retention should be at least 1 day
        let config = BackgroundTaskConfig::default();
        assert!(config.auth_failures_retention_days >= 1);
    }

    #[test]
    fn test_default_retention_days() {
        // Default retention is 30 days
        assert_eq!(DEFAULT_AUTH_FAILURES_RETENTION_DAYS, 30);
    }
}
