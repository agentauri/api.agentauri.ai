//! Dead Letter Queue (DLQ) for failed jobs
//!
//! Jobs that fail after all retries are moved to the DLQ for manual review.

#![allow(dead_code)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use shared::{ActionJob, ACTION_JOBS_DLQ};

use crate::error::{WorkerError, WorkerResult};
use crate::metrics;

/// Entry in the Dead Letter Queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqEntry {
    /// Original job that failed
    pub job: ActionJob,
    /// Error message from the last failure
    pub error: String,
    /// Number of attempts made
    pub attempts: u32,
    /// When the job was moved to DLQ
    pub failed_at: DateTime<Utc>,
}

impl DlqEntry {
    /// Create a new DLQ entry
    pub fn new(job: ActionJob, error: String, attempts: u32) -> Self {
        Self {
            job,
            error,
            attempts,
            failed_at: Utc::now(),
        }
    }
}

/// Dead Letter Queue trait for testability
#[async_trait]
pub trait DeadLetterQueue: Send + Sync {
    /// Push a failed job to the DLQ
    async fn push(&self, entry: DlqEntry) -> WorkerResult<()>;

    /// Get current DLQ length
    async fn len(&self) -> WorkerResult<u64>;

    /// Pop a job from the DLQ (for reprocessing)
    async fn pop(&self) -> WorkerResult<Option<DlqEntry>>;

    /// Peek at the first job in the DLQ without removing it
    async fn peek(&self) -> WorkerResult<Option<DlqEntry>>;
}

/// Redis-backed Dead Letter Queue
#[derive(Clone)]
pub struct RedisDlq {
    conn: MultiplexedConnection,
    queue_name: String,
}

impl RedisDlq {
    /// Create a new Redis DLQ
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self {
            conn,
            queue_name: ACTION_JOBS_DLQ.to_string(),
        }
    }

    /// Create with custom queue name (for testing)
    #[cfg(test)]
    pub fn with_queue_name(conn: MultiplexedConnection, queue_name: &str) -> Self {
        Self {
            conn,
            queue_name: queue_name.to_string(),
        }
    }
}

#[async_trait]
impl DeadLetterQueue for RedisDlq {
    async fn push(&self, entry: DlqEntry) -> WorkerResult<()> {
        let json = serde_json::to_string(&entry)?;

        let mut conn = self.conn.clone();
        conn.lpush::<_, _, ()>(&self.queue_name, &json)
            .await
            .map_err(WorkerError::Redis)?;

        tracing::error!(
            job_id = %entry.job.id,
            trigger_id = %entry.job.trigger_id,
            action_type = %entry.job.action_type,
            error = %entry.error,
            attempts = entry.attempts,
            "Job moved to Dead Letter Queue"
        );

        metrics::record_job_dlq(&entry.job.action_type.to_string());

        // Update DLQ size metric
        if let Ok(len) = self.len().await {
            metrics::set_dlq_size(len);
        }

        Ok(())
    }

    async fn len(&self) -> WorkerResult<u64> {
        let mut conn = self.conn.clone();
        let len: u64 = conn
            .llen(&self.queue_name)
            .await
            .map_err(WorkerError::Redis)?;
        Ok(len)
    }

    async fn pop(&self) -> WorkerResult<Option<DlqEntry>> {
        let mut conn = self.conn.clone();
        let result: Option<String> = conn
            .rpop(&self.queue_name, None)
            .await
            .map_err(WorkerError::Redis)?;

        match result {
            Some(json) => {
                let entry: DlqEntry = serde_json::from_str(&json)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    async fn peek(&self) -> WorkerResult<Option<DlqEntry>> {
        let mut conn = self.conn.clone();
        let result: Option<String> = conn
            .lindex(&self.queue_name, -1)
            .await
            .map_err(WorkerError::Redis)?;

        match result {
            Some(json) => {
                let entry: DlqEntry = serde_json::from_str(&json)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }
}

/// In-memory DLQ for testing
#[derive(Default)]
pub struct InMemoryDlq {
    entries: std::sync::Mutex<Vec<DlqEntry>>,
}

impl InMemoryDlq {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all entries (for test inspection)
    pub fn entries(&self) -> Vec<DlqEntry> {
        self.entries.lock().unwrap().clone()
    }
}

#[async_trait]
impl DeadLetterQueue for InMemoryDlq {
    async fn push(&self, entry: DlqEntry) -> WorkerResult<()> {
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }

    async fn len(&self) -> WorkerResult<u64> {
        Ok(self.entries.lock().unwrap().len() as u64)
    }

    async fn pop(&self) -> WorkerResult<Option<DlqEntry>> {
        Ok(self.entries.lock().unwrap().pop())
    }

    async fn peek(&self) -> WorkerResult<Option<DlqEntry>> {
        Ok(self.entries.lock().unwrap().last().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::ActionType;

    fn create_test_job() -> ActionJob {
        ActionJob::new(
            "trigger-123",
            "event-456",
            ActionType::Telegram,
            1,
            serde_json::json!({"chat_id": "123"}),
        )
    }

    #[tokio::test]
    async fn test_in_memory_dlq_push_and_pop() {
        let dlq = InMemoryDlq::new();

        let entry = DlqEntry::new(create_test_job(), "test error".to_string(), 3);

        dlq.push(entry.clone()).await.unwrap();

        assert_eq!(dlq.len().await.unwrap(), 1);

        let popped = dlq.pop().await.unwrap();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().error, "test error");

        assert_eq!(dlq.len().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_dlq_peek() {
        let dlq = InMemoryDlq::new();

        let entry = DlqEntry::new(create_test_job(), "test error".to_string(), 3);

        dlq.push(entry).await.unwrap();

        // Peek should not remove
        let peeked = dlq.peek().await.unwrap();
        assert!(peeked.is_some());
        assert_eq!(dlq.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_dlq_entry_serialization() {
        let job = create_test_job();
        let entry = DlqEntry::new(job, "serialization test".to_string(), 2);

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: DlqEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.error, "serialization test");
        assert_eq!(deserialized.attempts, 2);
        assert_eq!(deserialized.job.trigger_id, "trigger-123");
    }

    #[tokio::test]
    async fn test_empty_dlq() {
        let dlq = InMemoryDlq::new();

        assert_eq!(dlq.len().await.unwrap(), 0);
        assert!(dlq.pop().await.unwrap().is_none());
        assert!(dlq.peek().await.unwrap().is_none());
    }
}
