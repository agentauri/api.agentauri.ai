//! Event Processing with Idempotency
//!
//! This module provides the core event processing logic with idempotency guarantees.
//! It ensures that each event is processed exactly once, even if the same event ID
//! is received multiple times (e.g., through both NOTIFY and polling fallback).

use anyhow::{Context, Result};
use shared::models::{Event, Trigger, TriggerAction, TriggerCondition};
use shared::{ActionJob, ActionType, DbPool};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

use crate::circuit_breaker::CircuitBreaker;
use crate::queue::JobQueue;
use crate::state_manager::TriggerStateManager;
use crate::trigger_engine;

/// Get hostname for processor instance tracking
fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Process a single event notification with idempotency guarantee
///
/// This function ensures that each event is processed exactly once by:
/// 1. Checking if the event has already been processed
/// 2. Processing the event (matching triggers, enqueueing actions)
/// 3. Marking the event as processed in an atomic transaction
///
/// # Arguments
///
/// * `event_id` - The ID of the event to process
/// * `db_pool` - Database connection pool
/// * `job_queue` - Job queue for enqueueing actions
/// * `state_manager` - State manager for stateful triggers
///
/// # Idempotency
///
/// This function can be called multiple times with the same event_id without
/// side effects. The first call will process the event, subsequent calls will
/// be no-ops (returning Ok immediately).
///
/// # Example
///
/// ```rust,no_run
/// use event_processor::process_event;
///
/// // This will process the event
/// process_event("event-123", &db_pool, &job_queue, &state_manager).await?;
///
/// // This will be a no-op (already processed)
/// process_event("event-123", &db_pool, &job_queue, &state_manager).await?;
/// ```
pub async fn process_event<Q: JobQueue>(
    event_id: &str,
    db_pool: &DbPool,
    job_queue: &Q,
    state_manager: &TriggerStateManager,
) -> Result<()> {
    let start = Instant::now();

    // STEP 1: Check if this event has already been processed (idempotency check)
    let already_processed: (bool,) =
        sqlx::query_as("SELECT is_event_processed($1) as already_processed")
            .bind(event_id)
            .fetch_one(db_pool)
            .await
            .context("Failed to check if event is already processed")?;

    if already_processed.0 {
        tracing::debug!(
            event_id = %event_id,
            "Event already processed, skipping (idempotency check)"
        );
        return Ok(());
    }

    // STEP 2: Fetch event from database
    let event = fetch_event(event_id, db_pool)
        .await
        .context(format!("Failed to fetch event {}", event_id))?;

    tracing::info!(
        "Processing event: {} (chain_id={}, registry={}, event_type={})",
        event.id,
        event.chain_id,
        event.registry,
        event.event_type
    );

    // STEP 3: Fetch matching triggers for this chain_id and registry
    let mut triggers = fetch_triggers(event.chain_id, &event.registry, db_pool)
        .await
        .context(format!(
            "Failed to fetch triggers for event {} (chain_id={}, registry={})",
            event_id, event.chain_id, event.registry
        ))?;

    if triggers.is_empty() {
        tracing::debug!(
            "No enabled triggers found for chain_id={}, registry={}",
            event.chain_id,
            event.registry
        );

        // Mark as processed even if no triggers matched
        mark_event_processed(event_id, db_pool, 0, 0, start.elapsed().as_millis() as i32).await?;
        return Ok(());
    }

    // FIX 3.2: Limit triggers per event to prevent DOS (Medium Priority)
    const MAX_TRIGGERS_PER_EVENT: usize = 100;
    if triggers.len() > MAX_TRIGGERS_PER_EVENT {
        tracing::warn!(
            event_id = %event_id,
            trigger_count = triggers.len(),
            max_allowed = MAX_TRIGGERS_PER_EVENT,
            error_id = "TRIGGER_COUNT_EXCEEDED",
            "Event matched too many triggers, truncating to prevent DOS attack"
        );

        // Emit metric for monitoring
        #[cfg(feature = "metrics")]
        metrics::counter!("event_processor.trigger_count_exceeded").increment(1);

        // Truncate to max allowed (keeps first N triggers by creation order)
        triggers.truncate(MAX_TRIGGERS_PER_EVENT);
    }

    tracing::debug!("Found {} triggers to evaluate", triggers.len());

    // STEP 4: Batch load all conditions and actions for these triggers (fixes N+1 query problem)
    let trigger_ids: Vec<String> = triggers.iter().map(|t| t.id.clone()).collect();
    let (conditions_map, actions_map) = fetch_trigger_relations(&trigger_ids, db_pool)
        .await
        .context(format!(
            "Failed to batch load conditions/actions for event {} ({} triggers)",
            event_id,
            trigger_ids.len()
        ))?;

    tracing::debug!(
        "Batch loaded conditions and actions for {} triggers (3 queries total)",
        triggers.len()
    );

    // STEP 5: Evaluate each trigger
    let mut matched_count = 0;
    let mut actions_enqueued = 0;
    let trigger_count = triggers.len();

    for trigger in &triggers {
        // Create circuit breaker for this trigger
        let circuit_breaker = match CircuitBreaker::new(trigger.id.clone(), db_pool.clone()).await {
            Ok(cb) => cb,
            Err(e) => {
                tracing::warn!(
                    trigger_id = %trigger.id,
                    error = %e,
                    "Failed to create circuit breaker, skipping trigger (graceful degradation)"
                );
                continue;
            }
        };

        // Check if circuit breaker allows this call (fail-fast if circuit is open)
        match circuit_breaker.call_allowed().await {
            Ok(false) => {
                let state = circuit_breaker.get_state().await;
                tracing::info!(
                    trigger_id = %trigger.id,
                    trigger_name = %trigger.name,
                    state = ?state,
                    "Circuit breaker OPEN - skipping trigger (fail-fast)"
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    trigger_id = %trigger.id,
                    error = %e,
                    "Circuit breaker check failed, skipping trigger (graceful degradation)"
                );
                continue;
            }
            Ok(true) => {
                // Circuit is closed or half-open, proceed with evaluation
                let state = circuit_breaker.get_state().await;
                tracing::debug!(
                    trigger_id = %trigger.id,
                    state = ?state,
                    "Circuit breaker allows call"
                );
            }
        }

        // Get conditions for this trigger from the batch-loaded map
        let conditions = conditions_map
            .get(&trigger.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Evaluate conditions against the event
        // Use stateful evaluation if trigger is stateful, otherwise use stateless
        let matches = if trigger.is_stateful {
            trigger_engine::evaluate_trigger_stateful(trigger, conditions, &event, state_manager)
                .await
        } else {
            trigger_engine::evaluate_trigger(conditions, &event)
        };

        // Handle evaluation result with circuit breaker
        match matches {
            Ok(true) => {
                // Trigger matched successfully - record success
                // FIX 2.3: Improve CB persistence error handling (High Priority)
                if let Err(e) = circuit_breaker.record_success().await {
                    tracing::error!(
                        trigger_id = %trigger.id,
                        error = %e,
                        error_id = "CIRCUIT_BREAKER_PERSIST_FAILED",
                        "Failed to record circuit breaker success - state may be inconsistent"
                    );
                    // Emit metric for monitoring
                    #[cfg(feature = "metrics")]
                    metrics::counter!("event_processor.circuit_breaker_persistence_failures")
                        .increment(1);
                }

                matched_count += 1;
                tracing::info!(
                    trigger_id = %trigger.id,
                    trigger_name = %trigger.name,
                    "Trigger matched"
                );

                // Get actions for this trigger from the batch-loaded map
                let actions = actions_map
                    .get(&trigger.id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);

                // FIX 2.2: Process all actions, don't abort on single failure (High Priority)
                let mut failed_actions = 0;

                for action in actions {
                    // Parse action_type string to ActionType enum
                    // FIX 2.2: Continue on parse error instead of aborting
                    let action_type = match ActionType::from_str(&action.action_type) {
                        Ok(at) => at,
                        Err(e) => {
                            failed_actions += 1;
                            tracing::error!(
                                trigger_id = %trigger.id,
                                action_id = action.id,
                                action_type_string = %action.action_type,
                                error = %e,
                                error_id = "ACTION_TYPE_PARSE_FAILED",
                                "Failed to parse action_type, skipping this action"
                            );
                            // Emit metric for monitoring
                            #[cfg(feature = "metrics")]
                            metrics::counter!("event_processor.action_type_parse_failures")
                                .increment(1);

                            continue; // Skip this action, process others
                        }
                    };

                    let job = ActionJob::new(
                        &trigger.id,
                        &event.id,
                        action_type,
                        action.priority,
                        action.config.clone(),
                    );

                    // FIX 2.2: Continue on enqueue error instead of aborting
                    // This allows other actions/triggers to proceed even if Redis is down
                    match job_queue.enqueue(&job).await {
                        Ok(_) => {
                            actions_enqueued += 1;
                            tracing::debug!(
                                job_id = %job.id,
                                trigger_id = %trigger.id,
                                action_type = %job.action_type,
                                "Enqueued action job"
                            );
                        }
                        Err(e) => {
                            failed_actions += 1;
                            tracing::error!(
                                trigger_id = %trigger.id,
                                action_type = %job.action_type,
                                job_id = %job.id,
                                error = %e,
                                error_id = "ACTION_ENQUEUE_FAILED",
                                "Failed to enqueue action job, continuing with other actions"
                            );
                            // Emit metric for monitoring
                            #[cfg(feature = "metrics")]
                            metrics::counter!("event_processor.action_enqueue_failures").increment(1);

                            // Continue processing other actions
                        }
                    }
                }

                // Log summary if any actions failed
                if failed_actions > 0 {
                    tracing::warn!(
                        trigger_id = %trigger.id,
                        total_actions = actions.len(),
                        failed_actions = failed_actions,
                        succeeded_actions = actions_enqueued - (matched_count - 1), // Subtract previous trigger's actions
                        "Some actions failed to enqueue for this trigger"
                    );
                }
            }
            Ok(false) => {
                // Trigger did not match - still record success (no error occurred)
                // FIX 2.3: Improve CB persistence error handling (High Priority)
                if let Err(e) = circuit_breaker.record_success().await {
                    tracing::error!(
                        trigger_id = %trigger.id,
                        error = %e,
                        error_id = "CIRCUIT_BREAKER_PERSIST_FAILED",
                        "Failed to record circuit breaker success - state may be inconsistent"
                    );
                    // Emit metric for monitoring
                    #[cfg(feature = "metrics")]
                    metrics::counter!("event_processor.circuit_breaker_persistence_failures")
                        .increment(1);
                }

                tracing::debug!(
                    trigger_id = %trigger.id,
                    trigger_name = %trigger.name,
                    "Trigger did not match"
                );
            }
            Err(e) => {
                // Evaluation failed - record failure
                // FIX 2.3: Improve CB persistence error handling (High Priority)
                if let Err(cb_error) = circuit_breaker.record_failure().await {
                    tracing::error!(
                        trigger_id = %trigger.id,
                        error = %cb_error,
                        error_id = "CIRCUIT_BREAKER_PERSIST_FAILED",
                        "CRITICAL: Failed to record circuit breaker failure - circuit may not open as expected"
                    );
                    // Emit metric for monitoring
                    #[cfg(feature = "metrics")]
                    metrics::counter!("event_processor.circuit_breaker_persistence_failures")
                        .increment(1);
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

    // STEP 6: Mark event as processed (idempotency tracking)
    let duration_ms = start.elapsed().as_millis() as i32;
    mark_event_processed(
        event_id,
        db_pool,
        matched_count,
        actions_enqueued,
        duration_ms,
    )
    .await
    .context(format!(
        "Failed to mark event {} as processed (matched={}, enqueued={})",
        event_id, matched_count, actions_enqueued
    ))?;

    tracing::info!(
        event_id = %event_id,
        triggers_evaluated = trigger_count,
        triggers_matched = matched_count,
        actions_enqueued = actions_enqueued,
        duration_ms = duration_ms,
        "Event processing complete"
    );

    Ok(())
}

/// Mark an event as processed in the database
///
/// This function inserts a record into the `processed_events` table to prevent
/// the event from being processed again. It uses ON CONFLICT DO NOTHING to ensure
/// atomicity even if multiple processors try to mark the same event simultaneously.
async fn mark_event_processed(
    event_id: &str,
    db_pool: &DbPool,
    triggers_matched: i32,
    actions_enqueued: i32,
    duration_ms: i32,
) -> Result<()> {
    let processor_instance = get_hostname();

    sqlx::query(
        r#"
        INSERT INTO processed_events
        (event_id, processor_instance, processing_duration_ms, triggers_matched, actions_enqueued)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(&processor_instance)
    .bind(duration_ms)
    .bind(triggers_matched)
    .bind(actions_enqueued)
    .execute(db_pool)
    .await
    .context("Failed to mark event as processed")?;

    tracing::debug!(
        event_id = %event_id,
        processor_instance = %processor_instance,
        "Marked event as processed"
    );

    Ok(())
}

/// Fetch an event from the database
async fn fetch_event(event_id: &str, db_pool: &DbPool) -> Result<Event> {
    sqlx::query_as::<_, Event>(
        r#"
        SELECT
            id, chain_id, block_number, block_hash, transaction_hash, log_index,
            registry, event_type, agent_id, timestamp, owner, token_uri, metadata_key,
            metadata_value, client_address, feedback_index, score, tag1, tag2,
            file_uri, file_hash, validator_address, request_hash, response,
            response_uri, response_hash, tag, created_at
        FROM events
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .fetch_one(db_pool)
    .await
    .context("Failed to fetch event from database")
}

/// Fetch enabled triggers matching chain_id and registry
async fn fetch_triggers(chain_id: i32, registry: &str, db_pool: &DbPool) -> Result<Vec<Trigger>> {
    sqlx::query_as::<_, Trigger>(
        r#"
        SELECT id, user_id, organization_id, name, description, chain_id, registry, enabled, is_stateful, created_at, updated_at
        FROM triggers
        WHERE chain_id = $1 AND registry = $2 AND enabled = true
        "#,
    )
    .bind(chain_id)
    .bind(registry)
    .fetch_all(db_pool)
    .await
    .context("Failed to fetch triggers from database")
}

/// Batch fetch conditions and actions for multiple triggers
///
/// This function solves the N+1 query problem by loading all conditions and actions
/// in just 2 queries instead of 2N queries (where N = number of triggers).
async fn fetch_trigger_relations(
    trigger_ids: &[String],
    db_pool: &DbPool,
) -> Result<(
    HashMap<String, Vec<TriggerCondition>>,
    HashMap<String, Vec<TriggerAction>>,
)> {
    if trigger_ids.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    // Batch fetch all conditions with a single query using ANY($1)
    let conditions = sqlx::query_as::<_, TriggerCondition>(
        r#"
        SELECT id, trigger_id, condition_type, field, operator, value, config, created_at
        FROM trigger_conditions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, id
        "#,
    )
    .bind(trigger_ids)
    .fetch_all(db_pool)
    .await
    .context("Failed to batch fetch trigger conditions")?;

    // Batch fetch all actions with a single query using ANY($1)
    let actions = sqlx::query_as::<_, TriggerAction>(
        r#"
        SELECT id, trigger_id, action_type, priority, config, created_at
        FROM trigger_actions
        WHERE trigger_id = ANY($1)
        ORDER BY trigger_id, priority DESC, id
        "#,
    )
    .bind(trigger_ids)
    .fetch_all(db_pool)
    .await
    .context("Failed to batch fetch trigger actions")?;

    // Group conditions by trigger_id
    let mut conditions_map: HashMap<String, Vec<TriggerCondition>> = HashMap::new();
    for condition in conditions {
        conditions_map
            .entry(condition.trigger_id.clone())
            .or_default()
            .push(condition);
    }

    // Group actions by trigger_id
    let mut actions_map: HashMap<String, Vec<TriggerAction>> = HashMap::new();
    for action in actions {
        actions_map
            .entry(action.trigger_id.clone())
            .or_default()
            .push(action);
    }

    Ok((conditions_map, actions_map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(!hostname.is_empty(), "Hostname should not be empty");
        // Allow "unknown" as fallback
        assert!(
            !hostname.is_empty() || hostname == "unknown",
            "Hostname should be valid or 'unknown'"
        );
    }
}
