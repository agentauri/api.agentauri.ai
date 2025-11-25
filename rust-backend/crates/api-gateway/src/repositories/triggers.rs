//! Trigger repository for database operations

use anyhow::{Context, Result};
use shared::models::Trigger;
use shared::DbPool;
use uuid::Uuid;

pub struct TriggerRepository;

impl TriggerRepository {
    /// Create a new trigger
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &DbPool,
        user_id: &str,
        organization_id: &str,
        name: &str,
        description: Option<&str>,
        chain_id: i32,
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
}
