//! Trigger state manager
//!
//! Manages persistent state for stateful triggers (EMA, rate counters, etc.)
//! Uses PostgreSQL JSONB storage with UPSERT for atomic updates.

use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::PgPool;
use tracing::{debug, warn};

/// Manages trigger state persistence
pub struct TriggerStateManager {
    pool: PgPool,
}

impl TriggerStateManager {
    /// Create a new state manager
    ///
    /// # Arguments
    ///
    /// * `pool` - PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load state for a trigger
    ///
    /// # Arguments
    ///
    /// * `trigger_id` - ID of the trigger
    ///
    /// # Returns
    ///
    /// - `Some(state_data)` if state exists
    /// - `None` if trigger has no state (first evaluation)
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn load_state(&self, trigger_id: &str) -> Result<Option<Value>> {
        debug!(trigger_id = trigger_id, "Loading trigger state");

        let result = sqlx::query!(
            r#"
            SELECT state_data
            FROM trigger_state
            WHERE trigger_id = $1
            "#,
            trigger_id
        )
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to load state for trigger {}", trigger_id))?;

        match result {
            Some(record) => {
                debug!(trigger_id = trigger_id, "State loaded");
                Ok(Some(record.state_data))
            }
            None => {
                debug!(trigger_id = trigger_id, "No existing state");
                Ok(None)
            }
        }
    }

    /// Update state for a trigger (atomic UPSERT)
    ///
    /// # Arguments
    ///
    /// * `trigger_id` - ID of the trigger
    /// * `state_data` - New state data (JSONB)
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn update_state(&self, trigger_id: &str, state_data: Value) -> Result<()> {
        debug!(trigger_id = trigger_id, "Updating trigger state");

        sqlx::query!(
            r#"
            INSERT INTO trigger_state (trigger_id, state_data, last_updated)
            VALUES ($1, $2, NOW())
            ON CONFLICT (trigger_id)
            DO UPDATE SET
                state_data = EXCLUDED.state_data,
                last_updated = EXCLUDED.last_updated
            "#,
            trigger_id,
            state_data
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update state for trigger {}", trigger_id))?;

        debug!(trigger_id = trigger_id, "State updated");
        Ok(())
    }

    /// Delete state for a trigger
    ///
    /// # Arguments
    ///
    /// * `trigger_id` - ID of the trigger
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn delete_state(&self, trigger_id: &str) -> Result<()> {
        debug!(trigger_id = trigger_id, "Deleting trigger state");

        sqlx::query!(
            r#"
            DELETE FROM trigger_state
            WHERE trigger_id = $1
            "#,
            trigger_id
        )
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to delete state for trigger {}", trigger_id))?;

        debug!(trigger_id = trigger_id, "State deleted");
        Ok(())
    }

    /// Cleanup expired state records
    ///
    /// Deletes state for triggers that haven't been updated recently.
    /// This prevents unbounded growth of the trigger_state table.
    ///
    /// # Arguments
    ///
    /// * `retention_days` - Number of days to retain inactive state
    ///
    /// # Returns
    ///
    /// Number of state records deleted
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn cleanup_expired(&self, retention_days: i32) -> Result<u64> {
        debug!(
            retention_days = retention_days,
            "Starting state cleanup"
        );

        let result = sqlx::query!(
            r#"
            DELETE FROM trigger_state
            WHERE last_updated < NOW() - INTERVAL '1 day' * $1
            "#,
            retention_days as f64
        )
        .execute(&self.pool)
        .await
        .context("Failed to cleanup expired state records")?;

        let deleted = result.rows_affected();

        if deleted > 0 {
            warn!(
                deleted = deleted,
                retention_days = retention_days,
                "Cleaned up expired trigger state records"
            );
        } else {
            debug!("No expired state records to cleanup");
        }

        Ok(deleted)
    }

    /// Get state count statistics
    ///
    /// Returns the total number of state records in the database.
    /// Useful for monitoring and capacity planning.
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get_state_count(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM trigger_state
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get state count")?;

        Ok(result.count.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Note: These are unit tests that don't require a real database
    // Integration tests with real PostgreSQL are in tests/stateful_triggers_test.rs

    // Note: Constructor test removed because PgPool::connect_lazy requires Tokio context
    // Integration tests with real database are in tests/stateful_triggers_test.rs

    #[test]
    fn test_state_data_serialization() {
        // Test that state data can be serialized to JSON
        let ema_state = json!({
            "ema": 75.5,
            "count": 10,
            "last_updated": "2025-01-23T12:00:00Z"
        });

        let serialized = serde_json::to_string(&ema_state).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(ema_state, deserialized);
    }

    #[test]
    fn test_rate_counter_state_serialization() {
        // Test rate counter state JSON format
        let rate_state = json!({
            "window_start": "2025-01-23T12:00:00Z",
            "count": 15,
            "recent_timestamps": [1234567890, 1234567900, 1234567910]
        });

        let serialized = serde_json::to_string(&rate_state).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(rate_state, deserialized);
    }
}
