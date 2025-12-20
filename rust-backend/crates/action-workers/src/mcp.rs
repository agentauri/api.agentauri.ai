//! MCP (Model Context Protocol) action worker
//!
//! Executes tool calls via MCP servers using JSON-RPC 2.0 over HTTP.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::error::WorkerError;

/// Default timeout in milliseconds
const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// Maximum URL length for security
const MAX_URL_LENGTH: usize = 2048;

/// Maximum tool name length
const MAX_TOOL_NAME_LENGTH: usize = 256;

/// MCP action configuration
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    /// MCP server URL (HTTP endpoint)
    pub server_url: String,

    /// Tool name to call
    pub tool_name: String,

    /// Arguments template (supports {{variable}} placeholders)
    #[serde(default)]
    pub arguments_template: serde_json::Value,

    /// Request timeout in milliseconds
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Optional authentication token (Bearer)
    #[serde(default)]
    pub auth_token: Option<String>,
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

impl McpConfig {
    /// Validate the MCP configuration
    ///
    /// # Security
    ///
    /// - Validates URL format and length
    /// - Validates tool name
    /// - Validates timeout is reasonable
    pub fn validate(&self) -> Result<(), WorkerError> {
        // Validate URL
        validate_url(&self.server_url)?;

        // Validate tool name
        validate_tool_name(&self.tool_name)?;

        // Validate timeout
        if self.timeout_ms == 0 || self.timeout_ms > 300000 {
            return Err(WorkerError::invalid_config(
                "Timeout must be between 1 and 300000 milliseconds (5 minutes)",
            ));
        }

        Ok(())
    }

    /// Get timeout as Duration
    pub fn get_timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Validate URL format and security constraints
fn validate_url(url: &str) -> Result<(), WorkerError> {
    if url.is_empty() {
        return Err(WorkerError::invalid_config("Server URL cannot be empty"));
    }

    if url.len() > MAX_URL_LENGTH {
        return Err(WorkerError::invalid_config(format!(
            "URL too long: {} characters (max: {})",
            url.len(),
            MAX_URL_LENGTH
        )));
    }

    // Parse URL to validate format
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| WorkerError::invalid_config(format!("Invalid URL format: {}", e)))?;

    // Security: Only allow http/https schemes
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(WorkerError::invalid_config(format!(
            "Unsupported URL scheme: {} (only http/https allowed)",
            parsed.scheme()
        )));
    }

    // Security: Block requests to private/internal IP ranges (SSRF protection)
    if let Some(host) = parsed.host() {
        if is_private_host(&host) {
            return Err(WorkerError::invalid_config(format!(
                "URL host '{}' is a private/internal address (SSRF protection)",
                host
            )));
        }
    }

    Ok(())
}

/// Check if a URL host is a private/internal address
fn is_private_host(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Ipv4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
        }
        url::Host::Ipv6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6
                    .to_ipv4_mapped()
                    .map(|v4| v4.is_loopback() || v4.is_private() || v4.is_link_local())
                    .unwrap_or(false)
        }
        url::Host::Domain(domain) => {
            let lower = domain.to_lowercase();
            lower == "localhost"
                || lower == "localhost.localdomain"
                || lower.ends_with(".localhost")
                || lower.ends_with(".local")
        }
    }
}

/// Validate tool name
fn validate_tool_name(name: &str) -> Result<(), WorkerError> {
    if name.is_empty() {
        return Err(WorkerError::invalid_config("Tool name cannot be empty"));
    }

    if name.len() > MAX_TOOL_NAME_LENGTH {
        return Err(WorkerError::invalid_config(format!(
            "Tool name too long: {} characters (max: {})",
            name.len(),
            MAX_TOOL_NAME_LENGTH
        )));
    }

    // Only allow alphanumeric, underscore, hyphen, and slash
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '/')
    {
        return Err(WorkerError::invalid_config(format!(
            "Tool name contains invalid characters: {}. Only alphanumeric, underscore, hyphen, and slash allowed",
            name
        )));
    }

    Ok(())
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: JsonRpcParams,
}

/// JSON-RPC 2.0 params for tools/call
#[derive(Debug, Clone, Serialize)]
struct JsonRpcParams {
    name: String,
    arguments: serde_json::Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: String,
    result: Option<McpToolResult>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

/// MCP tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpToolResult {
    #[allow(dead_code)]
    content: Option<Vec<McpContent>>,
}

/// MCP content item
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpContent {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    content_type: String,
    #[allow(dead_code)]
    text: Option<String>,
}

/// MCP response for worker
#[derive(Debug, Clone)]
pub struct McpResponse {
    /// Whether the call was successful
    pub success: bool,
    /// Result data (if successful)
    #[allow(dead_code)]
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// MCP client trait for testability
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Call a tool on an MCP server
    ///
    /// # Arguments
    ///
    /// * `config` - MCP configuration
    /// * `arguments` - Tool arguments (already rendered from template)
    async fn call_tool(
        &self,
        config: &McpConfig,
        arguments: serde_json::Value,
    ) -> Result<McpResponse, WorkerError>;
}

/// HTTP-based MCP client implementation
pub struct HttpMcpClient {
    client: Client,
}

impl HttpMcpClient {
    /// Create a new MCP client with connection pooling
    pub fn new() -> Result<Self, WorkerError> {
        let client = Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("agentauri-mcp-worker/1.0")
            .build()
            .map_err(|e| {
                WorkerError::invalid_config(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }
}

impl Default for HttpMcpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default MCP client")
    }
}

impl Clone for HttpMcpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

#[async_trait]
impl McpClient for HttpMcpClient {
    async fn call_tool(
        &self,
        config: &McpConfig,
        arguments: serde_json::Value,
    ) -> Result<McpResponse, WorkerError> {
        // Validate configuration
        config.validate()?;

        // Build JSON-RPC request
        let request_id = Uuid::new_v4().to_string();
        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            method: "tools/call".to_string(),
            params: JsonRpcParams {
                name: config.tool_name.clone(),
                arguments,
            },
        };

        tracing::info!(
            server_url = %config.server_url,
            tool_name = %config.tool_name,
            request_id = %request_id,
            "Calling MCP tool"
        );

        // Build HTTP request
        let mut request_builder = self
            .client
            .post(&config.server_url)
            .timeout(config.get_timeout())
            .header("Content-Type", "application/json");

        // Add auth token if provided
        if let Some(ref token) = config.auth_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
            tracing::debug!("Added Bearer token authentication");
        }

        // Send request
        let response = request_builder
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WorkerError::mcp(format!("Request timeout after {}ms", config.timeout_ms))
                } else if e.is_connect() {
                    WorkerError::mcp("Connection failed")
                } else {
                    WorkerError::mcp(format!("HTTP request failed: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(WorkerError::mcp(format!(
                "HTTP {} from MCP server: {}",
                status,
                truncate_string(&body, 200)
            )));
        }

        // Parse JSON-RPC response
        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| WorkerError::mcp(format!("Failed to parse MCP response: {}", e)))?;

        // Check for JSON-RPC error
        if let Some(error) = rpc_response.error {
            tracing::warn!(
                error_code = error.code,
                error_message = %error.message,
                "MCP tool call returned error"
            );
            return Ok(McpResponse {
                success: false,
                result: None,
                error: Some(format!("[{}] {}", error.code, error.message)),
            });
        }

        tracing::info!(
            request_id = %request_id,
            "MCP tool call completed successfully"
        );

        Ok(McpResponse {
            success: true,
            result: rpc_response
                .result
                .map(|r| serde_json::to_value(r).unwrap_or_default()),
            error: None,
        })
    }
}

/// Truncate a string for logging
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Mock MCP client for testing
#[cfg(test)]
#[derive(Clone, Default)]
pub struct MockMcpClient {
    /// Simulated response
    response: std::sync::Arc<std::sync::Mutex<Option<McpResponse>>>,
    /// Simulated error
    error: std::sync::Arc<std::sync::Mutex<Option<WorkerError>>>,
    /// Track executed calls
    calls: std::sync::Arc<std::sync::Mutex<Vec<MockMcpCall>>>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockMcpCall {
    #[allow(dead_code)]
    pub server_url: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[cfg(test)]
impl MockMcpClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a successful response
    pub fn with_success(self) -> Self {
        *self.response.lock().unwrap() = Some(McpResponse {
            success: true,
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        });
        self
    }

    /// Set an error response
    pub fn with_error(self, error: WorkerError) -> Self {
        *self.error.lock().unwrap() = Some(error);
        self
    }

    /// Get all executed calls
    pub fn calls(&self) -> Vec<MockMcpCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Get count of executed calls
    #[allow(dead_code)]
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[cfg(test)]
#[async_trait]
impl McpClient for MockMcpClient {
    async fn call_tool(
        &self,
        config: &McpConfig,
        arguments: serde_json::Value,
    ) -> Result<McpResponse, WorkerError> {
        // Validate config
        config.validate()?;

        // Record call
        self.calls.lock().unwrap().push(MockMcpCall {
            server_url: config.server_url.clone(),
            tool_name: config.tool_name.clone(),
            arguments: arguments.clone(),
        });

        // Return error if configured
        if let Some(ref error) = *self.error.lock().unwrap() {
            return Err(WorkerError::mcp(error.to_string()));
        }

        // Return response if configured
        if let Some(response) = self.response.lock().unwrap().clone() {
            return Ok(response);
        }

        // Default success response
        Ok(McpResponse {
            success: true,
            result: Some(serde_json::json!({"status": "ok"})),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_config_deserialization() {
        let json = r#"{
            "server_url": "https://mcp.example.com/rpc",
            "tool_name": "update_agent_state",
            "arguments_template": {
                "agent_id": "{{agent_id}}",
                "score": "{{score}}"
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.server_url, "https://mcp.example.com/rpc");
        assert_eq!(config.tool_name, "update_agent_state");
        assert_eq!(config.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_mcp_config_with_auth() {
        let json = r#"{
            "server_url": "https://mcp.example.com/rpc",
            "tool_name": "my-tool",
            "timeout_ms": 60000,
            "auth_token": "secret-token"
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.auth_token, Some("secret-token".to_string()));
    }

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://mcp.example.com").is_ok());
        assert!(validate_url("https://api.example.com/mcp/v1").is_ok());
        assert!(validate_url("http://8.8.8.8/mcp").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("").is_err());
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://invalid.com").is_err());
        assert!(validate_url(&"a".repeat(3000)).is_err());
    }

    #[test]
    fn test_validate_url_blocks_ssrf() {
        // localhost
        assert!(validate_url("http://127.0.0.1/mcp").is_err());
        assert!(validate_url("http://localhost/mcp").is_err());

        // Private networks
        assert!(validate_url("http://10.0.0.1/mcp").is_err());
        assert!(validate_url("http://172.16.0.1/mcp").is_err());
        assert!(validate_url("http://192.168.1.1/mcp").is_err());

        // AWS metadata
        assert!(validate_url("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn test_validate_tool_name_valid() {
        assert!(validate_tool_name("update_agent").is_ok());
        assert!(validate_tool_name("get-status").is_ok());
        assert!(validate_tool_name("tools/call").is_ok());
        assert!(validate_tool_name("MyTool123").is_ok());
    }

    #[test]
    fn test_validate_tool_name_invalid() {
        assert!(validate_tool_name("").is_err());
        assert!(validate_tool_name(&"a".repeat(300)).is_err());
        assert!(validate_tool_name("tool with spaces").is_err());
        assert!(validate_tool_name("tool;injection").is_err());
    }

    #[test]
    fn test_config_validate_success() {
        let config = McpConfig {
            server_url: "https://mcp.example.com".to_string(),
            tool_name: "my_tool".to_string(),
            arguments_template: json!({}),
            timeout_ms: 30000,
            auth_token: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid_timeout() {
        let config = McpConfig {
            server_url: "https://mcp.example.com".to_string(),
            tool_name: "my_tool".to_string(),
            arguments_template: json!({}),
            timeout_ms: 0,
            auth_token: None,
        };

        assert!(config.validate().is_err());

        let config = McpConfig {
            server_url: "https://mcp.example.com".to_string(),
            tool_name: "my_tool".to_string(),
            arguments_template: json!({}),
            timeout_ms: 500000,
            auth_token: None,
        };

        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_mock_client_success() {
        let client = MockMcpClient::new().with_success();

        let config = McpConfig {
            server_url: "https://mcp.example.com".to_string(),
            tool_name: "test_tool".to_string(),
            arguments_template: json!({}),
            timeout_ms: 30000,
            auth_token: None,
        };

        let result = client.call_tool(&config, json!({"key": "value"})).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.success);
        assert!(response.error.is_none());

        let calls = client.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "test_tool");
        assert_eq!(calls[0].arguments, json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_mock_client_error() {
        let client = MockMcpClient::new().with_error(WorkerError::mcp("Connection failed"));

        let config = McpConfig {
            server_url: "https://mcp.example.com".to_string(),
            tool_name: "test_tool".to_string(),
            arguments_template: json!({}),
            timeout_ms: 30000,
            auth_token: None,
        };

        let result = client.call_tool(&config, json!({})).await;
        assert!(result.is_err());
    }
}
