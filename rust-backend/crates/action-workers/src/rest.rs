//! REST/HTTP action worker
//!
//! Executes HTTP requests to external APIs.

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

/// REST action configuration
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RestConfig {
    /// HTTP method
    pub method: String,
    /// Target URL
    pub url: String,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body template
    pub body_template: Option<serde_json::Value>,
    /// Request timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[allow(dead_code)]
fn default_timeout() -> u64 {
    30000 // 30 seconds
}

/// Execute a REST action
///
/// # Arguments
///
/// * `config` - REST action configuration
/// * `event_data` - Event data for template substitution
///
/// # Returns
///
/// Result indicating success or failure
#[allow(dead_code)]
pub async fn execute(_config: RestConfig, _event_data: serde_json::Value) -> Result<()> {
    // TODO: Implement REST API call
    // This will be implemented in Phase 4
    tracing::debug!("REST action execution (placeholder)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_config_deserialization() {
        let json = r#"{
            "method": "POST",
            "url": "https://api.example.com/webhook",
            "headers": {
                "Authorization": "Bearer token123"
            },
            "body_template": {
                "agent_id": "{{agent_id}}"
            }
        }"#;

        let config: RestConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.method, "POST");
        assert_eq!(config.url, "https://api.example.com/webhook");
        assert_eq!(config.timeout_ms, 30000);
    }
}
