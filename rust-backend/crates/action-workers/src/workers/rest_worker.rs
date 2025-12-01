//! REST worker implementation
//!
//! Processes REST/HTTP action jobs from the queue.

use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

use shared::ActionJob;

use crate::dlq::{DeadLetterQueue, DlqEntry};
use crate::error::WorkerError;
use crate::metrics;
use crate::rest::{HttpClient, RestConfig};
use crate::result_logger::{ActionResult, ResultLogger};
use crate::retry::{execute_with_retry, RetryPolicy};

/// REST worker that processes REST/HTTP action jobs
pub struct RestWorker<C, L, D>
where
    C: HttpClient,
    L: ResultLogger,
    D: DeadLetterQueue,
{
    client: Arc<C>,
    logger: Arc<L>,
    dlq: Arc<D>,
    retry_policy: RetryPolicy,
}

impl<C, L, D> RestWorker<C, L, D>
where
    C: HttpClient + 'static,
    L: ResultLogger + 'static,
    D: DeadLetterQueue + 'static,
{
    /// Create a new REST worker
    pub fn new(client: Arc<C>, logger: Arc<L>, dlq: Arc<D>, retry_policy: RetryPolicy) -> Self {
        Self {
            client,
            logger,
            dlq,
            retry_policy,
        }
    }

    /// Process a single REST action job
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
            "Processing REST job"
        );

        // Parse configuration
        let config: RestConfig = serde_json::from_value(job.config.clone()).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse REST config");
            WorkerError::invalid_config(format!("Invalid REST config: {}", e))
        })?;

        // Validate configuration (security: validates URL, method, headers, etc.)
        config.validate()?;

        // Clone Arc reference for the retry closure
        let client = self.client.clone();
        let config_clone = config.clone();
        let event_data_clone = event_data.clone();

        // Execute with retry
        let result = execute_with_retry(&self.retry_policy, "rest", || {
            let client = client.clone();
            let config = config_clone.clone();
            let event_data = event_data_clone.clone();
            async move {
                // Execute HTTP request with template rendering
                client.execute_request(&config, &event_data).await
            }
        })
        .await;

        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as i64;

        match result {
            Ok(response) => {
                // Success - log result
                metrics::record_job_success("rest", duration.as_secs_f64());

                self.logger
                    .log(ActionResult::success(
                        job.id.clone(),
                        job.trigger_id.clone(),
                        job.event_id.clone(),
                        "rest".to_string(),
                        duration_ms,
                    ))
                    .await?;

                tracing::info!(
                    job_id = %job.id,
                    duration_ms = duration_ms,
                    status_code = response.status,
                    "REST job completed successfully"
                );

                Ok(())
            }
            Err(e) => {
                // Failure - move to DLQ and log result
                metrics::record_job_failure("rest", duration.as_secs_f64());

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
                        "rest".to_string(),
                        duration_ms,
                        error_msg.clone(),
                        self.retry_policy.max_attempts as i32,
                    ))
                    .await?;

                tracing::error!(
                    job_id = %job.id,
                    error = %error_msg,
                    duration_ms = duration_ms,
                    "REST job failed, moved to DLQ"
                );

                Err(e)
            }
        }
    }
}

impl<C, L, D> Clone for RestWorker<C, L, D>
where
    C: HttpClient,
    L: ResultLogger,
    D: DeadLetterQueue,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            logger: self.logger.clone(),
            dlq: self.dlq.clone(),
            retry_policy: self.retry_policy.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlq::InMemoryDlq;
    use crate::rest::MockHttpClient;
    use crate::result_logger::{ActionStatus, InMemoryResultLogger};
    use serde_json::json;
    use shared::ActionType;
    use std::collections::HashMap;

    fn create_test_job(config: serde_json::Value) -> ActionJob {
        ActionJob::new("trigger-1", "event-1", ActionType::Rest, 1, config)
    }

    fn create_worker(
        client: MockHttpClient,
    ) -> RestWorker<MockHttpClient, InMemoryResultLogger, InMemoryDlq> {
        RestWorker::new(
            Arc::new(client),
            Arc::new(InMemoryResultLogger::new()),
            Arc::new(InMemoryDlq::new()),
            RetryPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40)),
        )
    }

    #[tokio::test]
    async fn test_process_get_request_success() {
        let client = MockHttpClient::new().with_response(200, Some(json!({"status": "ok"})));
        let worker = create_worker(client);

        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/webhook",
            "timeout_seconds": 30,
            "expected_status_codes": [200]
        }));

        let event_data = json!({});

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_post_request_with_body() {
        let client = MockHttpClient::new().with_response(201, Some(json!({"id": 123})));
        let worker = create_worker(client.clone());

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let job = create_test_job(json!({
            "method": "POST",
            "url": "https://api.example.com/events",
            "headers": headers,
            "body": {
                "agent_id": "{{agent_id}}",
                "score": "{{score}}"
            },
            "timeout_seconds": 30,
            "expected_status_codes": [200, 201]
        }));

        let event_data = json!({
            "agent_id": "42",
            "score": 85
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        // Verify request was made
        let requests = client.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].url, "https://api.example.com/events");
    }

    #[tokio::test]
    async fn test_process_template_rendering_in_url() {
        let client = MockHttpClient::new().with_response(200, None);
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/agents/{{agent_id}}/feedback",
            "timeout_seconds": 30
        }));

        let event_data = json!({
            "agent_id": "42"
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        let requests = client.requests();
        assert_eq!(
            requests[0].url,
            "https://api.example.com/agents/42/feedback"
        );
    }

    #[tokio::test]
    async fn test_process_template_rendering_in_headers() {
        let client = MockHttpClient::new().with_response(200, None);
        let worker = create_worker(client.clone());

        let mut headers = HashMap::new();
        headers.insert("X-Agent-ID".to_string(), "{{agent_id}}".to_string());
        headers.insert("X-Score".to_string(), "{{score}}".to_string());

        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/webhook",
            "headers": headers,
            "timeout_seconds": 30
        }));

        let event_data = json!({
            "agent_id": "42",
            "score": 85
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        let requests = client.requests();
        assert_eq!(
            requests[0].headers.get("X-Agent-ID"),
            Some(&"42".to_string())
        );
        assert_eq!(requests[0].headers.get("X-Score"), Some(&"85".to_string()));
    }

    #[tokio::test]
    async fn test_process_failure_moves_to_dlq() {
        let client = MockHttpClient::new().with_error(WorkerError::telegram("Connection failed"));
        let dlq = Arc::new(InMemoryDlq::new());
        let logger = Arc::new(InMemoryResultLogger::new());

        let worker = RestWorker::new(
            Arc::new(client),
            logger.clone(),
            dlq.clone(),
            RetryPolicy::new(
                2, // Only 2 attempts for faster test
                Duration::from_millis(10),
                Duration::from_millis(20),
            ),
        );

        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/webhook",
            "timeout_seconds": 30
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
        let client = MockHttpClient::new();
        let worker = create_worker(client);

        // Missing required fields
        let job = create_test_job(json!({
            "invalid_field": "value"
        }));

        let result = worker.process(&job, &json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_invalid_url() {
        let client = MockHttpClient::new();
        let worker = create_worker(client);

        let job = create_test_job(json!({
            "method": "GET",
            "url": "not-a-valid-url",
            "timeout_seconds": 30
        }));

        let result = worker.process(&job, &json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_unexpected_status_code() {
        let client = MockHttpClient::new().with_response(404, Some(json!({"error": "Not found"})));
        let dlq = Arc::new(InMemoryDlq::new());
        let logger = Arc::new(InMemoryResultLogger::new());

        let worker = RestWorker::new(
            Arc::new(client),
            logger.clone(),
            dlq.clone(),
            RetryPolicy::new(1, Duration::from_millis(10), Duration::from_millis(10)),
        );

        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/webhook",
            "timeout_seconds": 30,
            "expected_status_codes": [200]
        }));

        let result = worker.process(&job, &json!({})).await;
        assert!(result.is_err());

        // 4xx errors are not retryable, so should fail immediately
        assert_eq!(dlq.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_worker_clone() {
        let client = MockHttpClient::new();
        let worker1 = create_worker(client);
        let worker2 = worker1.clone();

        // Both workers should be functional
        let job = create_test_job(json!({
            "method": "GET",
            "url": "https://api.example.com/webhook",
            "timeout_seconds": 30
        }));

        assert!(worker1.process(&job, &json!({})).await.is_ok());
        assert!(worker2.process(&job, &json!({})).await.is_ok());
    }

    #[tokio::test]
    async fn test_process_all_http_methods() {
        for method in ["GET", "POST", "PUT", "DELETE", "PATCH"] {
            let client = MockHttpClient::new().with_response(200, None);
            let worker = create_worker(client.clone());

            let job = create_test_job(json!({
                "method": method,
                "url": "https://api.example.com/webhook",
                "timeout_seconds": 30
            }));

            let result = worker.process(&job, &json!({})).await;
            assert!(result.is_ok(), "Method {} should succeed", method);

            let requests = client.requests();
            assert_eq!(requests[0].method, method);
        }
    }

    #[tokio::test]
    async fn test_process_complex_template_rendering() {
        let client = MockHttpClient::new().with_response(200, None);
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "method": "POST",
            "url": "https://api.example.com/events",
            "headers": {
                "Authorization": "Bearer secret123",
                "X-Agent": "{{agent_id}}"
            },
            "body": {
                "event_type": "{{event_type}}",
                "agent": {
                    "id": "{{agent_id}}",
                    "owner": "{{owner}}"
                },
                "reputation": {
                    "score": "{{score}}",
                    "client": "{{client_address}}"
                }
            },
            "timeout_seconds": 30
        }));

        let event_data = json!({
            "event_type": "NewFeedback",
            "agent_id": "42",
            "owner": "0x123",
            "score": 85,
            "client_address": "0xABC"
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        let requests = client.requests();
        let body = requests[0].body.as_ref().unwrap();

        // Template rendering preserves the JSON structure and renders template variables
        // Numbers in event_data are rendered as numbers after parsing
        assert_eq!(body["event_type"], "NewFeedback");
        assert_eq!(body["agent"]["id"], json!(42)); // Parsed as number
        assert_eq!(body["agent"]["owner"], "0x123");
        assert_eq!(body["reputation"]["score"], json!(85)); // Parsed as number
        assert_eq!(body["reputation"]["client"], "0xABC");
        assert_eq!(requests[0].headers.get("X-Agent"), Some(&"42".to_string()));
    }
}
