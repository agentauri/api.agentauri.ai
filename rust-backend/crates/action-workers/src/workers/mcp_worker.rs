//! MCP worker implementation
//!
//! Processes MCP (Model Context Protocol) action jobs from the queue.

use std::sync::Arc;
use std::time::Instant;

use shared::ActionJob;

use crate::dlq::{DeadLetterQueue, DlqEntry};
use crate::error::WorkerError;
use crate::mcp::{McpClient, McpConfig};
use crate::metrics;
use crate::result_logger::{ActionResult, ResultLogger};
use crate::retry::{execute_with_retry, RetryPolicy};
use crate::template::render_template;

/// MCP worker that processes MCP action jobs
pub struct McpWorker<C, L, D>
where
    C: McpClient,
    L: ResultLogger,
    D: DeadLetterQueue,
{
    client: Arc<C>,
    logger: Arc<L>,
    dlq: Arc<D>,
    retry_policy: RetryPolicy,
}

impl<C, L, D> McpWorker<C, L, D>
where
    C: McpClient + 'static,
    L: ResultLogger + 'static,
    D: DeadLetterQueue + 'static,
{
    /// Create a new MCP worker
    pub fn new(client: Arc<C>, logger: Arc<L>, dlq: Arc<D>, retry_policy: RetryPolicy) -> Self {
        Self {
            client,
            logger,
            dlq,
            retry_policy,
        }
    }

    /// Process a single MCP action job
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
            "Processing MCP job"
        );

        // Parse configuration
        let config: McpConfig = serde_json::from_value(job.config.clone()).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse MCP config");
            WorkerError::invalid_config(format!("Invalid MCP config: {}", e))
        })?;

        // Validate configuration
        config.validate()?;

        // Render arguments template
        let arguments = render_json_template(&config.arguments_template, event_data)?;

        tracing::debug!(
            tool_name = %config.tool_name,
            arguments = %truncate_json(&arguments, 200),
            "Rendered MCP arguments"
        );

        // Clone Arc references for the retry closure
        let client = self.client.clone();
        let config_clone = config.clone();
        let arguments_clone = arguments.clone();

        // Execute with retry
        let result = execute_with_retry(&self.retry_policy, "mcp", || {
            let client = client.clone();
            let config = config_clone.clone();
            let arguments = arguments_clone.clone();
            async move {
                let response = client.call_tool(&config, arguments).await?;

                // Check if the MCP call reported an error
                if !response.success {
                    if let Some(error) = response.error {
                        return Err(WorkerError::mcp(format!("MCP tool error: {}", error)));
                    }
                    return Err(WorkerError::mcp("MCP tool call failed with unknown error"));
                }

                Ok(response)
            }
        })
        .await;

        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as i64;

        match result {
            Ok(response) => {
                // Success - log result
                metrics::record_job_success("mcp", duration.as_secs_f64());

                self.logger
                    .log(ActionResult::success(
                        job.id.clone(),
                        job.trigger_id.clone(),
                        job.event_id.clone(),
                        "mcp".to_string(),
                        duration_ms,
                    ))
                    .await?;

                tracing::info!(
                    job_id = %job.id,
                    tool_name = %config.tool_name,
                    duration_ms = duration_ms,
                    success = response.success,
                    "MCP job completed successfully"
                );

                Ok(())
            }
            Err(e) => {
                // Failure - move to DLQ and log result
                metrics::record_job_failure("mcp", duration.as_secs_f64());

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
                        "mcp".to_string(),
                        duration_ms,
                        error_msg.clone(),
                        self.retry_policy.max_attempts as i32,
                    ))
                    .await?;

                tracing::error!(
                    job_id = %job.id,
                    tool_name = %config.tool_name,
                    error = %error_msg,
                    duration_ms = duration_ms,
                    "MCP job failed, moved to DLQ"
                );

                Err(e)
            }
        }
    }
}

impl<C, L, D> Clone for McpWorker<C, L, D>
where
    C: McpClient,
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

/// Render template variables in a JSON value recursively
fn render_json_template(
    template: &serde_json::Value,
    variables: &serde_json::Value,
) -> Result<serde_json::Value, WorkerError> {
    match template {
        serde_json::Value::String(s) => {
            // Render string template
            let rendered = render_template(s, variables)?;
            // Try to parse as JSON number/bool/null, otherwise keep as string
            serde_json::from_str(&rendered).or_else(|_| Ok(serde_json::Value::String(rendered)))
        }
        serde_json::Value::Object(map) => {
            // Recursively render object properties
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                result.insert(key.clone(), render_json_template(value, variables)?);
            }
            Ok(serde_json::Value::Object(result))
        }
        serde_json::Value::Array(arr) => {
            // Recursively render array elements
            let result: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| render_json_template(v, variables))
                .collect();
            Ok(serde_json::Value::Array(result?))
        }
        // Keep other types as-is (numbers, bools, null)
        other => Ok(other.clone()),
    }
}

/// Truncate a JSON value for logging
fn truncate_json(value: &serde_json::Value, max_len: usize) -> String {
    let json_str = value.to_string();
    if json_str.len() <= max_len {
        json_str
    } else {
        format!("{}...", &json_str[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dlq::InMemoryDlq;
    use crate::mcp::MockMcpClient;
    use crate::result_logger::{ActionStatus, InMemoryResultLogger};
    use crate::retry::RetryPolicy;
    use serde_json::json;
    use shared::ActionType;
    use std::time::Duration;

    fn create_test_job(config: serde_json::Value) -> ActionJob {
        ActionJob::new(
            "trigger-1",
            "event-1",
            ActionType::Mcp,
            1,
            config,
            serde_json::json!({}),
        )
    }

    fn create_worker(
        client: MockMcpClient,
    ) -> McpWorker<MockMcpClient, InMemoryResultLogger, InMemoryDlq> {
        McpWorker::new(
            Arc::new(client),
            Arc::new(InMemoryResultLogger::new()),
            Arc::new(InMemoryDlq::new()),
            RetryPolicy::new(3, Duration::from_millis(10), Duration::from_millis(40)),
        )
    }

    #[tokio::test]
    async fn test_process_success() {
        let client = MockMcpClient::new().with_success();
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "server_url": "https://mcp.example.com",
            "tool_name": "test_tool",
            "arguments_template": {"agent_id": "{{agent_id}}"}
        }));

        let event_data = json!({"agent_id": "42"});

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        // Verify call was made
        let calls = client.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "test_tool");
        assert_eq!(calls[0].arguments, json!({"agent_id": 42})); // Parsed as number
    }

    #[tokio::test]
    async fn test_process_failure_moves_to_dlq() {
        let client = MockMcpClient::new().with_error(WorkerError::mcp("Connection failed"));
        let dlq = Arc::new(InMemoryDlq::new());
        let logger = Arc::new(InMemoryResultLogger::new());

        let worker = McpWorker::new(
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
            "server_url": "https://mcp.example.com",
            "tool_name": "test_tool"
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
        let client = MockMcpClient::new();
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
        let client = MockMcpClient::new().with_success();
        let worker = create_worker(client.clone());

        let job = create_test_job(json!({
            "server_url": "https://mcp.example.com",
            "tool_name": "update_agent",
            "arguments_template": {
                "agent_id": "{{agent_id}}",
                "score": "{{score}}",
                "event": "{{event_type}}"
            }
        }));

        let event_data = json!({
            "agent_id": 42,
            "score": 85,
            "event_type": "NewFeedback"
        });

        let result = worker.process(&job, &event_data).await;
        assert!(result.is_ok());

        let calls = client.calls();
        assert_eq!(calls[0].arguments["agent_id"], 42);
        assert_eq!(calls[0].arguments["score"], 85);
        assert_eq!(calls[0].arguments["event"], "NewFeedback");
    }

    #[tokio::test]
    async fn test_worker_clone() {
        let client = MockMcpClient::new().with_success();
        let worker1 = create_worker(client);
        let worker2 = worker1.clone();

        // Both workers should be functional
        let job = create_test_job(json!({
            "server_url": "https://mcp.example.com",
            "tool_name": "test_tool"
        }));

        assert!(worker1.process(&job, &json!({})).await.is_ok());
        assert!(worker2.process(&job, &json!({})).await.is_ok());
    }

    #[test]
    fn test_render_json_template_string() {
        let template = json!("Hello {{agent_id}}");
        let vars = json!({"agent_id": "42"});

        let result = render_json_template(&template, &vars).unwrap();
        assert_eq!(result, json!("Hello 42"));
    }

    #[test]
    fn test_render_json_template_object() {
        let template = json!({
            "event": "NewFeedback",
            "agent_id": "{{agent_id}}",
            "score": "{{score}}"
        });
        let vars = json!({"agent_id": "42", "score": 85});

        let result = render_json_template(&template, &vars).unwrap();
        assert_eq!(result["event"], "NewFeedback");
        assert_eq!(result["agent_id"], json!(42)); // Parsed as number
        assert_eq!(result["score"], json!(85)); // Parsed as number
    }

    #[test]
    fn test_render_json_template_nested() {
        let template = json!({
            "data": {
                "agent": "{{agent_id}}",
                "tags": ["{{tag1}}", "{{tag2}}"]
            }
        });
        let vars = json!({"agent_id": "42", "tag1": "trade", "tag2": "reliable"});

        let result = render_json_template(&template, &vars).unwrap();
        assert_eq!(result["data"]["agent"], json!(42));
        assert_eq!(result["data"]["tags"][0], "trade");
        assert_eq!(result["data"]["tags"][1], "reliable");
    }
}
