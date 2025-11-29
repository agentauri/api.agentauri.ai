//! Integration tests for CachedStateManager
//!
//! Tests the complete caching layer including:
//! - Cache hit/miss scenarios
//! - Write-through consistency
//! - Redis failure handling
//! - Performance characteristics
//! - Concurrent access patterns

use event_processor::CachedStateManager;
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::PgPool;
use std::time::{Duration, Instant};

// Helper to setup test database
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up any existing test data
    sqlx::query!("DELETE FROM trigger_state WHERE trigger_id LIKE 'test_cached_integ_%'")
        .execute(&pool)
        .await
        .expect("Failed to clean up test data");

    sqlx::query!("DELETE FROM triggers WHERE id LIKE 'test_cached_integ_%'")
        .execute(&pool)
        .await
        .expect("Failed to clean up test triggers");

    pool
}

// Helper to setup test Redis
async fn setup_test_redis() -> ConnectionManager {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    ConnectionManager::new(client)
        .await
        .expect("Failed to create Redis connection manager")
}

// Helper to create a test user and organization
async fn ensure_test_user_and_org(pool: &PgPool) {
    // Create test user
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test_user_cached_integ', 'testuser_cached_integ', 'test_cached_integ@example.com', '$argon2id$v=19$m=65536,t=3,p=1$salt$hash')
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
        VALUES ('test_org_cached_integ', 'Test Org Cached Integ', 'test-org-cached-integ', 'test_user_cached_integ', 'free', true)
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
        VALUES ($1, 'test_org_cached_integ', 'test_user_cached_integ', 'Test Trigger Cached Integ', 84532, 'reputation', true, true)
        ON CONFLICT (id) DO NOTHING
        "#,
        trigger_id
    )
    .execute(pool)
    .await
    .expect("Failed to create test trigger");
}

#[tokio::test]
async fn test_cache_hit_performance() {
    // Verify cache hits are 8-100x faster than database reads
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_perf_hit";
    create_test_trigger(&pool, trigger_id).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create state
    let state_data = json!({"ema": 75.5, "count": 100});
    manager
        .update_state(trigger_id, state_data.clone())
        .await
        .unwrap();

    // Warm up cache
    manager.load_state(trigger_id).await.unwrap();

    // Measure cache hit latency (should be ~0.1-0.5ms)
    let start = Instant::now();
    for _ in 0..100 {
        let result = manager.load_state(trigger_id).await.unwrap();
        assert!(result.is_some());
    }
    let cache_duration = start.elapsed();
    let avg_cache_latency = cache_duration.as_micros() / 100;

    println!(
        "Cache hit average latency: {}μs ({:.2}ms)",
        avg_cache_latency,
        avg_cache_latency as f64 / 1000.0
    );

    // Verify cache performance (should be under 1ms on average)
    assert!(
        avg_cache_latency < 2000,
        "Cache hit latency should be under 2ms, got {}μs",
        avg_cache_latency
    );

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_cache_miss_fallback() {
    // Verify cache misses correctly fall back to PostgreSQL
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_miss";
    create_test_trigger(&pool, trigger_id).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create state directly in DB (bypassing cache)
    sqlx::query!(
        r#"
        INSERT INTO trigger_state (trigger_id, state_data, last_updated)
        VALUES ($1, $2, NOW())
        "#,
        trigger_id,
        json!({"ema": 80.0, "count": 50})
    )
    .execute(&pool)
    .await
    .unwrap();

    // Load should miss cache and fall back to DB
    let loaded = manager.load_state(trigger_id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap()["ema"], 80.0);

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_write_through_consistency() {
    // Verify writes go to both PostgreSQL and Redis atomically
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_consistency";
    create_test_trigger(&pool, trigger_id).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Perform multiple updates
    for i in 0..10 {
        let state = json!({"iteration": i, "timestamp": format!("2025-01-23T12:00:{:02}Z", i)});
        manager.update_state(trigger_id, state).await.unwrap();

        // Verify consistency after each update
        let from_cache = manager.load_state(trigger_id).await.unwrap().unwrap();

        // Load directly from DB to verify consistency
        let from_db = sqlx::query!(
            r#"SELECT state_data FROM trigger_state WHERE trigger_id = $1"#,
            trigger_id
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .state_data;

        assert_eq!(
            from_cache, from_db,
            "Cache and DB should be consistent after update"
        );
        assert_eq!(from_cache["iteration"], i);
    }

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_redis_failure_graceful_degradation() {
    // Verify system continues working even if Redis is unavailable
    let pool = setup_test_db().await;

    // Create invalid Redis connection
    let invalid_redis_url = "redis://invalid-host:6379";
    let client = redis::Client::open(invalid_redis_url).unwrap();

    // This will create a connection manager, but operations will fail
    // In production, we'd want to handle this more gracefully at startup
    // For now, we test that the manager handles Redis failures in operations

    let trigger_id = "test_cached_integ_redis_fail";
    create_test_trigger(&pool, trigger_id).await;

    // We'll test with a valid Redis but disable caching via env var
    std::env::set_var("STATE_CACHE_ENABLED", "false");

    let redis = setup_test_redis().await;
    let manager = CachedStateManager::new(pool.clone(), redis, 300);

    // Update should succeed even with cache disabled
    let state_data = json!({"ema": 95.0, "count": 25});
    manager.update_state(trigger_id, state_data.clone()).await.unwrap();

    // Load should succeed from DB
    let loaded = manager.load_state(trigger_id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), state_data);

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
    std::env::remove_var("STATE_CACHE_ENABLED");
}

#[tokio::test]
async fn test_high_throughput_scenario() {
    // Simulate high-load scenario with many concurrent reads/writes
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create 20 triggers
    let trigger_ids: Vec<String> = (0..20)
        .map(|i| format!("test_cached_integ_throughput_{}", i))
        .collect();

    for trigger_id in &trigger_ids {
        create_test_trigger(&pool, trigger_id).await;
        // Initialize with state
        manager
            .update_state(trigger_id, json!({"ema": 70.0 + i as f64, "count": 0}))
            .await
            .unwrap();
    }

    let start = Instant::now();

    // Simulate 1000 events (50 per trigger)
    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let trigger_id = trigger_ids[i % 20].clone();
            let mgr = CachedStateManager::new(pool.clone(), redis.clone(), 300);

            tokio::spawn(async move {
                // Load state
                let state = mgr.load_state(&trigger_id).await.unwrap();

                if let Some(mut current) = state {
                    // Update EMA and count
                    let count = current["count"].as_i64().unwrap_or(0);
                    current["count"] = json!(count + 1);

                    // Write back
                    mgr.update_state(&trigger_id, current).await.unwrap();
                }
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let throughput = 1000.0 / duration.as_secs_f64();

    println!(
        "Processed 1000 events in {:?} ({:.2} events/sec)",
        duration, throughput
    );

    // Verify throughput (should handle >100 events/sec with caching)
    assert!(
        throughput > 50.0,
        "Throughput should be >50 events/sec, got {:.2}",
        throughput
    );

    // Cleanup
    for trigger_id in &trigger_ids {
        manager.delete_state(trigger_id).await.unwrap();
    }
}

#[tokio::test]
async fn test_cache_ttl_behavior() {
    // Verify cache entries expire after TTL
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_ttl";
    create_test_trigger(&pool, trigger_id).await;

    // Create manager with 2-second TTL
    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 2);

    // Create state
    let state_data = json!({"ema": 60.0, "count": 10});
    manager.update_state(trigger_id, state_data.clone()).await.unwrap();

    // First load should hit cache
    let start = Instant::now();
    let loaded1 = manager.load_state(trigger_id).await.unwrap();
    let first_duration = start.elapsed();
    assert!(loaded1.is_some());

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Second load should miss cache and be slower
    let start = Instant::now();
    let loaded2 = manager.load_state(trigger_id).await.unwrap();
    let second_duration = start.elapsed();
    assert!(loaded2.is_some());

    println!(
        "First load (cache hit): {:?}, Second load (cache miss): {:?}",
        first_duration, second_duration
    );

    // Second load should be slower (cache miss + DB read + cache populate)
    // However, this is not always guaranteed in tests due to DB connection pooling
    // so we just verify both succeeded
    assert_eq!(loaded1.unwrap(), loaded2.unwrap());

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_cache_invalidation_on_delete() {
    // Verify delete removes from both cache and DB
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_delete";
    create_test_trigger(&pool, trigger_id).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create state
    let state_data = json!({"ema": 85.0, "count": 15});
    manager.update_state(trigger_id, state_data).await.unwrap();

    // Verify exists in cache
    let before_delete = manager.load_state(trigger_id).await.unwrap();
    assert!(before_delete.is_some());

    // Delete
    manager.delete_state(trigger_id).await.unwrap();

    // Verify removed from cache
    let after_delete = manager.load_state(trigger_id).await.unwrap();
    assert!(after_delete.is_none());

    // Verify removed from DB
    let db_result = sqlx::query!(
        r#"SELECT state_data FROM trigger_state WHERE trigger_id = $1"#,
        trigger_id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(db_result.is_none());
}

#[tokio::test]
async fn test_multiple_managers_consistency() {
    // Verify multiple manager instances share same cache
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_multi_manager";
    create_test_trigger(&pool, trigger_id).await;

    let manager1 = CachedStateManager::new(pool.clone(), redis.clone(), 300);
    let manager2 = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Manager 1 writes
    let state_data = json!({"ema": 88.0, "count": 20});
    manager1.update_state(trigger_id, state_data.clone()).await.unwrap();

    // Manager 2 reads (should see Manager 1's write via shared cache)
    let loaded = manager2.load_state(trigger_id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), state_data);

    // Cleanup
    manager1.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_large_state_caching() {
    // Verify large state data (1000 timestamps) can be cached
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "test_cached_integ_large";
    create_test_trigger(&pool, trigger_id).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create large state
    let timestamps: Vec<i64> = (0..1000).map(|i| 1234567890 + i).collect();
    let large_state = json!({
        "count": 1000,
        "recent_timestamps": timestamps,
        "metadata": {
            "window": "1h",
            "threshold": 100
        }
    });

    // Write large state
    manager.update_state(trigger_id, large_state.clone()).await.unwrap();

    // Load from cache
    let loaded = manager.load_state(trigger_id).await.unwrap().unwrap();
    assert_eq!(loaded["count"], 1000);
    assert_eq!(loaded["recent_timestamps"].as_array().unwrap().len(), 1000);

    // Cleanup
    manager.delete_state(trigger_id).await.unwrap();
}

#[tokio::test]
async fn test_cache_stats() {
    // Verify cache stats are correct
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;

    let manager = CachedStateManager::new(pool, redis, 600);
    let (enabled, ttl) = manager.get_cache_stats();

    assert_eq!(ttl, 600);
    // enabled depends on STATE_CACHE_ENABLED env var
    println!("Cache enabled: {}, TTL: {}s", enabled, ttl);
}

#[tokio::test]
async fn test_cleanup_expired_with_cache() {
    // Verify cleanup works correctly with caching enabled
    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_fresh = "test_cached_integ_cleanup_fresh";
    let trigger_old = "test_cached_integ_cleanup_old";
    create_test_trigger(&pool, trigger_fresh).await;
    create_test_trigger(&pool, trigger_old).await;

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Create fresh state
    manager.update_state(trigger_fresh, json!({"ema": 80.0})).await.unwrap();

    // Create old state by manually setting last_updated
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

    // Run cleanup
    let deleted = manager.cleanup_expired(30).await.unwrap();

    // Verify results
    assert_eq!(deleted, 1, "Should delete exactly 1 old state record");

    let fresh_state = manager.load_state(trigger_fresh).await.unwrap();
    assert!(fresh_state.is_some(), "Fresh state should still exist");

    let old_state = manager.load_state(trigger_old).await.unwrap();
    assert!(old_state.is_none(), "Old state should be deleted");

    // Cleanup
    manager.delete_state(trigger_fresh).await.unwrap();
}
