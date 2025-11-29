// Integration tests for batch loading optimization (N+1 query fix)
// These tests verify that batch loading works correctly and provides performance improvements

use anyhow::Result;
use serde_json::json;
use sqlx::PgPool;
use std::time::Instant;

// Test helper to setup test database
async fn setup_test_db() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests. See database/README.md for setup instructions.");

    let pool = PgPool::connect(&database_url).await?;

    // Clean up any existing test data
    cleanup_test_data(&pool).await?;

    Ok(pool)
}

async fn cleanup_test_data(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM trigger_actions WHERE trigger_id LIKE 'batch_test_%'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM trigger_conditions WHERE trigger_id LIKE 'batch_test_%'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM triggers WHERE id LIKE 'batch_test_%'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM events WHERE id LIKE 'batch_test_%'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM organization_members WHERE organization_id = 'batch_test_org'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM organizations WHERE id = 'batch_test_org'")
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM users WHERE id = 'batch_test_user'")
        .execute(pool)
        .await?;

    Ok(())
}

// Helper to create test user and organization
async fn ensure_test_user_and_org(pool: &PgPool) -> Result<()> {
    // Create test user
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('batch_test_user', 'batchtest', 'batch@example.com', '$argon2id$v=19$m=65536,t=3,p=1$salt$hash')
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    // Create test organization
    sqlx::query!(
        r#"
        INSERT INTO organizations (id, name, slug, owner_id, plan, is_personal)
        VALUES ('batch_test_org', 'Batch Test Org', 'batch-test-org', 'batch_test_user', 'free', true)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Helper to create multiple test triggers with conditions and actions
async fn create_test_triggers(pool: &PgPool, count: usize) -> Result<Vec<String>> {
    ensure_test_user_and_org(pool).await?;

    let mut trigger_ids = Vec::new();

    for i in 0..count {
        let trigger_id = format!("batch_test_trigger_{}", i);

        // Create trigger
        sqlx::query!(
            r#"
            INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
            VALUES ($1, 'batch_test_org', 'batch_test_user', $2, 84532, 'reputation', true, false)
            "#,
            trigger_id,
            format!("Batch Test Trigger {}", i)
        )
        .execute(pool)
        .await?;

        // Create 2 conditions per trigger
        sqlx::query!(
            r#"
            INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value)
            VALUES
                ($1, 'agent_id_equals', 'agent_id', '=', '42'),
                ($1, 'score_threshold', 'score', '<', '60')
            "#,
            trigger_id
        )
        .execute(pool)
        .await?;

        // Create 1 action per trigger
        sqlx::query!(
            r#"
            INSERT INTO trigger_actions (trigger_id, action_type, priority, config)
            VALUES ($1, 'telegram', 1, $2)
            "#,
            trigger_id,
            json!({"chat_id": "123456"})
        )
        .execute(pool)
        .await?;

        trigger_ids.push(trigger_id);
    }

    Ok(trigger_ids)
}

// Helper to create a test event
// Note: This function is kept for future use in end-to-end tests
#[allow(dead_code)]
async fn create_test_event(pool: &PgPool, event_id: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO events (
            id, chain_id, block_number, block_hash, transaction_hash, log_index,
            registry, event_type, agent_id, timestamp, client_address, score
        )
        VALUES (
            $1, 84532, 1000, '0xabc', '0xdef', 0,
            'reputation', 'NewFeedback', 42, 1234567890, '0x123', 55
        )
        "#,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_batch_loading_correctness() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create 10 triggers with conditions and actions
    let trigger_ids = create_test_triggers(&pool, 10).await?;

    // Batch fetch conditions and actions
    let conditions = sqlx::query!(
        r#"
        SELECT trigger_id, condition_type, field, operator, value
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let actions = sqlx::query!(
        r#"
        SELECT trigger_id, action_type, priority
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, priority DESC, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    // Verify we got the correct number of records
    assert_eq!(conditions.len(), 20, "Expected 20 conditions (2 per trigger)");
    assert_eq!(actions.len(), 10, "Expected 10 actions (1 per trigger)");

    // Verify conditions are correctly associated
    for trigger_id in &trigger_ids {
        let trigger_conditions: Vec<_> = conditions
            .iter()
            .filter(|c| &c.trigger_id == trigger_id)
            .collect();

        assert_eq!(trigger_conditions.len(), 2, "Expected 2 conditions per trigger");
        assert_eq!(trigger_conditions[0].condition_type, "agent_id_equals");
        assert_eq!(trigger_conditions[1].condition_type, "score_threshold");
    }

    // Verify actions are correctly associated
    for trigger_id in &trigger_ids {
        let trigger_actions: Vec<_> = actions
            .iter()
            .filter(|a| &a.trigger_id == trigger_id)
            .collect();

        assert_eq!(trigger_actions.len(), 1, "Expected 1 action per trigger");
        assert_eq!(trigger_actions[0].action_type, "telegram");
    }

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_with_empty_list() -> Result<()> {
    let pool = setup_test_db().await?;

    // Batch fetch with empty trigger_ids list
    let trigger_ids: Vec<String> = vec![];

    let conditions = sqlx::query!(
        r#"
        SELECT trigger_id, condition_type
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(conditions.len(), 0, "Expected 0 conditions for empty list");

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_with_triggers_without_conditions() -> Result<()> {
    let pool = setup_test_db().await?;
    ensure_test_user_and_org(&pool).await?;

    // Create trigger without conditions
    let trigger_id = "batch_test_no_conditions";
    sqlx::query!(
        r#"
        INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
        VALUES ($1, 'batch_test_org', 'batch_test_user', 'No Conditions', 84532, 'reputation', true, false)
        "#,
        trigger_id
    )
    .execute(&pool)
    .await?;

    // Batch fetch conditions
    let conditions = sqlx::query!(
        r#"
        SELECT trigger_id, condition_type
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        "#,
        &vec![trigger_id.to_string()]
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(conditions.len(), 0, "Expected 0 conditions for trigger without conditions");

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_performance() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create 100 triggers with conditions and actions
    let trigger_ids = create_test_triggers(&pool, 100).await?;

    // Measure batch loading time (2 queries)
    let start = Instant::now();

    let conditions = sqlx::query!(
        r#"
        SELECT id, trigger_id, condition_type, field, operator, value, config, created_at
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let actions = sqlx::query!(
        r#"
        SELECT id, trigger_id, action_type, priority, config, created_at
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, priority DESC, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let batch_duration = start.elapsed();

    // Verify results
    assert_eq!(conditions.len(), 200, "Expected 200 conditions");
    assert_eq!(actions.len(), 100, "Expected 100 actions");

    // Performance assertion: batch loading should complete in < 100ms
    assert!(
        batch_duration.as_millis() < 100,
        "Batch loading took {}ms, expected < 100ms",
        batch_duration.as_millis()
    );

    println!(
        "✅ Batch loading performance: {}ms for 100 triggers (2 queries)",
        batch_duration.as_millis()
    );

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_vs_n_plus_one() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create 50 triggers for realistic comparison
    let trigger_ids = create_test_triggers(&pool, 50).await?;

    // Measure N+1 approach (1 + 50 + 50 = 101 queries)
    let start = Instant::now();

    for trigger_id in &trigger_ids {
        let _conditions = sqlx::query!(
            r#"
            SELECT id, trigger_id, condition_type, field, operator, value, config, created_at
            FROM trigger_conditions
            WHERE trigger_id = $1
            ORDER BY id
            "#,
            trigger_id
        )
        .fetch_all(&pool)
        .await?;

        let _actions = sqlx::query!(
            r#"
            SELECT id, trigger_id, action_type, priority, config, created_at
            FROM trigger_actions
            WHERE trigger_id = $1
            ORDER BY priority DESC, id
            "#,
            trigger_id
        )
        .fetch_all(&pool)
        .await?;
    }

    let n_plus_one_duration = start.elapsed();

    // Measure batch loading approach (2 queries)
    let start = Instant::now();

    let _conditions = sqlx::query!(
        r#"
        SELECT id, trigger_id, condition_type, field, operator, value, config, created_at
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let _actions = sqlx::query!(
        r#"
        SELECT id, trigger_id, action_type, priority, config, created_at
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, priority DESC, id
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let batch_duration = start.elapsed();

    // Calculate improvement
    let improvement_ratio = n_plus_one_duration.as_millis() as f64 / batch_duration.as_millis() as f64;

    println!("\n📊 Performance Comparison (50 triggers):");
    println!("   N+1 approach: {}ms (100 queries)", n_plus_one_duration.as_millis());
    println!("   Batch loading: {}ms (2 queries)", batch_duration.as_millis());
    println!("   Improvement: {:.1}x faster", improvement_ratio);

    // Batch loading should be at least 10x faster
    assert!(
        improvement_ratio >= 10.0,
        "Expected at least 10x improvement, got {:.1}x",
        improvement_ratio
    );

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_preserves_ordering() -> Result<()> {
    let pool = setup_test_db().await?;
    ensure_test_user_and_org(&pool).await?;

    // Create trigger with multiple actions at different priorities
    let trigger_id = "batch_test_ordering";
    sqlx::query!(
        r#"
        INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
        VALUES ($1, 'batch_test_org', 'batch_test_user', 'Ordering Test', 84532, 'reputation', true, false)
        "#,
        trigger_id
    )
    .execute(&pool)
    .await?;

    // Insert actions with different priorities
    sqlx::query!(
        r#"
        INSERT INTO trigger_actions (trigger_id, action_type, priority, config)
        VALUES
            ($1, 'telegram', 3, $2),
            ($1, 'rest', 1, $2),
            ($1, 'mcp', 2, $2)
        "#,
        trigger_id,
        json!({"test": "config"})
    )
    .execute(&pool)
    .await?;

    // Batch fetch actions
    let actions = sqlx::query!(
        r#"
        SELECT action_type, priority
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, priority DESC, id
        "#,
        &vec![trigger_id.to_string()]
    )
    .fetch_all(&pool)
    .await?;

    // Verify ordering: should be sorted by priority DESC
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0].action_type, "telegram");
    assert_eq!(actions[0].priority, Some(3));
    assert_eq!(actions[1].action_type, "mcp");
    assert_eq!(actions[1].priority, Some(2));
    assert_eq!(actions[2].action_type, "rest");
    assert_eq!(actions[2].priority, Some(1));

    cleanup_test_data(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn test_batch_loading_with_large_dataset() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create 200 triggers (stress test)
    let trigger_ids = create_test_triggers(&pool, 200).await?;

    // Measure batch loading time
    let start = Instant::now();

    let conditions = sqlx::query!(
        r#"
        SELECT trigger_id, condition_type
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let actions = sqlx::query!(
        r#"
        SELECT trigger_id, action_type
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        "#,
        &trigger_ids
    )
    .fetch_all(&pool)
    .await?;

    let batch_duration = start.elapsed();

    // Verify results
    assert_eq!(conditions.len(), 400, "Expected 400 conditions (2 per trigger)");
    assert_eq!(actions.len(), 200, "Expected 200 actions (1 per trigger)");

    // Should handle 200 triggers efficiently (< 200ms)
    assert!(
        batch_duration.as_millis() < 200,
        "Large dataset batch loading took {}ms, expected < 200ms",
        batch_duration.as_millis()
    );

    println!(
        "✅ Large dataset performance: {}ms for 200 triggers",
        batch_duration.as_millis()
    );

    cleanup_test_data(&pool).await?;
    Ok(())
}
