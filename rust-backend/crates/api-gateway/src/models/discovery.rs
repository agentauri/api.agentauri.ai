//! Discovery endpoint data structures
//!
//! Agent Card specification for /.well-known/agent.json endpoint.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Agent Card - System metadata for discovery endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentCardResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub api_version: String,
    pub base_url: String,
    pub capabilities: Capabilities,
    pub endpoints: Endpoints,
    pub contact: Contact,
    pub protocol_version: String,
    pub generated_at: DateTime<Utc>,
}

/// System capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Capabilities {
    pub push_layer: PushLayer,
    pub pull_layer: PullLayer,
    pub authentication: Authentication,
    pub rate_limiting: RateLimiting,
}

/// Push layer capabilities (event-driven triggers)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PushLayer {
    pub enabled: bool,
    pub features: Vec<String>,
    pub supported_chains: Vec<ChainInfo>,
}

/// Pull layer capabilities (agent queries)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PullLayer {
    pub enabled: bool,
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Blockchain chain information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChainInfo {
    pub chain_id: i32,
    pub name: String,
    pub registries: Vec<String>,
}

/// Authentication capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Authentication {
    pub methods: Vec<String>,
    pub oauth2_supported: bool,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimiting {
    pub enabled: bool,
    pub tiers: Vec<RateLimitTier>,
}

/// Rate limit tier configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitTier {
    pub tier: String,
    pub rate_limit: String,
    pub authentication: String,
}

/// API endpoint references
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Endpoints {
    pub api_documentation: String,
    pub health_check: String,
    pub authentication: AuthenticationEndpoints,
    pub triggers: String,
}

/// Authentication endpoint URLs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationEndpoints {
    pub register: String,
    pub login: String,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Contact {
    pub email: String,
    pub github: String,
    pub documentation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_agent_card_serialization() {
        let agent_card = AgentCardResponse {
            name: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: "Test description".to_string(),
            api_version: "v1".to_string(),
            base_url: "https://api.test.dev".to_string(),
            capabilities: Capabilities {
                push_layer: PushLayer {
                    enabled: true,
                    features: vec!["triggers".to_string()],
                    supported_chains: vec![ChainInfo {
                        chain_id: 11155111,
                        name: "Ethereum Sepolia".to_string(),
                        registries: vec!["identity".to_string()],
                    }],
                },
                pull_layer: PullLayer {
                    enabled: false,
                    features: vec![],
                    note: Some("Coming soon".to_string()),
                },
                authentication: Authentication {
                    methods: vec!["jwt".to_string()],
                    oauth2_supported: false,
                },
                rate_limiting: RateLimiting {
                    enabled: true,
                    tiers: vec![RateLimitTier {
                        tier: "starter".to_string(),
                        rate_limit: "100 calls/hour".to_string(),
                        authentication: "api_key".to_string(),
                    }],
                },
            },
            endpoints: Endpoints {
                api_documentation: "https://api.test.dev/docs".to_string(),
                health_check: "https://api.test.dev/api/v1/health".to_string(),
                authentication: AuthenticationEndpoints {
                    register: "https://api.test.dev/api/v1/auth/register".to_string(),
                    login: "https://api.test.dev/api/v1/auth/login".to_string(),
                },
                triggers: "https://api.test.dev/api/v1/triggers".to_string(),
            },
            contact: Contact {
                email: "test@test.dev".to_string(),
                github: "https://github.com/test".to_string(),
                documentation: "https://docs.test.dev".to_string(),
            },
            protocol_version: "erc-8004-v1.0".to_string(),
            generated_at: Utc::now(),
        };

        let json = serde_json::to_string(&agent_card).expect("Failed to serialize AgentCard");
        assert!(json.contains("Test API"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("Ethereum Sepolia"));
    }

    #[test]
    fn test_chain_info_serialization() {
        let chain_info = ChainInfo {
            chain_id: 84532,
            name: "Base Sepolia".to_string(),
            registries: vec![
                "identity".to_string(),
                "reputation".to_string(),
                "validation".to_string(),
            ],
        };

        let json = serde_json::to_string(&chain_info).expect("Failed to serialize ChainInfo");
        assert!(json.contains("84532"));
        assert!(json.contains("Base Sepolia"));
        assert!(json.contains("identity"));
    }

    #[test]
    fn test_pull_layer_note_optional() {
        // With note
        let pull_layer = PullLayer {
            enabled: false,
            features: vec![],
            note: Some("Coming soon in Phase 5".to_string()),
        };

        let json = serde_json::to_string(&pull_layer).expect("Failed to serialize");
        assert!(json.contains("Coming soon"));

        // Without note (should be omitted from JSON)
        let pull_layer_no_note = PullLayer {
            enabled: true,
            features: vec!["queries".to_string()],
            note: None,
        };

        let json_no_note = serde_json::to_string(&pull_layer_no_note).expect("Failed to serialize");
        assert!(!json_no_note.contains("note"));
    }

    #[test]
    fn test_rate_limit_tier_deserialization() {
        let json = r#"{
            "tier": "enterprise",
            "rate_limit": "2000 calls/hour",
            "authentication": "api_key"
        }"#;

        let tier: RateLimitTier = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(tier.tier, "enterprise");
        assert_eq!(tier.rate_limit, "2000 calls/hour");
        assert_eq!(tier.authentication, "api_key");
    }
}
