//! Refresh Token Repository
//!
//! Handles storage and validation of user refresh tokens for JWT authentication.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::DbPool;
use sqlx::FromRow;

/// Refresh token record from database
#[derive(Debug, FromRow)]
pub struct RefreshTokenRecord {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

pub struct RefreshTokenRepository;

impl RefreshTokenRepository {
    /// Create a new refresh token
    pub async fn create(
        pool: &DbPool,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<String> {
        let id = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO user_refresh_tokens (user_id, token_hash, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip_address)
        .fetch_one(pool)
        .await
        .context("Failed to create refresh token")?;

        Ok(id)
    }

    /// Find a valid (non-expired, non-revoked) refresh token by hash
    pub async fn find_valid_by_hash(
        pool: &DbPool,
        token_hash: &str,
    ) -> Result<Option<RefreshTokenRecord>> {
        let result = sqlx::query_as::<_, RefreshTokenRecord>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, revoked_at, user_agent, ip_address
            FROM user_refresh_tokens
            WHERE token_hash = $1
              AND expires_at > NOW()
              AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
        .context("Failed to find refresh token")?;

        Ok(result)
    }

    /// Revoke a specific refresh token by ID
    pub async fn revoke(pool: &DbPool, token_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE user_refresh_tokens
            SET revoked_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_id)
        .execute(pool)
        .await
        .context("Failed to revoke refresh token")?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all refresh tokens for a user (logout from all devices)
    pub async fn revoke_all_for_user(pool: &DbPool, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE user_refresh_tokens
            SET revoked_at = NOW()
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to revoke all refresh tokens for user")?;

        Ok(result.rows_affected())
    }

    /// Delete expired and revoked tokens (cleanup task)
    pub async fn cleanup_expired(pool: &DbPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_refresh_tokens
            WHERE expires_at < NOW() - INTERVAL '7 days'
               OR revoked_at < NOW() - INTERVAL '7 days'
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to cleanup expired refresh tokens")?;

        Ok(result.rows_affected())
    }

    /// Count active refresh tokens for a user
    #[allow(dead_code)]
    pub async fn count_active_for_user(pool: &DbPool, user_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM user_refresh_tokens
            WHERE user_id = $1
              AND expires_at > NOW()
              AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to count active refresh tokens")?;

        Ok(count)
    }
}
