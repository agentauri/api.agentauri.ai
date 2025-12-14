//! PostgreSQL NOTIFY/LISTEN implementation
//!
//! Listens for new events and processes them through the trigger matching engine.
//! This is the PRIMARY event processing path (99% of events).
//! The polling fallback serves as a safety net for the remaining 1%.
//!
//! # Architecture
//!
//! This module implements CRITICAL FIX for silent task failures:
//! - Bounded concurrency with Semaphore (max 100 concurrent events)
//! - Task lifecycle tracking with JoinSet
//! - 30-second timeout per event processing
//! - Panic detection and recovery
//! - Metrics for task failures/panics

use anyhow::{Context, Result};
use event_processor::processor::process_event;
use event_processor::queue::RedisJobQueue;
use event_processor::state_manager::TriggerStateManager;
use redis::aio::MultiplexedConnection;
use serde::Deserialize;
use shared::DbPool;
use sqlx::postgres::PgListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Event notification payload from PostgreSQL NOTIFY
#[derive(Debug, Deserialize)]
struct EventNotification {
    event_id: String,
    chain_id: i32,
    block_number: i64,
    event_type: String,
    registry: String,
}

/// Maximum concurrent event processing tasks
/// Prevents unbounded task spawning during NOTIFY floods
const MAX_CONCURRENT_EVENTS: usize = 100;

/// Timeout for individual event processing (30 seconds)
/// Prevents slow queries from blocking indefinitely
const EVENT_PROCESSING_TIMEOUT: Duration = Duration::from_secs(30);

/// Start listening to PostgreSQL NOTIFY events
///
/// # Arguments
///
/// * `db_pool` - Database connection pool
/// * `redis_conn` - Redis connection for job queueing
///
/// # Critical Fix
///
/// This function implements bounded concurrency and task monitoring to prevent
/// silent failures. Key improvements over previous implementation:
/// - Semaphore limits concurrent tasks to 100 (prevents DOS)
/// - JoinSet tracks all spawned tasks (detects panics)
/// - 30-second timeout per event (prevents hangs)
/// - Metrics for task failures and panics
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

    // CRITICAL FIX: Bounded concurrency with semaphore
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_EVENTS));

    // CRITICAL FIX: Task tracking with JoinSet
    let mut tasks: JoinSet<Result<String>> = JoinSet::new();

    // Track consecutive errors for exponential backoff
    let mut consecutive_errors = 0u32;
    const MAX_CONSECUTIVE_ERRORS: u32 = 10;

    // Metrics tracking
    let mut total_tasks_spawned = 0u64;
    let mut total_tasks_succeeded = 0u64;
    let mut total_tasks_failed = 0u64;
    let mut total_tasks_panicked = 0u64;
    let mut total_tasks_timeout = 0u64;

    loop {
        tokio::select! {
            // Handle incoming NOTIFY events
            notification_result = listener.recv() => {
                match notification_result {
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

                        // CRITICAL FIX: Acquire semaphore permit (blocks if at max concurrency)
                        let permit = semaphore.clone().acquire_owned().await.unwrap();

                        // Clone resources for task
                        let db_pool = db_pool.clone();
                        let job_queue = job_queue.clone();
                        let event_id_clone = event_id.clone();

                        // CRITICAL FIX: Spawn task with JoinSet tracking
                        tasks.spawn(async move {
                            // Hold permit for task lifetime
                            let _permit = permit;

                            // Create state manager for this event processing
                            let state_manager = TriggerStateManager::new(db_pool.clone());

                            // CRITICAL FIX: Wrap with timeout
                            let result = tokio::time::timeout(
                                EVENT_PROCESSING_TIMEOUT,
                                process_event(&event_id_clone, &db_pool, &job_queue, &state_manager)
                            ).await;

                            match result {
                                Ok(Ok(())) => {
                                    tracing::debug!(event_id = %event_id_clone, "Event processed successfully");
                                    Ok(event_id_clone)
                                }
                                Ok(Err(e)) => {
                                    tracing::error!(
                                        event_id = %event_id_clone,
                                        error = %e,
                                        "Error processing event"
                                    );
                                    Err(e).context(format!("Event processing failed: {}", event_id_clone))
                                }
                                Err(_) => {
                                    tracing::error!(
                                        event_id = %event_id_clone,
                                        timeout_secs = EVENT_PROCESSING_TIMEOUT.as_secs(),
                                        error_id = "EVENT_PROCESSING_TIMEOUT",
                                        "Event processing timeout exceeded"
                                    );
                                    anyhow::bail!("Event processing timeout: {}", event_id_clone)
                                }
                            }
                        });

                        total_tasks_spawned += 1;

                        // Log metrics periodically (every 100 tasks)
                        if total_tasks_spawned.is_multiple_of(100) {
                            tracing::info!(
                                tasks_spawned = total_tasks_spawned,
                                tasks_succeeded = total_tasks_succeeded,
                                tasks_failed = total_tasks_failed,
                                tasks_panicked = total_tasks_panicked,
                                tasks_timeout = total_tasks_timeout,
                                active_tasks = tasks.len(),
                                "Event processing metrics"
                            );
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;

                        // FIX 3.5: Distinguish fatal vs transient errors (Medium Priority)
                        let error_str = e.to_string().to_lowercase();
                        let is_fatal = error_str.contains("authentication")
                            || error_str.contains("permission denied")
                            || error_str.contains("database does not exist")
                            || error_str.contains("relation does not exist");

                        if is_fatal {
                            tracing::error!(
                                error = %e,
                                error_id = "LISTENER_FATAL_ERROR",
                                "Fatal error in listener, cannot recover - exiting for restart"
                            );
                            anyhow::bail!("Fatal listener error (unrecoverable): {}", e);
                        }

                        // Calculate exponential backoff: min(2^errors, 60) seconds
                        let backoff_secs = std::cmp::min(2u64.pow(consecutive_errors), 60);

                        tracing::error!(
                            error = %e,
                            consecutive_errors = consecutive_errors,
                            backoff_secs = backoff_secs,
                            error_id = "LISTENER_TRANSIENT_ERROR",
                            "Transient error receiving notification, will retry with backoff"
                        );

                        // After too many consecutive errors, exit to trigger app restart
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            tracing::error!(
                                consecutive_errors = consecutive_errors,
                                max_allowed = MAX_CONSECUTIVE_ERRORS,
                                error_id = "LISTENER_MAX_ERRORS_EXCEEDED",
                                "Listener exceeded maximum consecutive errors, exiting for restart"
                            );
                            anyhow::bail!(
                                "Listener exceeded {} consecutive errors, exiting for restart",
                                MAX_CONSECUTIVE_ERRORS
                            );
                        }

                        // Emit metric for transient errors
                        #[cfg(feature = "metrics")]
                        metrics::counter!("event_processor.listener_transient_errors").increment(1);

                        // Exponential backoff before retry
                        tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    }
                }
            }

            // CRITICAL FIX: Monitor completed/failed/panicked tasks
            Some(task_result) = tasks.join_next() => {
                match task_result {
                    Ok(Ok(event_id)) => {
                        // Task completed successfully
                        total_tasks_succeeded += 1;
                        tracing::trace!(
                            event_id = %event_id,
                            "Event processing task completed successfully"
                        );
                    }
                    Ok(Err(e)) => {
                        // Task failed with error
                        total_tasks_failed += 1;

                        // Check if this was a timeout
                        if e.to_string().contains("timeout") {
                            total_tasks_timeout += 1;
                        }

                        tracing::error!(
                            error = %e,
                            "Event processing task failed"
                        );
                        // Note: We don't bail here - continue processing other events
                    }
                    Err(join_error) => {
                        // Task panicked!
                        total_tasks_panicked += 1;

                        tracing::error!(
                            error = %join_error,
                            error_id = "TASK_PANIC",
                            "CRITICAL: Event processing task panicked"
                        );

                        // Increment panic metric (would be Prometheus in production)
                        #[cfg(feature = "metrics")]
                        metrics::counter!("event_processor.spawned_tasks_panicked").increment(1);

                        // Note: We don't bail here - polling fallback will catch the event
                    }
                }
            }
        }
    }
}

// Note: process_event is now defined in processor.rs module
// This module only handles the NOTIFY/LISTEN mechanism

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use event_processor::queue::JobQueue; // FIX 1.2: Add missing import
    use shared::{ActionJob, ActionType}; // FIX 1.2: Add missing imports
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
            serde_json::json!({"agent_id": 42}), // event_data
        );

        queue.enqueue(&job).await.unwrap();

        let jobs = queue.get_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].trigger_id, "trigger-1");
        assert_eq!(jobs[0].event_id, "event-1");
    }
}
