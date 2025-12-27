//! OAuth Temporary Code Repository
//!
//! Handles storage and validation of temporary authorization codes for OAuth flows.
//! These codes are short-lived (5 minutes) and single-use, exchanged for tokens
//! to avoid exposing tokens in URLs.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::DbPool;
use sqlx::FromRow;

/// OAuth temporary code record from database
#[derive(Debug, FromRow)]
pub struct OAuthTempCodeRecord {
    pub id: String,
    pub code_hash: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

pub struct OAuthTempCodeRepository;

impl OAuthTempCodeRepository {
    /// Create a new temporary authorization code
    pub async fn create(
        pool: &DbPool,
        user_id: &str,
        code_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<String> {
        let id = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO oauth_temp_codes (user_id, code_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(code_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .context("Failed to create OAuth temporary code")?;

        Ok(id)
    }

    /// Find a valid (non-expired, non-used) code by hash
    pub async fn find_valid_by_hash(
        pool: &DbPool,
        code_hash: &str,
    ) -> Result<Option<OAuthTempCodeRecord>> {
        let result = sqlx::query_as::<_, OAuthTempCodeRecord>(
            r#"
            SELECT id, code_hash, user_id, expires_at, used_at, created_at
            FROM oauth_temp_codes
            WHERE code_hash = $1
              AND expires_at > NOW()
              AND used_at IS NULL
            "#,
        )
        .bind(code_hash)
        .fetch_optional(pool)
        .await
        .context("Failed to find OAuth temporary code")?;

        Ok(result)
    }

    /// Atomically find and mark a code as used
    ///
    /// Returns the user_id if successful, None if code not found/expired/already used.
    /// Uses SELECT ... FOR UPDATE to prevent race conditions.
    pub async fn exchange_code(pool: &DbPool, code_hash: &str) -> Result<Option<String>> {
        let mut tx = pool.begin().await.context("Failed to begin transaction")?;

        // Find and lock the code row
        let record = sqlx::query_as::<_, OAuthTempCodeRecord>(
            r#"
            SELECT id, code_hash, user_id, expires_at, used_at, created_at
            FROM oauth_temp_codes
            WHERE code_hash = $1
              AND expires_at > NOW()
              AND used_at IS NULL
            FOR UPDATE
            "#,
        )
        .bind(code_hash)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to find OAuth temporary code for exchange")?;

        // If not found, transaction is automatically rolled back when dropped
        let Some(record) = record else {
            return Ok(None);
        };

        let user_id = record.user_id.clone();

        // Mark the code as used
        sqlx::query(
            r#"
            UPDATE oauth_temp_codes
            SET used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(&record.id)
        .execute(&mut *tx)
        .await
        .context("Failed to mark OAuth code as used")?;

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(Some(user_id))
    }

    /// Delete expired and used codes (cleanup task)
    pub async fn cleanup_expired(pool: &DbPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_temp_codes
            WHERE expires_at < NOW() - INTERVAL '1 hour'
               OR used_at < NOW() - INTERVAL '1 hour'
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to cleanup expired OAuth codes")?;

        Ok(result.rows_affected())
    }

    /// Invalidate all pending codes for a user (e.g., on logout)
    #[allow(dead_code)]
    pub async fn invalidate_for_user(pool: &DbPool, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_temp_codes
            SET used_at = NOW()
            WHERE user_id = $1
              AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to invalidate OAuth codes for user")?;

        Ok(result.rows_affected())
    }
}
