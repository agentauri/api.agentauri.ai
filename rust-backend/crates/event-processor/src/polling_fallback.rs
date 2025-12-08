//! Polling Fallback for Event Processing
//!
//! This module provides a backup mechanism for event processing that polls the database
//! for unprocessed events. This ensures zero event loss even during database downtime,
//! listener disconnection, or missed PostgreSQL NOTIFY notifications.
//!
//! ## Architecture
//!
//! The polling fallback works alongside the NOTIFY-based listener:
//! - **Primary path** (99% of events): PostgreSQL NOTIFY → PgListener → process_event()
//! - **Fallback path** (1% of events): Polling → discover unprocessed → process_event()
//!
//! ## Design Rationale
//!
//! PostgreSQL's `pg_notify()` is NOT persistent - if the listener is disconnected when
//! an event is inserted, the notification is lost forever. This creates a silent event
//! loss scenario that is unacceptable for a production system.
//!
//! The polling fallback provides:
//! 1. **Guaranteed processing**: Every event will eventually be processed
//! 2. **Self-healing**: Automatically recovers from missed notifications
//! 3. **Observability**: Metrics show how often fallback is used
//! 4. **Low overhead**: Polling interval is 60 seconds, minimal DB impact

use crate::processor::process_event;
use crate::queue::RedisJobQueue;
use crate::state_manager::TriggerStateManager;
use anyhow::{Context, Result};
use shared::db::DbPool;
use sqlx::FromRow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Maximum number of events to process per polling iteration
const MAX_EVENTS_PER_POLL: i64 = 100;

/// Polling interval in seconds
const POLL_INTERVAL_SECS: u64 = 60;

/// Maximum failures per batch before aborting (FIX 3.1 - Medium Priority)
/// Prevents continuing to process when there's a systemic issue (e.g., DB down)
const MAX_FAILURES_PER_BATCH: usize = 10;

/// Maximum polling iterations before requiring restart (FIX 3.3 - Medium Priority)
/// Prevents infinite loops and memory leaks from long-running processes
/// At 60s intervals, 1M iterations = ~2 years uptime (reasonable restart cycle)
const MAX_POLLING_ITERATIONS: u64 = 1_000_000;

/// Simplified event structure for polling queries
#[derive(Debug, Clone, FromRow)]
struct UnprocessedEvent {
    id: String,
    chain_id: i32, // matches INTEGER in PostgreSQL view
    block_number: i64,
    #[allow(dead_code)] // Used in SQL query and debug logs
    registry: String,
    #[allow(dead_code)] // Used in SQL query and debug logs
    event_type: String,
}

/// Polling fallback mechanism for event processing
///
/// This struct implements a background polling loop that queries the database
/// for events that have not been processed yet. It serves as a safety net
/// to catch any events that were missed by the NOTIFY-based listener.
pub struct PollingFallback {
    db_pool: DbPool,
    job_queue: RedisJobQueue,
    state_manager: Arc<TriggerStateManager>,
    last_poll_time: Arc<RwLock<Option<std::time::Instant>>>,
    events_recovered: Arc<RwLock<u64>>,
}

impl PollingFallback {
    /// Create a new polling fallback instance
    pub fn new(
        db_pool: DbPool,
        job_queue: RedisJobQueue,
        state_manager: Arc<TriggerStateManager>,
    ) -> Self {
        Self {
            db_pool,
            job_queue,
            state_manager,
            last_poll_time: Arc::new(RwLock::new(None)),
            events_recovered: Arc::new(RwLock::new(0)),
        }
    }

    /// Start the polling fallback loop
    ///
    /// This function runs indefinitely, polling the database every 60 seconds
    /// for unprocessed events. It should be spawned as a separate Tokio task.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let fallback = PollingFallback::new(db_pool, redis_conn, state_manager);
    /// tokio::spawn(async move {
    ///     if let Err(e) = fallback.start().await {
    ///         error!("Polling fallback failed: {}", e);
    ///     }
    /// });
    /// ```
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!(
            "Starting polling fallback loop (interval: {}s, batch size: {}, max iterations: {})",
            POLL_INTERVAL_SECS, MAX_EVENTS_PER_POLL, MAX_POLLING_ITERATIONS
        );

        // FIX 3.3: Track iterations to prevent infinite loops (Medium Priority)
        let mut iteration_count = 0u64;

        loop {
            iteration_count += 1;

            // Check iteration limit
            if iteration_count > MAX_POLLING_ITERATIONS {
                error!(
                    iteration_count = iteration_count,
                    max_iterations = MAX_POLLING_ITERATIONS,
                    error_id = "POLLING_ITERATION_LIMIT_EXCEEDED",
                    "Polling fallback exceeded maximum iterations, exiting for restart"
                );
                anyhow::bail!(
                    "Polling fallback exceeded {} iterations, requiring restart",
                    MAX_POLLING_ITERATIONS
                );
            }

            // Update last poll time
            *self.last_poll_time.write().await = Some(std::time::Instant::now());

            // Poll for unprocessed events
            match self.poll_unprocessed_events().await {
                Ok(count) => {
                    if count > 0 {
                        warn!(
                            "Polling fallback recovered {} unprocessed events (total recovered: {})",
                            count,
                            *self.events_recovered.read().await
                        );
                        // Increment Prometheus metric
                        #[cfg(feature = "metrics")]
                        metrics::counter!("event_processor.polling_fallback.events_recovered")
                            .increment(count as u64);
                    } else {
                        debug!("Polling fallback: no unprocessed events found");
                    }
                }
                Err(e) => {
                    error!("Polling fallback error: {}", e);
                    // Continue polling even after errors
                }
            }

            // Sleep until next poll
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    }

    /// Poll the database for unprocessed events and process them
    ///
    /// This function queries the `unprocessed_events` view (created by migration)
    /// and processes each event using the same `process_event()` function as
    /// the NOTIFY-based listener. This ensures consistent behavior.
    ///
    /// # Returns
    ///
    /// The number of events that were successfully processed.
    async fn poll_unprocessed_events(&self) -> Result<usize> {
        // Query for unprocessed events using the view created in migration
        let events = sqlx::query_as::<_, UnprocessedEvent>(
            r#"
            SELECT id, chain_id, block_number, registry, event_type
            FROM unprocessed_events
            LIMIT $1
            "#,
        )
        .bind(MAX_EVENTS_PER_POLL)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to query unprocessed events")?;

        let event_count = events.len();

        if event_count == 0 {
            return Ok(0);
        }

        info!(
            "Polling fallback found {} unprocessed events, processing...",
            event_count
        );

        // FIX 3.1: Track failures and abort if too many (Medium Priority)
        let mut failed_count = 0;
        let mut succeeded_count = 0;

        // Process each event sequentially
        // We could parallelize this, but sequential processing is safer
        // to avoid overwhelming the system if there's a large backlog
        for event in events {
            debug!(
                "Polling fallback processing event: {} (chain: {}, block: {})",
                event.id, event.chain_id, event.block_number
            );

            // Use the same process_event function as the NOTIFY listener
            // This ensures idempotency (won't double-process if already done)
            match process_event(
                &event.id,
                &self.db_pool,
                &self.job_queue,
                &self.state_manager,
            )
            .await
            {
                Ok(_) => {
                    succeeded_count += 1;
                }
                Err(e) => {
                    failed_count += 1;
                    error!(
                        event_id = %event.id,
                        chain_id = event.chain_id,
                        error = %e,
                        failed_count = failed_count,
                        error_id = "POLLING_FALLBACK_EVENT_FAILED",
                        "Polling fallback failed to process event"
                    );

                    // FIX 3.1: Abort batch if too many failures (systemic issue)
                    if failed_count >= MAX_FAILURES_PER_BATCH {
                        error!(
                            failed_count = failed_count,
                            succeeded_count = succeeded_count,
                            batch_size = event_count,
                            threshold = MAX_FAILURES_PER_BATCH,
                            error_id = "POLLING_FALLBACK_BATCH_ABORTED",
                            "Too many failures in polling batch, aborting to prevent cascade (likely systemic issue)"
                        );

                        // Emit metric for monitoring
                        #[cfg(feature = "metrics")]
                        metrics::counter!("event_processor.polling_fallback.batch_aborted")
                            .increment(1);

                        break; // Abort this batch, will retry in next poll
                    }
                }
            }
        }

        // Update recovered count (only successful events)
        let mut recovered = self.events_recovered.write().await;
        *recovered += succeeded_count as u64;

        // Log summary with failure rate
        if failed_count > 0 {
            let failure_rate = (failed_count as f64 / event_count as f64) * 100.0;
            warn!(
                batch_size = event_count,
                succeeded = succeeded_count,
                failed = failed_count,
                failure_rate = format!("{:.1}%", failure_rate),
                "Polling fallback completed with failures"
            );

            // Emit metrics
            #[cfg(feature = "metrics")]
            {
                metrics::counter!("event_processor.polling_fallback.events_failed")
                    .increment(failed_count as u64);
                metrics::gauge!("event_processor.polling_fallback.failure_rate").set(failure_rate);
            }
        }

        Ok(succeeded_count)
    }

    /// Get the number of events recovered by polling fallback
    pub async fn get_events_recovered(&self) -> u64 {
        *self.events_recovered.read().await
    }

    /// Get the time since last poll
    pub async fn get_time_since_last_poll(&self) -> Option<Duration> {
        self.last_poll_time
            .read()
            .await
            .map(|instant| instant.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAX_EVENTS_PER_POLL, 100, "Batch size should be 100");
        assert_eq!(POLL_INTERVAL_SECS, 60, "Poll interval should be 60 seconds");
    }

    #[test]
    fn test_unprocessed_event_struct() {
        // Ensure UnprocessedEvent has correct fields
        let event = UnprocessedEvent {
            id: "test-id".to_string(),
            chain_id: 1,
            block_number: 12345,
            registry: "reputation".to_string(),
            event_type: "NewFeedback".to_string(),
        };

        assert_eq!(event.id, "test-id");
        assert_eq!(event.chain_id, 1);
    }
}
