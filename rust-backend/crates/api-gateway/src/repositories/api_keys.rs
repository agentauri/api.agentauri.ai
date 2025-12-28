//! API Key Database Repository
//!
//! This module provides database access for API keys and their audit logs.
//!
//! # Security Considerations
//!
//! - Keys are stored as Argon2id hashes (never plaintext)
//! - Lookup is by prefix only (first 16 chars)
//! - Revoked keys are kept for audit trail
//! - All operations are logged to audit table
//!
//! # Transaction Support
//!
//! Key rotation requires atomicity - use `rotate_with_executor` with a transaction.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use shared::models::{ApiKey, ApiKeyAuditLog, AuthFailure};
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

// ============================================================================
// ApiKeyRepository
// ============================================================================

pub struct ApiKeyRepository;

impl ApiKeyRepository {
    /// Create a new API key
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &DbPool,
        organization_id: &str,
        key_hash: &str,
        name: &str,
        prefix: &str,
        environment: &str,
        key_type: &str,
        permissions: &[String],
        rate_limit_override: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
        created_by: &str,
    ) -> Result<ApiKey> {
        Self::create_with_executor(
            pool,
            organization_id,
            key_hash,
            name,
            prefix,
            environment,
            key_type,
            permissions,
            rate_limit_override,
            expires_at,
            created_by,
        )
        .await
    }

    /// Create a new API key with a generic executor (supports transactions)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_executor<'e, E>(
        executor: E,
        organization_id: &str,
        key_hash: &str,
        name: &str,
        prefix: &str,
        environment: &str,
        key_type: &str,
        permissions: &[String],
        rate_limit_override: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
        created_by: &str,
    ) -> Result<ApiKey>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let key_id = Uuid::new_v4().to_string();
        let permissions_json =
            serde_json::to_value(permissions).context("Failed to serialize permissions")?;

        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            INSERT INTO api_keys (
                id, organization_id, key_hash, name, prefix, environment,
                key_type, permissions, rate_limit_override, expires_at, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(&key_id)
        .bind(organization_id)
        .bind(key_hash)
        .bind(name)
        .bind(prefix)
        .bind(environment)
        .bind(key_type)
        .bind(&permissions_json)
        .bind(rate_limit_override)
        .bind(expires_at)
        .bind(created_by)
        .fetch_one(executor)
        .await
        .context("Failed to create API key")?;

        Ok(key)
    }

    /// Find API key by prefix (for authentication)
    ///
    /// Only returns active (non-revoked) keys
    pub async fn find_by_prefix(pool: &DbPool, prefix: &str) -> Result<Option<ApiKey>> {
        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE prefix = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(prefix)
        .fetch_optional(pool)
        .await
        .context("Failed to find API key by prefix")?;

        Ok(key)
    }

    /// Find API key by ID
    pub async fn find_by_id(pool: &DbPool, key_id: &str) -> Result<Option<ApiKey>> {
        let key = sqlx::query_as::<_, ApiKey>(r#"SELECT * FROM api_keys WHERE id = $1"#)
            .bind(key_id)
            .fetch_optional(pool)
            .await
            .context("Failed to find API key by ID")?;

        Ok(key)
    }

    /// Find API key by ID with row lock for atomic operations
    ///
    /// SECURITY: Uses SELECT FOR UPDATE to prevent race conditions in
    /// rotation and revocation operations. Must be called within a transaction.
    pub async fn find_by_id_for_update<'e, E>(executor: E, key_id: &str) -> Result<Option<ApiKey>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let key = sqlx::query_as::<_, ApiKey>(r#"SELECT * FROM api_keys WHERE id = $1 FOR UPDATE"#)
            .bind(key_id)
            .fetch_optional(executor)
            .await
            .context("Failed to find API key by ID with lock")?;

        Ok(key)
    }

    /// List API keys for an organization with pagination
    ///
    /// By default only returns active (non-revoked) keys.
    /// Set `include_revoked` to `true` to include all keys.
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        include_revoked: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>> {
        let query = if include_revoked {
            r#"
            SELECT * FROM api_keys
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        } else {
            r#"
            SELECT * FROM api_keys
            WHERE organization_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        };

        let keys = sqlx::query_as::<_, ApiKey>(query)
            .bind(organization_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to list API keys")?;

        Ok(keys)
    }

    /// Count API keys for an organization
    pub async fn count_by_organization(
        pool: &DbPool,
        organization_id: &str,
        include_revoked: bool,
    ) -> Result<i64> {
        let query = if include_revoked {
            r#"SELECT COUNT(*) FROM api_keys WHERE organization_id = $1"#
        } else {
            r#"SELECT COUNT(*) FROM api_keys WHERE organization_id = $1 AND revoked_at IS NULL"#
        };

        let count = sqlx::query_scalar::<_, i64>(query)
            .bind(organization_id)
            .fetch_one(pool)
            .await
            .context("Failed to count API keys")?;

        Ok(count)
    }

    /// Revoke an API key
    pub async fn revoke(
        pool: &DbPool,
        key_id: &str,
        revoked_by: &str,
        reason: Option<&str>,
    ) -> Result<ApiKey> {
        Self::revoke_with_executor(pool, key_id, revoked_by, reason).await
    }

    /// Revoke an API key with a generic executor (supports transactions)
    pub async fn revoke_with_executor<'e, E>(
        executor: E,
        key_id: &str,
        revoked_by: &str,
        reason: Option<&str>,
    ) -> Result<ApiKey>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = chrono::Utc::now();

        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            UPDATE api_keys
            SET revoked_at = $1, revoked_by = $2, revocation_reason = $3
            WHERE id = $4 AND revoked_at IS NULL
            RETURNING *
            "#,
        )
        .bind(now)
        .bind(revoked_by)
        .bind(reason)
        .bind(key_id)
        .fetch_one(executor)
        .await
        .context("Failed to revoke API key")?;

        Ok(key)
    }

    /// Update an API key's name and/or expiration
    pub async fn update(
        pool: &DbPool,
        key_id: &str,
        name: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ApiKey> {
        // Build dynamic update query based on provided fields
        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            UPDATE api_keys
            SET
                name = COALESCE($1, name),
                expires_at = CASE WHEN $2 THEN $3 ELSE expires_at END
            WHERE id = $4 AND revoked_at IS NULL
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(expires_at.is_some()) // Flag to indicate if expires_at should be updated
        .bind(expires_at)
        .bind(key_id)
        .fetch_one(pool)
        .await
        .context("Failed to update API key")?;

        Ok(key)
    }

    /// Update last used timestamp and IP
    pub async fn update_last_used(
        pool: &DbPool,
        key_id: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE api_keys
            SET last_used_at = $1, last_used_ip = $2
            WHERE id = $3
            "#,
        )
        .bind(now)
        .bind(ip_address)
        .bind(key_id)
        .execute(pool)
        .await
        .context("Failed to update last used")?;

        Ok(())
    }

    /// Check if a key is expired
    pub fn is_expired(key: &ApiKey) -> bool {
        if let Some(expires_at) = key.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if a key is revoked
    pub fn is_revoked(key: &ApiKey) -> bool {
        key.revoked_at.is_some()
    }

    /// Check if a key is active (not revoked and not expired)
    pub fn is_active(key: &ApiKey) -> bool {
        !Self::is_revoked(key) && !Self::is_expired(key)
    }
}

// ============================================================================
// ApiKeyAuditRepository
// ============================================================================

pub struct ApiKeyAuditRepository;

impl ApiKeyAuditRepository {
    /// Log an API key event
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        pool: &DbPool,
        api_key_id: Option<&str>,
        organization_id: &str,
        event_type: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        endpoint: Option<&str>,
        actor_user_id: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<ApiKeyAuditLog> {
        Self::log_with_executor(
            pool,
            api_key_id,
            organization_id,
            event_type,
            ip_address,
            user_agent,
            endpoint,
            actor_user_id,
            details,
        )
        .await
    }

    /// Log an API key event with a generic executor (supports transactions)
    #[allow(clippy::too_many_arguments)]
    pub async fn log_with_executor<'e, E>(
        executor: E,
        api_key_id: Option<&str>,
        organization_id: &str,
        event_type: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        endpoint: Option<&str>,
        actor_user_id: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<ApiKeyAuditLog>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let log = sqlx::query_as::<_, ApiKeyAuditLog>(
            r#"
            INSERT INTO api_key_audit_log (
                api_key_id, organization_id, event_type, ip_address,
                user_agent, endpoint, actor_user_id, details
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(api_key_id)
        .bind(organization_id)
        .bind(event_type)
        .bind(ip_address)
        .bind(user_agent)
        .bind(endpoint)
        .bind(actor_user_id)
        .bind(details)
        .fetch_one(executor)
        .await
        .context("Failed to log API key event")?;

        Ok(log)
    }

    /// List audit logs for an API key
    #[allow(dead_code)]
    pub async fn list_by_key(
        pool: &DbPool,
        api_key_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKeyAuditLog>> {
        let logs = sqlx::query_as::<_, ApiKeyAuditLog>(
            r#"
            SELECT * FROM api_key_audit_log
            WHERE api_key_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(api_key_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list audit logs for key")?;

        Ok(logs)
    }

    /// List audit logs for an organization
    #[allow(dead_code)]
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKeyAuditLog>> {
        let logs = sqlx::query_as::<_, ApiKeyAuditLog>(
            r#"
            SELECT * FROM api_key_audit_log
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to list audit logs for organization")?;

        Ok(logs)
    }

    /// Count recent auth failures by IP (for rate limiting / abuse detection)
    #[allow(dead_code)]
    pub async fn count_recent_failures_by_ip(
        pool: &DbPool,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM api_key_audit_log
            WHERE ip_address = $1
              AND event_type = 'auth_failed'
              AND created_at >= $2
            "#,
        )
        .bind(ip_address)
        .bind(since)
        .fetch_one(pool)
        .await
        .context("Failed to count recent auth failures")?;

        Ok(count)
    }
}

// ============================================================================
// AuthFailureRepository
// ============================================================================

/// Repository for authentication failures without organization context
///
/// This repository handles authentication failures where we cannot determine
/// the organization, such as:
/// - Invalid key format (not sk_live_ or sk_test_)
/// - Key prefix not found in database
/// - Rate limited requests
pub struct AuthFailureRepository;

impl AuthFailureRepository {
    /// Log an authentication failure
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `failure_type` - Type of failure (invalid_format, prefix_not_found, rate_limited)
    /// * `key_prefix` - First 16 chars of the attempted key (for pattern analysis)
    /// * `ip_address` - Client IP address
    /// * `user_agent` - Client user agent
    /// * `endpoint` - API endpoint that was accessed
    /// * `details` - Additional context (error message, etc.)
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        pool: &DbPool,
        failure_type: &str,
        key_prefix: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        endpoint: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<AuthFailure> {
        let log = sqlx::query_as::<_, AuthFailure>(
            r#"
            INSERT INTO auth_failures (
                failure_type, key_prefix, ip_address, user_agent, endpoint, details
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(failure_type)
        .bind(key_prefix)
        .bind(ip_address)
        .bind(user_agent)
        .bind(endpoint)
        .bind(details)
        .fetch_one(pool)
        .await
        .context("Failed to log auth failure")?;

        Ok(log)
    }

    /// Count recent auth failures by IP (for rate limiting decisions)
    #[allow(dead_code)]
    pub async fn count_recent_by_ip(
        pool: &DbPool,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM auth_failures
            WHERE ip_address = $1
              AND created_at >= $2
            "#,
        )
        .bind(ip_address)
        .bind(since)
        .fetch_one(pool)
        .await
        .context("Failed to count recent auth failures")?;

        Ok(count)
    }

    /// Get recent failures by IP for security analysis
    #[allow(dead_code)]
    pub async fn get_recent_by_ip(
        pool: &DbPool,
        ip_address: &str,
        limit: i64,
    ) -> Result<Vec<AuthFailure>> {
        let failures = sqlx::query_as::<_, AuthFailure>(
            r#"
            SELECT * FROM auth_failures
            WHERE ip_address = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(ip_address)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to get recent auth failures")?;

        Ok(failures)
    }
}
