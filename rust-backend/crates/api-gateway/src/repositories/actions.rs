//! Trigger action repository for database operations

use anyhow::{Context, Result};
use shared::models::TriggerAction;
use shared::DbPool;

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
    pub async fn update(
        pool: &DbPool,
        action_id: i32,
        action_type: Option<&str>,
        priority: Option<i32>,
        config: Option<&serde_json::Value>,
    ) -> Result<TriggerAction> {
        // Build dynamic update query
        let mut query = String::from("UPDATE trigger_actions SET id = id");
        let mut param_count = 1;

        if action_type.is_some() {
            query.push_str(&format!(", action_type = ${}", param_count));
            param_count += 1;
        }
        if priority.is_some() {
            query.push_str(&format!(", priority = ${}", param_count));
            param_count += 1;
        }
        if config.is_some() {
            query.push_str(&format!(", config = ${}", param_count));
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        // Execute query with bindings
        let mut q = sqlx::query_as::<_, TriggerAction>(&query);

        if let Some(v) = action_type {
            q = q.bind(v);
        }
        if let Some(v) = priority {
            q = q.bind(v);
        }
        if let Some(v) = config {
            q = q.bind(v);
        }

        q = q.bind(action_id);

        let action = q.fetch_one(pool).await.context("Failed to update action")?;

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
