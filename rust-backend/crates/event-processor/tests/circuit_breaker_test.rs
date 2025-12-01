//! Integration tests for Circuit Breaker
//!
//! Tests cover:
//! - State transitions (Closed → Open → Half-Open → Closed)
//! - Failure threshold behavior
//! - Recovery timeout
//! - Half-open state behavior
//! - Concurrent access
//! - Database persistence and recovery

use anyhow::Result;
use chrono::{Duration, Utc};
use event_processor::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, CircuitState};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Barrier;

// Test database setup helper
async fn setup_test_db() -> Result<PgPool> {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url).await?;
    Ok(pool)
}

// Create a test trigger with custom circuit breaker config
async fn create_test_trigger(
    pool: &PgPool,
    trigger_id: &str,
    config: Option<CircuitBreakerConfig>,
    state: Option<CircuitBreakerState>,
) -> Result<()> {
    // First ensure we have a test user and organization
    let user_id = format!("test_user_{}", trigger_id);
    let org_id = format!("test_org_{}", trigger_id);

    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
        user_id,
        format!("user_{}", trigger_id),
        format!("{}@test.com", user_id),
        "test_hash"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO organizations (id, name, slug, owner_id, is_personal, created_at)
        VALUES ($1, $2, $3, $4, true, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
        org_id,
        format!("Org {}", trigger_id),
        format!("org-{}", trigger_id),
        user_id
    )
    .execute(pool)
    .await?;

    let config_json = config
        .map(|c| serde_json::to_value(c).unwrap())
        .or_else(|| {
            Some(serde_json::json!({
                "failure_threshold": 10,
                "recovery_timeout_seconds": 3600,
                "half_open_max_calls": 1
            }))
        });

    let state_json = state.map(|s| serde_json::to_value(s).unwrap()).or_else(|| {
        Some(serde_json::json!({
            "state": "Closed",
            "failure_count": 0,
            "half_open_calls": 0
        }))
    });

    sqlx::query!(
        r#"
        INSERT INTO triggers (
            id, organization_id, user_id, name, description, chain_id, registry,
            enabled, is_stateful, circuit_breaker_config, circuit_breaker_state
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (id) DO UPDATE
        SET circuit_breaker_config = EXCLUDED.circuit_breaker_config,
            circuit_breaker_state = EXCLUDED.circuit_breaker_state
        "#,
        trigger_id,
        org_id,
        user_id,
        format!("Test Trigger {}", trigger_id),
        "Test trigger for circuit breaker",
        84532,
        "reputation",
        true,
        false,
        config_json,
        state_json
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Cleanup test trigger
async fn cleanup_test_trigger(pool: &PgPool, trigger_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM triggers WHERE id = $1", trigger_id)
        .execute(pool)
        .await?;

    let user_id = format!("test_user_{}", trigger_id);
    let org_id = format!("test_org_{}", trigger_id);

    sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await?;

    Ok(())
}

// ========================================================================
// State Transition Tests (5 tests)
// ========================================================================

#[tokio::test]
async fn test_transition_closed_to_open() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_closed_to_open";

    // Create trigger with low failure threshold for faster testing
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };
    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Initial state should be Closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);
    assert!(cb.call_allowed().await?);

    // Record 2 failures (below threshold)
    cb.record_failure().await?;
    cb.record_failure().await?;
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    // Record 3rd failure (reaches threshold)
    cb.record_failure().await?;
    assert_eq!(cb.get_state().await, CircuitState::Open);

    // Calls should now be rejected
    assert!(!cb.call_allowed().await?);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_transition_open_to_half_open() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_open_to_half_open";

    // Create trigger with short recovery timeout for testing
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 1, // 1 second
        half_open_max_calls: 1,
    };

    // Create in Open state with opened_at in the past
    let state = CircuitBreakerState {
        state: CircuitState::Open,
        failure_count: 10,
        last_failure_time: Some(Utc::now() - Duration::seconds(2)),
        opened_at: Some(Utc::now() - Duration::seconds(2)),
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Initial state should be Open
    assert_eq!(cb.get_state().await, CircuitState::Open);

    // First call_allowed should transition to Half-Open (timeout passed)
    assert!(cb.call_allowed().await?);
    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_transition_half_open_to_closed_on_success() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_half_open_to_closed";

    let config = CircuitBreakerConfig::default();
    let state = CircuitBreakerState {
        state: CircuitState::HalfOpen,
        failure_count: 10,
        last_failure_time: Some(Utc::now()),
        opened_at: Some(Utc::now()),
        half_open_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    // Record success should transition to Closed
    cb.record_success().await?;
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_transition_half_open_to_open_on_failure() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_half_open_to_open";

    let config = CircuitBreakerConfig::default();
    let state = CircuitBreakerState {
        state: CircuitState::HalfOpen,
        failure_count: 10,
        last_failure_time: Some(Utc::now()),
        opened_at: Some(Utc::now()),
        half_open_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    // Record failure should transition back to Open
    cb.record_failure().await?;
    assert_eq!(cb.get_state().await, CircuitState::Open);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_full_cycle_closed_open_half_open_closed() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_full_cycle";

    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        recovery_timeout_seconds: 1, // 1 second
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Start in Closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    // Transition to Open
    cb.record_failure().await?;
    cb.record_failure().await?;
    assert_eq!(cb.get_state().await, CircuitState::Open);

    // Wait for recovery timeout
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Transition to Half-Open
    assert!(cb.call_allowed().await?);
    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    // Transition back to Closed
    cb.record_success().await?;
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Failure Threshold Tests (2 tests)
// ========================================================================

#[tokio::test]
async fn test_failure_threshold_not_reached() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_threshold_not_reached";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Record 9 failures (below threshold of 10)
    for _ in 0..9 {
        cb.record_failure().await?;
    }

    // Should still be Closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);
    assert!(cb.call_allowed().await?);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_failure_threshold_exactly_reached() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_threshold_reached";

    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Record exactly 5 failures (threshold)
    for _ in 0..5 {
        cb.record_failure().await?;
    }

    // Should transition to Open
    assert_eq!(cb.get_state().await, CircuitState::Open);
    assert!(!cb.call_allowed().await?);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Recovery Timeout Tests (2 tests)
// ========================================================================

#[tokio::test]
async fn test_recovery_timeout_not_passed() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_timeout_not_passed";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600, // 1 hour
        half_open_max_calls: 1,
    };

    let state = CircuitBreakerState {
        state: CircuitState::Open,
        failure_count: 10,
        last_failure_time: Some(Utc::now()),
        opened_at: Some(Utc::now()), // Just opened
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Should remain Open (timeout not passed)
    assert!(!cb.call_allowed().await?);
    assert_eq!(cb.get_state().await, CircuitState::Open);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_recovery_timeout_passed() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_timeout_passed";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 1, // 1 second
        half_open_max_calls: 1,
    };

    let state = CircuitBreakerState {
        state: CircuitState::Open,
        failure_count: 10,
        last_failure_time: Some(Utc::now() - Duration::seconds(2)),
        opened_at: Some(Utc::now() - Duration::seconds(2)), // Opened 2 seconds ago
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Should transition to Half-Open (timeout passed)
    assert!(cb.call_allowed().await?);
    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Half-Open Behavior Tests (2 tests)
// ========================================================================

#[tokio::test]
async fn test_half_open_allows_limited_calls() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_half_open_limited";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 2, // Allow 2 calls
    };

    let state = CircuitBreakerState {
        state: CircuitState::HalfOpen,
        failure_count: 10,
        last_failure_time: Some(Utc::now()),
        opened_at: Some(Utc::now()),
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // First call allowed
    assert!(cb.call_allowed().await?);
    // Second call allowed
    assert!(cb.call_allowed().await?);
    // Third call rejected (max reached)
    assert!(!cb.call_allowed().await?);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_half_open_single_call_default() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_half_open_single";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1, // Default: allow 1 call
    };

    let state = CircuitBreakerState {
        state: CircuitState::HalfOpen,
        failure_count: 10,
        last_failure_time: Some(Utc::now()),
        opened_at: Some(Utc::now()),
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // First call allowed
    assert!(cb.call_allowed().await?);
    // Second call rejected
    assert!(!cb.call_allowed().await?);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Concurrent Access Tests (2 tests)
// ========================================================================

#[tokio::test]
async fn test_concurrent_failure_recording() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_concurrent_failures";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = Arc::new(CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?);

    // Create 5 concurrent tasks that each record 2 failures (total 10)
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    for _ in 0..5 {
        let cb_clone = Arc::clone(&cb);
        let barrier_clone = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            barrier_clone.wait().await; // Synchronize start
            cb_clone.record_failure().await?;
            cb_clone.record_failure().await?;
            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    // Circuit should be Open (10 failures)
    assert_eq!(cb.get_state().await, CircuitState::Open);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_call_allowed_checks() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_concurrent_checks";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = Arc::new(CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?);

    // Create 10 concurrent call_allowed checks
    let mut handles = vec![];

    for _ in 0..10 {
        let cb_clone = Arc::clone(&cb);

        let handle = tokio::spawn(async move { cb_clone.call_allowed().await });

        handles.push(handle);
    }

    // Wait for all tasks and collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await??);
    }

    // All should be allowed (Closed state)
    assert!(results.iter().all(|&allowed| allowed));

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Database Persistence Tests (2 tests)
// ========================================================================

#[tokio::test]
async fn test_state_persisted_after_transition() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_persistence";

    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Transition to Open
    cb.record_failure().await?;
    cb.record_failure().await?;

    // Create new instance to verify persistence
    let cb2 = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;
    assert_eq!(cb2.get_state().await, CircuitState::Open);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_recovery_after_restart() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_restart";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 1,
        half_open_max_calls: 1,
    };

    let state = CircuitBreakerState {
        state: CircuitState::Open,
        failure_count: 10,
        last_failure_time: Some(Utc::now() - Duration::seconds(2)),
        opened_at: Some(Utc::now() - Duration::seconds(2)),
        half_open_calls: 0,
    };

    create_test_trigger(&pool, trigger_id, Some(config), Some(state)).await?;

    // Simulate restart by creating new instance
    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Should load Open state from database
    assert_eq!(cb.get_state().await, CircuitState::Open);

    // Should allow transition to Half-Open (timeout passed)
    assert!(cb.call_allowed().await?);
    assert_eq!(cb.get_state().await, CircuitState::HalfOpen);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

// ========================================================================
// Edge Cases and Special Scenarios (2 tests)
// ========================================================================

#[tokio::test]
async fn test_success_resets_failure_count_in_closed_state() -> Result<()> {
    let pool = setup_test_db().await?;
    let trigger_id = "circuit_breaker_test_reset_on_success";

    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        recovery_timeout_seconds: 3600,
        half_open_max_calls: 1,
    };

    create_test_trigger(&pool, trigger_id, Some(config), None).await?;

    let cb = CircuitBreaker::new(trigger_id.to_string(), pool.clone()).await?;

    // Record some failures
    cb.record_failure().await?;
    cb.record_failure().await?;
    cb.record_failure().await?;

    // Should still be Closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    // Record success should reset failure count
    cb.record_success().await?;

    // Now we should be able to have 9 more failures before opening
    for _ in 0..9 {
        cb.record_failure().await?;
    }
    assert_eq!(cb.get_state().await, CircuitState::Closed);

    // 10th failure should open
    cb.record_failure().await?;
    assert_eq!(cb.get_state().await, CircuitState::Open);

    cleanup_test_trigger(&pool, trigger_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_invalid_trigger_id_returns_error() -> Result<()> {
    let pool = setup_test_db().await?;

    let result = CircuitBreaker::new("nonexistent_trigger".to_string(), pool).await;
    assert!(result.is_err());

    Ok(())
}
