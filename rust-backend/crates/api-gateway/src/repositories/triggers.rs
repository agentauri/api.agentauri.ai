//! Trigger repository for database operations

use anyhow::{Context, Result};
use shared::models::Trigger;
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub struct TriggerRepository;

impl TriggerRepository {
    /// Create a new trigger
    ///
    /// `chain_id` can be `None` for wildcard triggers (matches all chains).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &DbPool,
        user_id: &str,
        organization_id: &str,
        name: &str,
        description: Option<&str>,
        chain_id: Option<i32>,
        registry: &str,
        enabled: bool,
        is_stateful: bool,
    ) -> Result<Trigger> {
        let trigger_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let trigger = sqlx::query_as::<_, Trigger>(
            r#"
            INSERT INTO triggers (id, user_id, organization_id, name, description, chain_id, registry, enabled, is_stateful, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(&trigger_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(name)
        .bind(description)
        .bind(chain_id)
        .bind(registry)
        .bind(enabled)
        .bind(is_stateful)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .context("Failed to create trigger")?;

        Ok(trigger)
    }

    /// Create a new trigger within a transaction
    ///
    /// `chain_id` can be `None` for wildcard triggers (matches all chains).
    #[allow(clippy::too_many_arguments)]
    pub async fn create_in_tx<'e, E>(
        executor: E,
        user_id: &str,
        organization_id: &str,
        name: &str,
        description: Option<&str>,
        chain_id: Option<i32>,
        registry: &str,
        enabled: bool,
        is_stateful: bool,
    ) -> Result<Trigger>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let trigger_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let trigger = sqlx::query_as::<_, Trigger>(
            r#"
            INSERT INTO triggers (id, user_id, organization_id, name, description, chain_id, registry, enabled, is_stateful, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(&trigger_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(name)
        .bind(description)
        .bind(chain_id)
        .bind(registry)
        .bind(enabled)
        .bind(is_stateful)
        .bind(now)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create trigger")?;

        Ok(trigger)
    }

    /// Delete trigger within a transaction
    pub async fn delete_in_tx<'e, E>(executor: E, trigger_id: &str) -> Result<bool>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM triggers
            WHERE id = $1
            "#,
        )
        .bind(trigger_id)
        .execute(executor)
        .await
        .context("Failed to delete trigger")?;

        Ok(result.rows_affected() > 0)
    }

    /// Update trigger enabled status within a transaction
    pub async fn update_enabled_in_tx<'e, E>(
        executor: E,
        trigger_id: &str,
        enabled: bool,
    ) -> Result<()>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE triggers SET
                enabled = $1,
                updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(enabled)
        .bind(now)
        .bind(trigger_id)
        .execute(executor)
        .await
        .context("Failed to update trigger enabled status")?;

        Ok(())
    }

    /// Find trigger by ID
    pub async fn find_by_id(pool: &DbPool, trigger_id: &str) -> Result<Option<Trigger>> {
        let trigger = sqlx::query_as::<_, Trigger>(
            r#"
            SELECT * FROM triggers
            WHERE id = $1
            "#,
        )
        .bind(trigger_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find trigger by ID")?;

        Ok(trigger)
    }

    /// List triggers for a user with pagination (deprecated, use list_by_organization)
    #[allow(dead_code)]
    pub async fn list_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Trigger>> {
        let triggers = sqlx::query_as::<_, Trigger>(
            r#"
            SELECT * FROM triggers
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
        .context("Failed to list triggers")?;

        Ok(triggers)
    }

    /// List triggers for an organization with pagination
    pub async fn list_by_organization(
        pool: &DbPool,
        organization_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Trigger>> {
        let triggers = sqlx::query_as::<_, Trigger>(
            r#"
            SELECT * FROM triggers
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
        .context("Failed to list triggers")?;

        Ok(triggers)
    }

    /// Count total triggers for a user (deprecated, use count_by_organization)
    #[allow(dead_code)]
    pub async fn count_by_user(pool: &DbPool, user_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM triggers
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to count triggers")?;

        Ok(count)
    }

    /// Count total triggers for an organization
    pub async fn count_by_organization(pool: &DbPool, organization_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM triggers
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to count triggers")?;

        Ok(count)
    }

    /// Update trigger
    ///
    /// Uses a safe COALESCE/CASE pattern instead of dynamic SQL to prevent potential issues
    /// and make the query easier to audit.
    ///
    /// - Simple fields (`name`, `chain_id`, `registry`, `enabled`, `is_stateful`): `None` = keep existing, `Some(value)` = update
    /// - `description`: `None` = keep existing, `Some(None)` = set to NULL, `Some(Some(value))` = update to value
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &DbPool,
        trigger_id: &str,
        name: Option<&str>,
        description: Option<Option<&str>>,
        chain_id: Option<i32>,
        registry: Option<&str>,
        enabled: Option<bool>,
        is_stateful: Option<bool>,
    ) -> Result<Trigger> {
        let now = chrono::Utc::now();

        // Use a static query with COALESCE/CASE patterns for safe updates
        // $1 = updated_at, $2 = name, $3 = should_update_description, $4 = description,
        // $5 = chain_id, $6 = registry, $7 = enabled, $8 = is_stateful, $9 = trigger_id
        let trigger = sqlx::query_as::<_, Trigger>(
            r#"
            UPDATE triggers SET
                updated_at = $1,
                name = COALESCE($2, name),
                description = CASE WHEN $3 THEN $4 ELSE description END,
                chain_id = COALESCE($5, chain_id),
                registry = COALESCE($6, registry),
                enabled = COALESCE($7, enabled),
                is_stateful = COALESCE($8, is_stateful)
            WHERE id = $9
            RETURNING *
            "#,
        )
        .bind(now)
        .bind(name)
        .bind(description.is_some())
        .bind(description.flatten())
        .bind(chain_id)
        .bind(registry)
        .bind(enabled)
        .bind(is_stateful)
        .bind(trigger_id)
        .fetch_one(pool)
        .await
        .context("Failed to update trigger")?;

        Ok(trigger)
    }

    /// Delete trigger (cascade will handle conditions and actions)
    pub async fn delete(pool: &DbPool, trigger_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM triggers
            WHERE id = $1
            "#,
        )
        .bind(trigger_id)
        .execute(pool)
        .await
        .context("Failed to delete trigger")?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if trigger belongs to user (deprecated, use belongs_to_organization)
    #[allow(dead_code)]
    pub async fn belongs_to_user(pool: &DbPool, trigger_id: &str, user_id: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM triggers
                WHERE id = $1 AND user_id = $2
            )
            "#,
        )
        .bind(trigger_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to check trigger ownership")?;

        Ok(result)
    }

    /// Check if trigger belongs to an organization
    pub async fn belongs_to_organization(
        pool: &DbPool,
        trigger_id: &str,
        organization_id: &str,
    ) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM triggers
                WHERE id = $1 AND organization_id = $2
            )
            "#,
        )
        .bind(trigger_id)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .context("Failed to check trigger organization")?;

        Ok(result)
    }

    /// Get circuit breaker info for a trigger
    ///
    /// Returns trigger name along with circuit breaker config and state.
    pub async fn get_circuit_breaker_info(
        pool: &DbPool,
        trigger_id: &str,
    ) -> Result<Option<CircuitBreakerInfo>> {
        let record = sqlx::query_as::<_, CircuitBreakerInfo>(
            r#"
            SELECT
                id,
                name,
                circuit_breaker_config,
                circuit_breaker_state
            FROM triggers
            WHERE id = $1
            "#,
        )
        .bind(trigger_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch circuit breaker info")?;

        Ok(record)
    }

    /// Update circuit breaker configuration
    ///
    /// Merges the provided config values with existing config (COALESCE pattern).
    pub async fn update_circuit_breaker_config(
        pool: &DbPool,
        trigger_id: &str,
        failure_threshold: Option<u32>,
        recovery_timeout_seconds: Option<u64>,
        half_open_max_calls: Option<u32>,
    ) -> Result<()> {
        let now = chrono::Utc::now();

        // Build the new config by merging with existing (or default)
        // Using jsonb_set operations for partial updates
        sqlx::query(
            r#"
            UPDATE triggers SET
                updated_at = $1,
                circuit_breaker_config = jsonb_strip_nulls(
                    COALESCE(circuit_breaker_config, '{"failure_threshold": 10, "recovery_timeout_seconds": 3600, "half_open_max_calls": 1}'::jsonb)
                    || jsonb_strip_nulls(jsonb_build_object(
                        'failure_threshold', $2::int,
                        'recovery_timeout_seconds', $3::bigint,
                        'half_open_max_calls', $4::int
                    ))
                )
            WHERE id = $5
            "#,
        )
        .bind(now)
        .bind(failure_threshold.map(|v| v as i32))
        .bind(recovery_timeout_seconds.map(|v| v as i64))
        .bind(half_open_max_calls.map(|v| v as i32))
        .bind(trigger_id)
        .execute(pool)
        .await
        .context("Failed to update circuit breaker config")?;

        Ok(())
    }

    /// Reset circuit breaker state to Closed
    ///
    /// Resets failure_count, clears timestamps, and sets state to "Closed".
    pub async fn reset_circuit_breaker_state(pool: &DbPool, trigger_id: &str) -> Result<()> {
        let now = chrono::Utc::now();

        let default_state = serde_json::json!({
            "state": "Closed",
            "failure_count": 0,
            "last_failure_time": null,
            "opened_at": null,
            "half_open_calls": 0
        });

        sqlx::query(
            r#"
            UPDATE triggers SET
                updated_at = $1,
                circuit_breaker_state = $2
            WHERE id = $3
            "#,
        )
        .bind(now)
        .bind(&default_state)
        .bind(trigger_id)
        .execute(pool)
        .await
        .context("Failed to reset circuit breaker state")?;

        Ok(())
    }
}

/// Circuit breaker info from database
#[derive(Debug, sqlx::FromRow)]
pub struct CircuitBreakerInfo {
    pub id: String,
    pub name: String,
    pub circuit_breaker_config: Option<serde_json::Value>,
    pub circuit_breaker_state: Option<serde_json::Value>,
}
