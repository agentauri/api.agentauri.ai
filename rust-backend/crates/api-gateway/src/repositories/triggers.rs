//! Trigger repository for database operations

use anyhow::{Context, Result};
use shared::models::Trigger;
use shared::DbPool;
use uuid::Uuid;

pub struct TriggerRepository;

impl TriggerRepository {
    /// Create a new trigger
    pub async fn create(
        pool: &DbPool,
        user_id: &str,
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
            INSERT INTO triggers (id, user_id, name, description, chain_id, registry, enabled, is_stateful, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(&trigger_id)
        .bind(user_id)
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

    /// List triggers for a user with pagination
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

    /// Count total triggers for a user
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

    /// Update trigger
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

        // Build dynamic update query
        let mut query = String::from("UPDATE triggers SET updated_at = $1");
        let mut param_count = 2;

        if name.is_some() {
            query.push_str(&format!(", name = ${}", param_count));
            param_count += 1;
        }
        if description.is_some() {
            query.push_str(&format!(", description = ${}", param_count));
            param_count += 1;
        }
        if chain_id.is_some() {
            query.push_str(&format!(", chain_id = ${}", param_count));
            param_count += 1;
        }
        if registry.is_some() {
            query.push_str(&format!(", registry = ${}", param_count));
            param_count += 1;
        }
        if enabled.is_some() {
            query.push_str(&format!(", enabled = ${}", param_count));
            param_count += 1;
        }
        if is_stateful.is_some() {
            query.push_str(&format!(", is_stateful = ${}", param_count));
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        // Execute query with bindings
        let mut q = sqlx::query_as::<_, Trigger>(&query).bind(now);

        if let Some(v) = name {
            q = q.bind(v);
        }
        if let Some(v) = description {
            q = q.bind(v);
        }
        if let Some(v) = chain_id {
            q = q.bind(v);
        }
        if let Some(v) = registry {
            q = q.bind(v);
        }
        if let Some(v) = enabled {
            q = q.bind(v);
        }
        if let Some(v) = is_stateful {
            q = q.bind(v);
        }

        q = q.bind(trigger_id);

        let trigger = q
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

    /// Check if trigger belongs to user
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
}
