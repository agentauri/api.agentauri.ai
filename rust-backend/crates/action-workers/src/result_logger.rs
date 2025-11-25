//! Result logging for action execution
//!
//! Logs action execution results to PostgreSQL for audit and analytics.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::{WorkerError, WorkerResult};

/// Action execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Success,
    Failed,
    Retrying,
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionStatus::Success => write!(f, "success"),
            ActionStatus::Failed => write!(f, "failed"),
            ActionStatus::Retrying => write!(f, "retrying"),
        }
    }
}

/// Action execution result for logging
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Unique job identifier
    pub job_id: String,
    /// Trigger that created this job
    pub trigger_id: String,
    /// Event that triggered the action
    pub event_id: String,
    /// Type of action (telegram, rest, mcp)
    pub action_type: String,
    /// Execution status
    pub status: ActionStatus,
    /// Execution duration in milliseconds
    pub duration_ms: i64,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Number of retry attempts made
    pub retry_count: i32,
}

impl ActionResult {
    /// Create a success result
    pub fn success(
        job_id: String,
        trigger_id: String,
        event_id: String,
        action_type: String,
        duration_ms: i64,
    ) -> Self {
        Self {
            job_id,
            trigger_id,
            event_id,
            action_type,
            status: ActionStatus::Success,
            duration_ms,
            error_message: None,
            retry_count: 0,
        }
    }

    /// Create a failure result
    pub fn failure(
        job_id: String,
        trigger_id: String,
        event_id: String,
        action_type: String,
        duration_ms: i64,
        error: String,
        retry_count: i32,
    ) -> Self {
        Self {
            job_id,
            trigger_id,
            event_id,
            action_type,
            status: ActionStatus::Failed,
            duration_ms,
            error_message: Some(error),
            retry_count,
        }
    }
}

/// Result logger trait for testability
#[async_trait]
pub trait ResultLogger: Send + Sync {
    /// Log an action result
    async fn log(&self, result: ActionResult) -> WorkerResult<()>;

    /// Get recent results for a trigger (for debugging)
    async fn get_recent(&self, trigger_id: &str, limit: i64) -> WorkerResult<Vec<LoggedResult>>;
}

/// Logged result from database
#[derive(Debug, Clone)]
pub struct LoggedResult {
    pub id: i64,
    pub job_id: String,
    pub trigger_id: String,
    pub event_id: String,
    pub action_type: String,
    pub status: String,
    pub duration_ms: i64,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
}

/// PostgreSQL result logger
pub struct PostgresResultLogger {
    pool: PgPool,
}

impl PostgresResultLogger {
    /// Create a new PostgreSQL result logger
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ResultLogger for PostgresResultLogger {
    async fn log(&self, result: ActionResult) -> WorkerResult<()> {
        sqlx::query(
            r#"
            INSERT INTO action_results
            (job_id, trigger_id, event_id, action_type, status, duration_ms, error_message, retry_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&result.job_id)
        .bind(&result.trigger_id)
        .bind(&result.event_id)
        .bind(&result.action_type)
        .bind(result.status.to_string())
        .bind(result.duration_ms)
        .bind(&result.error_message)
        .bind(result.retry_count)
        .execute(&self.pool)
        .await
        .map_err(WorkerError::Database)?;

        tracing::debug!(
            job_id = %result.job_id,
            status = %result.status,
            duration_ms = result.duration_ms,
            "Logged action result"
        );

        Ok(())
    }

    async fn get_recent(&self, trigger_id: &str, limit: i64) -> WorkerResult<Vec<LoggedResult>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, String, String, i64, Option<String>, i32, DateTime<Utc>)>(
            r#"
            SELECT id, job_id, trigger_id, event_id, action_type, status, duration_ms, error_message, retry_count, created_at
            FROM action_results
            WHERE trigger_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(trigger_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(WorkerError::Database)?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    job_id,
                    trigger_id,
                    event_id,
                    action_type,
                    status,
                    duration_ms,
                    error_message,
                    retry_count,
                    created_at,
                )| {
                    LoggedResult {
                        id,
                        job_id,
                        trigger_id,
                        event_id,
                        action_type,
                        status,
                        duration_ms,
                        error_message,
                        retry_count,
                        created_at,
                    }
                },
            )
            .collect())
    }
}

/// In-memory result logger for testing
#[derive(Default)]
pub struct InMemoryResultLogger {
    results: std::sync::Mutex<Vec<ActionResult>>,
}

impl InMemoryResultLogger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all logged results
    pub fn results(&self) -> Vec<ActionResult> {
        self.results.lock().unwrap().clone()
    }

    /// Get count of results by status
    pub fn count_by_status(&self, status: ActionStatus) -> usize {
        self.results
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.status == status)
            .count()
    }
}

#[async_trait]
impl ResultLogger for InMemoryResultLogger {
    async fn log(&self, result: ActionResult) -> WorkerResult<()> {
        self.results.lock().unwrap().push(result);
        Ok(())
    }

    async fn get_recent(&self, trigger_id: &str, limit: i64) -> WorkerResult<Vec<LoggedResult>> {
        let results = self.results.lock().unwrap();
        Ok(results
            .iter()
            .filter(|r| r.trigger_id == trigger_id)
            .take(limit as usize)
            .enumerate()
            .map(|(i, r)| LoggedResult {
                id: i as i64,
                job_id: r.job_id.clone(),
                trigger_id: r.trigger_id.clone(),
                event_id: r.event_id.clone(),
                action_type: r.action_type.clone(),
                status: r.status.to_string(),
                duration_ms: r.duration_ms,
                error_message: r.error_message.clone(),
                retry_count: r.retry_count,
                created_at: Utc::now(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_logger() {
        let logger = InMemoryResultLogger::new();

        let result = ActionResult::success(
            "job-1".to_string(),
            "trigger-1".to_string(),
            "event-1".to_string(),
            "telegram".to_string(),
            100,
        );

        logger.log(result).await.unwrap();

        assert_eq!(logger.results().len(), 1);
        assert_eq!(logger.count_by_status(ActionStatus::Success), 1);
        assert_eq!(logger.count_by_status(ActionStatus::Failed), 0);
    }

    #[tokio::test]
    async fn test_failure_result() {
        let logger = InMemoryResultLogger::new();

        let result = ActionResult::failure(
            "job-2".to_string(),
            "trigger-1".to_string(),
            "event-2".to_string(),
            "telegram".to_string(),
            500,
            "Connection timeout".to_string(),
            3,
        );

        logger.log(result).await.unwrap();

        let results = logger.results();
        assert_eq!(results[0].status, ActionStatus::Failed);
        assert_eq!(
            results[0].error_message,
            Some("Connection timeout".to_string())
        );
        assert_eq!(results[0].retry_count, 3);
    }

    #[tokio::test]
    async fn test_get_recent() {
        let logger = InMemoryResultLogger::new();

        // Log multiple results
        for i in 0..5 {
            let result = ActionResult::success(
                format!("job-{}", i),
                "trigger-1".to_string(),
                format!("event-{}", i),
                "telegram".to_string(),
                100 + i as i64,
            );
            logger.log(result).await.unwrap();
        }

        let recent = logger.get_recent("trigger-1", 3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_action_status_display() {
        assert_eq!(ActionStatus::Success.to_string(), "success");
        assert_eq!(ActionStatus::Failed.to_string(), "failed");
        assert_eq!(ActionStatus::Retrying.to_string(), "retrying");
    }
}
