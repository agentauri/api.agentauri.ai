//! User identities repository for multi-provider authentication

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::models::UserIdentity;
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub struct UserIdentityRepository;

#[allow(dead_code)] // Some methods used for future account linking feature
impl UserIdentityRepository {
    /// Create a new user identity
    pub async fn create<'e, E>(
        executor: E,
        user_id: &str,
        provider: &str,
        provider_user_id: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<UserIdentity>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            INSERT INTO user_identities (
                id, user_id, provider, provider_user_id,
                email, display_name, avatar_url, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(provider)
        .bind(provider_user_id)
        .bind(email)
        .bind(display_name)
        .bind(avatar_url)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create user identity")?;

        Ok(identity)
    }

    /// Create a wallet identity
    pub async fn create_wallet<'e, E>(
        executor: E,
        user_id: &str,
        wallet_address: &str,
        chain_id: i32,
    ) -> Result<UserIdentity>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let provider_user_id = format!("{}:{}", chain_id, wallet_address.to_lowercase());

        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            INSERT INTO user_identities (
                id, user_id, provider, provider_user_id,
                wallet_address, chain_id, created_at
            )
            VALUES ($1, $2, 'wallet', $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&provider_user_id)
        .bind(wallet_address)
        .bind(chain_id)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create wallet identity")?;

        Ok(identity)
    }

    /// Find identity by provider and provider_user_id
    pub async fn find_by_provider(
        pool: &DbPool,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<UserIdentity>> {
        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            SELECT * FROM user_identities
            WHERE provider = $1 AND provider_user_id = $2
            "#,
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find identity by provider")?;

        Ok(identity)
    }

    /// Find identity by wallet address and chain
    pub async fn find_by_wallet(
        pool: &DbPool,
        wallet_address: &str,
        chain_id: i32,
    ) -> Result<Option<UserIdentity>> {
        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            SELECT * FROM user_identities
            WHERE provider = 'wallet'
              AND wallet_address = $1
              AND chain_id = $2
            "#,
        )
        .bind(wallet_address)
        .bind(chain_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find identity by wallet")?;

        Ok(identity)
    }

    /// Find all identities for a user
    pub async fn find_by_user_id(pool: &DbPool, user_id: &str) -> Result<Vec<UserIdentity>> {
        let identities = sqlx::query_as::<_, UserIdentity>(
            r#"
            SELECT * FROM user_identities
            WHERE user_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("Failed to find identities for user")?;

        Ok(identities)
    }

    /// Find identity by email (across all providers)
    pub async fn find_by_email(pool: &DbPool, email: &str) -> Result<Option<UserIdentity>> {
        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            SELECT * FROM user_identities
            WHERE email = $1
            LIMIT 1
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await
        .context("Failed to find identity by email")?;

        Ok(identity)
    }

    /// Update last used timestamp
    pub async fn update_last_used(pool: &DbPool, identity_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE user_identities
            SET last_used_at = $1
            WHERE id = $2
            "#,
        )
        .bind(now)
        .bind(identity_id)
        .execute(pool)
        .await
        .context("Failed to update last used")?;

        Ok(())
    }

    /// Update OAuth tokens for an identity
    pub async fn update_tokens(
        pool: &DbPool,
        identity_id: &str,
        access_token_encrypted: Option<&str>,
        refresh_token_encrypted: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_identities
            SET access_token_encrypted = $1,
                refresh_token_encrypted = $2,
                token_expires_at = $3
            WHERE id = $4
            "#,
        )
        .bind(access_token_encrypted)
        .bind(refresh_token_encrypted)
        .bind(token_expires_at)
        .bind(identity_id)
        .execute(pool)
        .await
        .context("Failed to update tokens")?;

        Ok(())
    }

    /// Delete an identity (unlink provider)
    pub async fn delete(pool: &DbPool, identity_id: &str, user_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_identities
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(identity_id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to delete identity")?;

        Ok(result.rows_affected() > 0)
    }

    /// Count identities for a user
    pub async fn count_by_user(pool: &DbPool, user_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM user_identities
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to count identities")?;

        Ok(count)
    }

    /// Check if user has a specific provider linked
    pub async fn has_provider(pool: &DbPool, user_id: &str, provider: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_identities
                WHERE user_id = $1 AND provider = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(provider)
        .fetch_one(pool)
        .await
        .context("Failed to check provider")?;

        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_provider_user_id_format() {
        // Wallet provider_user_id format
        let chain_id = 1;
        let wallet = "0x1234567890abcdef";
        let provider_user_id = format!("{}:{}", chain_id, wallet.to_lowercase());
        assert_eq!(provider_user_id, "1:0x1234567890abcdef");
    }
}
