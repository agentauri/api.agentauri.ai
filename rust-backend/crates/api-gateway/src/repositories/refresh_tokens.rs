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

    /// Atomically rotate a refresh token: validate, revoke old, create new
    ///
    /// Uses SELECT ... FOR UPDATE to prevent race conditions where two concurrent
    /// requests could both validate the same token before either revokes it.
    ///
    /// Returns (user_id, new_token_id) on success.
    pub async fn atomic_rotate(
        pool: &DbPool,
        token_hash: &str,
        new_token_hash: &str,
        new_expires_at: DateTime<Utc>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<Option<(String, String)>> {
        let mut tx = pool.begin().await.context("Failed to begin transaction")?;

        // Find and lock the token row (FOR UPDATE prevents concurrent access)
        let record = sqlx::query_as::<_, RefreshTokenRecord>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, revoked_at, user_agent, ip_address
            FROM user_refresh_tokens
            WHERE token_hash = $1
              AND expires_at > NOW()
              AND revoked_at IS NULL
            FOR UPDATE
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to find refresh token for rotation")?;

        let record = match record {
            Some(r) => r,
            None => {
                tx.rollback().await.ok();
                return Ok(None);
            }
        };

        let user_id = record.user_id.clone();

        // Revoke the old token
        sqlx::query(
            r#"
            UPDATE user_refresh_tokens
            SET revoked_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(&record.id)
        .execute(&mut *tx)
        .await
        .context("Failed to revoke old refresh token")?;

        // Create the new token
        let new_id = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO user_refresh_tokens (user_id, token_hash, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&user_id)
        .bind(new_token_hash)
        .bind(new_expires_at)
        .bind(user_agent)
        .bind(ip_address)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to create new refresh token")?;

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(Some((user_id, new_id)))
    }

    /// Revoke oldest sessions if user exceeds max allowed sessions
    ///
    /// This enforces a limit on concurrent sessions per user for security.
    /// Returns the number of sessions revoked.
    pub async fn enforce_session_limit(
        pool: &DbPool,
        user_id: &str,
        max_sessions: i64,
    ) -> Result<u64> {
        // Revoke oldest sessions beyond the limit
        let result = sqlx::query(
            r#"
            UPDATE user_refresh_tokens
            SET revoked_at = NOW()
            WHERE id IN (
                SELECT id FROM user_refresh_tokens
                WHERE user_id = $1
                  AND expires_at > NOW()
                  AND revoked_at IS NULL
                ORDER BY created_at ASC
                OFFSET $2
            )
            "#,
        )
        .bind(user_id)
        .bind(max_sessions)
        .execute(pool)
        .await
        .context("Failed to enforce session limit")?;

        Ok(result.rows_affected())
    }
}
