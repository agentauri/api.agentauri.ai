//! Circuit Breaker Management DTOs
//!
//! Request and response types for circuit breaker API endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Response for circuit breaker state endpoint
#[derive(Debug, Serialize, ToSchema)]
pub struct CircuitBreakerStateResponse {
    /// Trigger ID
    pub trigger_id: String,
    /// Trigger name
    pub trigger_name: String,
    /// Current circuit state: "Closed", "Open", or "HalfOpen"
    pub state: String,
    /// Consecutive failure count
    pub failure_count: u32,
    /// Timestamp of last failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_time: Option<DateTime<Utc>>,
    /// Timestamp when circuit was opened
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_at: Option<DateTime<Utc>>,
    /// Number of calls made in half-open state
    pub half_open_calls: u32,
    /// Circuit breaker configuration
    pub config: CircuitBreakerConfigResponse,
}

/// Circuit breaker configuration in response
#[derive(Debug, Serialize, ToSchema)]
pub struct CircuitBreakerConfigResponse {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting recovery (seconds)
    pub recovery_timeout_seconds: u64,
    /// Maximum calls allowed in half-open state
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfigResponse {
    fn default() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout_seconds: 3600,
            half_open_max_calls: 1,
        }
    }
}

/// Request to update circuit breaker configuration
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"failure_threshold": 20, "recovery_timeout_seconds": 7200}))]
pub struct UpdateCircuitBreakerConfigRequest {
    /// Number of consecutive failures before opening circuit (1-1000)
    #[validate(range(
        min = 1,
        max = 1000,
        message = "failure_threshold must be between 1 and 1000"
    ))]
    pub failure_threshold: Option<u32>,

    /// Time to wait before attempting recovery in seconds (60-604800, i.e., 1 min to 7 days)
    #[validate(range(
        min = 60,
        max = 604800,
        message = "recovery_timeout_seconds must be between 60 and 604800"
    ))]
    pub recovery_timeout_seconds: Option<u64>,

    /// Maximum calls allowed in half-open state (1-10)
    #[validate(range(
        min = 1,
        max = 10,
        message = "half_open_max_calls must be between 1 and 10"
    ))]
    pub half_open_max_calls: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_request_valid() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: Some(20),
            recovery_timeout_seconds: Some(7200),
            half_open_max_calls: Some(3),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_request_partial_valid() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: Some(5),
            recovery_timeout_seconds: None,
            half_open_max_calls: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_request_empty_valid() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: None,
            recovery_timeout_seconds: None,
            half_open_max_calls: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_update_request_failure_threshold_too_low() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: Some(0),
            recovery_timeout_seconds: None,
            half_open_max_calls: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_request_failure_threshold_too_high() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: Some(1001),
            recovery_timeout_seconds: None,
            half_open_max_calls: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_request_timeout_too_low() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: None,
            recovery_timeout_seconds: Some(59),
            half_open_max_calls: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_request_timeout_too_high() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: None,
            recovery_timeout_seconds: Some(604801),
            half_open_max_calls: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_request_half_open_too_low() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: None,
            recovery_timeout_seconds: None,
            half_open_max_calls: Some(0),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_update_request_half_open_too_high() {
        let request = UpdateCircuitBreakerConfigRequest {
            failure_threshold: None,
            recovery_timeout_seconds: None,
            half_open_max_calls: Some(11),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_config_response_default() {
        let config = CircuitBreakerConfigResponse::default();
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.recovery_timeout_seconds, 3600);
        assert_eq!(config.half_open_max_calls, 1);
    }

    #[test]
    fn test_state_response_serialization() {
        let response = CircuitBreakerStateResponse {
            trigger_id: "trigger_123".to_string(),
            trigger_name: "Test Trigger".to_string(),
            state: "Closed".to_string(),
            failure_count: 0,
            last_failure_time: None,
            opened_at: None,
            half_open_calls: 0,
            config: CircuitBreakerConfigResponse::default(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["trigger_id"], "trigger_123");
        assert_eq!(json["state"], "Closed");
        assert_eq!(json["failure_count"], 0);
        // last_failure_time and opened_at should be absent (skip_serializing_if)
        assert!(json.get("last_failure_time").is_none());
        assert!(json.get("opened_at").is_none());
    }

    #[test]
    fn test_state_response_with_timestamps() {
        let now = Utc::now();
        let response = CircuitBreakerStateResponse {
            trigger_id: "trigger_123".to_string(),
            trigger_name: "Test Trigger".to_string(),
            state: "Open".to_string(),
            failure_count: 10,
            last_failure_time: Some(now),
            opened_at: Some(now),
            half_open_calls: 0,
            config: CircuitBreakerConfigResponse::default(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["state"], "Open");
        assert!(json.get("last_failure_time").is_some());
        assert!(json.get("opened_at").is_some());
    }
}
