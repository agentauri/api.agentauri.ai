//! OAuth Database Repository
//!
//! This module provides database access for OAuth clients and tokens.
//!
//! # Security Considerations
//!
//! - Client secrets are stored as Argon2id hashes (p=4, never plaintext)
//! - Access/refresh tokens are stored as Argon2id hashes (p=4, never plaintext)
//! - Revoked tokens are kept for audit trail
//! - Expired tokens should be cleaned up regularly via background task
//!
//! # Transaction Support
//!
//! Token creation may require atomicity - use `create_token_with_executor` with a transaction.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::models::{OAuthClient, OAuthToken};
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

// ============================================================================
// OAuthClientRepository
// ============================================================================

pub struct OAuthClientRepository;

impl OAuthClientRepository {
    /// Create a new OAuth client
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &DbPool,
        client_id: &str,
        client_secret_hash: &str,
        client_name: &str,
        redirect_uris: &[String],
        scopes: &[String],
        owner_organization_id: &str,
        grant_types: &[String],
        is_trusted: bool,
    ) -> Result<OAuthClient> {
        let id = Uuid::new_v4().to_string();

        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            INSERT INTO oauth_clients (
                id, client_id, client_secret_hash, client_name,
                redirect_uris, scopes, owner_organization_id,
                grant_types, is_trusted
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(client_id)
        .bind(client_secret_hash)
        .bind(client_name)
        .bind(redirect_uris)
        .bind(scopes)
        .bind(owner_organization_id)
        .bind(grant_types)
        .bind(is_trusted)
        .fetch_one(pool)
        .await
        .context("Failed to create OAuth client")?;

        Ok(client)
    }

    /// Find OAuth client by client_id
    pub async fn find_by_client_id(pool: &DbPool, client_id: &str) -> Result<Option<OAuthClient>> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"SELECT * FROM oauth_clients WHERE client_id = $1"#,
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find OAuth client by client_id")?;

        Ok(client)
    }

    /// Find OAuth client by internal ID
    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Option<OAuthClient>> {
        let client = sqlx::query_as::<_, OAuthClient>(r#"SELECT * FROM oauth_clients WHERE id = $1"#)
            .bind(id)
            .fetch_optional(pool)
            .await
            .context("Failed to find OAuth client by ID")?;

        Ok(client)
    }

    /// List OAuth clients for an organization
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OAuthClient>> {
        let clients = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT * FROM oauth_clients
            WHERE owner_organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list OAuth clients")?;

        Ok(clients)
    }

    /// Count OAuth clients for an organization
    pub async fn count_by_organization(pool: &DbPool, organization_id: &str) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM oauth_clients
            WHERE owner_organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to count OAuth clients")?;

        Ok(count.0)
    }

    /// Delete an OAuth client
    ///
    /// This will CASCADE delete all tokens associated with this client
    pub async fn delete(pool: &DbPool, client_id: &str) -> Result<()> {
        sqlx::query(r#"DELETE FROM oauth_clients WHERE client_id = $1"#)
            .bind(client_id)
            .execute(pool)
            .await
            .context("Failed to delete OAuth client")?;

        Ok(())
    }

    /// Update client (for rotation, scope changes, etc.)
    #[allow(dead_code)]
    pub async fn update(
        pool: &DbPool,
        client_id: &str,
        client_name: Option<&str>,
        redirect_uris: Option<&[String]>,
        scopes: Option<&[String]>,
    ) -> Result<OAuthClient> {
        // Build dynamic update query
        let mut query = String::from("UPDATE oauth_clients SET updated_at = NOW()");
        let mut param_count = 1;

        if client_name.is_some() {
            param_count += 1;
            query.push_str(&format!(", client_name = ${}", param_count));
        }
        if redirect_uris.is_some() {
            param_count += 1;
            query.push_str(&format!(", redirect_uris = ${}", param_count));
        }
        if scopes.is_some() {
            param_count += 1;
            query.push_str(&format!(", scopes = ${}", param_count));
        }

        query.push_str(&format!(" WHERE client_id = $1 RETURNING *"));

        let mut q = sqlx::query_as::<_, OAuthClient>(&query).bind(client_id);

        if let Some(name) = client_name {
            q = q.bind(name);
        }
        if let Some(uris) = redirect_uris {
            q = q.bind(uris);
        }
        if let Some(s) = scopes {
            q = q.bind(s);
        }

        let client = q
            .fetch_one(pool)
            .await
            .context("Failed to update OAuth client")?;

        Ok(client)
    }
}

// ============================================================================
// OAuthTokenRepository
// ============================================================================

pub struct OAuthTokenRepository;

impl OAuthTokenRepository {
    /// Create a new OAuth token
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &DbPool,
        access_token_hash: &str,
        refresh_token_hash: Option<&str>,
        client_id: &str,
        user_id: &str,
        organization_id: &str,
        scopes: &[String],
        expires_at: DateTime<Utc>,
        refresh_token_expires_at: Option<DateTime<Utc>>,
    ) -> Result<OAuthToken> {
        Self::create_with_executor(
            pool,
            access_token_hash,
            refresh_token_hash,
            client_id,
            user_id,
            organization_id,
            scopes,
            expires_at,
            refresh_token_expires_at,
        )
        .await
    }

    /// Create a new OAuth token with a generic executor (supports transactions)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_executor<'e, E>(
        executor: E,
        access_token_hash: &str,
        refresh_token_hash: Option<&str>,
        client_id: &str,
        user_id: &str,
        organization_id: &str,
        scopes: &[String],
        expires_at: DateTime<Utc>,
        refresh_token_expires_at: Option<DateTime<Utc>>,
    ) -> Result<OAuthToken>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4().to_string();

        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            INSERT INTO oauth_tokens (
                id, access_token_hash, refresh_token_hash, client_id,
                user_id, organization_id, scopes,
                expires_at, refresh_token_expires_at, revoked
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(access_token_hash)
        .bind(refresh_token_hash)
        .bind(client_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(scopes)
        .bind(expires_at)
        .bind(refresh_token_expires_at)
        .fetch_one(executor)
        .await
        .context("Failed to create OAuth token")?;

        Ok(token)
    }

    /// Find active token by access token hash
    ///
    /// Only returns non-revoked, non-expired tokens
    pub async fn find_by_access_token_hash(
        pool: &DbPool,
        access_token_hash: &str,
    ) -> Result<Option<OAuthToken>> {
        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT * FROM oauth_tokens
            WHERE access_token_hash = $1
              AND NOT revoked
              AND expires_at > NOW()
            "#,
        )
        .bind(access_token_hash)
        .fetch_optional(pool)
        .await
        .context("Failed to find OAuth token by access_token_hash")?;

        Ok(token)
    }

    /// Find active token by refresh token hash
    ///
    /// Only returns non-revoked, non-expired refresh tokens
    pub async fn find_by_refresh_token_hash(
        pool: &DbPool,
        refresh_token_hash: &str,
    ) -> Result<Option<OAuthToken>> {
        let token = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT * FROM oauth_tokens
            WHERE refresh_token_hash = $1
              AND NOT revoked
              AND refresh_token_expires_at > NOW()
            "#,
        )
        .bind(refresh_token_hash)
        .fetch_optional(pool)
        .await
        .context("Failed to find OAuth token by refresh_token_hash")?;

        Ok(token)
    }

    /// Revoke a token by ID
    pub async fn revoke(pool: &DbPool, token_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE oauth_tokens
            SET revoked = true
            WHERE id = $1
            "#,
        )
        .bind(token_id)
        .execute(pool)
        .await
        .context("Failed to revoke OAuth token")?;

        Ok(())
    }

    /// Revoke all tokens for a user
    #[allow(dead_code)]
    pub async fn revoke_all_for_user(pool: &DbPool, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_tokens
            SET revoked = true
            WHERE user_id = $1 AND NOT revoked
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to revoke all tokens for user")?;

        Ok(result.rows_affected())
    }

    /// Revoke all tokens for a client
    #[allow(dead_code)]
    pub async fn revoke_all_for_client(pool: &DbPool, client_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_tokens
            SET revoked = true
            WHERE client_id = $1 AND NOT revoked
            "#,
        )
        .bind(client_id)
        .execute(pool)
        .await
        .context("Failed to revoke all tokens for client")?;

        Ok(result.rows_affected())
    }

    /// Clean up expired tokens (for background task)
    ///
    /// Deletes tokens that have been expired for more than 30 days
    pub async fn cleanup_expired_tokens(pool: &DbPool) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_tokens
            WHERE expires_at < NOW() - INTERVAL '30 days'
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to cleanup expired OAuth tokens")?;

        Ok(result.rows_affected())
    }

    /// List tokens for a user (for user's token management page)
    #[allow(dead_code)]
    pub async fn list_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OAuthToken>> {
        let tokens = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT * FROM oauth_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list OAuth tokens for user")?;

        Ok(tokens)
    }

    /// List tokens for a client (for monitoring)
    #[allow(dead_code)]
    pub async fn list_by_client(
        pool: &DbPool,
        client_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OAuthToken>> {
        let tokens = sqlx::query_as::<_, OAuthToken>(
            r#"
            SELECT * FROM oauth_tokens
            WHERE client_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(client_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list OAuth tokens for client")?;

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running database with migrations applied
    // They are integration tests and should be run with `cargo test --features=integration`
    //
    // For now, we only include unit tests for the structure.

    #[test]
    fn test_oauth_client_repository_exists() {
        // Verify the repository struct exists
        let _ = OAuthClientRepository;
    }

    #[test]
    fn test_oauth_token_repository_exists() {
        // Verify the repository struct exists
        let _ = OAuthTokenRepository;
    }
}
