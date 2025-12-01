# Circuit Breaker Integration Guide

## Overview

The Circuit Breaker pattern has been implemented to prevent cascade failures when triggers repeatedly fail. This document explains how to integrate it into the event-processor's listener.

## State Machine

```text
CLOSED (normal operation)
  ↓ (10 consecutive failures)
OPEN (trigger disabled, rejects all events)
  ↓ (after 1 hour)
HALF-OPEN (test mode, allows 1 event through)
  ↓ (success) → CLOSED
  ↓ (failure) → OPEN
```

## Integration Steps

### Step 1: Import Circuit Breaker

Add to `listener.rs`:

```rust
use crate::circuit_breaker::CircuitBreaker;
```

### Step 2: Check Circuit Breaker Before Evaluation

Modify the trigger evaluation loop in `process_event()` function:

```rust
// Evaluate each trigger
let mut matched_count = 0;
let trigger_count = triggers.len();
for trigger in &triggers {
    // *** NEW: Create circuit breaker for this trigger ***
    let circuit_breaker = match CircuitBreaker::new(trigger.id.clone(), db_pool.clone()).await {
        Ok(cb) => cb,
        Err(e) => {
            tracing::error!(
                trigger_id = %trigger.id,
                error = %e,
                "Failed to create circuit breaker, skipping trigger"
            );
            continue;
        }
    };

    // *** NEW: Check if circuit breaker allows call ***
    match circuit_breaker.call_allowed().await {
        Ok(false) => {
            tracing::warn!(
                trigger_id = %trigger.id,
                trigger_name = %trigger.name,
                state = ?circuit_breaker.get_state().await,
                "Circuit breaker OPEN - skipping trigger"
            );
            continue;
        }
        Err(e) => {
            tracing::error!(
                trigger_id = %trigger.id,
                error = %e,
                "Circuit breaker check failed, skipping trigger"
            );
            continue;
        }
        Ok(true) => {
            // Circuit is closed or half-open, proceed with evaluation
        }
    }

    // Get conditions for this trigger from the batch-loaded map
    let conditions = conditions_map
        .get(&trigger.id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    // Evaluate conditions against the event
    let matches = if trigger.is_stateful {
        trigger_engine::evaluate_trigger_stateful(trigger, conditions, &event, state_manager)
            .await
    } else {
        trigger_engine::evaluate_trigger(conditions, &event)
    };

    // *** NEW: Handle evaluation result with circuit breaker ***
    match matches {
        Ok(true) => {
            // Trigger matched - record success
            if let Err(e) = circuit_breaker.record_success().await {
                tracing::error!(
                    trigger_id = %trigger.id,
                    error = %e,
                    "Failed to record circuit breaker success"
                );
            }

            matched_count += 1;
            tracing::info!(
                trigger_id = %trigger.id,
                trigger_name = %trigger.name,
                "Trigger matched"
            );

            // Enqueue actions...
            let actions = actions_map
                .get(&trigger.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            for action in actions {
                let action_type = ActionType::from_str(&action.action_type)
                    .context("Failed to parse action_type")?;

                let job = ActionJob::new(
                    &trigger.id,
                    &event.id,
                    action_type,
                    action.priority,
                    action.config.clone(),
                );

                job_queue.enqueue(&job).await?;

                tracing::debug!(
                    job_id = %job.id,
                    action_type = %job.action_type,
                    "Enqueued action job"
                );
            }
        }
        Ok(false) => {
            // Trigger did not match - still record success (no error occurred)
            if let Err(e) = circuit_breaker.record_success().await {
                tracing::error!(
                    trigger_id = %trigger.id,
                    error = %e,
                    "Failed to record circuit breaker success"
                );
            }

            tracing::debug!(
                trigger_id = %trigger.id,
                trigger_name = %trigger.name,
                "Trigger did not match"
            );
        }
        Err(e) => {
            // Evaluation failed - record failure
            if let Err(cb_error) = circuit_breaker.record_failure().await {
                tracing::error!(
                    trigger_id = %trigger.id,
                    error = %cb_error,
                    "Failed to record circuit breaker failure"
                );
            }

            tracing::error!(
                trigger_id = %trigger.id,
                trigger_name = %trigger.name,
                error = %e,
                "Trigger evaluation failed"
            );
        }
    }
}
```

## Configuration

Per-trigger configuration is stored in `triggers.circuit_breaker_config` (JSONB):

```json
{
  "failure_threshold": 10,
  "recovery_timeout_seconds": 3600,
  "half_open_max_calls": 1
}
```

### Adjusting Thresholds

To change the failure threshold for a specific trigger:

```sql
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
  circuit_breaker_config,
  '{failure_threshold}',
  '5'
)
WHERE id = 'trigger-id-here';
```

To change the recovery timeout to 30 minutes:

```sql
UPDATE triggers
SET circuit_breaker_config = jsonb_set(
  circuit_breaker_config,
  '{recovery_timeout_seconds}',
  '1800'
)
WHERE id = 'trigger-id-here';
```

## Monitoring

### Check Circuit Breaker State

Query all triggers with open circuits:

```sql
SELECT
  id,
  name,
  circuit_breaker_state->>'state' AS state,
  (circuit_breaker_state->>'failure_count')::int AS failures,
  circuit_breaker_state->>'opened_at' AS opened_at
FROM triggers
WHERE circuit_breaker_state->>'state' IN ('Open', 'HalfOpen')
ORDER BY opened_at DESC;
```

### Reset Circuit Breaker

Manually reset a circuit breaker to Closed state:

```sql
UPDATE triggers
SET circuit_breaker_state = '{
  "state": "Closed",
  "failure_count": 0,
  "half_open_calls": 0
}'::jsonb
WHERE id = 'trigger-id-here';
```

### View Failure History

Check recent failures for a trigger:

```sql
SELECT
  id,
  name,
  (circuit_breaker_state->>'failure_count')::int AS consecutive_failures,
  circuit_breaker_state->>'last_failure_time' AS last_failure,
  circuit_breaker_state->>'state' AS current_state
FROM triggers
WHERE (circuit_breaker_state->>'failure_count')::int > 0
ORDER BY last_failure DESC
LIMIT 20;
```

## Metrics (Future Prometheus Integration)

The circuit breaker is instrumented for future Prometheus metrics:

```rust
// Metrics to expose (add when Prometheus integration is complete)
// - circuit_breaker_state_transitions_total{trigger_id, from_state, to_state}
// - circuit_breaker_failure_count{trigger_id}
// - circuit_breaker_open_duration_seconds{trigger_id}
// - circuit_breaker_half_open_calls_total{trigger_id}
```

## Testing

Run the full test suite:

```bash
cd rust-backend
export DATABASE_URL="postgresql://user:pass@localhost:5432/erc8004_backend"
cargo test --package event-processor circuit_breaker
```

Test breakdown:
- 5 unit tests (data structures, serialization)
- 17 integration tests (state transitions, concurrency, persistence)

Total: 22 tests

## Performance Considerations

1. **Database Access**: Circuit breaker state is persisted to PostgreSQL on every state transition. This adds ~1-2ms latency per transition.

2. **Graceful Degradation**: If database persistence fails, the circuit breaker continues working with in-memory state (logged as warning).

3. **Memory Usage**: Each circuit breaker instance uses ~200 bytes (config + state + Arc overhead).

4. **Concurrency**: Uses `Arc<RwLock<>>` for thread-safe access. Multiple readers can access state simultaneously, but state changes are serialized.

## Error Handling

The circuit breaker implements graceful error handling:

1. **Database errors during persistence**: Logged as warning, continues with in-memory state
2. **Invalid state in database**: Resets to Closed state
3. **Trigger not found**: Returns error, trigger is skipped

## Example Scenarios

### Scenario 1: Trigger Failing Repeatedly

1. Trigger evaluation fails 10 times consecutively
2. Circuit transitions from Closed → Open
3. All subsequent events are rejected (fail-fast)
4. After 1 hour, circuit transitions to Half-Open
5. Next event is allowed through as a test
6. If successful, circuit transitions to Closed
7. If failed, circuit returns to Open for another hour

### Scenario 2: Intermittent Failures

1. Trigger fails 5 times
2. Then succeeds once
3. Failure count resets to 0
4. Circuit remains Closed
5. Trigger continues normal operation

### Scenario 3: Service Restart

1. Circuit was Open before restart (state persisted to DB)
2. Service restarts, new CircuitBreaker instance created
3. State loaded from database (still Open)
4. Circuit continues protecting trigger after restart
5. Recovery timeout continues from original `opened_at` timestamp

## Best Practices

1. **Set appropriate thresholds**: Default 10 failures may be too high or too low depending on trigger criticality
2. **Monitor open circuits**: Set up alerts for triggers stuck in Open state
3. **Review failure patterns**: Analyze which triggers fail frequently and why
4. **Adjust recovery timeouts**: Critical triggers may need longer recovery periods
5. **Test failure scenarios**: Simulate failures to verify circuit breaker behavior

## Troubleshooting

### Circuit stuck in Open state

Check if recovery timeout has passed:

```sql
SELECT
  id,
  name,
  circuit_breaker_state->>'opened_at' AS opened_at,
  NOW() - (circuit_breaker_state->>'opened_at')::timestamptz AS time_since_open,
  (circuit_breaker_config->>'recovery_timeout_seconds')::int AS timeout_seconds
FROM triggers
WHERE circuit_breaker_state->>'state' = 'Open';
```

### High failure rates

Investigate root cause:

1. Check action_results table for error messages
2. Review trigger conditions for bugs
3. Verify external services are accessible
4. Check database query performance

### Circuit breaker not activating

Verify configuration:

```sql
SELECT
  id,
  name,
  circuit_breaker_config,
  circuit_breaker_state
FROM triggers
WHERE id = 'trigger-id-here';
```

Ensure `failure_threshold` is not too high and that failures are being recorded correctly.

## Future Enhancements

1. **Adaptive thresholds**: Automatically adjust failure threshold based on historical success rate
2. **Health checks**: Proactive health checks before transitioning from Open to Half-Open
3. **Custom recovery strategies**: Different recovery patterns for different failure types
4. **Dashboard integration**: Real-time circuit breaker state visualization
5. **Auto-remediation**: Automatically restart services or scale resources on persistent failures
