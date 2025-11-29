//! PostgreSQL NOTIFY/LISTEN implementation
//!
//! Listens for new events and processes them through the trigger matching engine.

use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use serde::Deserialize;
use shared::models::{Event, Trigger, TriggerAction, TriggerCondition};
use shared::{ActionJob, ActionType, DbPool};
use sqlx::postgres::PgListener;
use std::collections::HashMap;
use std::str::FromStr;

use crate::queue::{JobQueue, RedisJobQueue};
use crate::state_manager::TriggerStateManager;
use crate::trigger_engine;

/// Event notification payload from PostgreSQL NOTIFY
#[derive(Debug, Deserialize)]
struct EventNotification {
    event_id: String,
    chain_id: i32,
    block_number: i64,
    event_type: String,
    registry: String,
}

/// Start listening to PostgreSQL NOTIFY events
///
/// # Arguments
///
/// * `db_pool` - Database connection pool
/// * `redis_conn` - Redis connection for job queueing
pub async fn start_listening(db_pool: DbPool, redis_conn: MultiplexedConnection) -> Result<()> {
    // Create PostgreSQL listener
    let mut listener = PgListener::connect_with(&db_pool)
        .await
        .context("Failed to create PostgreSQL listener")?;

    // Listen to the 'new_event' channel
    listener
        .listen("new_event")
        .await
        .context("Failed to listen to 'new_event' channel")?;

    tracing::info!("Listening for PostgreSQL NOTIFY events on channel 'new_event'");

    // Create job queue
    let job_queue = RedisJobQueue::new(redis_conn);

    // Track consecutive errors for exponential backoff
    let mut consecutive_errors = 0u32;
    const MAX_CONSECUTIVE_ERRORS: u32 = 10;

    loop {
        // Wait for a notification
        match listener.recv().await {
            Ok(notification) => {
                // Reset error counter on success
                consecutive_errors = 0;

                let payload = notification.payload();

                // Try to parse the enhanced JSON payload, fall back to raw event_id
                let event_id = match serde_json::from_str::<EventNotification>(payload) {
                    Ok(event_notif) => {
                        tracing::debug!(
                            "Received event notification: {} (chain_id={}, block={}, type={}, registry={})",
                            event_notif.event_id,
                            event_notif.chain_id,
                            event_notif.block_number,
                            event_notif.event_type,
                            event_notif.registry
                        );
                        event_notif.event_id
                    }
                    Err(parse_err) => {
                        // Fall back to treating payload as raw event_id for backward compatibility
                        tracing::warn!(
                            error = %parse_err,
                            payload = %payload,
                            "Failed to parse EventNotification JSON, treating as raw event_id"
                        );
                        payload.to_string()
                    }
                };

                // Process the event in a separate task
                let db_pool = db_pool.clone();
                let job_queue = job_queue.clone();
                tokio::spawn(async move {
                    // Create state manager for this event processing
                    let state_manager = TriggerStateManager::new(db_pool.clone());

                    if let Err(e) = process_event(&event_id, &db_pool, &job_queue, &state_manager).await {
                        tracing::error!("Error processing event {}: {}", event_id, e);
                    }
                });
            }
            Err(e) => {
                consecutive_errors += 1;

                // Calculate exponential backoff: min(2^errors, 60) seconds
                let backoff_secs = std::cmp::min(2u64.pow(consecutive_errors), 60);

                tracing::error!(
                    error = %e,
                    consecutive_errors = consecutive_errors,
                    backoff_secs = backoff_secs,
                    "Error receiving notification"
                );

                // After too many consecutive errors, exit to trigger app restart
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    anyhow::bail!(
                        "Listener exceeded {} consecutive errors, exiting for restart",
                        MAX_CONSECUTIVE_ERRORS
                    );
                }

                // Exponential backoff before retry
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
            }
        }
    }
}

/// Process a single event notification
///
/// # Arguments
///
/// * `event_id` - The ID of the event to process
/// * `db_pool` - Database connection pool
/// * `job_queue` - Job queue for enqueueing actions
/// * `state_manager` - State manager for stateful triggers
async fn process_event<Q: JobQueue>(
    event_id: &str,
    db_pool: &DbPool,
    job_queue: &Q,
    state_manager: &TriggerStateManager,
) -> Result<()> {
    // Fetch event from database
    let event = fetch_event(event_id, db_pool).await?;

    tracing::info!(
        "Processing event: {} (chain_id={}, registry={}, event_type={})",
        event.id,
        event.chain_id,
        event.registry,
        event.event_type
    );

    // Fetch matching triggers for this chain_id and registry
    let triggers = fetch_triggers(event.chain_id, &event.registry, db_pool).await?;

    if triggers.is_empty() {
        tracing::debug!(
            "No enabled triggers found for chain_id={}, registry={}",
            event.chain_id,
            event.registry
        );
        return Ok(());
    }

    tracing::debug!("Found {} triggers to evaluate", triggers.len());

    // Batch load all conditions and actions for these triggers (fixes N+1 query problem)
    let trigger_ids: Vec<String> = triggers.iter().map(|t| t.id.clone()).collect();
    let (conditions_map, actions_map) =
        fetch_trigger_relations(&trigger_ids, db_pool).await?;

    tracing::debug!(
        "Batch loaded conditions and actions for {} triggers (3 queries total)",
        triggers.len()
    );

    // Evaluate each trigger
    let mut matched_count = 0;
    let trigger_count = triggers.len();
    for trigger in &triggers {
        // Get conditions for this trigger from the batch-loaded map
        let conditions = conditions_map
            .get(&trigger.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Evaluate conditions against the event
        // Use stateful evaluation if trigger is stateful, otherwise use stateless
        let matches = if trigger.is_stateful {
            trigger_engine::evaluate_trigger_stateful(trigger, conditions, &event, state_manager)
                .await?
        } else {
            trigger_engine::evaluate_trigger(conditions, &event)?
        };

        if matches {
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

            for action in actions {
                // Parse action_type string to ActionType enum
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
        } else {
            tracing::debug!(
                trigger_id = %trigger.id,
                trigger_name = %trigger.name,
                "Trigger did not match"
            );
        }
    }

    tracing::info!(
        event_id = %event_id,
        triggers_evaluated = trigger_count,
        triggers_matched = matched_count,
        "Event processing complete"
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
///
/// # Performance
///
/// - Before: 1 + N + N queries (for N triggers)
/// - After: 1 + 2 queries (regardless of N)
/// - Example with 100 triggers: 201 queries → 3 queries (66x reduction)
///
/// # Arguments
///
/// * `trigger_ids` - List of trigger IDs to fetch relations for
/// * `db_pool` - Database connection pool
///
/// # Returns
///
/// Returns a tuple of (conditions_map, actions_map) where:
/// - conditions_map: HashMap<trigger_id, Vec<TriggerCondition>>
/// - actions_map: HashMap<trigger_id, Vec<TriggerAction>>
///
/// Triggers with no conditions/actions will not have entries in the maps.
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

    // Batch fetch all conditions with a single query using IN clause
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

    // Batch fetch all actions with a single query using IN clause
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
            .or_insert_with(Vec::new)
            .push(condition);
    }

    // Group actions by trigger_id
    let mut actions_map: HashMap<String, Vec<TriggerAction>> = HashMap::new();
    for action in actions {
        actions_map
            .entry(action.trigger_id.clone())
            .or_insert_with(Vec::new)
            .push(action);
    }

    Ok((conditions_map, actions_map))
}

/// Fetch conditions for a trigger
///
/// # Deprecated
///
/// This function is deprecated in favor of `fetch_trigger_relations` for batch loading.
/// It's kept for backward compatibility and single-trigger use cases.
#[allow(dead_code)]
async fn fetch_conditions(trigger_id: &str, db_pool: &DbPool) -> Result<Vec<TriggerCondition>> {
    sqlx::query_as::<_, TriggerCondition>(
        r#"
        SELECT id, trigger_id, condition_type, field, operator, value, config, created_at
        FROM trigger_conditions
        WHERE trigger_id = $1
        ORDER BY id
        "#,
    )
    .bind(trigger_id)
    .fetch_all(db_pool)
    .await
    .context("Failed to fetch trigger conditions from database")
}

/// Fetch actions for a trigger
///
/// # Deprecated
///
/// This function is deprecated in favor of `fetch_trigger_relations` for batch loading.
/// It's kept for backward compatibility and single-trigger use cases.
#[allow(dead_code)]
async fn fetch_actions(trigger_id: &str, db_pool: &DbPool) -> Result<Vec<TriggerAction>> {
    sqlx::query_as::<_, TriggerAction>(
        r#"
        SELECT id, trigger_id, action_type, priority, config, created_at
        FROM trigger_actions
        WHERE trigger_id = $1
        ORDER BY priority DESC, id
        "#,
    )
    .bind(trigger_id)
    .fetch_all(db_pool)
    .await
    .context("Failed to fetch trigger actions from database")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// Mock job queue for testing
    struct MockJobQueue {
        jobs: Arc<Mutex<Vec<ActionJob>>>,
    }

    impl MockJobQueue {
        fn new() -> Self {
            Self {
                jobs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_jobs(&self) -> Vec<ActionJob> {
            self.jobs.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl JobQueue for MockJobQueue {
        async fn enqueue(&self, job: &ActionJob) -> Result<()> {
            self.jobs.lock().unwrap().push(job.clone());
            Ok(())
        }
    }

    #[test]
    fn test_event_notification_parsing() {
        let json = r#"{
            "event_id": "test-123",
            "chain_id": 84532,
            "block_number": 1000,
            "event_type": "NewFeedback",
            "registry": "reputation"
        }"#;

        let notif: EventNotification = serde_json::from_str(json).unwrap();

        assert_eq!(notif.event_id, "test-123");
        assert_eq!(notif.chain_id, 84532);
        assert_eq!(notif.block_number, 1000);
        assert_eq!(notif.event_type, "NewFeedback");
        assert_eq!(notif.registry, "reputation");
    }

    #[tokio::test]
    async fn test_mock_job_queue() {
        let queue = MockJobQueue::new();

        let job = ActionJob::new(
            "trigger-1",
            "event-1",
            ActionType::Telegram,
            1,
            serde_json::json!({"chat_id": "123"}),
        );

        queue.enqueue(&job).await.unwrap();

        let jobs = queue.get_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].trigger_id, "trigger-1");
        assert_eq!(jobs[0].event_id, "event-1");
    }
}
