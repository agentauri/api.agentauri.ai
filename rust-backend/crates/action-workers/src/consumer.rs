//! Job consumer for reading jobs from Redis queue
//!
//! Provides a trait-based abstraction for job consumption with blocking pop.
//!
//! # Security
//!
//! - Jobs have a TTL (time-to-live) to prevent processing of stale jobs
//! - Expired jobs are rejected and not processed

use async_trait::async_trait;
use chrono::Utc;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use shared::{ActionJob, ACTION_JOBS_QUEUE};

use crate::error::{WorkerError, WorkerResult};

/// Default job TTL in seconds (1 hour)
pub const DEFAULT_JOB_TTL_SECS: i64 = 3600;

/// Job consumer trait for testability
#[async_trait]
pub trait JobConsumer: Send + Sync {
    /// Block and wait for next job from queue
    ///
    /// # Arguments
    ///
    /// * `timeout_secs` - Maximum time to block waiting for a job
    ///
    /// # Returns
    ///
    /// `Some(ActionJob)` if a job was received, `None` if timeout
    async fn consume(&self, timeout_secs: u64) -> WorkerResult<Option<ActionJob>>;

    /// Get current queue length
    async fn queue_len(&self) -> WorkerResult<u64>;
}

/// Redis-backed job consumer implementation
#[derive(Clone)]
pub struct RedisJobConsumer {
    conn: MultiplexedConnection,
    queue_name: String,
}

impl RedisJobConsumer {
    /// Create a new Redis job consumer
    ///
    /// # Arguments
    ///
    /// * `conn` - Multiplexed Redis connection
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self {
            conn,
            queue_name: ACTION_JOBS_QUEUE.to_string(),
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
impl JobConsumer for RedisJobConsumer {
    async fn consume(&self, timeout_secs: u64) -> WorkerResult<Option<ActionJob>> {
        let mut conn = self.conn.clone();

        // BRPOP blocks until a job is available or timeout
        // Returns (queue_name, value) tuple
        let result: Option<(String, String)> = conn
            .brpop(&self.queue_name, timeout_secs as f64)
            .await
            .map_err(WorkerError::Redis)?;

        match result {
            Some((_, json)) => {
                let job: ActionJob = serde_json::from_str(&json).map_err(|e| {
                    tracing::warn!(
                        error = %e,
                        "Failed to parse job JSON from queue (payload omitted for security)"
                    );
                    WorkerError::Serialization(e)
                })?;

                // Check job TTL (security: reject stale jobs)
                let age_secs = (Utc::now() - job.created_at).num_seconds();
                if age_secs > DEFAULT_JOB_TTL_SECS {
                    tracing::warn!(
                        job_id = %job.id,
                        age_secs = age_secs,
                        ttl_secs = DEFAULT_JOB_TTL_SECS,
                        "Job expired, skipping"
                    );
                    return Ok(None); // Treat as no job available
                }

                tracing::debug!(
                    job_id = %job.id,
                    trigger_id = %job.trigger_id,
                    action_type = %job.action_type,
                    age_secs = age_secs,
                    "Consumed job from queue"
                );

                Ok(Some(job))
            }
            None => {
                // Timeout - no job available
                Ok(None)
            }
        }
    }

    async fn queue_len(&self) -> WorkerResult<u64> {
        let mut conn = self.conn.clone();
        let len: u64 = conn
            .llen(&self.queue_name)
            .await
            .map_err(WorkerError::Redis)?;
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    // Mock JobConsumer for testing components that depend on it
    mock! {
        pub JobConsumer {}

        #[async_trait]
        impl JobConsumer for JobConsumer {
            async fn consume(&self, timeout_secs: u64) -> WorkerResult<Option<ActionJob>>;
            async fn queue_len(&self) -> WorkerResult<u64>;
        }
    }

    #[tokio::test]
    async fn test_mock_consumer_returns_job() {
        let mut mock = MockJobConsumer::new();

        mock.expect_consume().times(1).returning(|_| {
            Ok(Some(ActionJob::new(
                "trigger-1",
                "event-1",
                shared::ActionType::Telegram,
                1,
                serde_json::json!({"chat_id": "123"}),
            )))
        });

        let result = mock.consume(5).await.unwrap();
        assert!(result.is_some());
        let job = result.unwrap();
        assert_eq!(job.trigger_id, "trigger-1");
    }

    #[tokio::test]
    async fn test_mock_consumer_timeout() {
        let mut mock = MockJobConsumer::new();

        mock.expect_consume().times(1).returning(|_| Ok(None));

        let result = mock.consume(5).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_queue_len() {
        let mut mock = MockJobConsumer::new();

        mock.expect_queue_len().times(1).returning(|| Ok(42));

        let len = mock.queue_len().await.unwrap();
        assert_eq!(len, 42);
    }
}
