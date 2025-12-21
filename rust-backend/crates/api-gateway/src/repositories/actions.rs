//! Trigger action repository for database operations

use anyhow::{Context, Result};
use shared::models::TriggerAction;
use shared::DbPool;
use sqlx::{Executor, Postgres};

pub struct ActionRepository;

impl ActionRepository {
    /// Create a new action
    pub async fn create(
        pool: &DbPool,
        trigger_id: &str,
        action_type: &str,
        priority: i32,
        config: &serde_json::Value,
    ) -> Result<TriggerAction> {
        let now = chrono::Utc::now();

        let action = sqlx::query_as::<_, TriggerAction>(
            r#"
            INSERT INTO trigger_actions (trigger_id, action_type, priority, config, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(trigger_id)
        .bind(action_type)
        .bind(priority)
        .bind(config)
        .bind(now)
        .fetch_one(pool)
        .await
        .context("Failed to create action")?;

        Ok(action)
    }

    /// Create a new action within a transaction
    pub async fn create_in_tx<'e, E>(
        executor: E,
        trigger_id: &str,
        action_type: &str,
        priority: i32,
        config: &serde_json::Value,
    ) -> Result<TriggerAction>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = chrono::Utc::now();

        let action = sqlx::query_as::<_, TriggerAction>(
            r#"
            INSERT INTO trigger_actions (trigger_id, action_type, priority, config, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(trigger_id)
        .bind(action_type)
        .bind(priority)
        .bind(config)
        .bind(now)
        .fetch_one(executor)
        .await
        .context("Failed to create action")?;

        Ok(action)
    }

    /// Delete all actions for a trigger within a transaction
    pub async fn delete_by_trigger_in_tx<'e, E>(executor: E, trigger_id: &str) -> Result<u64>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM trigger_actions
            WHERE trigger_id = $1
            "#,
        )
        .bind(trigger_id)
        .execute(executor)
        .await
        .context("Failed to delete actions")?;

        Ok(result.rows_affected())
    }

    /// Find action by ID
    #[allow(dead_code)]
    pub async fn find_by_id(pool: &DbPool, action_id: i32) -> Result<Option<TriggerAction>> {
        let action = sqlx::query_as::<_, TriggerAction>(
            r#"
            SELECT * FROM trigger_actions
            WHERE id = $1
            "#,
        )
        .bind(action_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find action by ID")?;

        Ok(action)
    }

    /// List actions for a trigger
    pub async fn list_by_trigger(pool: &DbPool, trigger_id: &str) -> Result<Vec<TriggerAction>> {
        let actions = sqlx::query_as::<_, TriggerAction>(
            r#"
            SELECT * FROM trigger_actions
            WHERE trigger_id = $1
            ORDER BY priority ASC, id ASC
            "#,
        )
        .bind(trigger_id)
        .fetch_all(pool)
        .await
        .context("Failed to list actions")?;

        Ok(actions)
    }

    /// Update action
    ///
    /// Uses a safe COALESCE pattern instead of dynamic SQL.
    /// `None` = keep existing value, `Some(value)` = update to new value.
    pub async fn update(
        pool: &DbPool,
        action_id: i32,
        action_type: Option<&str>,
        priority: Option<i32>,
        config: Option<&serde_json::Value>,
    ) -> Result<TriggerAction> {
        // Use a static query with COALESCE for safe updates
        let action = sqlx::query_as::<_, TriggerAction>(
            r#"
            UPDATE trigger_actions SET
                action_type = COALESCE($1, action_type),
                priority = COALESCE($2, priority),
                config = COALESCE($3, config)
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(action_type)
        .bind(priority)
        .bind(config)
        .bind(action_id)
        .fetch_one(pool)
        .await
        .context("Failed to update action")?;

        Ok(action)
    }

    /// Delete action
    pub async fn delete(pool: &DbPool, action_id: i32) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM trigger_actions
            WHERE id = $1
            "#,
        )
        .bind(action_id)
        .execute(pool)
        .await
        .context("Failed to delete action")?;

        Ok(result.rows_affected() > 0)
    }

    /// Get trigger_id for an action
    pub async fn get_trigger_id(pool: &DbPool, action_id: i32) -> Result<Option<String>> {
        let trigger_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT trigger_id FROM trigger_actions
            WHERE id = $1
            "#,
        )
        .bind(action_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get trigger_id for action")?;

        Ok(trigger_id)
    }
}
