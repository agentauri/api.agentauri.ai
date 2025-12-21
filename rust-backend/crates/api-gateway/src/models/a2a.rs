//! A2A Protocol DTOs for JSON-RPC 2.0 requests and responses
//!
//! Reference: docs/protocols/A2A_INTEGRATION.md

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// JSON-RPC 2.0 Request/Response Types
// ============================================================================

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "jsonrpc": "2.0",
    "method": "tasks/send",
    "params": {
        "task": {
            "tool": "getReputationSummary",
            "arguments": {"agentId": 42}
        }
    },
    "id": "request-123"
}))]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Method name (tasks/send, tasks/get, tasks/cancel)
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: serde_json::Value,
    /// Request ID (string or number)
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Success Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "jsonrpc": "2.0",
    "result": {"task_id": "abc123", "status": "submitted"},
    "id": "request-123"
}))]
pub struct JsonRpcResponse<T> {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Result payload (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    /// Error payload (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Request ID (echoed from request)
    pub id: serde_json::Value,
}

impl<T> JsonRpcResponse<T> {
    /// Create a success response
    pub fn success(result: T, id: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: JsonRpcError, id: serde_json::Value) -> JsonRpcResponse<()> {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "code": -32001,
    "message": "Insufficient credits",
    "data": {"required": "0.05 USDC", "available": "0.02 USDC"}
}))]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    // Standard JSON-RPC error codes
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    pub fn invalid_params(details: &str) -> Self {
        Self::new(-32602, format!("Invalid params: {}", details))
    }

    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }

    // Application-specific error codes
    pub fn insufficient_credits(required: &str, available: &str) -> Self {
        Self::with_data(
            -32001,
            "Insufficient credits",
            serde_json::json!({
                "required": required,
                "available": available
            }),
        )
    }

    pub fn rate_limited() -> Self {
        Self::new(-32002, "Rate limited")
    }

    pub fn task_not_found() -> Self {
        Self::new(-32003, "Task not found")
    }

    pub fn task_expired() -> Self {
        Self::new(-32004, "Task result expired")
    }
}

// ============================================================================
// Task Types
// ============================================================================

/// Task status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Submitted,
    Working,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Submitted => write!(f, "submitted"),
            TaskStatus::Working => write!(f, "working"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Task definition in tasks/send request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskDefinition {
    /// Query tool name (e.g., getReputationSummary)
    pub tool: String,
    /// Tool arguments
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Parameters for tasks/send method
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskSendParams {
    /// Task definition
    pub task: TaskDefinition,
    /// Optional metadata
    #[serde(default)]
    pub meta: Option<TaskMeta>,
}

/// Task metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskMeta {
    /// Organization ID (optional, derived from auth if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    /// Payment method preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
}

/// Parameters for tasks/get method
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskGetParams {
    /// Task ID
    pub task_id: String,
}

/// Parameters for tasks/cancel method
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskCancelParams {
    /// Task ID
    pub task_id: String,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for tasks/send
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskSendResult {
    /// Created task ID
    pub task_id: String,
    /// Initial status
    pub status: TaskStatus,
    /// Estimated cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost: Option<String>,
}

/// Response for tasks/get
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskGetResult {
    /// Task ID
    pub task_id: String,
    /// Current status
    pub status: TaskStatus,
    /// Progress (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    /// Result (when completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (when failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Actual cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// Response for tasks/cancel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskCancelResult {
    /// Task ID
    pub task_id: String,
    /// New status (cancelled)
    pub status: TaskStatus,
}

// ============================================================================
// Database Model
// ============================================================================

/// A2A Task database row
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct A2aTask {
    pub id: Uuid,
    pub organization_id: String, // TEXT in DB for legacy reasons
    pub tool: String,
    pub arguments: serde_json::Value,
    pub status: String,
    pub progress: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub cost: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl A2aTask {
    /// Convert to TaskGetResult for API response
    pub fn to_get_result(&self) -> TaskGetResult {
        let status = match self.status.as_str() {
            "submitted" => TaskStatus::Submitted,
            "working" => TaskStatus::Working,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Failed, // Fallback
        };

        let duration_ms = match (&self.started_at, &self.completed_at) {
            (Some(start), Some(end)) => Some(end.timestamp_millis() - start.timestamp_millis()),
            _ => None,
        };

        TaskGetResult {
            task_id: self.id.to_string(),
            status,
            progress: self.progress.as_ref().and_then(|s| s.parse::<f64>().ok()),
            result: self.result.clone(),
            error: self.error.clone(),
            cost: self.cost.as_ref().map(|c| format!("{} USDC", c)),
            duration_ms,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_deserialization() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "tasks/send",
            "params": {"task": {"tool": "getReputationSummary", "arguments": {"agentId": 42}}},
            "id": "req-1"
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tasks/send");
        assert_eq!(req.id, serde_json::json!("req-1"));
    }

    #[test]
    fn test_json_rpc_response_success() {
        let result = TaskSendResult {
            task_id: "abc123".to_string(),
            status: TaskStatus::Submitted,
            estimated_cost: Some("0.01 USDC".to_string()),
        };
        let resp = JsonRpcResponse::success(result, serde_json::json!("req-1"));

        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let error = JsonRpcError::method_not_found();
        let resp = JsonRpcResponse::<()>::error(error, serde_json::json!("req-1"));

        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_task_status_serialization() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Submitted).unwrap(),
            "\"submitted\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Working).unwrap(),
            "\"working\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn test_json_rpc_error_codes() {
        assert_eq!(JsonRpcError::parse_error().code, -32700);
        assert_eq!(JsonRpcError::invalid_request().code, -32600);
        assert_eq!(JsonRpcError::method_not_found().code, -32601);
        assert_eq!(JsonRpcError::internal_error().code, -32603);
        assert_eq!(
            JsonRpcError::insufficient_credits("0.05", "0.02").code,
            -32001
        );
        assert_eq!(JsonRpcError::rate_limited().code, -32002);
        assert_eq!(JsonRpcError::task_not_found().code, -32003);
    }
}
