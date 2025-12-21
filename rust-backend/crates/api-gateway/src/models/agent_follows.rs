//! Agent Follow DTOs
//!
//! Simplified interface for following all activities of an ERC-8004 agent
//! across identity, reputation, and validation registries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to follow an agent
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "chain_id": 84532,
    "actions": [{
        "action_type": "telegram",
        "config": {"chat_id": "123456789", "message_template": "Agent {{agent_id}} activity: {{event_type}}"}
    }]
}))]
pub struct FollowAgentRequest {
    /// Blockchain chain ID where the agent is registered
    #[validate(range(min = 1))]
    pub chain_id: i32,

    /// Actions to execute when any event occurs for this agent
    #[validate(length(min = 1, max = 10))]
    pub actions: Vec<FollowActionRequest>,
}

/// Action configuration for agent follow
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct FollowActionRequest {
    /// Action type: telegram, rest, or mcp
    #[validate(custom(function = "validate_action_type"))]
    pub action_type: String,

    /// Action-specific configuration (e.g., chat_id, webhook_url)
    pub config: serde_json::Value,
}

/// Request to update follow settings
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "enabled": false,
    "actions": [{
        "action_type": "telegram",
        "config": {"chat_id": "987654321"}
    }]
}))]
pub struct UpdateFollowRequest {
    /// Enable or disable the follow
    pub enabled: Option<bool>,

    /// Replace all actions with new configuration
    #[validate(length(min = 1, max = 10))]
    pub actions: Option<Vec<FollowActionRequest>>,
}

/// Response for agent follow
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentFollowResponse {
    pub id: String,
    pub agent_id: i64,
    pub chain_id: i32,
    pub organization_id: String,
    pub enabled: bool,
    /// Number of registries being monitored (always 3)
    pub registries_monitored: i32,
    /// Summary of configured actions
    pub actions: Vec<FollowActionSummary>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Detailed follow response including underlying trigger IDs
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentFollowDetailResponse {
    #[serde(flatten)]
    pub follow: AgentFollowResponse,
    /// Underlying trigger IDs (for advanced users)
    pub trigger_ids: TriggerIds,
}

/// Trigger IDs for each registry
#[derive(Debug, Serialize, ToSchema)]
pub struct TriggerIds {
    pub identity: String,
    pub reputation: String,
    pub validation: String,
}

/// Summary of an action in a follow
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FollowActionSummary {
    pub action_type: String,
    /// Sanitized config (secrets redacted)
    pub config_preview: serde_json::Value,
}

/// Path parameters for follow endpoints
#[derive(Debug, Deserialize)]
pub struct AgentFollowPath {
    pub agent_id: i64,
}

/// Query parameters for unfollow/update endpoints
#[derive(Debug, Deserialize)]
pub struct ChainIdQuery {
    pub chain_id: i32,
}

/// Query parameters for list follows
#[derive(Debug, Deserialize)]
pub struct ListFollowsQuery {
    /// Filter by chain_id (optional)
    pub chain_id: Option<i32>,
    /// Filter by enabled status (optional)
    pub enabled: Option<bool>,
}

/// Custom validator for action_type
fn validate_action_type(action_type: &str) -> Result<(), validator::ValidationError> {
    if !["telegram", "rest", "mcp"].contains(&action_type) {
        return Err(validator::ValidationError::new("invalid_action_type"));
    }
    Ok(())
}

impl From<shared::models::AgentFollow> for AgentFollowResponse {
    fn from(follow: shared::models::AgentFollow) -> Self {
        Self {
            id: follow.id,
            agent_id: follow.agent_id,
            chain_id: follow.chain_id,
            organization_id: follow.organization_id,
            enabled: follow.enabled,
            registries_monitored: 3,
            actions: vec![], // Actions are loaded separately
            created_at: follow.created_at,
            updated_at: follow.updated_at,
        }
    }
}

/// Redact sensitive fields from config
pub fn redact_secrets(config: &serde_json::Value) -> serde_json::Value {
    const SENSITIVE_KEYS: &[&str] = &[
        "api_key",
        "token",
        "secret",
        "password",
        "authorization",
        "bearer",
    ];

    match config {
        serde_json::Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                let lower_key = key.to_lowercase();
                if SENSITIVE_KEYS.iter().any(|s| lower_key.contains(s)) {
                    redacted.insert(key.clone(), serde_json::Value::String("***".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_secrets(value));
                }
            }
            serde_json::Value::Object(redacted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redact_secrets).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_follow_agent_request_valid() {
        let req = FollowAgentRequest {
            chain_id: 84532,
            actions: vec![FollowActionRequest {
                action_type: "telegram".to_string(),
                config: serde_json::json!({"chat_id": "123456"}),
            }],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_follow_agent_request_empty_actions() {
        let req = FollowAgentRequest {
            chain_id: 84532,
            actions: vec![],
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("actions"));
    }

    #[test]
    fn test_follow_agent_request_invalid_chain_id() {
        let req = FollowAgentRequest {
            chain_id: 0,
            actions: vec![FollowActionRequest {
                action_type: "telegram".to_string(),
                config: serde_json::json!({}),
            }],
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_follow_action_request_invalid_type() {
        let req = FollowActionRequest {
            action_type: "invalid".to_string(),
            config: serde_json::json!({}),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_follow_action_request_all_valid_types() {
        for action_type in ["telegram", "rest", "mcp"] {
            let req = FollowActionRequest {
                action_type: action_type.to_string(),
                config: serde_json::json!({}),
            };
            assert!(
                req.validate().is_ok(),
                "Action type '{}' should be valid",
                action_type
            );
        }
    }

    #[test]
    fn test_update_follow_request_valid() {
        let req = UpdateFollowRequest {
            enabled: Some(false),
            actions: Some(vec![FollowActionRequest {
                action_type: "rest".to_string(),
                config: serde_json::json!({"url": "https://example.com/webhook"}),
            }]),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_follow_request_empty_all_none() {
        let req = UpdateFollowRequest {
            enabled: None,
            actions: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_redact_secrets() {
        let config = serde_json::json!({
            "chat_id": "123456",
            "api_key": "secret-key",
            "nested": {
                "token": "bearer-token",
                "public": "value"
            }
        });

        let redacted = redact_secrets(&config);

        assert_eq!(redacted["chat_id"], "123456");
        assert_eq!(redacted["api_key"], "***");
        assert_eq!(redacted["nested"]["token"], "***");
        assert_eq!(redacted["nested"]["public"], "value");
    }

    #[test]
    fn test_agent_follow_response_serialization() {
        let response = AgentFollowResponse {
            id: "follow-123".to_string(),
            agent_id: 42,
            chain_id: 84532,
            organization_id: "org-456".to_string(),
            enabled: true,
            registries_monitored: 3,
            actions: vec![FollowActionSummary {
                action_type: "telegram".to_string(),
                config_preview: serde_json::json!({"chat_id": "123"}),
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("follow-123"));
        assert!(json.contains("42"));
        assert!(json.contains("telegram"));
    }
}
