//! Data models matching the PostgreSQL database schema

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// User account
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trigger {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub chain_id: i32,
    pub registry: String,
    pub enabled: bool,
    pub is_stateful: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trigger condition
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerCondition {
    pub id: i32,
    pub trigger_id: String,
    pub condition_type: String,
    pub field: String,
    pub operator: String,
    pub value: String,
    #[sqlx(json)]
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Trigger action
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerAction {
    pub id: i32,
    pub trigger_id: String,
    pub action_type: String,
    pub priority: i32,
    #[sqlx(json)]
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Trigger state for stateful triggers
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerState {
    pub trigger_id: String,
    #[sqlx(json)]
    pub state_data: serde_json::Value,
    pub last_updated: DateTime<Utc>,
}

/// Blockchain event
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: String,
    pub chain_id: i32,
    pub block_number: i64,
    pub block_hash: String,
    pub transaction_hash: String,
    pub log_index: i32,
    pub registry: String,
    pub event_type: String,
    pub agent_id: Option<i64>,
    pub timestamp: i64,
    // Identity Registry fields
    pub owner: Option<String>,
    pub token_uri: Option<String>,
    pub metadata_key: Option<String>,
    pub metadata_value: Option<String>,
    // Reputation Registry fields
    pub client_address: Option<String>,
    pub feedback_index: Option<i64>,
    pub score: Option<i32>,
    pub tag1: Option<String>,
    pub tag2: Option<String>,
    pub file_uri: Option<String>,
    pub file_hash: Option<String>,
    // Validation Registry fields
    pub validator_address: Option<String>,
    pub request_hash: Option<String>,
    pub response: Option<i32>,
    pub response_uri: Option<String>,
    pub response_hash: Option<String>,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Checkpoint for tracking last processed block per chain
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Checkpoint {
    pub chain_id: i32,
    pub last_block_number: i64,
    pub last_block_hash: String,
    pub updated_at: DateTime<Utc>,
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionResult {
    pub id: String,
    pub job_id: String,
    pub trigger_id: Option<String>,
    pub event_id: Option<String>,
    pub action_type: String,
    pub status: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    #[sqlx(json)]
    pub response_data: Option<serde_json::Value>,
    pub retry_count: i32,
}

// ============================================================================
// Request/Response DTOs for API
// ============================================================================

/// Request to create a new trigger
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTriggerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub chain_id: i32,
    #[validate(custom(function = "validate_registry"))]
    pub registry: String,
    pub enabled: Option<bool>,
    pub is_stateful: Option<bool>,
    #[validate(length(min = 1))]
    pub conditions: Vec<CreateConditionRequest>,
    #[validate(length(min = 1))]
    pub actions: Vec<CreateActionRequest>,
}

/// Request to create a condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConditionRequest {
    pub condition_type: String,
    pub field: String,
    pub operator: String,
    pub value: String,
    pub config: Option<serde_json::Value>,
}

/// Request to create an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActionRequest {
    pub action_type: String,
    pub priority: Option<i32>,
    pub config: serde_json::Value,
}

/// Custom validator for registry field
fn validate_registry(registry: &str) -> Result<(), validator::ValidationError> {
    if !["identity", "reputation", "validation"].contains(&registry) {
        return Err(validator::ValidationError::new(
            "invalid_registry",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_registry_valid() {
        assert!(validate_registry("identity").is_ok());
        assert!(validate_registry("reputation").is_ok());
        assert!(validate_registry("validation").is_ok());
    }

    #[test]
    fn test_validate_registry_invalid() {
        assert!(validate_registry("invalid").is_err());
        assert!(validate_registry("").is_err());
    }
}
