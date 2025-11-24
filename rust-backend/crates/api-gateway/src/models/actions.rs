//! Trigger Action DTOs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request to create a new action
#[derive(Debug, Deserialize, Validate)]
pub struct CreateActionRequest {
    #[validate(length(min = 1, max = 100))]
    #[validate(custom(function = "validate_action_type"))]
    pub action_type: String,

    pub priority: Option<i32>,

    pub config: serde_json::Value,
}

/// Request to update an action
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateActionRequest {
    #[validate(length(min = 1, max = 100))]
    #[validate(custom(function = "validate_action_type"))]
    pub action_type: Option<String>,

    pub priority: Option<i32>,

    pub config: Option<serde_json::Value>,
}

/// Custom validator for action_type field
fn validate_action_type(action_type: &str) -> Result<(), validator::ValidationError> {
    if !["telegram", "rest", "mcp"].contains(&action_type) {
        return Err(validator::ValidationError::new("invalid_action_type"));
    }
    Ok(())
}
