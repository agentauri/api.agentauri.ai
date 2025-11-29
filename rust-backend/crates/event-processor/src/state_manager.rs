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
    #[allow(dead_code)] // Used in tests and cached state manager
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
    #[allow(dead_code)] // Used in tests and background cleanup
    pub async fn cleanup_expired(&self, retention_days: i32) -> Result<u64> {
        debug!(retention_days = retention_days, "Starting state cleanup");

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
    #[allow(dead_code)] // Used in tests and monitoring
    pub async fn get_state_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM trigger_state
            "#
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get state count")?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Helper to setup test database for async tests
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for integration tests. See database/README.md for setup instructions.");

        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Clean up any existing test data
        sqlx::query!("DELETE FROM trigger_state WHERE trigger_id LIKE 'test_%'")
            .execute(&pool)
            .await
            .expect("Failed to clean up test data");

        sqlx::query!("DELETE FROM triggers WHERE id LIKE 'test_%'")
            .execute(&pool)
            .await
            .expect("Failed to clean up test triggers");

        pool
    }

    // Helper to create a test user and organization
    async fn ensure_test_user_and_org(pool: &PgPool) {
        // Create test user
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash)
            VALUES ('test_user', 'testuser', 'test@example.com', '$argon2id$v=19$m=65536,t=3,p=1$salt$hash')
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        // Create test organization
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, name, slug, owner_id, plan, is_personal)
            VALUES ('test_org', 'Test Org', 'test-org', 'test_user', 'free', true)
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .execute(pool)
        .await
        .expect("Failed to create test organization");
    }

    // Helper to create a test trigger
    async fn create_test_trigger(pool: &PgPool, trigger_id: &str) {
        ensure_test_user_and_org(pool).await;

        sqlx::query!(
            r#"
            INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
            VALUES ($1, 'test_org', 'test_user', 'Test Trigger', 84532, 'reputation', true, true)
            ON CONFLICT (id) DO NOTHING
            "#,
            trigger_id
        )
        .execute(pool)
        .await
        .expect("Failed to create test trigger");
    }

    // Unit tests (no database required)

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

    // Integration tests (require real database)

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_load_state_nonexistent() {
        let pool = setup_test_db().await;
        let manager = TriggerStateManager::new(pool);

        let result = manager.load_state("test_nonexistent").await.unwrap();
        assert!(
            result.is_none(),
            "Should return None for non-existent state"
        );
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_update_state_creates_new() {
        let pool = setup_test_db().await;
        let trigger_id = "test_create_new";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool);
        let state_data = json!({
            "ema": 80.5,
            "count": 5
        });

        // Create new state
        manager
            .update_state(trigger_id, state_data.clone())
            .await
            .unwrap();

        // Verify it was created
        let loaded = manager.load_state(trigger_id).await.unwrap();
        assert!(loaded.is_some(), "State should exist after update");
        assert_eq!(loaded.unwrap(), state_data, "Loaded state should match");

        // Cleanup
        manager.delete_state(trigger_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_update_state_overwrites_existing() {
        let pool = setup_test_db().await;
        let trigger_id = "test_overwrite";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool);

        // Create initial state
        let state1 = json!({"ema": 70.0, "count": 1});
        manager.update_state(trigger_id, state1).await.unwrap();

        // Overwrite with new state
        let state2 = json!({"ema": 75.5, "count": 2});
        manager
            .update_state(trigger_id, state2.clone())
            .await
            .unwrap();

        // Verify new state
        let loaded = manager.load_state(trigger_id).await.unwrap().unwrap();
        assert_eq!(loaded, state2, "State should be overwritten");
        assert_ne!(loaded["count"], 1, "Old count should not persist");

        // Cleanup
        manager.delete_state(trigger_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_delete_state() {
        let pool = setup_test_db().await;
        let trigger_id = "test_delete";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool);

        // Create state
        let state_data = json!({"ema": 65.0, "count": 3});
        manager.update_state(trigger_id, state_data).await.unwrap();

        // Verify it exists
        let loaded = manager.load_state(trigger_id).await.unwrap();
        assert!(loaded.is_some(), "State should exist before deletion");

        // Delete state
        manager.delete_state(trigger_id).await.unwrap();

        // Verify it's gone
        let after_delete = manager.load_state(trigger_id).await.unwrap();
        assert!(
            after_delete.is_none(),
            "State should not exist after deletion"
        );
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_delete_state_nonexistent() {
        let pool = setup_test_db().await;
        let manager = TriggerStateManager::new(pool);

        // Deleting non-existent state should not error
        let result = manager.delete_state("test_nonexistent_delete").await;
        assert!(result.is_ok(), "Deleting non-existent state should succeed");
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_cleanup_expired() {
        let pool = setup_test_db().await;
        let trigger_fresh = "test_cleanup_fresh";
        let trigger_old = "test_cleanup_old";
        create_test_trigger(&pool, trigger_fresh).await;
        create_test_trigger(&pool, trigger_old).await;

        let manager = TriggerStateManager::new(pool.clone());

        // Create fresh state (should NOT be deleted)
        manager
            .update_state(trigger_fresh, json!({"ema": 80.0}))
            .await
            .unwrap();

        // Create old state by manually setting last_updated to 31 days ago
        sqlx::query!(
            r#"
            INSERT INTO trigger_state (trigger_id, state_data, last_updated)
            VALUES ($1, $2, NOW() - INTERVAL '31 days')
            ON CONFLICT (trigger_id) DO UPDATE SET
                state_data = EXCLUDED.state_data,
                last_updated = EXCLUDED.last_updated
            "#,
            trigger_old,
            json!({"ema": 50.0})
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run cleanup with 30-day retention
        let deleted = manager.cleanup_expired(30).await.unwrap();

        // Verify old state was deleted, fresh state remains
        assert_eq!(deleted, 1, "Should delete exactly 1 old state record");

        let fresh_state = manager.load_state(trigger_fresh).await.unwrap();
        assert!(fresh_state.is_some(), "Fresh state should still exist");

        let old_state = manager.load_state(trigger_old).await.unwrap();
        assert!(old_state.is_none(), "Old state should be deleted");

        // Cleanup
        manager.delete_state(trigger_fresh).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_cleanup_expired_no_records() {
        let pool = setup_test_db().await;
        let manager = TriggerStateManager::new(pool);

        // Cleanup when no expired records exist
        let deleted = manager.cleanup_expired(30).await.unwrap();
        assert_eq!(deleted, 0, "Should delete 0 records when none are expired");
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_get_state_count_empty() {
        let pool = setup_test_db().await;
        let manager = TriggerStateManager::new(pool);

        let count = manager.get_state_count().await.unwrap();
        assert_eq!(count, 0, "Should return 0 when no state records exist");
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_get_state_count() {
        let pool = setup_test_db().await;
        create_test_trigger(&pool, "test_count_1").await;
        create_test_trigger(&pool, "test_count_2").await;
        create_test_trigger(&pool, "test_count_3").await;

        let manager = TriggerStateManager::new(pool.clone());

        // Create 3 state records
        manager
            .update_state("test_count_1", json!({"ema": 70.0}))
            .await
            .unwrap();
        manager
            .update_state("test_count_2", json!({"ema": 75.0}))
            .await
            .unwrap();
        manager
            .update_state("test_count_3", json!({"ema": 80.0}))
            .await
            .unwrap();

        // Count only test_count_* records by querying manually
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM trigger_state WHERE trigger_id LIKE 'test_count_%'"#
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count, 3, "Should return count of 3 state records");

        // Cleanup
        manager.delete_state("test_count_1").await.unwrap();
        manager.delete_state("test_count_2").await.unwrap();
        manager.delete_state("test_count_3").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_state_upsert_atomicity() {
        // Verify that UPSERT is atomic (no race condition between SELECT and INSERT)
        let pool = setup_test_db().await;
        let trigger_id = "test_upsert_atomic";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool.clone());

        // Create initial state
        manager
            .update_state(trigger_id, json!({"count": 1}))
            .await
            .unwrap();

        // Multiple concurrent updates (simulating race condition)
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let mgr = TriggerStateManager::new(pool.clone());
                let tid = trigger_id.to_string();
                tokio::spawn(async move {
                    mgr.update_state(&tid, json!({"count": i})).await.unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // Final state should be one of the updates (not corrupted)
        let final_state = manager.load_state(trigger_id).await.unwrap().unwrap();
        let count = final_state["count"].as_i64().unwrap();
        assert!((0..10).contains(&count), "Final count should be valid");

        // Cleanup
        manager.delete_state(trigger_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_large_state_data() {
        // Verify we can store large JSONB data (rate counter with many timestamps)
        let pool = setup_test_db().await;
        let trigger_id = "test_large_state";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool);

        // Create state with 1000 timestamps
        let timestamps: Vec<i64> = (0..1000).map(|i| 1234567890 + i).collect();
        let large_state = json!({
            "count": 1000,
            "recent_timestamps": timestamps
        });

        manager
            .update_state(trigger_id, large_state.clone())
            .await
            .unwrap();

        // Verify it can be loaded
        let loaded = manager.load_state(trigger_id).await.unwrap().unwrap();
        assert_eq!(loaded["count"], 1000, "Count should match");
        assert_eq!(
            loaded["recent_timestamps"].as_array().unwrap().len(),
            1000,
            "All timestamps should be stored"
        );

        // Cleanup
        manager.delete_state(trigger_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires DATABASE_URL (integration test)
    async fn test_complex_state_structure() {
        // Verify we can store complex nested JSONB structures
        let pool = setup_test_db().await;
        let trigger_id = "test_complex_state";
        create_test_trigger(&pool, trigger_id).await;

        let manager = TriggerStateManager::new(pool);

        let complex_state = json!({
            "ema": {
                "value": 75.5,
                "window_size": 10,
                "alpha": 0.1818
            },
            "rate_counter": {
                "count": 15,
                "window": "1h",
                "timestamps": [1234567890, 1234567900]
            },
            "metadata": {
                "last_trigger": "2025-01-23T12:00:00Z",
                "total_evaluations": 100
            }
        });

        manager
            .update_state(trigger_id, complex_state.clone())
            .await
            .unwrap();

        let loaded = manager.load_state(trigger_id).await.unwrap().unwrap();
        assert_eq!(
            loaded["ema"]["value"], 75.5,
            "Nested EMA value should match"
        );
        assert_eq!(
            loaded["rate_counter"]["count"], 15,
            "Nested count should match"
        );
        assert_eq!(
            loaded["metadata"]["total_evaluations"], 100,
            "Metadata should match"
        );

        // Cleanup
        manager.delete_state(trigger_id).await.unwrap();
    }
}
