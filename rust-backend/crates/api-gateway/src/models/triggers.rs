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
    pub organization_id: String,
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
            organization_id: trigger.organization_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreateTriggerRequest validation tests
    // ========================================================================

    #[test]
    fn test_create_trigger_request_valid() {
        let req = CreateTriggerRequest {
            name: "Low Score Alert".to_string(),
            description: Some("Alert when score is below 60".to_string()),
            chain_id: 84532,
            registry: "reputation".to_string(),
            enabled: Some(true),
            is_stateful: Some(false),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_trigger_request_minimal() {
        let req = CreateTriggerRequest {
            name: "Alert".to_string(),
            description: None,
            chain_id: 1,
            registry: "identity".to_string(),
            enabled: None,
            is_stateful: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_trigger_request_empty_name() {
        let req = CreateTriggerRequest {
            name: "".to_string(),
            description: None,
            chain_id: 84532,
            registry: "reputation".to_string(),
            enabled: None,
            is_stateful: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_create_trigger_request_name_too_long() {
        let req = CreateTriggerRequest {
            name: "a".repeat(256), // max 255
            description: None,
            chain_id: 84532,
            registry: "reputation".to_string(),
            enabled: None,
            is_stateful: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_create_trigger_request_description_too_long() {
        let req = CreateTriggerRequest {
            name: "Test".to_string(),
            description: Some("a".repeat(1001)), // max 1000
            chain_id: 84532,
            registry: "reputation".to_string(),
            enabled: None,
            is_stateful: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("description"));
    }

    #[test]
    fn test_create_trigger_request_invalid_registry() {
        let req = CreateTriggerRequest {
            name: "Test".to_string(),
            description: None,
            chain_id: 84532,
            registry: "invalid".to_string(),
            enabled: None,
            is_stateful: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("registry"));
    }

    #[test]
    fn test_create_trigger_request_all_valid_registries() {
        for registry in ["identity", "reputation", "validation"] {
            let req = CreateTriggerRequest {
                name: "Test".to_string(),
                description: None,
                chain_id: 1,
                registry: registry.to_string(),
                enabled: None,
                is_stateful: None,
            };
            assert!(
                req.validate().is_ok(),
                "Registry '{}' should be valid",
                registry
            );
        }
    }

    // ========================================================================
    // UpdateTriggerRequest validation tests
    // ========================================================================

    #[test]
    fn test_update_trigger_request_valid() {
        let req = UpdateTriggerRequest {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            chain_id: Some(1),
            registry: Some("validation".to_string()),
            enabled: Some(false),
            is_stateful: Some(true),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_trigger_request_empty_all_none() {
        let req = UpdateTriggerRequest {
            name: None,
            description: None,
            chain_id: None,
            registry: None,
            enabled: None,
            is_stateful: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_trigger_request_invalid_registry() {
        let req = UpdateTriggerRequest {
            name: None,
            description: None,
            chain_id: None,
            registry: Some("invalid".to_string()),
            enabled: None,
            is_stateful: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // Serialization tests
    // ========================================================================

    #[test]
    fn test_trigger_response_serialization() {
        let response = TriggerResponse {
            id: "trigger-123".to_string(),
            user_id: "user-456".to_string(),
            organization_id: "org-789".to_string(),
            name: "Test Trigger".to_string(),
            description: Some("A test trigger".to_string()),
            chain_id: 84532,
            registry: "reputation".to_string(),
            enabled: true,
            is_stateful: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("trigger-123"));
        assert!(json.contains("org-789"));
        assert!(json.contains("Test Trigger"));
        assert!(json.contains("reputation"));
    }

    #[test]
    fn test_condition_response_serialization() {
        let response = ConditionResponse {
            id: 1,
            trigger_id: "trigger-123".to_string(),
            condition_type: "score_threshold".to_string(),
            field: "score".to_string(),
            operator: "<".to_string(),
            value: "60".to_string(),
            config: Some(serde_json::json!({"extra": "data"})),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("score_threshold"));
        assert!(json.contains("extra"));
    }

    #[test]
    fn test_action_response_serialization() {
        let response = ActionResponse {
            id: 1,
            trigger_id: "trigger-123".to_string(),
            action_type: "telegram".to_string(),
            priority: 10,
            config: serde_json::json!({
                "chat_id": "123456789",
                "message_template": "Hello {{agent_id}}!"
            }),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("telegram"));
        assert!(json.contains("chat_id"));
    }

    // ========================================================================
    // validate_registry tests
    // ========================================================================

    #[test]
    fn test_validate_registry_valid_values() {
        assert!(validate_registry("identity").is_ok());
        assert!(validate_registry("reputation").is_ok());
        assert!(validate_registry("validation").is_ok());
    }

    #[test]
    fn test_validate_registry_invalid_values() {
        assert!(validate_registry("invalid").is_err());
        assert!(validate_registry("").is_err());
        assert!(validate_registry("Identity").is_err()); // case-sensitive
        assert!(validate_registry("REPUTATION").is_err());
    }
}
