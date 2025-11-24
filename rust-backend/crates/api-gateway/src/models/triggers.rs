//! Trigger DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request to create a new trigger
#[derive(Debug, Deserialize, Validate)]
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
}

/// Request to update a trigger
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTriggerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    pub chain_id: Option<i32>,

    #[validate(custom(function = "validate_registry"))]
    pub registry: Option<String>,

    pub enabled: Option<bool>,

    pub is_stateful: Option<bool>,
}

/// Trigger response (basic info without conditions/actions)
#[derive(Debug, Serialize)]
pub struct TriggerResponse {
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

impl From<shared::models::Trigger> for TriggerResponse {
    fn from(trigger: shared::models::Trigger) -> Self {
        Self {
            id: trigger.id,
            user_id: trigger.user_id,
            name: trigger.name,
            description: trigger.description,
            chain_id: trigger.chain_id,
            registry: trigger.registry,
            enabled: trigger.enabled,
            is_stateful: trigger.is_stateful,
            created_at: trigger.created_at,
            updated_at: trigger.updated_at,
        }
    }
}

/// Detailed trigger response with conditions and actions
#[derive(Debug, Serialize)]
pub struct TriggerDetailResponse {
    #[serde(flatten)]
    pub trigger: TriggerResponse,
    pub conditions: Vec<ConditionResponse>,
    pub actions: Vec<ActionResponse>,
}

/// Condition response
#[derive(Debug, Serialize, Clone)]
pub struct ConditionResponse {
    pub id: i32,
    pub trigger_id: String,
    pub condition_type: String,
    pub field: String,
    pub operator: String,
    pub value: String,
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<shared::models::TriggerCondition> for ConditionResponse {
    fn from(condition: shared::models::TriggerCondition) -> Self {
        Self {
            id: condition.id,
            trigger_id: condition.trigger_id,
            condition_type: condition.condition_type,
            field: condition.field,
            operator: condition.operator,
            value: condition.value,
            config: condition.config,
            created_at: condition.created_at,
        }
    }
}

/// Action response
#[derive(Debug, Serialize, Clone)]
pub struct ActionResponse {
    pub id: i32,
    pub trigger_id: String,
    pub action_type: String,
    pub priority: i32,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<shared::models::TriggerAction> for ActionResponse {
    fn from(action: shared::models::TriggerAction) -> Self {
        Self {
            id: action.id,
            trigger_id: action.trigger_id,
            action_type: action.action_type,
            priority: action.priority,
            config: action.config,
            created_at: action.created_at,
        }
    }
}

/// Custom validator for registry field
fn validate_registry(registry: &str) -> Result<(), validator::ValidationError> {
    if !["identity", "reputation", "validation"].contains(&registry) {
        return Err(validator::ValidationError::new("invalid_registry"));
    }
    Ok(())
}
