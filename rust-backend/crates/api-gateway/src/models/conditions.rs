//! Trigger Condition DTOs

use serde::Deserialize;
use validator::Validate;

/// Request to create a new condition
#[derive(Debug, Deserialize, Validate)]
pub struct CreateConditionRequest {
    #[validate(length(min = 1, max = 100))]
    pub condition_type: String,

    #[validate(length(min = 1, max = 255))]
    pub field: String,

    #[validate(length(min = 1, max = 50))]
    pub operator: String,

    #[validate(length(min = 1, max = 1000))]
    pub value: String,

    pub config: Option<serde_json::Value>,
}

/// Request to update a condition
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateConditionRequest {
    #[validate(length(min = 1, max = 100))]
    pub condition_type: Option<String>,

    #[validate(length(min = 1, max = 255))]
    pub field: Option<String>,

    #[validate(length(min = 1, max = 50))]
    pub operator: Option<String>,

    #[validate(length(min = 1, max = 1000))]
    pub value: Option<String>,

    pub config: Option<serde_json::Value>,
}
