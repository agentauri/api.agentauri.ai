//! Wallet repository for nonce management
//!
//! **Note**: Wallet authentication is a Layer 2 feature that will be completed in Phase 4-5.
//!
//! Handles storage and validation of wallet authentication nonces.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::DbPool;
use sqlx::{Executor, Postgres};

use crate::models::wallet::UsedNonce;

pub struct NonceRepository;

impl NonceRepository {
    /// Store a used nonce (marks it as consumed)
    pub async fn store_nonce<'e, E>(
        executor: E,
        nonce: &str,
        wallet_address: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<()>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            INSERT INTO used_nonces (nonce, wallet_address, used_at, expires_at)
            VALUES ($1, $2, NOW(), $3)
            "#,
        )
        .bind(nonce)
        .bind(wallet_address)
        .bind(expires_at)
        .execute(executor)
        .await
        .context("Failed to store nonce")?;

        Ok(())
    }

    /// Check if a nonce has already been used
    pub async fn is_nonce_used(pool: &DbPool, nonce: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM used_nonces
                WHERE nonce = $1
            )
            "#,
        )
        .bind(nonce)
        .fetch_one(pool)
        .await
        .context("Failed to check nonce usage")?;

        Ok(result)
    }

    /// Find a nonce by value
    #[allow(dead_code)] // Future feature: Layer 2 wallet authentication
    pub async fn find_nonce(pool: &DbPool, nonce: &str) -> Result<Option<UsedNonce>> {
        let result = sqlx::query_as::<_, UsedNonce>(
            r#"
            SELECT nonce, wallet_address, used_at, expires_at
            FROM used_nonces
            WHERE nonce = $1
            "#,
        )
        .bind(nonce)
        .fetch_optional(pool)
        .await
        .context("Failed to find nonce")?;

        Ok(result)
    }

    /// Clean up expired nonces (maintenance task)
    pub async fn cleanup_expired_nonces(pool: &DbPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM used_nonces
            WHERE expires_at < NOW()
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to cleanup expired nonces")?;

        Ok(result.rows_affected())
    }
}
