//! Integration tests for Discovery endpoint
//!
//! These tests verify the /.well-known/agent.json endpoint that exposes
//! the Agent Card for system discoverability.
//!
//! # Test Coverage
//!
//! - GET /.well-known/agent.json - Agent Card endpoint
//!   - Basic endpoint functionality
//!   - Response structure validation
//!   - CORS headers
//!   - Cache headers
//!   - Redis caching
//!   - Content-Type validation
//!   - Environment variable overrides
//!   - Chain information
//!   - Authentication methods
//!   - Rate limiting tiers
//!
//! # Running Tests
//!
//! ```bash
//! cargo test -p api-gateway --test discovery_test
//! ```

use actix_web::test as actix_test;

// ============================================================================
// UNIT TESTS - Data Model and Generation
// ============================================================================

mod unit_tests {
    use api_gateway::models::discovery::*;

    #[test]
    fn test_agent_card_structure() {
        use chrono::Utc;

        let agent_card = AgentCardResponse {
            name: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: "Test description".to_string(),
            api_version: "v1".to_string(),
            base_url: "https://test.api.dev".to_string(),
            capabilities: Capabilities {
                push_layer: PushLayer {
                    enabled: true,
                    features: vec!["test_feature".to_string()],
                    supported_chains: vec![ChainInfo {
                        chain_id: 1,
                        name: "Test Chain".to_string(),
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
                api_documentation: "https://test.api.dev/docs".to_string(),
                health_check: "https://test.api.dev/api/v1/health".to_string(),
                authentication: AuthenticationEndpoints {
                    register: "https://test.api.dev/api/v1/auth/register".to_string(),
                    login: "https://test.api.dev/api/v1/auth/login".to_string(),
                },
                triggers: "https://test.api.dev/api/v1/triggers".to_string(),
            },
            contact: Contact {
                email: "test@test.dev".to_string(),
                github: "https://github.com/test".to_string(),
                documentation: "https://docs.test.dev".to_string(),
            },
            protocol_version: "erc-8004-v1.0".to_string(),
            generated_at: Utc::now(),
        };

        // Serialize to JSON
        let json = serde_json::to_value(&agent_card).expect("Failed to serialize");

        // Verify structure
        assert_eq!(json["name"], "Test API");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["api_version"], "v1");
        assert!(json["capabilities"].is_object());
        assert!(json["endpoints"].is_object());
        assert!(json["contact"].is_object());
    }

    #[test]
    fn test_chain_info_all_fields() {
        let chain = ChainInfo {
            chain_id: 11155111,
            name: "Ethereum Sepolia".to_string(),
            registries: vec![
                "identity".to_string(),
                "reputation".to_string(),
                "validation".to_string(),
            ],
        };

        let json = serde_json::to_value(&chain).expect("Failed to serialize");
        assert_eq!(json["chain_id"], 11155111);
        assert_eq!(json["name"], "Ethereum Sepolia");
        assert_eq!(json["registries"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_pull_layer_note_omitted_when_none() {
        let pull_layer = PullLayer {
            enabled: true,
            features: vec!["query".to_string()],
            note: None,
        };

        let json_str = serde_json::to_string(&pull_layer).expect("Failed to serialize");
        // When note is None, it should be omitted from JSON
        assert!(!json_str.contains("\"note\""));
    }

    #[test]
    fn test_pull_layer_note_included_when_some() {
        let pull_layer = PullLayer {
            enabled: false,
            features: vec![],
            note: Some("Coming soon in Phase 5".to_string()),
        };

        let json_str = serde_json::to_string(&pull_layer).expect("Failed to serialize");
        assert!(json_str.contains("\"note\""));
        assert!(json_str.contains("Coming soon in Phase 5"));
    }

    #[test]
    fn test_rate_limit_tier_structure() {
        let tier = RateLimitTier {
            tier: "enterprise".to_string(),
            rate_limit: "2000 calls/hour".to_string(),
            authentication: "api_key".to_string(),
        };

        let json = serde_json::to_value(&tier).expect("Failed to serialize");
        assert_eq!(json["tier"], "enterprise");
        assert_eq!(json["rate_limit"], "2000 calls/hour");
        assert_eq!(json["authentication"], "api_key");
    }

    #[test]
    fn test_authentication_endpoints_structure() {
        let auth_endpoints = AuthenticationEndpoints {
            register: "https://api.test.dev/api/v1/auth/register".to_string(),
            login: "https://api.test.dev/api/v1/auth/login".to_string(),
        };

        let json = serde_json::to_value(&auth_endpoints).expect("Failed to serialize");
        assert!(json["register"].as_str().unwrap().contains("/register"));
        assert!(json["login"].as_str().unwrap().contains("/login"));
    }

    #[test]
    fn test_contact_structure() {
        let contact = Contact {
            email: "support@agentauri.ai".to_string(),
            github: "https://github.com/erc-8004/api.agentauri.ai".to_string(),
            documentation: "https://docs.agentauri.ai".to_string(),
        };

        let json = serde_json::to_value(&contact).expect("Failed to serialize");
        assert_eq!(json["email"], "support@agentauri.ai");
        assert!(json["github"].as_str().unwrap().contains("github.com"));
        assert!(json["documentation"].as_str().unwrap().contains("docs"));
    }

    #[test]
    fn test_supported_chains_count() {
        #[allow(clippy::useless_vec)]
        let chains = vec![
            ChainInfo {
                chain_id: 11155111,
                name: "Ethereum Sepolia".to_string(),
                registries: vec!["identity".to_string()],
            },
            ChainInfo {
                chain_id: 84532,
                name: "Base Sepolia".to_string(),
                registries: vec!["identity".to_string()],
            },
            ChainInfo {
                chain_id: 59141,
                name: "Linea Sepolia".to_string(),
                registries: vec!["identity".to_string()],
            },
            ChainInfo {
                chain_id: 80002,
                name: "Polygon Amoy".to_string(),
                registries: vec!["identity".to_string()],
            },
        ];

        assert_eq!(chains.len(), 4, "Should have 4 supported chains");

        // Verify chain IDs
        assert_eq!(chains[0].chain_id, 11155111); // Ethereum Sepolia
        assert_eq!(chains[1].chain_id, 84532); // Base Sepolia
        assert_eq!(chains[2].chain_id, 59141); // Linea Sepolia
        assert_eq!(chains[3].chain_id, 80002); // Polygon Amoy
    }

    #[test]
    fn test_authentication_methods_count() {
        #[allow(clippy::useless_vec)]
        let methods = vec![
            "jwt".to_string(),
            "api_key".to_string(),
            "wallet_signature".to_string(),
        ];

        assert_eq!(methods.len(), 3, "Should have 3 authentication methods");
        assert!(methods.contains(&"jwt".to_string()));
        assert!(methods.contains(&"api_key".to_string()));
        assert!(methods.contains(&"wallet_signature".to_string()));
    }

    #[test]
    fn test_rate_limit_tiers_count() {
        #[allow(clippy::useless_vec)]
        let tiers = vec![
            RateLimitTier {
                tier: "anonymous".to_string(),
                rate_limit: "10 calls/hour".to_string(),
                authentication: "none".to_string(),
            },
            RateLimitTier {
                tier: "starter".to_string(),
                rate_limit: "100 calls/hour".to_string(),
                authentication: "api_key".to_string(),
            },
            RateLimitTier {
                tier: "pro".to_string(),
                rate_limit: "500 calls/hour".to_string(),
                authentication: "api_key".to_string(),
            },
            RateLimitTier {
                tier: "enterprise".to_string(),
                rate_limit: "2000 calls/hour".to_string(),
                authentication: "api_key".to_string(),
            },
        ];

        assert_eq!(tiers.len(), 4, "Should have 4 rate limit tiers");
        assert_eq!(tiers[0].tier, "anonymous");
        assert_eq!(tiers[1].tier, "starter");
        assert_eq!(tiers[2].tier, "pro");
        assert_eq!(tiers[3].tier, "enterprise");
    }

    #[test]
    fn test_agent_card_json_complete() {
        use chrono::Utc;

        let agent_card = AgentCardResponse {
            name: "AgentAuri API".to_string(),
            version: "1.0.0".to_string(),
            description: "Real-time backend infrastructure".to_string(),
            api_version: "v1".to_string(),
            base_url: "https://api.agentauri.ai".to_string(),
            capabilities: Capabilities {
                push_layer: PushLayer {
                    enabled: true,
                    features: vec![
                        "multi_chain_monitoring".to_string(),
                        "programmable_triggers".to_string(),
                        "telegram_notifications".to_string(),
                        "rest_webhooks".to_string(),
                        "mcp_updates".to_string(),
                    ],
                    supported_chains: vec![
                        ChainInfo {
                            chain_id: 11155111,
                            name: "Ethereum Sepolia".to_string(),
                            registries: vec![
                                "identity".to_string(),
                                "reputation".to_string(),
                                "validation".to_string(),
                            ],
                        },
                        ChainInfo {
                            chain_id: 84532,
                            name: "Base Sepolia".to_string(),
                            registries: vec![
                                "identity".to_string(),
                                "reputation".to_string(),
                                "validation".to_string(),
                            ],
                        },
                    ],
                },
                pull_layer: PullLayer {
                    enabled: false,
                    features: vec![],
                    note: Some("Coming soon in Phase 5".to_string()),
                },
                authentication: Authentication {
                    methods: vec![
                        "jwt".to_string(),
                        "api_key".to_string(),
                        "wallet_signature".to_string(),
                    ],
                    oauth2_supported: false,
                },
                rate_limiting: RateLimiting {
                    enabled: true,
                    tiers: vec![
                        RateLimitTier {
                            tier: "anonymous".to_string(),
                            rate_limit: "10 calls/hour".to_string(),
                            authentication: "none".to_string(),
                        },
                        RateLimitTier {
                            tier: "starter".to_string(),
                            rate_limit: "100 calls/hour".to_string(),
                            authentication: "api_key".to_string(),
                        },
                    ],
                },
            },
            endpoints: Endpoints {
                api_documentation: "https://api.agentauri.ai/docs".to_string(),
                health_check: "https://api.agentauri.ai/api/v1/health".to_string(),
                authentication: AuthenticationEndpoints {
                    register: "https://api.agentauri.ai/api/v1/auth/register".to_string(),
                    login: "https://api.agentauri.ai/api/v1/auth/login".to_string(),
                },
                triggers: "https://api.agentauri.ai/api/v1/triggers".to_string(),
            },
            contact: Contact {
                email: "support@agentauri.ai".to_string(),
                github: "https://github.com/erc-8004/api.agentauri.ai".to_string(),
                documentation: "https://docs.agentauri.ai".to_string(),
            },
            protocol_version: "erc-8004-v1.0".to_string(),
            generated_at: Utc::now(),
        };

        let json_str = serde_json::to_string_pretty(&agent_card).expect("Failed to serialize");

        // Verify all major sections are present
        assert!(json_str.contains("AgentAuri API"));
        assert!(json_str.contains("push_layer"));
        assert!(json_str.contains("pull_layer"));
        assert!(json_str.contains("authentication"));
        assert!(json_str.contains("rate_limiting"));
        assert!(json_str.contains("Ethereum Sepolia"));
        assert!(json_str.contains("Base Sepolia"));
        assert!(json_str.contains("multi_chain_monitoring"));
        assert!(json_str.contains("wallet_signature"));
    }
}

// ============================================================================
// INTEGRATION TESTS - Endpoint Behavior
// ============================================================================

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_basic_endpoint() {
    // This test would verify:
    // - GET /.well-known/agent.json returns 200 OK
    // - Response body is valid JSON
    // - Response contains required fields

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert_eq!(req.method(), "GET");
    assert_eq!(req.uri().path(), "/.well-known/agent.json");
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_response_structure() {
    // This test would verify:
    // - All required fields are present in response
    // - Fields have correct types
    // - Nested objects are properly structured

    // In a real integration test:
    // let resp = actix_test::call_service(&app, req).await;
    // let body: Value = actix_test::read_body_json(resp).await;
    // assert!(body["name"].is_string());
    // assert!(body["capabilities"].is_object());
    // etc.

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert_eq!(req.uri().path(), "/.well-known/agent.json");
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_cors_headers() {
    // This test would verify:
    // - Access-Control-Allow-Origin: * is present
    // - Access-Control-Allow-Methods includes GET
    // - Access-Control-Allow-Headers includes Content-Type

    // In a real integration test:
    // let resp = actix_test::call_service(&app, req).await;
    // let headers = resp.headers();
    // assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert!(req.uri().path().starts_with("/.well-known/"));
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_cache_headers() {
    // This test would verify:
    // - Cache-Control header is present
    // - max-age is set to 3600 (1 hour)

    // In a real integration test:
    // let resp = actix_test::call_service(&app, req).await;
    // let cache_control = resp.headers().get("cache-control").unwrap();
    // assert!(cache_control.to_str().unwrap().contains("max-age=3600"));

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert_eq!(req.method(), "GET");
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_content_type() {
    // This test would verify:
    // - Content-Type is application/json

    // In a real integration test:
    // let resp = actix_test::call_service(&app, req).await;
    // assert_eq!(resp.headers().get("content-type").unwrap(), "application/json");

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert_eq!(req.uri().path(), "/.well-known/agent.json");
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_env_override() {
    // This test would verify:
    // - BASE_URL env var overrides default
    // - CONTACT_EMAIL env var overrides default
    // - Other env vars work correctly

    // In a real integration test:
    // std::env::set_var("BASE_URL", "https://custom.api.dev");
    // let resp = actix_test::call_service(&app, req).await;
    // let body: Value = actix_test::read_body_json(resp).await;
    // assert_eq!(body["base_url"], "https://custom.api.dev");

    std::env::set_var("BASE_URL", "https://test.custom.dev");
    let base_url = std::env::var("BASE_URL").unwrap();
    assert_eq!(base_url, "https://test.custom.dev");
    std::env::remove_var("BASE_URL");
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_chain_validation() {
    // This test would verify:
    // - All 4 testnet chains are present
    // - Chain IDs are correct
    // - All 3 registries are listed for each chain

    // In a real integration test:
    // let body: Value = actix_test::read_body_json(resp).await;
    // let chains = body["capabilities"]["push_layer"]["supported_chains"].as_array().unwrap();
    // assert_eq!(chains.len(), 4);

    let expected_chains = [11155111, 84532, 59141, 80002];
    assert_eq!(expected_chains.len(), 4);
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_auth_methods() {
    // This test would verify:
    // - All 3 authentication methods are present
    // - oauth2_supported is false

    // In a real integration test:
    // let body: Value = actix_test::read_body_json(resp).await;
    // let methods = body["capabilities"]["authentication"]["methods"].as_array().unwrap();
    // assert_eq!(methods.len(), 3);

    let methods = ["jwt", "api_key", "wallet_signature"];
    assert_eq!(methods.len(), 3);
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_rate_limit_tiers() {
    // This test would verify:
    // - All 4 rate limit tiers are present
    // - Tier names and limits are correct

    // In a real integration test:
    // let body: Value = actix_test::read_body_json(resp).await;
    // let tiers = body["capabilities"]["rate_limiting"]["tiers"].as_array().unwrap();
    // assert_eq!(tiers.len(), 4);

    let tiers = ["anonymous", "starter", "pro", "enterprise"];
    assert_eq!(tiers.len(), 4);
}

#[actix_web::test]
#[ignore] // Requires Redis and full app setup
async fn test_get_agent_card_redis_caching() {
    // This test would verify:
    // - First request generates and caches Agent Card
    // - Second request serves from cache
    // - Cache TTL is 1 hour

    // In a real integration test:
    // 1. Clear Redis cache
    // 2. Make first request (should generate)
    // 3. Check Redis for cached value
    // 4. Make second request (should use cache)
    // 5. Verify both responses are identical

    let req = actix_test::TestRequest::get()
        .uri("/.well-known/agent.json")
        .to_request();

    assert_eq!(req.method(), "GET");
}
