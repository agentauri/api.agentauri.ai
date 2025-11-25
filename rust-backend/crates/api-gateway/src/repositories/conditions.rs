//! Trigger condition repository for database operations

use anyhow::{Context, Result};
use shared::models::TriggerCondition;
use shared::DbPool;

pub struct ConditionRepository;

impl ConditionRepository {
    /// Create a new condition
    pub async fn create(
        pool: &DbPool,
        trigger_id: &str,
        condition_type: &str,
        field: &str,
        operator: &str,
        value: &str,
        config: Option<&serde_json::Value>,
    ) -> Result<TriggerCondition> {
        let now = chrono::Utc::now();

        let condition = sqlx::query_as::<_, TriggerCondition>(
            r#"
            INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value, config, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(trigger_id)
        .bind(condition_type)
        .bind(field)
        .bind(operator)
        .bind(value)
        .bind(config)
        .bind(now)
        .fetch_one(pool)
        .await
        .context("Failed to create condition")?;

        Ok(condition)
    }

    /// Find condition by ID
    #[allow(dead_code)]
    pub async fn find_by_id(pool: &DbPool, condition_id: i32) -> Result<Option<TriggerCondition>> {
        let condition = sqlx::query_as::<_, TriggerCondition>(
            r#"
            SELECT * FROM trigger_conditions
            WHERE id = $1
            "#,
        )
        .bind(condition_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find condition by ID")?;

        Ok(condition)
    }

    /// List conditions for a trigger
    pub async fn list_by_trigger(pool: &DbPool, trigger_id: &str) -> Result<Vec<TriggerCondition>> {
        let conditions = sqlx::query_as::<_, TriggerCondition>(
            r#"
            SELECT * FROM trigger_conditions
            WHERE trigger_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(trigger_id)
        .fetch_all(pool)
        .await
        .context("Failed to list conditions")?;

        Ok(conditions)
    }

    /// Update condition
    ///
    /// Uses a safe COALESCE/CASE pattern instead of dynamic SQL.
    /// - Simple fields: `None` = keep existing, `Some(value)` = update
    /// - `config`: `None` = keep existing, `Some(None)` = set to NULL, `Some(Some(value))` = update
    pub async fn update(
        pool: &DbPool,
        condition_id: i32,
        condition_type: Option<&str>,
        field: Option<&str>,
        operator: Option<&str>,
        value: Option<&str>,
        config: Option<Option<&serde_json::Value>>,
    ) -> Result<TriggerCondition> {
        // Use a static query with COALESCE/CASE for safe updates
        // $1-$4 = simple fields, $5 = should_update_config flag, $6 = config value, $7 = condition_id
        let condition = sqlx::query_as::<_, TriggerCondition>(
            r#"
            UPDATE trigger_conditions SET
                condition_type = COALESCE($1, condition_type),
                field = COALESCE($2, field),
                operator = COALESCE($3, operator),
                value = COALESCE($4, value),
                config = CASE WHEN $5 THEN $6 ELSE config END
            WHERE id = $7
            RETURNING *
            "#,
        )
        .bind(condition_type)
        .bind(field)
        .bind(operator)
        .bind(value)
        .bind(config.is_some())
        .bind(config.flatten())
        .bind(condition_id)
        .fetch_one(pool)
        .await
        .context("Failed to update condition")?;

        Ok(condition)
    }

    /// Delete condition
    pub async fn delete(pool: &DbPool, condition_id: i32) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM trigger_conditions
            WHERE id = $1
            "#,
        )
        .bind(condition_id)
        .execute(pool)
        .await
        .context("Failed to delete condition")?;

        Ok(result.rows_affected() > 0)
    }

    /// Get trigger_id for a condition
    pub async fn get_trigger_id(pool: &DbPool, condition_id: i32) -> Result<Option<String>> {
        let trigger_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT trigger_id FROM trigger_conditions
            WHERE id = $1
            "#,
        )
        .bind(condition_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get trigger_id for condition")?;

        Ok(trigger_id)
    }
}
