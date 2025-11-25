//! Job queue abstraction for action job enqueueing
//!
//! Provides a trait-based abstraction over the job queue to enable testing.

use anyhow::{Context, Result};
use async_trait::async_trait;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

use crate::jobs::ActionJob;

/// Queue name for action jobs
pub const ACTION_JOBS_QUEUE: &str = "action_jobs";

/// Abstract job queue interface for testability
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a job for processing
    ///
    /// # Arguments
    ///
    /// * `job` - The action job to enqueue
    async fn enqueue(&self, job: &ActionJob) -> Result<()>;
}

/// Redis-backed job queue implementation
#[derive(Clone)]
pub struct RedisJobQueue {
    conn: MultiplexedConnection,
}

impl RedisJobQueue {
    /// Create a new Redis job queue
    ///
    /// # Arguments
    ///
    /// * `conn` - Multiplexed Redis connection
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl JobQueue for RedisJobQueue {
    async fn enqueue(&self, job: &ActionJob) -> Result<()> {
        let job_json = serde_json::to_string(job).context("Failed to serialize action job")?;

        // NOTE: LPUSH maintains FIFO order and ignores job.priority field.
        // Priority-based consumption can be implemented in action workers if needed
        // by batching jobs and sorting by priority before execution.
        // For now, we use simple FIFO ordering (LPUSH + BRPOP).
        let mut conn = self.conn.clone();
        conn.lpush::<_, _, ()>(ACTION_JOBS_QUEUE, &job_json)
            .await
            .context("Failed to enqueue action job to Redis")?;

        tracing::debug!(
            job_id = %job.id,
            trigger_id = %job.trigger_id,
            action_type = %job.action_type,
            "Enqueued action job"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use serde_json::json;

    // Mock implementation of JobQueue for testing
    mock! {
        pub JobQueue {}

        #[async_trait]
        impl JobQueue for JobQueue {
            async fn enqueue(&self, job: &ActionJob) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_mock_job_queue() {
        let mut mock_queue = MockJobQueue::new();

        mock_queue.expect_enqueue().times(1).returning(|_| Ok(()));

        let job = ActionJob::new(
            "trigger-1",
            "event-1",
            crate::jobs::ActionType::Telegram,
            1,
            json!({"chat_id": "123"}),
        );

        let result = mock_queue.enqueue(&job).await;
        assert!(result.is_ok());
    }
}
