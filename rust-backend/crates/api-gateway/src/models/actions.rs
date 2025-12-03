//! Trigger Action DTOs

use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request to create a new action
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"action_type": "telegram", "priority": 10, "config": {"chat_id": "123456789", "message_template": "Alert: {{event}}"}}))]
pub struct CreateActionRequest {
    #[validate(length(min = 1, max = 100))]
    #[validate(custom(function = "validate_action_type"))]
    pub action_type: String,

    pub priority: Option<i32>,

    pub config: serde_json::Value,
}

/// Request to update an action
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"priority": 5}))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreateActionRequest validation tests
    // ========================================================================

    #[test]
    fn test_create_action_request_valid_telegram() {
        let req = CreateActionRequest {
            action_type: "telegram".to_string(),
            priority: Some(10),
            config: serde_json::json!({
                "chat_id": "123456789",
                "message_template": "Hello {{agent_id}}!"
            }),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_action_request_valid_rest() {
        let req = CreateActionRequest {
            action_type: "rest".to_string(),
            priority: Some(5),
            config: serde_json::json!({
                "url": "https://api.example.com/webhook",
                "method": "POST"
            }),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_action_request_valid_mcp() {
        let req = CreateActionRequest {
            action_type: "mcp".to_string(),
            priority: None,
            config: serde_json::json!({
                "endpoint": "https://agent.example.com/mcp",
                "tool": "agent.receiveFeedback"
            }),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_action_request_invalid_action_type() {
        let req = CreateActionRequest {
            action_type: "email".to_string(), // not supported
            priority: None,
            config: serde_json::json!({}),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("action_type"));
    }

    #[test]
    fn test_create_action_request_empty_action_type() {
        let req = CreateActionRequest {
            action_type: "".to_string(),
            priority: None,
            config: serde_json::json!({}),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("action_type"));
    }

    #[test]
    fn test_create_action_request_action_type_too_long() {
        let req = CreateActionRequest {
            action_type: "a".repeat(101), // max 100
            priority: None,
            config: serde_json::json!({}),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("action_type"));
    }

    #[test]
    fn test_create_action_request_all_valid_types() {
        for action_type in ["telegram", "rest", "mcp"] {
            let req = CreateActionRequest {
                action_type: action_type.to_string(),
                priority: None,
                config: serde_json::json!({}),
            };
            assert!(
                req.validate().is_ok(),
                "Action type '{}' should be valid",
                action_type
            );
        }
    }

    // ========================================================================
    // UpdateActionRequest validation tests
    // ========================================================================

    #[test]
    fn test_update_action_request_valid() {
        let req = UpdateActionRequest {
            action_type: Some("rest".to_string()),
            priority: Some(20),
            config: Some(serde_json::json!({"url": "https://new.example.com"})),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_action_request_all_none() {
        let req = UpdateActionRequest {
            action_type: None,
            priority: None,
            config: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_action_request_invalid_action_type() {
        let req = UpdateActionRequest {
            action_type: Some("webhook".to_string()), // not supported
            priority: None,
            config: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_action_request_partial() {
        let req = UpdateActionRequest {
            action_type: None,
            priority: Some(15),
            config: None,
        };
        assert!(req.validate().is_ok());
    }

    // ========================================================================
    // validate_action_type tests
    // ========================================================================

    #[test]
    fn test_validate_action_type_valid_values() {
        assert!(validate_action_type("telegram").is_ok());
        assert!(validate_action_type("rest").is_ok());
        assert!(validate_action_type("mcp").is_ok());
    }

    #[test]
    fn test_validate_action_type_invalid_values() {
        assert!(validate_action_type("invalid").is_err());
        assert!(validate_action_type("").is_err());
        assert!(validate_action_type("Telegram").is_err()); // case-sensitive
        assert!(validate_action_type("REST").is_err());
        assert!(validate_action_type("email").is_err());
        assert!(validate_action_type("webhook").is_err());
    }
}
