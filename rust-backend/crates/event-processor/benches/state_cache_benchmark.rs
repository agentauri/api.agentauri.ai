//! Performance benchmarks for state manager with and without caching
//!
//! Measures:
//! - Cache hit latency vs DB read latency
//! - Throughput improvement with caching
//! - Concurrent access performance
//!
//! Run with:
//! ```bash
//! cargo bench --bench state_cache_benchmark
//! ```

use event_processor::{CachedStateManager, TriggerStateManager};
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::PgPool;
use std::time::Instant;

// Setup helpers
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for benchmarks");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up any existing benchmark data
    sqlx::query!("DELETE FROM trigger_state WHERE trigger_id LIKE 'bench_%'")
        .execute(&pool)
        .await
        .expect("Failed to clean up benchmark data");

    sqlx::query!("DELETE FROM triggers WHERE id LIKE 'bench_%'")
        .execute(&pool)
        .await
        .expect("Failed to clean up benchmark triggers");

    pool
}

async fn setup_test_redis() -> ConnectionManager {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    ConnectionManager::new(client)
        .await
        .expect("Failed to create Redis connection manager")
}

async fn ensure_test_user_and_org(pool: &PgPool) {
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('bench_user', 'benchuser', 'bench@example.com', '$argon2id$v=19$m=65536,t=3,p=1$salt$hash')
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await
    .expect("Failed to create benchmark user");

    sqlx::query!(
        r#"
        INSERT INTO organizations (id, name, slug, owner_id, plan, is_personal)
        VALUES ('bench_org', 'Bench Org', 'bench-org', 'bench_user', 'free', true)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .execute(pool)
    .await
    .expect("Failed to create benchmark organization");
}

async fn create_test_trigger(pool: &PgPool, trigger_id: &str) {
    ensure_test_user_and_org(pool).await;

    sqlx::query!(
        r#"
        INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled, is_stateful)
        VALUES ($1, 'bench_org', 'bench_user', 'Bench Trigger', 84532, 'reputation', true, true)
        ON CONFLICT (id) DO NOTHING
        "#,
        trigger_id
    )
    .execute(pool)
    .await
    .expect("Failed to create benchmark trigger");
}

// Benchmark: Single read latency (no cache vs cached)
async fn bench_single_read_latency() {
    println!("\n=== Benchmark: Single Read Latency ===");

    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "bench_single_read";
    create_test_trigger(&pool, trigger_id).await;

    // Setup state
    let state_data = json!({"ema": 75.5, "count": 100});

    // Benchmark uncached (TriggerStateManager)
    let uncached_manager = TriggerStateManager::new(pool.clone());
    uncached_manager
        .update_state(trigger_id, state_data.clone())
        .await
        .unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        uncached_manager.load_state(trigger_id).await.unwrap();
    }
    let uncached_duration = start.elapsed();
    let uncached_avg = uncached_duration.as_micros() / 100;

    // Benchmark cached (CachedStateManager)
    let cached_manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Warm up cache
    cached_manager.load_state(trigger_id).await.unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        cached_manager.load_state(trigger_id).await.unwrap();
    }
    let cached_duration = start.elapsed();
    let cached_avg = cached_duration.as_micros() / 100;

    let speedup = uncached_avg as f64 / cached_avg as f64;

    println!("Uncached (PostgreSQL): {}μs ({:.2}ms)", uncached_avg, uncached_avg as f64 / 1000.0);
    println!("Cached (Redis):        {}μs ({:.2}ms)", cached_avg, cached_avg as f64 / 1000.0);
    println!("Speedup:               {:.1}x", speedup);

    // Cleanup
    cached_manager.delete_state(trigger_id).await.unwrap();
}

// Benchmark: Write latency (no cache vs cached)
async fn bench_write_latency() {
    println!("\n=== Benchmark: Write Latency ===");

    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "bench_write";
    create_test_trigger(&pool, trigger_id).await;

    // Benchmark uncached writes
    let uncached_manager = TriggerStateManager::new(pool.clone());

    let start = Instant::now();
    for i in 0..100 {
        uncached_manager
            .update_state(trigger_id, json!({"iteration": i}))
            .await
            .unwrap();
    }
    let uncached_duration = start.elapsed();
    let uncached_avg = uncached_duration.as_micros() / 100;

    // Benchmark cached writes
    let cached_manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    let start = Instant::now();
    for i in 0..100 {
        cached_manager
            .update_state(trigger_id, json!({"iteration": i}))
            .await
            .unwrap();
    }
    let cached_duration = start.elapsed();
    let cached_avg = cached_duration.as_micros() / 100;

    let overhead = (cached_avg as f64 - uncached_avg as f64) / uncached_avg as f64 * 100.0;

    println!("Uncached (PostgreSQL only): {}μs ({:.2}ms)", uncached_avg, uncached_avg as f64 / 1000.0);
    println!("Cached (PostgreSQL + Redis): {}μs ({:.2}ms)", cached_avg, cached_avg as f64 / 1000.0);
    println!("Overhead:                    {:.1}%", overhead);

    // Cleanup
    cached_manager.delete_state(trigger_id).await.unwrap();
}

// Benchmark: Mixed workload (80% read, 20% write)
async fn bench_mixed_workload() {
    println!("\n=== Benchmark: Mixed Workload (80% read, 20% write) ===");

    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;
    let trigger_id = "bench_mixed";
    create_test_trigger(&pool, trigger_id).await;

    // Initialize state
    let initial_state = json!({"ema": 75.5, "count": 0});

    // Benchmark uncached
    let uncached_manager = TriggerStateManager::new(pool.clone());
    uncached_manager
        .update_state(trigger_id, initial_state.clone())
        .await
        .unwrap();

    let start = Instant::now();
    for i in 0..1000 {
        if i % 5 == 0 {
            // 20% writes
            uncached_manager
                .update_state(trigger_id, json!({"count": i}))
                .await
                .unwrap();
        } else {
            // 80% reads
            uncached_manager.load_state(trigger_id).await.unwrap();
        }
    }
    let uncached_duration = start.elapsed();
    let uncached_throughput = 1000.0 / uncached_duration.as_secs_f64();

    // Benchmark cached
    let cached_manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);
    cached_manager
        .update_state(trigger_id, initial_state.clone())
        .await
        .unwrap();

    let start = Instant::now();
    for i in 0..1000 {
        if i % 5 == 0 {
            // 20% writes
            cached_manager
                .update_state(trigger_id, json!({"count": i}))
                .await
                .unwrap();
        } else {
            // 80% reads
            cached_manager.load_state(trigger_id).await.unwrap();
        }
    }
    let cached_duration = start.elapsed();
    let cached_throughput = 1000.0 / cached_duration.as_secs_f64();

    let improvement = (cached_throughput - uncached_throughput) / uncached_throughput * 100.0;

    println!("Uncached: {:?} ({:.0} ops/sec)", uncached_duration, uncached_throughput);
    println!("Cached:   {:?} ({:.0} ops/sec)", cached_duration, cached_throughput);
    println!("Improvement: {:.1}%", improvement);

    // Cleanup
    cached_manager.delete_state(trigger_id).await.unwrap();
}

// Benchmark: Concurrent access (simulating high event rate)
async fn bench_concurrent_access() {
    println!("\n=== Benchmark: Concurrent Access (100 triggers, 1000 events) ===");

    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;

    // Create 100 triggers
    let trigger_ids: Vec<String> = (0..100)
        .map(|i| format!("bench_concurrent_{}", i))
        .collect();

    for trigger_id in &trigger_ids {
        create_test_trigger(&pool, trigger_id).await;
    }

    // Benchmark uncached
    let uncached_manager = TriggerStateManager::new(pool.clone());

    // Initialize all triggers
    for trigger_id in &trigger_ids {
        uncached_manager
            .update_state(trigger_id, json!({"ema": 70.0, "count": 0}))
            .await
            .unwrap();
    }

    let start = Instant::now();
    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let trigger_id = trigger_ids[i % 100].clone();
            let mgr = TriggerStateManager::new(pool.clone());

            tokio::spawn(async move {
                let state = mgr.load_state(&trigger_id).await.unwrap();
                if let Some(mut current) = state {
                    let count = current["count"].as_i64().unwrap_or(0);
                    current["count"] = json!(count + 1);
                    mgr.update_state(&trigger_id, current).await.unwrap();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
    let uncached_duration = start.elapsed();
    let uncached_throughput = 1000.0 / uncached_duration.as_secs_f64();

    // Benchmark cached
    let cached_manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Re-initialize all triggers
    for trigger_id in &trigger_ids {
        cached_manager
            .update_state(trigger_id, json!({"ema": 70.0, "count": 0}))
            .await
            .unwrap();
    }

    let start = Instant::now();
    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let trigger_id = trigger_ids[i % 100].clone();
            let mgr = CachedStateManager::new(pool.clone(), redis.clone(), 300);

            tokio::spawn(async move {
                let state = mgr.load_state(&trigger_id).await.unwrap();
                if let Some(mut current) = state {
                    let count = current["count"].as_i64().unwrap_or(0);
                    current["count"] = json!(count + 1);
                    mgr.update_state(&trigger_id, current).await.unwrap();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
    let cached_duration = start.elapsed();
    let cached_throughput = 1000.0 / cached_duration.as_secs_f64();

    let improvement = (cached_throughput - uncached_throughput) / uncached_throughput * 100.0;

    println!("Uncached: {:?} ({:.0} events/sec)", uncached_duration, uncached_throughput);
    println!("Cached:   {:?} ({:.0} events/sec)", cached_duration, cached_throughput);
    println!("Improvement: {:.1}%", improvement);

    // Cleanup
    for trigger_id in &trigger_ids {
        cached_manager.delete_state(trigger_id).await.unwrap();
    }
}

// Benchmark: Cache hit rate
async fn bench_cache_hit_rate() {
    println!("\n=== Benchmark: Cache Hit Rate Measurement ===");

    let pool = setup_test_db().await;
    let redis = setup_test_redis().await;

    // Create 50 triggers
    let trigger_ids: Vec<String> = (0..50)
        .map(|i| format!("bench_hit_rate_{}", i))
        .collect();

    for trigger_id in &trigger_ids {
        create_test_trigger(&pool, trigger_id).await;
    }

    let manager = CachedStateManager::new(pool.clone(), redis.clone(), 300);

    // Initialize all triggers
    for trigger_id in &trigger_ids {
        manager
            .update_state(trigger_id, json!({"ema": 75.0, "count": 0}))
            .await
            .unwrap();
    }

    // Simulate realistic access pattern (Zipf distribution)
    // 80% of reads go to 20% of triggers
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    let total_ops = 10000;

    let start = Instant::now();

    for i in 0..total_ops {
        let trigger_idx = if i % 10 < 8 {
            // 80% of traffic to top 20% triggers (0-9)
            i % 10
        } else {
            // 20% of traffic to bottom 80% triggers (10-49)
            10 + (i % 40)
        };

        let trigger_id = &trigger_ids[trigger_idx];

        // Clear cache randomly to simulate cold reads (10%)
        if i % 10 == 0 {
            let mut conn = redis.clone();
            use redis::AsyncCommands;
            let _: Result<(), redis::RedisError> = conn.del(format!("trigger:state:{}", trigger_id)).await;
            cache_misses += 1;
        } else {
            cache_hits += 1;
        }

        manager.load_state(trigger_id).await.unwrap();
    }

    let duration = start.elapsed();
    let throughput = total_ops as f64 / duration.as_secs_f64();
    let hit_rate = cache_hits as f64 / total_ops as f64 * 100.0;

    println!("Total operations: {}", total_ops);
    println!("Cache hits:       {} ({:.1}%)", cache_hits, hit_rate);
    println!("Cache misses:     {} ({:.1}%)", cache_misses, 100.0 - hit_rate);
    println!("Throughput:       {:.0} ops/sec", throughput);
    println!("Duration:         {:?}", duration);

    // Cleanup
    for trigger_id in &trigger_ids {
        manager.delete_state(trigger_id).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    println!("====================================");
    println!("State Manager Cache Benchmark Suite");
    println!("====================================");

    bench_single_read_latency().await;
    bench_write_latency().await;
    bench_mixed_workload().await;
    bench_concurrent_access().await;
    bench_cache_hit_rate().await;

    println!("\n====================================");
    println!("Benchmark suite completed!");
    println!("====================================");
}
