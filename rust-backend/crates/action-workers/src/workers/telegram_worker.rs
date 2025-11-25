//! Telegram worker implementation
//!
//! Processes Telegram action jobs from the queue.

use std::sync::Arc;
use std::time::{Duration, Instant};

use shared::ActionJob;

use crate::dlq::{DeadLetterQueue, DlqEntry};
use crate::error::WorkerError;
use crate::metrics;
use crate::rate_limiter::RateLimiter;
use crate::result_logger::{ActionResult, ResultLogger};
use crate::retry::{execute_with_retry, RetryPolicy};
use crate::telegram::{TelegramClient, TelegramConfig};
use crate::template::render_template;

/// Telegram worker that processes Telegram action jobs
pub struct TelegramWorker<C, L, D, R>
where
    C: TelegramClient,
    L: ResultLogger,
    D: DeadLetterQueue,
    R: RateLimiter,
{
    client: Arc<C>,
    logger: Arc<L>,
    dlq: Arc<D>,
    rate_limiter: Arc<R>,
    retry_policy: RetryPolicy,
}

impl<C, L, D, R> TelegramWorker<C, L, D, R>
where
    C: TelegramClient + 'static,
    L: ResultLogger + 'static,
    D: DeadLetterQueue + 'static,
    R: RateLimiter + 'static,
{
    /// Create a new Telegram worker
    pub fn new(
        client: Arc<C>,
        logger: Arc<L>,
        dlq: Arc<D>,
        rate_limiter: Arc<R>,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self {
            client,
            logger,
            dlq,
            rate_limiter,
            retry_policy,
        }
    }

    /// Process a single Telegram action job
    ///
    /// # Arguments
    ///
    /// * `job` - The action job to process
    /// * `event_data` - Event data for template variable substitution
    ///
    /// # Returns
    ///
    /// Ok(()) on success, Err on permanent failure (job moved to DLQ)
    pub async fn process(
        &self,
        job: &ActionJob,
        event_data: &serde_json::Value,
    ) -> Result<(), WorkerError> {
        let start = Instant::now();

        tracing::info!(
            job_id = %job.id,
            trigger_id = %job.trigger_id,
            event_id = %job.event_id,
            "Processing Telegram job"
        );

        // Parse configuration
        let config: TelegramConfig = serde_json::from_value(job.config.clone()).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse Telegram config");
            WorkerError::invalid_config(format!("Invalid Telegram config: {}", e))
        })?;

        // Validate chat ID (security: prevent invalid/malicious chat IDs)
        config.validate_chat_id()?;

        // Render message template (security: validates against whitelist, checks length)
        let message = render_template(&config.message_template, event_data)?;
        let parse_mode = config.get_parse_mode();

        // Clone Arc references for the retry closure
        let client = self.client.clone();
        let rate_limiter = self.rate_limiter.clone();
        let chat_id = config.chat_id.clone();

        // Execute with retry
        let result = execute_with_retry(&self.retry_policy, "telegram", || {
            let client = client.clone();
            let rate_limiter = rate_limiter.clone();
            let chat_id = chat_id.clone();
            let message = message.clone();
            async move {
                // Acquire per-chat rate limit (also checks global limit)
                rate_limiter.acquire_for_key(&chat_id, Duration::from_secs(30)).await?;

                // Send message
                client.send_message(&chat_id, &message, parse_mode).await
            }
        })
        .await;

        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as i64;

        match result {
            Ok(()) => {
                // Success - log result
                metrics::record_job_success("telegram", duration.as_secs_f64());

                self.logger
                    .log(ActionResult::success(
                        job.id.clone(),
                        job.trigger_id.clone(),
                        job.event_id.clone(),
                        "telegram".to_string(),
                        duration_ms,
                    ))
                    .await?;

                tracing::info!(
                    job_id = %job.id,
                    duration_ms = duration_ms,
                    "Telegram job completed successfully"
                );

                Ok(())
            }
            Err(e) => {
                // Failure - move to DLQ and log result
                metrics::record_job_failure("telegram", duration.as_secs_f64());

                let error_msg = e.to_string();

                // Move to DLQ
                self.dlq
                    .push(DlqEntry::new(
                        job.clone(),
                        error_msg.clone(),
                        self.retry_policy.max_attempts,
                    ))
                    .await?;

                // Log failure
                self.logger
                    .log(ActionResult::failure(
                        job.id.clone(),
                        job.trigger_id.clone(),
                        job.event_id.clone(),
                        "telegram".to_string(),
                        duration_ms,
                        error_msg.clone(),
                        self.retry_policy.max_attempts as i32,
                    ))
                    .await?;

                tracing::error!(
                    job_id = %job.id,
                    error = %error_msg,
                    duration_ms = duration_ms,
                    "Telegram job failed, moved to DLQ"
                );

                Err(e)
            }
        }
    }
}

impl<C, L, D, R> Clone for TelegramWorker<C, L, D, R>
where
    C: TelegramClient,
    L: ResultLogger,
    D: DeadLetterQueue,
    R: RateLimiter,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            logger: self.logger.clone(),
            dlq: self.dlq.clone(),
            rate_limiter: self.rate_limiter.clone(),
            retry_policy: self.retry_policy.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlq::InMemoryDlq;
    use crate::rate_limiter::NoopRateLimiter;
    use crate::result_logger::{ActionStatus, InMemoryResultLogger};
    use crate::telegram::MockTelegramClient;
    use serde_json::json;
    use shared::ActionType;

    fn create_test_job(config: serde_json::Value) -> ActionJob {
        ActionJob::new("trigger-1", "event-1", ActionType::Telegram, 1, config)
    }

    fn create_worker(
        client: MockTelegramClient,
    ) -> TelegramWorker<MockTelegramClient, InMemoryResultLogger, InMemoryDlq, NoopRateLimiter>
    {
        TelegramWorker::new(
            Arc::new(client),
            Arc::new(InMemoryResultLogger::new()),
            Arc::new(InMemoryDlq::new()),
            Arc::new(NoopRateLimiter),
            RetryPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40)),
        )
    }

    #[tokio::test]
    async fn test_process_success() {
        let client = MockTelegramClient::new();
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "chat_id": "123456789",
            "message_template": "Hello agent {{agent_id}}!"
        }));

        let event_data = json!({"agent_id": "42"});

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        // Verify message was sent
        let messages = client.sent_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello agent 42!");
        assert_eq!(messages[0].chat_id, "123456789");
    }

    #[tokio::test]
    async fn test_process_failure_moves_to_dlq() {
        let client = MockTelegramClient::failing();
        let dlq = Arc::new(InMemoryDlq::new());
        let logger = Arc::new(InMemoryResultLogger::new());

        let worker = TelegramWorker::new(
            Arc::new(client),
            logger.clone(),
            dlq.clone(),
            Arc::new(NoopRateLimiter),
            RetryPolicy::new(
                2, // Only 2 attempts for faster test
                Duration::from_millis(10),
                Duration::from_millis(20),
            ),
        );

        let job = create_test_job(json!({
            "chat_id": "123456789",
            "message_template": "Test message"
        }));

        let result = worker.process(&job, &json!({})).await;

        assert!(result.is_err());

        // Verify job was moved to DLQ
        assert_eq!(dlq.len().await.unwrap(), 1);

        // Verify failure was logged
        assert_eq!(logger.count_by_status(ActionStatus::Failed), 1);
    }

    #[tokio::test]
    async fn test_process_invalid_config() {
        let client = MockTelegramClient::new();
        let worker = create_worker(client);

        // Missing required fields
        let job = create_test_job(json!({
            "invalid_field": "value"
        }));

        let result = worker.process(&job, &json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_template_rendering() {
        let client = MockTelegramClient::new();
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "chat_id": "123",
            "message_template": "Agent {{agent_id}} score: {{score}} in {{event_type}}"
        }));

        let event_data = json!({
            "agent_id": 42,
            "score": 85,
            "event_type": "NewFeedback"
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        let messages = client.sent_messages();
        assert_eq!(messages[0].text, "Agent 42 score: 85 in NewFeedback");
    }

    #[tokio::test]
    async fn test_worker_clone() {
        let client = MockTelegramClient::new();
        let worker1 = create_worker(client);
        let worker2 = worker1.clone();

        // Both workers should be functional
        let job = create_test_job(json!({
            "chat_id": "123",
            "message_template": "Test"
        }));

        assert!(worker1.process(&job, &json!({})).await.is_ok());
        assert!(worker2.process(&job, &json!({})).await.is_ok());
    }
}
