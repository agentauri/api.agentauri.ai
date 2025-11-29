// Integration tests for State Manager with real PostgreSQL
// These tests verify CRUD operations and state persistence

use anyhow::Result;
use event_processor::TriggerStateManager;
use serde_json::json;
use sqlx::PgPool;

// Test helper to setup test database
async fn setup_test_db() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests. See database/README.md for setup instructions.");

    let pool = PgPool::connect(&database_url).await?;

    // Clean up any existing test data
    sqlx::query!("DELETE FROM trigger_state WHERE trigger_id LIKE 'test_%'")
        .execute(&pool)
        .await?;

    sqlx::query!("DELETE FROM triggers WHERE id LIKE 'test_%'")
        .execute(&pool)
        .await?;

    Ok(pool)
}

// Helper to create a test user and organization
async fn ensure_test_user_and_org(pool: &PgPool) -> Result<()> {
    // Create test user
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test_user', 'testuser', 'test@example.com', '$argon2id$v=19$m=65536,t=3,p=1$salt$hash')
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    // Create test organization
    sqlx::query!(
        r#"
        INSERT INTO organizations (id, name, slug, owner_id, plan, is_personal)
        VALUES ('test_org', 'Test Org', 'test-org', 'test_user', 'free', true)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Helper to create a test trigger
async fn create_test_trigger(pool: &PgPool, trigger_id: &str) -> Result<()> {
    ensure_test_user_and_org(pool).await?;

    sqlx::query!(
        r#"
        INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
        VALUES ($1, 'test_org', 'test_user', 'Test Trigger', 84532, 'reputation', true, true)
        ON CONFLICT (id) DO NOTHING
        "#,
        trigger_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_state_manager_create_and_load() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_create_load";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool);
    let state = json!({
        "ema": 75.5,
        "count": 10
    });

    manager.update_state(trigger_id, state.clone()).await?;
    let loaded = manager.load_state(trigger_id).await?;
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), state);

    manager.delete_state(trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_state_manager_update_overwrites() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_update";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool);

    manager
        .update_state(trigger_id, json!({"count": 1}))
        .await?;
    manager
        .update_state(trigger_id, json!({"count": 2}))
        .await?;

    let loaded = manager.load_state(trigger_id).await?;
    assert_eq!(loaded.unwrap()["count"], 2);

    manager.delete_state(trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_state_manager_delete() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_delete";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool);

    manager
        .update_state(trigger_id, json!({"count": 1}))
        .await?;
    manager.delete_state(trigger_id).await?;

    let loaded = manager.load_state(trigger_id).await?;
    assert!(loaded.is_none());

    Ok(())
}

#[tokio::test]
async fn test_state_manager_cleanup_expired() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_fresh = "test_cleanup_fresh";
    let trigger_old = "test_cleanup_old";
    create_test_trigger(&pool, trigger_fresh).await?;
    create_test_trigger(&pool, trigger_old).await?;

    let manager = TriggerStateManager::new(pool.clone());

    manager
        .update_state(trigger_fresh, json!({"ema": 80.0}))
        .await?;

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
    .await?;

    let deleted = manager.cleanup_expired(30).await?;
    assert_eq!(deleted, 1);

    let fresh = manager.load_state(trigger_fresh).await?;
    assert!(fresh.is_some());

    let old = manager.load_state(trigger_old).await?;
    assert!(old.is_none());

    manager.delete_state(trigger_fresh).await?;
    Ok(())
}

#[tokio::test]
async fn test_state_manager_get_count() -> Result<()> {
    let pool = setup_test_db().await?;
    create_test_trigger(&pool, "test_count_1").await?;
    create_test_trigger(&pool, "test_count_2").await?;
    create_test_trigger(&pool, "test_count_3").await?;

    let manager = TriggerStateManager::new(pool.clone());

    manager
        .update_state("test_count_1", json!({"ema": 70.0}))
        .await?;
    manager
        .update_state("test_count_2", json!({"ema": 75.0}))
        .await?;
    manager
        .update_state("test_count_3", json!({"ema": 80.0}))
        .await?;

    // Count only test_count_* records to avoid interference from other tests
    let count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM trigger_state WHERE trigger_id LIKE 'test_count_%'"#
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, 3);

    manager.delete_state("test_count_1").await?;
    manager.delete_state("test_count_2").await?;
    manager.delete_state("test_count_3").await?;

    Ok(())
}

#[tokio::test]
async fn test_state_manager_large_jsonb() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_large_state";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool);

    let timestamps: Vec<i64> = (0..1000).map(|i| 1234567890 + i).collect();
    let large_state = json!({
        "count": 1000,
        "recent_timestamps": timestamps
    });

    manager
        .update_state(trigger_id, large_state.clone())
        .await?;

    let loaded = manager.load_state(trigger_id).await?;
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap()["count"], 1000);

    manager.delete_state(trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_state_manager_concurrent_updates() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_concurrent";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool.clone());

    manager
        .update_state(trigger_id, json!({"count": 0}))
        .await?;

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

    let final_state = manager.load_state(trigger_id).await?;
    assert!(final_state.is_some());
    let count = final_state.unwrap()["count"].as_i64().unwrap();
    assert!(count >= 0 && count < 10);

    manager.delete_state(trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_state_manager_performance() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "test_performance";
    create_test_trigger(&pool, trigger_id).await?;

    let manager = TriggerStateManager::new(pool);
    let state = json!({"ema": 75.5, "count": 100});

    let start = std::time::Instant::now();
    manager.update_state(trigger_id, state).await?;
    let write_duration = start.elapsed();
    assert!(write_duration.as_millis() < 50, "Write should be <50ms");

    let start = std::time::Instant::now();
    let _loaded = manager.load_state(trigger_id).await?;
    let read_duration = start.elapsed();
    assert!(read_duration.as_millis() < 50, "Read should be <50ms");

    manager.delete_state(trigger_id).await?;
    Ok(())
}
