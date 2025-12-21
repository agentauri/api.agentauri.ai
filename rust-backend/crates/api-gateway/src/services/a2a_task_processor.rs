//! A2A Task Processor Service
//!
//! Background task processor for A2A Protocol tasks.
//! Polls the database for submitted tasks and executes them using the QueryExecutor.
//!
//! ## Architecture
//!
//! 1. Polls `a2a_tasks` table for tasks with status = 'submitted'
//! 2. Updates task to 'working' and sets `started_at`
//! 3. Executes the query using QueryExecutor
//! 4. Updates task with result/error and sets `completed_at`
//!
//! ## Concurrency
//!
//! - Single processor instance to avoid duplicate processing
//! - Database-level locking via status update
//! - Future: Add worker pool for parallel processing

use metrics::{counter, gauge, histogram};
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use shared::DbPool;
use uuid::Uuid;

use super::a2a_audit::A2aAuditService;
use super::query_executor::QueryExecutor;
use super::tool_registry::ToolRegistry;

/// Default poll interval for checking new tasks
const DEFAULT_POLL_INTERVAL_SECS: u64 = 1;

/// Maximum tasks to claim per poll cycle
const MAX_TASKS_PER_CYCLE: i64 = 10;

/// Query execution timeout (prevents stuck queries)
const QUERY_EXECUTION_TIMEOUT_SECS: u64 = 30;

/// Maximum retry attempts for transient database errors
const MAX_DB_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds)
const RETRY_BASE_DELAY_MS: u64 = 100;

/// A2A Task Processor configuration
#[derive(Debug, Clone)]
pub struct A2aTaskProcessorConfig {
    /// Interval between poll cycles
    pub poll_interval: Duration,
    /// Maximum tasks to claim per cycle
    pub max_tasks_per_cycle: i64,
}

impl Default for A2aTaskProcessorConfig {
    fn default() -> Self {
        let poll_interval_secs = std::env::var("A2A_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECS);

        Self {
            poll_interval: Duration::from_secs(poll_interval_secs),
            max_tasks_per_cycle: MAX_TASKS_PER_CYCLE,
        }
    }
}

/// A2A Task Processor
///
/// Runs in the background and processes A2A tasks.
pub struct A2aTaskProcessor {
    pool: DbPool,
    config: A2aTaskProcessorConfig,
    executor: QueryExecutor,
}

impl A2aTaskProcessor {
    /// Create a new task processor
    pub fn new(pool: DbPool) -> Self {
        Self::with_config(pool.clone(), A2aTaskProcessorConfig::default())
    }

    /// Create a new task processor with custom configuration
    pub fn with_config(pool: DbPool, config: A2aTaskProcessorConfig) -> Self {
        let executor = QueryExecutor::new(pool.clone());
        Self {
            pool,
            config,
            executor,
        }
    }

    /// Start the task processor
    ///
    /// Runs until the cancellation token is triggered.
    pub async fn run(&self, cancel_token: CancellationToken) {
        let mut poll_interval = interval(self.config.poll_interval);

        info!(
            poll_interval_ms = ?self.config.poll_interval.as_millis(),
            max_tasks_per_cycle = self.config.max_tasks_per_cycle,
            "A2A Task Processor started"
        );

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("A2A Task Processor stopping due to shutdown");
                    break;
                }
                _ = poll_interval.tick() => {
                    if let Err(e) = self.process_pending_tasks().await {
                        error!(error = %e, "Error processing A2A tasks");
                    }
                }
            }
        }

        info!("A2A Task Processor stopped");
    }

    /// Process pending tasks
    async fn process_pending_tasks(&self) -> anyhow::Result<()> {
        // Claim tasks atomically by updating status from 'submitted' to 'working'
        let tasks = self.claim_tasks().await?;

        if tasks.is_empty() {
            debug!("No pending A2A tasks");
            return Ok(());
        }

        // METRICS: Track claimed tasks
        gauge!("a2a.processor.tasks_claimed").set(tasks.len() as f64);
        info!(count = tasks.len(), "Processing A2A tasks");

        // Process each task
        for task in tasks {
            self.process_task(&task).await;
        }

        Ok(())
    }

    /// Claim tasks for processing with retry logic for transient errors
    ///
    /// SECURITY FIX: Uses a single atomic CTE to prevent race conditions.
    /// The previous implementation with subquery SELECT could allow multiple
    /// processor instances to claim the same task under high concurrency.
    /// This CTE pattern ensures truly atomic claiming with FOR UPDATE SKIP LOCKED.
    ///
    /// Includes retry logic for transient database errors (connection issues,
    /// deadlocks, etc.) with exponential backoff.
    async fn claim_tasks(&self) -> anyhow::Result<Vec<ClaimedTask>> {
        let mut retries = 0;

        loop {
            let result = sqlx::query_as::<_, ClaimedTask>(
                r#"
                WITH claimed AS (
                    SELECT id
                    FROM a2a_tasks
                    WHERE status = 'submitted'
                    ORDER BY created_at ASC
                    LIMIT $1
                    FOR UPDATE SKIP LOCKED
                )
                UPDATE a2a_tasks t
                SET status = 'working', started_at = NOW(), updated_at = NOW()
                FROM claimed
                WHERE t.id = claimed.id
                RETURNING t.id, t.organization_id, t.tool, t.arguments
                "#,
            )
            .bind(self.config.max_tasks_per_cycle)
            .fetch_all(&self.pool)
            .await;

            match result {
                Ok(tasks) => return Ok(tasks),
                Err(e) if is_transient_error(&e) && retries < MAX_DB_RETRIES => {
                    retries += 1;
                    let delay_ms = RETRY_BASE_DELAY_MS * (1 << retries); // Exponential backoff
                    warn!(
                        error = %e,
                        retry = retries,
                        max_retries = MAX_DB_RETRIES,
                        delay_ms = delay_ms,
                        "Transient error claiming tasks, retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Process a single task
    ///
    /// SECURITY FIX: Added timeout to prevent queries from running indefinitely.
    /// This protects against stuck queries consuming resources.
    async fn process_task(&self, task: &ClaimedTask) {
        info!(
            task_id = %task.id,
            tool = %task.tool,
            "Processing A2A task"
        );

        // METRICS: Increment started counter
        counter!("a2a.tasks.started", "tool" => task.tool.clone()).increment(1);

        // AUDIT: Log task started
        if let Err(e) =
            A2aAuditService::log_started(&self.pool, &task.id, &task.organization_id, &task.tool)
                .await
        {
            warn!("Failed to log task started audit: {:?}", e);
        }

        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(QUERY_EXECUTION_TIMEOUT_SECS);

        // Execute the query with timeout
        let execution_result = tokio::time::timeout(
            timeout_duration,
            self.executor.execute(&task.tool, &task.arguments),
        )
        .await;

        match execution_result {
            Ok(Ok((result, cost))) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as i64;

                // Get cost in micro-USDC from ToolRegistry
                let cost_micro_usdc = ToolRegistry::get_cost_micro_usdc(&task.tool);

                // Update task as completed
                if let Err(e) = self.complete_task(&task.id, &result, cost).await {
                    error!(
                        task_id = %task.id,
                        error = %e,
                        "Failed to mark task as completed"
                    );
                } else {
                    // METRICS: Track completed tasks
                    counter!("a2a.tasks.completed", "tool" => task.tool.clone()).increment(1);
                    histogram!("a2a.tasks.duration_ms", "tool" => task.tool.clone())
                        .record(duration_ms as f64);

                    // AUDIT: Log task completed
                    if let Err(e) = A2aAuditService::log_completed(
                        &self.pool,
                        &task.id,
                        &task.organization_id,
                        &task.tool,
                        cost_micro_usdc,
                        duration_ms,
                    )
                    .await
                    {
                        warn!("Failed to log task completed audit: {:?}", e);
                    }

                    info!(
                        task_id = %task.id,
                        tool = %task.tool,
                        duration_ms = duration.as_millis(),
                        cost = cost,
                        "A2A task completed successfully"
                    );
                }
            }
            Ok(Err(e)) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as i64;
                let error_msg = e.to_string();

                // METRICS: Track failed tasks
                counter!("a2a.tasks.failed", "tool" => task.tool.clone(), "reason" => "error")
                    .increment(1);

                // Update task as failed (query error)
                if let Err(update_err) = self.fail_task(&task.id, &error_msg).await {
                    error!(
                        task_id = %task.id,
                        error = %update_err,
                        "Failed to mark task as failed"
                    );
                }

                // AUDIT: Log task failed
                if let Err(e) = A2aAuditService::log_failed(
                    &self.pool,
                    &task.id,
                    &task.organization_id,
                    &task.tool,
                    duration_ms,
                    &error_msg,
                )
                .await
                {
                    warn!("Failed to log task failed audit: {:?}", e);
                }

                warn!(
                    task_id = %task.id,
                    tool = %task.tool,
                    error = %error_msg,
                    duration_ms = duration.as_millis(),
                    "A2A task failed"
                );
            }
            Err(_elapsed) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as i64;

                // METRICS: Track timed out tasks
                counter!("a2a.tasks.failed", "tool" => task.tool.clone(), "reason" => "timeout")
                    .increment(1);

                // Update task as failed (timeout)
                let timeout_error = format!(
                    "Query execution timeout after {}s",
                    QUERY_EXECUTION_TIMEOUT_SECS
                );

                if let Err(update_err) = self.fail_task(&task.id, &timeout_error).await {
                    error!(
                        task_id = %task.id,
                        error = %update_err,
                        "Failed to mark task as timed out"
                    );
                }

                // AUDIT: Log task timeout
                if let Err(e) = A2aAuditService::log_timeout(
                    &self.pool,
                    &task.id,
                    &task.organization_id,
                    &task.tool,
                    duration_ms,
                )
                .await
                {
                    warn!("Failed to log task timeout audit: {:?}", e);
                }

                error!(
                    task_id = %task.id,
                    tool = %task.tool,
                    duration_ms = duration.as_millis(),
                    timeout_secs = QUERY_EXECUTION_TIMEOUT_SECS,
                    "A2A task timed out"
                );
            }
        }
    }

    /// Mark a task as completed
    async fn complete_task(
        &self,
        task_id: &Uuid,
        result: &serde_json::Value,
        cost: f64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE a2a_tasks
            SET
                status = 'completed',
                progress = 1.0,
                result = $2,
                cost = $3,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .bind(result)
        .bind(cost)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a task as failed
    async fn fail_task(&self, task_id: &Uuid, error: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE a2a_tasks
            SET
                status = 'failed',
                error = $2,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Task claimed for processing
#[derive(Debug, sqlx::FromRow)]
struct ClaimedTask {
    id: Uuid,
    organization_id: String,
    tool: String,
    arguments: serde_json::Value,
}

/// Check if a database error is transient and can be retried
///
/// Transient errors include:
/// - Connection errors (pool exhausted, network issues)
/// - Deadlocks (serialization failures)
/// - Lock timeouts
fn is_transient_error(error: &sqlx::Error) -> bool {
    match error {
        // Connection pool errors
        sqlx::Error::PoolTimedOut => true,
        sqlx::Error::PoolClosed => false, // Pool closed is not transient

        // IO/Network errors
        sqlx::Error::Io(_) => true,

        // Database errors - check for specific PostgreSQL error codes
        sqlx::Error::Database(db_err) => {
            // PostgreSQL error codes for transient errors:
            // 40001 - serialization_failure (deadlock)
            // 40P01 - deadlock_detected
            // 55P03 - lock_not_available
            // 57P01 - admin_shutdown
            // 08xxx - connection exceptions
            if let Some(code) = db_err.code() {
                let code_str = code.as_ref();
                matches!(
                    code_str,
                    "40001" | "40P01" | "55P03" | "57P01" | "57P02" | "57P03"
                ) || code_str.starts_with("08")
            } else {
                false
            }
        }

        // All other errors are not transient
        _ => false,
    }
}

/// Start the A2A task processor as a background task
///
/// Returns a cancellation token to stop the processor.
pub fn start_a2a_task_processor(pool: DbPool) -> CancellationToken {
    let processor = A2aTaskProcessor::new(pool);
    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();

    tokio::spawn(async move {
        processor.run(token_clone).await;
    });

    cancel_token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = A2aTaskProcessorConfig::default();
        assert_eq!(
            config.poll_interval,
            Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS)
        );
        assert_eq!(config.max_tasks_per_cycle, MAX_TASKS_PER_CYCLE);
    }

    #[test]
    fn test_custom_config() {
        let config = A2aTaskProcessorConfig {
            poll_interval: Duration::from_secs(5),
            max_tasks_per_cycle: 5,
        };
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert_eq!(config.max_tasks_per_cycle, 5);
    }
}
