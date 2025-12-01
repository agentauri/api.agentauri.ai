//! Discovery endpoint handler
//!
//! Implements the /.well-known/agent.json endpoint for system discoverability.

use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use redis::AsyncCommands;
use tracing::{debug, error, info, instrument};

use crate::models::discovery::{
    AgentCard, Authentication, AuthenticationEndpoints, Capabilities, ChainInfo, Contact,
    Endpoints, PullLayer, PushLayer, RateLimitTier, RateLimiting,
};

const CACHE_KEY: &str = "discovery:agent_card";
const CACHE_TTL_SECONDS: u64 = 3600; // 1 hour

/// GET /.well-known/agent.json
///
/// Returns the Agent Card with system metadata and capabilities.
///
/// # Authentication
///
/// None required - this is a public endpoint.
///
/// # Caching
///
/// Response is cached in Redis for 1 hour and includes HTTP Cache-Control header.
///
/// # CORS
///
/// Supports cross-origin requests (Access-Control-Allow-Origin: *).
///
/// # Response
///
/// ```json
/// {
///   "name": "ERC-8004 Backend API",
///   "version": "1.0.0",
///   "description": "...",
///   "capabilities": { ... },
///   "endpoints": { ... },
///   "contact": { ... }
/// }
/// ```
#[instrument(skip(config))]
pub async fn get_agent_card(config: web::Data<shared::Config>) -> Result<HttpResponse> {
    debug!("Discovery endpoint called");

    // Try Redis cache first
    match try_get_from_cache(&config.redis.connection_url()).await {
        Ok(Some(cached_json)) => {
            debug!("Serving Agent Card from cache");
            return Ok(build_response(cached_json));
        }
        Ok(None) => {
            debug!("Cache miss, generating fresh Agent Card");
        }
        Err(e) => {
            // Log but don't fail - continue with fresh generation
            error!(error = %e, "Redis cache error, generating fresh Agent Card");
        }
    }

    // Generate fresh Agent Card
    let agent_card = generate_agent_card();
    let json_body =
        serde_json::to_string(&agent_card).map_err(actix_web::error::ErrorInternalServerError)?;

    // Try to cache (non-blocking - don't fail if cache write fails)
    if let Err(e) = try_set_cache(&config.redis.connection_url(), &json_body).await {
        error!(error = %e, "Failed to cache Agent Card (non-critical)");
    }

    info!("Generated fresh Agent Card");
    Ok(build_response(json_body))
}

/// Try to get cached Agent Card from Redis
async fn try_get_from_cache(redis_url: &str) -> anyhow::Result<Option<String>> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    let cached: Option<String> = conn.get(CACHE_KEY).await?;
    Ok(cached)
}

/// Try to cache Agent Card in Redis
async fn try_set_cache(redis_url: &str, json_body: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: () = conn.set_ex(CACHE_KEY, json_body, CACHE_TTL_SECONDS).await?;
    Ok(())
}

/// Build HTTP response with appropriate headers
fn build_response(json_body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("Cache-Control", "public, max-age=3600"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Access-Control-Allow-Methods", "GET, OPTIONS"))
        .insert_header(("Access-Control-Allow-Headers", "Content-Type"))
        .body(json_body)
}

/// Generate Agent Card with system metadata
fn generate_agent_card() -> AgentCard {
    // Use environment variables for dynamic values
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.8004.dev".to_string());
    let contact_email =
        std::env::var("CONTACT_EMAIL").unwrap_or_else(|_| "support@8004.dev".to_string());
    let contact_github = std::env::var("CONTACT_GITHUB")
        .unwrap_or_else(|_| "https://github.com/erc-8004/api.8004.dev".to_string());
    let documentation_url =
        std::env::var("DOCUMENTATION_URL").unwrap_or_else(|_| "https://docs.8004.dev".to_string());

    AgentCard {
        name: "ERC-8004 Backend API".to_string(),
        version: "1.0.0".to_string(),
        description: "Real-time backend infrastructure for monitoring and reacting to ERC-8004 on-chain agent economy events".to_string(),
        api_version: "v1".to_string(),
        base_url: base_url.clone(),
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
                    ChainInfo {
                        chain_id: 59141,
                        name: "Linea Sepolia".to_string(),
                        registries: vec![
                            "identity".to_string(),
                            "reputation".to_string(),
                            "validation".to_string(),
                        ],
                    },
                    ChainInfo {
                        chain_id: 80002,
                        name: "Polygon Amoy".to_string(),
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
                ],
            },
        },
        endpoints: Endpoints {
            api_documentation: format!("{}/docs", base_url),
            health_check: format!("{}/api/v1/health", base_url),
            authentication: AuthenticationEndpoints {
                register: format!("{}/api/v1/auth/register", base_url),
                login: format!("{}/api/v1/auth/login", base_url),
            },
            triggers: format!("{}/api/v1/triggers", base_url),
        },
        contact: Contact {
            email: contact_email,
            github: contact_github,
            documentation: documentation_url,
        },
        protocol_version: "erc-8004-v1.0".to_string(),
        generated_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_agent_card() {
        let agent_card = generate_agent_card();

        // Basic fields
        assert_eq!(agent_card.name, "ERC-8004 Backend API");
        assert_eq!(agent_card.version, "1.0.0");
        assert_eq!(agent_card.api_version, "v1");
        assert_eq!(agent_card.protocol_version, "erc-8004-v1.0");

        // Push layer
        assert!(agent_card.capabilities.push_layer.enabled);
        assert_eq!(agent_card.capabilities.push_layer.features.len(), 5);
        assert_eq!(agent_card.capabilities.push_layer.supported_chains.len(), 4);

        // Pull layer
        assert!(!agent_card.capabilities.pull_layer.enabled);
        assert!(agent_card.capabilities.pull_layer.note.is_some());

        // Authentication
        assert_eq!(agent_card.capabilities.authentication.methods.len(), 3);
        assert!(!agent_card.capabilities.authentication.oauth2_supported);

        // Rate limiting
        assert!(agent_card.capabilities.rate_limiting.enabled);
        assert_eq!(agent_card.capabilities.rate_limiting.tiers.len(), 4);
    }

    #[test]
    fn test_generate_agent_card_supported_chains() {
        let agent_card = generate_agent_card();
        let chains = &agent_card.capabilities.push_layer.supported_chains;

        // Ethereum Sepolia
        assert_eq!(chains[0].chain_id, 11155111);
        assert_eq!(chains[0].name, "Ethereum Sepolia");
        assert_eq!(chains[0].registries.len(), 3);

        // Base Sepolia
        assert_eq!(chains[1].chain_id, 84532);
        assert_eq!(chains[1].name, "Base Sepolia");

        // Linea Sepolia
        assert_eq!(chains[2].chain_id, 59141);
        assert_eq!(chains[2].name, "Linea Sepolia");

        // Polygon Amoy
        assert_eq!(chains[3].chain_id, 80002);
        assert_eq!(chains[3].name, "Polygon Amoy");
    }

    #[test]
    fn test_generate_agent_card_rate_limit_tiers() {
        let agent_card = generate_agent_card();
        let tiers = &agent_card.capabilities.rate_limiting.tiers;

        // Anonymous tier
        assert_eq!(tiers[0].tier, "anonymous");
        assert_eq!(tiers[0].rate_limit, "10 calls/hour");
        assert_eq!(tiers[0].authentication, "none");

        // Starter tier
        assert_eq!(tiers[1].tier, "starter");
        assert_eq!(tiers[1].rate_limit, "100 calls/hour");
        assert_eq!(tiers[1].authentication, "api_key");

        // Pro tier
        assert_eq!(tiers[2].tier, "pro");
        assert_eq!(tiers[2].rate_limit, "500 calls/hour");

        // Enterprise tier
        assert_eq!(tiers[3].tier, "enterprise");
        assert_eq!(tiers[3].rate_limit, "2000 calls/hour");
    }

    #[test]
    fn test_generate_agent_card_authentication_methods() {
        let agent_card = generate_agent_card();
        let methods = &agent_card.capabilities.authentication.methods;

        assert!(methods.contains(&"jwt".to_string()));
        assert!(methods.contains(&"api_key".to_string()));
        assert!(methods.contains(&"wallet_signature".to_string()));
        assert_eq!(methods.len(), 3);
    }

    #[test]
    fn test_generate_agent_card_endpoints() {
        // Set test environment variable
        std::env::set_var("BASE_URL", "https://test.api.dev");

        let agent_card = generate_agent_card();

        assert_eq!(
            agent_card.endpoints.api_documentation,
            "https://test.api.dev/docs"
        );
        assert_eq!(
            agent_card.endpoints.health_check,
            "https://test.api.dev/api/v1/health"
        );
        assert_eq!(
            agent_card.endpoints.authentication.register,
            "https://test.api.dev/api/v1/auth/register"
        );
        assert_eq!(
            agent_card.endpoints.authentication.login,
            "https://test.api.dev/api/v1/auth/login"
        );
        assert_eq!(
            agent_card.endpoints.triggers,
            "https://test.api.dev/api/v1/triggers"
        );

        // Clean up
        std::env::remove_var("BASE_URL");
    }

    #[test]
    fn test_generate_agent_card_contact_env_override() {
        // Set test environment variables
        std::env::set_var("CONTACT_EMAIL", "test@example.com");
        std::env::set_var("CONTACT_GITHUB", "https://github.com/test/repo");
        std::env::set_var("DOCUMENTATION_URL", "https://test-docs.example.com");

        let agent_card = generate_agent_card();

        assert_eq!(agent_card.contact.email, "test@example.com");
        assert_eq!(agent_card.contact.github, "https://github.com/test/repo");
        assert_eq!(
            agent_card.contact.documentation,
            "https://test-docs.example.com"
        );

        // Clean up
        std::env::remove_var("CONTACT_EMAIL");
        std::env::remove_var("CONTACT_GITHUB");
        std::env::remove_var("DOCUMENTATION_URL");
    }

    #[test]
    fn test_build_response_headers() {
        let json_body = r#"{"test": "data"}"#.to_string();
        let response = build_response(json_body);

        // Check status code
        assert_eq!(response.status(), 200);

        // Check headers
        let headers = response.headers();
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
        assert_eq!(
            headers.get("cache-control").unwrap(),
            "public, max-age=3600"
        );
        assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
        assert_eq!(
            headers.get("access-control-allow-methods").unwrap(),
            "GET, OPTIONS"
        );
    }

    #[test]
    fn test_agent_card_serialization_complete() {
        let agent_card = generate_agent_card();
        let json = serde_json::to_string(&agent_card).expect("Serialization failed");

        // Verify all major sections are present
        assert!(json.contains("ERC-8004 Backend API"));
        assert!(json.contains("push_layer"));
        assert!(json.contains("pull_layer"));
        assert!(json.contains("authentication"));
        assert!(json.contains("rate_limiting"));
        assert!(json.contains("supported_chains"));
        assert!(json.contains("Ethereum Sepolia"));
        assert!(json.contains("Base Sepolia"));
        assert!(json.contains("Linea Sepolia"));
        assert!(json.contains("Polygon Amoy"));
    }
}
