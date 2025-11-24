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
    pub async fn update(
        pool: &DbPool,
        condition_id: i32,
        condition_type: Option<&str>,
        field: Option<&str>,
        operator: Option<&str>,
        value: Option<&str>,
        config: Option<Option<&serde_json::Value>>,
    ) -> Result<TriggerCondition> {
        // Build dynamic update query
        let mut query = String::from("UPDATE trigger_conditions SET id = id");
        let mut param_count = 1;

        if condition_type.is_some() {
            query.push_str(&format!(", condition_type = ${}", param_count));
            param_count += 1;
        }
        if field.is_some() {
            query.push_str(&format!(", field = ${}", param_count));
            param_count += 1;
        }
        if operator.is_some() {
            query.push_str(&format!(", operator = ${}", param_count));
            param_count += 1;
        }
        if value.is_some() {
            query.push_str(&format!(", value = ${}", param_count));
            param_count += 1;
        }
        if config.is_some() {
            query.push_str(&format!(", config = ${}", param_count));
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        // Execute query with bindings
        let mut q = sqlx::query_as::<_, TriggerCondition>(&query);

        if let Some(v) = condition_type {
            q = q.bind(v);
        }
        if let Some(v) = field {
            q = q.bind(v);
        }
        if let Some(v) = operator {
            q = q.bind(v);
        }
        if let Some(v) = value {
            q = q.bind(v);
        }
        if let Some(v) = config {
            q = q.bind(v);
        }

        q = q.bind(condition_id);

        let condition = q
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
