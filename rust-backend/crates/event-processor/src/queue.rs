//! Job queue abstraction for action job enqueueing
//!
//! Provides a trait-based abstraction over the job queue to enable testing.
//!
//! # Queue Overflow Protection (FIX 2.1)
//!
//! This module implements queue depth monitoring to prevent Redis memory exhaustion.
//! If queue depth exceeds threshold, warnings are logged and metrics emitted.
//! The polling fallback ensures events are not lost even if enqueue is rejected.

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use shared::{ActionJob, ACTION_JOBS_QUEUE};

/// Maximum queue depth before warnings (High Priority Fix 2.1)
/// Prevents Redis memory exhaustion under sustained load
const MAX_QUEUE_DEPTH: usize = 10_000;

/// Critical queue depth threshold (triggers backpressure)
/// If queue grows beyond this, new enqueues are rejected
const CRITICAL_QUEUE_DEPTH: usize = 50_000;

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
        // FIX 2.1: Check queue depth BEFORE enqueuing (High Priority)
        let mut conn = self.conn.clone();
        let queue_depth: usize = conn
            .llen(ACTION_JOBS_QUEUE)
            .await
            .context("Failed to get queue depth from Redis")?;

        // CRITICAL: Reject if queue is at critical depth (backpressure)
        if queue_depth >= CRITICAL_QUEUE_DEPTH {
            tracing::error!(
                queue_depth = queue_depth,
                critical_threshold = CRITICAL_QUEUE_DEPTH,
                job_id = %job.id,
                trigger_id = %job.trigger_id,
                error_id = "QUEUE_CRITICAL_DEPTH",
                "CRITICAL: Redis queue at critical depth, rejecting new job (backpressure)"
            );

            // Emit metric for monitoring
            #[cfg(feature = "metrics")]
            metrics::counter!("event_processor.queue_rejections").increment(1);

            bail!(
                "Redis queue depth {} exceeds critical threshold {} - rejecting job to prevent memory exhaustion",
                queue_depth,
                CRITICAL_QUEUE_DEPTH
            );
        }

        // WARNING: Log warning if queue exceeds normal threshold
        if queue_depth >= MAX_QUEUE_DEPTH {
            tracing::warn!(
                queue_depth = queue_depth,
                max_threshold = MAX_QUEUE_DEPTH,
                job_id = %job.id,
                trigger_id = %job.trigger_id,
                error_id = "QUEUE_HIGH_DEPTH",
                "Redis queue depth exceeds threshold - action workers may be falling behind"
            );

            // Emit metric for monitoring
            #[cfg(feature = "metrics")]
            metrics::gauge!("event_processor.queue_depth_high").set(1.0);
        }

        // Serialize job
        let job_json = serde_json::to_string(job).context("Failed to serialize action job")?;

        // NOTE: LPUSH maintains FIFO order and ignores job.priority field.
        // Priority-based consumption can be implemented in action workers if needed
        // by batching jobs and sorting by priority before execution.
        // For now, we use simple FIFO ordering (LPUSH + BRPOP).
        conn.lpush::<_, _, ()>(ACTION_JOBS_QUEUE, &job_json)
            .await
            .context("Failed to enqueue action job to Redis")?;

        // Emit queue depth metric for Prometheus
        #[cfg(feature = "metrics")]
        metrics::gauge!("event_processor.queue_depth").set(queue_depth as f64);

        tracing::debug!(
            job_id = %job.id,
            trigger_id = %job.trigger_id,
            action_type = %job.action_type,
            queue_depth = queue_depth,
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
    use shared::ActionType;

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
            ActionType::Telegram,
            1,
            json!({"chat_id": "123"}),
        );

        let result = mock_queue.enqueue(&job).await;
        assert!(result.is_ok());
    }
}
