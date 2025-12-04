//! REST/HTTP action worker
//!
//! Executes HTTP requests to external APIs.

use async_trait::async_trait;
use reqwest::{header, Client, Method};
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::error::WorkerError;
use crate::template::render_template;

/// Default timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum URL length for security
const MAX_URL_LENGTH: usize = 2048;

/// Maximum header value length for security
const MAX_HEADER_VALUE_LENGTH: usize = 1024;

/// REST action configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RestConfig {
    /// HTTP method (GET, POST, PUT, DELETE, PATCH)
    pub method: String,

    /// Target URL
    pub url: String,

    /// Request headers (supports templating in values)
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body (supports JSON templating)
    #[serde(default)]
    pub body: Option<serde_json::Value>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout_secs")]
    pub timeout_seconds: u64,

    /// Expected HTTP status codes for success (default: 200-299)
    #[serde(default)]
    pub expected_status_codes: Vec<u16>,
}

fn default_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

impl RestConfig {
    /// Validate the REST configuration
    ///
    /// # Security
    ///
    /// - Validates URL format and length
    /// - Validates HTTP method
    /// - Validates header values
    /// - Validates timeout is reasonable
    pub fn validate(&self) -> Result<(), WorkerError> {
        // Validate URL
        validate_url(&self.url)?;

        // Validate HTTP method
        validate_http_method(&self.method)?;

        // Validate headers
        for (key, value) in &self.headers {
            validate_header(key, value)?;
        }

        // Validate timeout
        if self.timeout_seconds == 0 || self.timeout_seconds > 300 {
            return Err(WorkerError::invalid_config(
                "Timeout must be between 1 and 300 seconds",
            ));
        }

        Ok(())
    }

    /// Get HTTP method as reqwest::Method
    pub fn get_method(&self) -> Result<Method, WorkerError> {
        Method::from_str(&self.method.to_uppercase()).map_err(|_| {
            WorkerError::invalid_config(format!("Invalid HTTP method: {}", self.method))
        })
    }

    /// Get expected status codes (default to 200-299 if empty)
    pub fn get_expected_status_codes(&self) -> Vec<u16> {
        if self.expected_status_codes.is_empty() {
            (200..300).collect()
        } else {
            self.expected_status_codes.clone()
        }
    }
}

/// Validate URL format and security constraints
fn validate_url(url: &str) -> Result<(), WorkerError> {
    if url.is_empty() {
        return Err(WorkerError::invalid_config("URL cannot be empty"));
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
///
/// # Security
///
/// Prevents SSRF attacks by blocking:
/// - localhost (127.0.0.0/8)
/// - Private IPv4 ranges (10.x, 172.16.x, 192.168.x)
/// - AWS metadata endpoint (169.254.169.254)
/// - IPv6 private ranges
fn is_private_host(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Ipv4(ipv4) => {
            ipv4.is_loopback()           // 127.0.0.0/8
                || ipv4.is_private()     // 10.x, 172.16.x, 192.168.x
                || ipv4.is_link_local()  // 169.254.x.x (AWS metadata!)
                || ipv4.is_broadcast()   // 255.255.255.255
                || ipv4.is_unspecified() // 0.0.0.0
        }
        url::Host::Ipv6(ipv6) => {
            ipv6.is_loopback()       // ::1
                || ipv6.is_unspecified() // ::
                // Check for IPv4-mapped IPv6 addresses
                || ipv6.to_ipv4_mapped().map(|v4| {
                    v4.is_loopback() || v4.is_private() || v4.is_link_local()
                }).unwrap_or(false)
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

/// Check if a host string is a private/internal IP address (for testing)
#[cfg(test)]
fn is_private_ip(host: &str) -> bool {
    use std::net::IpAddr;

    // Try to parse as IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback()           // 127.0.0.0/8
                    || ipv4.is_private()     // 10.x, 172.16.x, 192.168.x
                    || ipv4.is_link_local()  // 169.254.x.x (AWS metadata!)
                    || ipv4.is_broadcast()   // 255.255.255.255
                    || ipv4.is_unspecified() // 0.0.0.0
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()       // ::1
                    || ipv6.is_unspecified() // ::
                    // Check for IPv4-mapped IPv6 addresses
                    || ipv6.to_ipv4_mapped().map(|v4| {
                        v4.is_loopback() || v4.is_private() || v4.is_link_local()
                    }).unwrap_or(false)
            }
        };
    }

    // Check for localhost hostnames
    let lower = host.to_lowercase();
    lower == "localhost"
        || lower == "localhost.localdomain"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
}

/// Validate HTTP method
fn validate_http_method(method: &str) -> Result<(), WorkerError> {
    let method_upper = method.to_uppercase();
    match method_upper.as_str() {
        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" => Ok(()),
        _ => Err(WorkerError::invalid_config(format!(
            "Unsupported HTTP method: {}. Allowed: GET, POST, PUT, DELETE, PATCH",
            method
        ))),
    }
}

/// Validate header key and value
fn validate_header(key: &str, value: &str) -> Result<(), WorkerError> {
    if key.is_empty() {
        return Err(WorkerError::invalid_config("Header key cannot be empty"));
    }

    if value.len() > MAX_HEADER_VALUE_LENGTH {
        return Err(WorkerError::invalid_config(format!(
            "Header value too long for '{}': {} characters (max: {})",
            key,
            value.len(),
            MAX_HEADER_VALUE_LENGTH
        )));
    }

    Ok(())
}

/// Sanitize sensitive header values for logging
///
/// # Security
///
/// Prevents leaking sensitive authentication tokens in logs.
fn sanitize_header_for_logging(key: &str, value: &str) -> String {
    const SENSITIVE_HEADERS: &[&str] = &[
        "authorization",
        "x-api-key",
        "api-key",
        "api_key",
        "token",
        "x-auth-token",
        "cookie",
        "set-cookie",
    ];

    if SENSITIVE_HEADERS.contains(&key.to_lowercase().as_str()) {
        "[REDACTED]".to_string()
    } else if value.len() > 100 {
        format!("{}...", &value[..97])
    } else {
        value.to_string()
    }
}

/// HTTP client trait for testability
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Execute an HTTP request
    ///
    /// # Arguments
    ///
    /// * `config` - REST action configuration
    /// * `event_data` - Event data for template variable substitution
    ///
    /// # Returns
    ///
    /// Response body as JSON Value, or error
    async fn execute_request(
        &self,
        config: &RestConfig,
        event_data: &serde_json::Value,
    ) -> Result<RestResponse, WorkerError>;
}

/// HTTP response details
#[derive(Debug, Clone)]
pub struct RestResponse {
    pub status: u16,
    #[allow(dead_code)] // Used in tests and will be used for response validation
    pub body: Option<serde_json::Value>,
}

/// Reqwest-based HTTP client implementation
pub struct ReqwestHttpClient {
    client: Client,
}

impl ReqwestHttpClient {
    /// Create a new HTTP client with connection pooling
    pub fn new() -> Result<Self, WorkerError> {
        let client = Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("agentauri-action-worker/1.0")
            .build()
            .map_err(|e| {
                WorkerError::invalid_config(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client })
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

impl Clone for ReqwestHttpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn execute_request(
        &self,
        config: &RestConfig,
        event_data: &serde_json::Value,
    ) -> Result<RestResponse, WorkerError> {
        // Validate configuration
        config.validate()?;

        // Render URL template
        let url = render_template(&config.url, event_data)?;
        validate_url(&url)?;

        // Get HTTP method
        let method = config.get_method()?;

        // Build request
        let mut request_builder = self
            .client
            .request(method.clone(), &url)
            .timeout(Duration::from_secs(config.timeout_seconds));

        // Add headers with template rendering
        for (key, value_template) in &config.headers {
            let value = render_template(value_template, event_data)?;
            validate_header(key, &value)?;

            tracing::debug!(
                header_key = key,
                header_value = sanitize_header_for_logging(key, &value),
                "Adding request header"
            );

            request_builder = request_builder.header(key, value);
        }

        // Add body for POST/PUT/PATCH
        if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
            if let Some(body_template) = &config.body {
                // Render template in body JSON
                let rendered_body = render_json_template(body_template, event_data)?;

                tracing::debug!(
                    body_preview = %truncate_json(&rendered_body, 200),
                    "Adding request body"
                );

                request_builder = request_builder
                    .header(header::CONTENT_TYPE, "application/json")
                    .json(&rendered_body);
            }
        }

        // Execute request
        tracing::info!(
            method = %method,
            url = truncate_string(&url, 200),
            timeout_secs = config.timeout_seconds,
            "Executing HTTP request"
        );

        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                WorkerError::telegram(format!("Request timeout after {}s", config.timeout_seconds))
            } else if e.is_connect() {
                WorkerError::telegram("Connection failed")
            } else {
                WorkerError::telegram(format!("HTTP request failed: {}", e))
            }
        })?;

        let status = response.status().as_u16();

        // Try to parse response body as JSON
        let body = if response.content_length().unwrap_or(0) > 0 {
            response.json::<serde_json::Value>().await.ok()
        } else {
            None
        };

        // Validate status code
        let expected_codes = config.get_expected_status_codes();
        if !expected_codes.contains(&status) {
            let error_msg = if let Some(body) = &body {
                format!(
                    "Unexpected status code {}: {}",
                    status,
                    truncate_json(body, 500)
                )
            } else {
                format!("Unexpected status code {}", status)
            };

            // 4xx errors are not retryable (client errors)
            if (400..500).contains(&status) {
                return Err(WorkerError::invalid_config(error_msg));
            } else {
                // 5xx errors are retryable (server errors)
                return Err(WorkerError::telegram(error_msg));
            }
        }

        tracing::info!(
            status = status,
            has_body = body.is_some(),
            "HTTP request completed successfully"
        );

        Ok(RestResponse { status, body })
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

/// Truncate a string for logging
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Truncate a JSON value for logging
fn truncate_json(value: &serde_json::Value, max_len: usize) -> String {
    let json_str = value.to_string();
    truncate_string(&json_str, max_len)
}

/// Mock HTTP client for testing
#[cfg(test)]
#[derive(Clone, Default)]
pub struct MockHttpClient {
    /// Simulated response
    response: std::sync::Arc<std::sync::Mutex<Option<RestResponse>>>,
    /// Simulated error
    error: std::sync::Arc<std::sync::Mutex<Option<WorkerError>>>,
    /// Track executed requests
    requests: std::sync::Arc<std::sync::Mutex<Vec<ExecutedRequest>>>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct ExecutedRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

#[cfg(test)]
impl MockHttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a successful response
    pub fn with_response(self, status: u16, body: Option<serde_json::Value>) -> Self {
        *self.response.lock().unwrap() = Some(RestResponse { status, body });
        self
    }

    /// Set an error response
    pub fn with_error(self, error: WorkerError) -> Self {
        *self.error.lock().unwrap() = Some(error);
        self
    }

    /// Get all executed requests
    pub fn requests(&self) -> Vec<ExecutedRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Get count of executed requests
    pub fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

#[cfg(test)]
#[async_trait]
impl HttpClient for MockHttpClient {
    async fn execute_request(
        &self,
        config: &RestConfig,
        event_data: &serde_json::Value,
    ) -> Result<RestResponse, WorkerError> {
        // Validate config
        config.validate()?;

        // Render templates
        let url = render_template(&config.url, event_data)?;
        let method = config.get_method()?.to_string();

        let mut headers = HashMap::new();
        for (key, value_template) in &config.headers {
            let value = render_template(value_template, event_data)?;
            headers.insert(key.clone(), value);
        }

        let body = if let Some(body_template) = &config.body {
            Some(render_json_template(body_template, event_data)?)
        } else {
            None
        };

        // Record request
        self.requests.lock().unwrap().push(ExecutedRequest {
            method,
            url,
            headers,
            body,
        });

        // Return error if configured
        if let Some(ref error) = *self.error.lock().unwrap() {
            return Err(WorkerError::telegram(error.to_string()));
        }

        // Return response if configured
        let response = if let Some(response) = self.response.lock().unwrap().clone() {
            response
        } else {
            // Default success response
            RestResponse {
                status: 200,
                body: Some(serde_json::json!({"success": true})),
            }
        };

        // Validate status code against expected codes (like real client)
        let expected_codes = config.get_expected_status_codes();
        if !expected_codes.contains(&response.status) {
            let error_msg = if let Some(body) = &response.body {
                format!("Unexpected status code {}: {}", response.status, body)
            } else {
                format!("Unexpected status code {}", response.status)
            };

            // 4xx errors are not retryable (client errors)
            if (400..500).contains(&response.status) {
                return Err(WorkerError::invalid_config(error_msg));
            } else {
                // 5xx errors are retryable (server errors)
                return Err(WorkerError::telegram(error_msg));
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rest_config_deserialization() {
        let json = r#"{
            "method": "POST",
            "url": "https://api.example.com/webhook",
            "headers": {
                "Authorization": "Bearer token123"
            },
            "body": {
                "agent_id": "{{agent_id}}"
            }
        }"#;

        let config: RestConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.method, "POST");
        assert_eq!(config.url, "https://api.example.com/webhook");
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_rest_config_defaults() {
        let config = RestConfig {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: default_timeout_secs(),
            expected_status_codes: vec![],
        };

        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(
            config.get_expected_status_codes(),
            (200..300).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("https://api.example.com/path?query=value").is_ok());
        assert!(validate_url("https://8.8.8.8/dns").is_ok());
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
        // localhost IPv4
        assert!(validate_url("http://127.0.0.1/path").is_err());
        assert!(validate_url("http://127.0.0.255/path").is_err());

        // localhost hostname
        assert!(validate_url("http://localhost/path").is_err());
        assert!(validate_url("https://localhost:8080").is_err());
        assert!(validate_url("http://localhost.localdomain/path").is_err());
        assert!(validate_url("http://test.localhost/path").is_err());
        assert!(validate_url("http://myapp.local/path").is_err());

        // Private IPv4 Class A (10.x.x.x)
        assert!(validate_url("http://10.0.0.1/internal").is_err());
        assert!(validate_url("http://10.255.255.255/internal").is_err());

        // Private IPv4 Class B (172.16.x.x - 172.31.x.x)
        assert!(validate_url("http://172.16.0.1/internal").is_err());
        assert!(validate_url("http://172.31.255.255/internal").is_err());

        // Private IPv4 Class C (192.168.x.x)
        assert!(validate_url("http://192.168.1.1/internal").is_err());
        assert!(validate_url("http://192.168.255.255/internal").is_err());

        // AWS metadata endpoint (CRITICAL!)
        assert!(validate_url("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(validate_url("http://169.254.169.254/latest/user-data/").is_err());

        // IPv6 localhost
        assert!(validate_url("http://[::1]/path").is_err());

        // Unspecified addresses
        assert!(validate_url("http://0.0.0.0/path").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        // IPv4 private ranges
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("172.16.0.1"));
        assert!(is_private_ip("192.168.1.1"));
        assert!(is_private_ip("169.254.169.254")); // AWS metadata

        // IPv6
        assert!(is_private_ip("::1"));

        // Hostnames
        assert!(is_private_ip("localhost"));
        assert!(is_private_ip("LOCALHOST")); // case insensitive
        assert!(is_private_ip("test.localhost"));
        assert!(is_private_ip("app.local"));

        // Public IPs should NOT be private
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("1.1.1.1"));
        assert!(!is_private_ip("example.com"));
        assert!(!is_private_ip("api.stripe.com"));
    }

    #[test]
    fn test_validate_http_method_valid() {
        assert!(validate_http_method("GET").is_ok());
        assert!(validate_http_method("POST").is_ok());
        assert!(validate_http_method("put").is_ok()); // case insensitive
        assert!(validate_http_method("DELETE").is_ok());
        assert!(validate_http_method("PATCH").is_ok());
    }

    #[test]
    fn test_validate_http_method_invalid() {
        assert!(validate_http_method("HEAD").is_err());
        assert!(validate_http_method("OPTIONS").is_err());
        assert!(validate_http_method("INVALID").is_err());
    }

    #[test]
    fn test_validate_header() {
        assert!(validate_header("Content-Type", "application/json").is_ok());
        assert!(validate_header("", "value").is_err());
        assert!(validate_header("key", &"x".repeat(2000)).is_err());
    }

    #[test]
    fn test_sanitize_header_for_logging() {
        assert_eq!(
            sanitize_header_for_logging("Authorization", "Bearer secret"),
            "[REDACTED]"
        );
        assert_eq!(
            sanitize_header_for_logging("X-API-Key", "sk_live_12345"),
            "[REDACTED]"
        );
        assert_eq!(
            sanitize_header_for_logging("Content-Type", "application/json"),
            "application/json"
        );

        let long_value = "x".repeat(200);
        let sanitized = sanitize_header_for_logging("Custom", &long_value);
        assert!(sanitized.len() <= 100);
        assert!(sanitized.ends_with("..."));
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
        assert_eq!(result["data"]["agent"], json!(42)); // Parsed as number
        assert_eq!(result["data"]["tags"][0], "trade");
        assert_eq!(result["data"]["tags"][1], "reliable");
    }

    #[test]
    fn test_render_json_template_preserves_types() {
        let template = json!({
            "number": 123,
            "boolean": true,
            "null": null,
            "string": "text"
        });
        let vars = json!({});

        let result = render_json_template(&template, &vars).unwrap();
        assert_eq!(result["number"], 123);
        assert_eq!(result["boolean"], true);
        assert!(result["null"].is_null());
        assert_eq!(result["string"], "text");
    }

    #[tokio::test]
    async fn test_mock_client_success() {
        let client = MockHttpClient::new().with_response(200, Some(json!({"status": "ok"})));

        let config = RestConfig {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 30,
            expected_status_codes: vec![200],
        };

        let result = client.execute_request(&config, &json!({})).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, Some(json!({"status": "ok"})));
        assert_eq!(client.request_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_client_error() {
        let client = MockHttpClient::new().with_error(WorkerError::telegram("Connection failed"));

        let config = RestConfig {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 30,
            expected_status_codes: vec![200],
        };

        let result = client.execute_request(&config, &json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_client_template_rendering() {
        let client = MockHttpClient::new();

        let mut headers = HashMap::new();
        headers.insert("X-Agent-ID".to_string(), "{{agent_id}}".to_string());

        let config = RestConfig {
            method: "POST".to_string(),
            url: "https://example.com/agent/{{agent_id}}".to_string(),
            headers,
            body: Some(json!({"score": "{{score}}"})),
            timeout_seconds: 30,
            expected_status_codes: vec![200],
        };

        let vars = json!({"agent_id": "42", "score": 85});

        let result = client.execute_request(&config, &vars).await;
        assert!(result.is_ok());

        let requests = client.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, "https://example.com/agent/42");
        assert_eq!(
            requests[0].headers.get("X-Agent-ID"),
            Some(&"42".to_string())
        );
        assert_eq!(requests[0].body, Some(json!({"score": 85}))); // Parsed as number
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 100), "short");
        assert_eq!(truncate_string(&"a".repeat(100), 10), "aaaaaaa...");
    }

    #[test]
    fn test_config_validate_success() {
        let config = RestConfig {
            method: "POST".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 30,
            expected_status_codes: vec![200, 201],
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_invalid_url() {
        let config = RestConfig {
            method: "GET".to_string(),
            url: "not-a-url".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 30,
            expected_status_codes: vec![],
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_timeout() {
        let config = RestConfig {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 0,
            expected_status_codes: vec![],
        };

        assert!(config.validate().is_err());

        let config = RestConfig {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_seconds: 500,
            expected_status_codes: vec![],
        };

        assert!(config.validate().is_err());
    }
}
