//! OpenAPI Documentation Configuration
//!
//! This module configures the OpenAPI 3.0 specification for the API Gateway.
//! It uses utoipa to generate documentation from Rust types and handler annotations.

use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers;
use crate::handlers::agents::{AgentLinkResponse, LinkAgentRequest};
use crate::handlers::billing::PurchaseCreditsRequestWithOrg;
use crate::handlers::health::HealthResponse;
use crate::handlers::ponder::{ChainSyncStatus, PonderStatusError, PonderStatusResponse};
use crate::models;

/// OpenAPI documentation for the AgentAuri API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AgentAuri API",
        version = "1.0.0",
        description = "Real-time backend infrastructure for monitoring and reacting to ERC-8004 on-chain agent economy events.\n\n## Authentication\n\nThe API supports a 3-layer authentication system:\n\n- **Layer 0 (Anonymous)**: IP-based rate limiting (10 calls/hour)\n- **Layer 1 (API Key)**: Account-based access with `X-API-Key` header\n- **Layer 2 (JWT)**: Full user access with `Authorization: Bearer <token>`\n\n## Rate Limiting\n\nAll endpoints are rate limited. Check response headers for quota information:\n- `X-RateLimit-Limit`: Maximum requests per hour\n- `X-RateLimit-Remaining`: Remaining requests\n- `X-RateLimit-Reset`: Unix timestamp when limit resets",
        contact(
            name = "AgentAuri Team",
            email = "support@agentauri.ai",
            url = "https://github.com/agentauri/api.agentauri.ai"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.agentauri.ai", description = "Production server")
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Authentication", description = "User registration, login, and OAuth"),
        (name = "Organizations", description = "Organization management"),
        (name = "Members", description = "Organization member management"),
        (name = "API Keys", description = "API key management for programmatic access"),
        (name = "OAuth Clients", description = "OAuth 2.0 client management"),
        (name = "Triggers", description = "Trigger configuration for event-driven actions"),
        (name = "Conditions", description = "Trigger condition management"),
        (name = "Actions", description = "Trigger action management"),
        (name = "Circuit Breaker", description = "Circuit breaker state and configuration"),
        (name = "Agents", description = "On-chain agent linking"),
        (name = "Agent Follows", description = "Simplified agent monitoring across all registries"),
        (name = "Billing", description = "Credit balance and transactions"),
        (name = "Discovery", description = "API discovery and metadata"),
        (name = "Ponder", description = "Blockchain indexer status and metrics"),
        (name = "A2A Protocol", description = "Agent-to-Agent JSON-RPC 2.0 protocol for async task queries")
    ),
    modifiers(&SecurityAddon),
    paths(
        // Health
        handlers::health_check,
        // Discovery
        handlers::openapi_json,
        handlers::get_agent_card,
        handlers::get_security_txt,
        // Authentication
        handlers::register,
        handlers::login,
        // Social OAuth
        handlers::google_auth,
        handlers::google_callback,
        handlers::github_auth,
        handlers::github_callback,
        // Organizations
        handlers::create_organization,
        handlers::list_organizations,
        handlers::get_organization,
        handlers::update_organization,
        handlers::delete_organization,
        handlers::transfer_ownership,
        // Members
        handlers::add_member,
        handlers::list_members,
        handlers::update_member_role,
        handlers::remove_member,
        // API Keys
        handlers::create_api_key,
        handlers::list_api_keys,
        handlers::get_api_key,
        handlers::revoke_api_key,
        handlers::rotate_api_key,
        // OAuth Clients
        handlers::create_oauth_client,
        handlers::list_oauth_clients,
        handlers::delete_oauth_client,
        handlers::token_endpoint,
        // Triggers
        handlers::create_trigger,
        handlers::list_triggers,
        handlers::get_trigger,
        handlers::update_trigger,
        handlers::delete_trigger,
        // Conditions
        handlers::create_condition,
        handlers::list_conditions,
        handlers::update_condition,
        handlers::delete_condition,
        // Actions
        handlers::create_action,
        handlers::list_actions,
        handlers::update_action,
        handlers::delete_action,
        // Circuit Breaker
        handlers::get_circuit_breaker_state,
        handlers::update_circuit_breaker_config,
        handlers::reset_circuit_breaker,
        // Agents
        handlers::link_agent,
        handlers::list_linked_agents,
        handlers::unlink_agent,
        // Agent Follows
        handlers::follow_agent,
        handlers::list_following,
        handlers::update_follow,
        handlers::unfollow_agent,
        // Billing
        handlers::get_credits,
        handlers::purchase_credits,
        handlers::list_transactions,
        handlers::get_subscription,
        handlers::handle_stripe_webhook,
        // Ponder
        handlers::get_ponder_status,
        handlers::get_ponder_events,
        // A2A Protocol
        handlers::a2a_rpc,
        handlers::get_task_status,
        handlers::stream_task_progress,
    ),
    components(
        schemas(
            // Common
            models::ErrorResponse,
            models::SuccessResponse<serde_json::Value>,
            models::PaginationMeta,
            // Auth
            models::RegisterRequest,
            models::LoginRequest,
            models::AuthResponse,
            models::UserResponse,
            // Organizations
            models::CreateOrganizationRequest,
            models::UpdateOrganizationRequest,
            models::OrganizationResponse,
            models::OrganizationWithRoleResponse,
            // Members
            models::AddMemberRequest,
            models::UpdateMemberRoleRequest,
            models::MemberResponse,
            // API Keys
            models::CreateApiKeyRequest,
            models::ApiKeyResponse,
            models::ApiKeyCreatedResponse,
            models::ApiKeyListResponse,
            models::RevokeApiKeyRequest,
            models::RotateApiKeyRequest,
            models::RotateApiKeyResponse,
            // OAuth
            models::CreateOAuthClientRequest,
            models::CreateOAuthClientResponse,
            models::OAuthClientResponse,
            models::TokenRequest,
            models::TokenResponse,
            // Triggers
            models::CreateTriggerRequest,
            models::UpdateTriggerRequest,
            models::TriggerResponse,
            models::TriggerDetailResponse,
            // Conditions
            models::CreateConditionRequest,
            models::UpdateConditionRequest,
            models::ConditionResponse,
            // Actions
            models::CreateActionRequest,
            models::UpdateActionRequest,
            models::ActionResponse,
            // Circuit Breaker
            models::CircuitBreakerStateResponse,
            models::CircuitBreakerConfigResponse,
            models::UpdateCircuitBreakerConfigRequest,
            // Billing
            models::billing::CreditBalanceResponse,
            models::billing::CreditTransactionResponse,
            models::billing::PurchaseCreditsRequest,
            models::billing::PurchaseCreditsResponse,
            models::billing::SubscriptionResponse,
            PurchaseCreditsRequestWithOrg,
            // Agents
            LinkAgentRequest,
            AgentLinkResponse,
            // Agent Follows
            models::agent_follows::FollowAgentRequest,
            models::agent_follows::FollowActionRequest,
            models::agent_follows::UpdateFollowRequest,
            models::agent_follows::AgentFollowResponse,
            models::agent_follows::AgentFollowDetailResponse,
            models::agent_follows::FollowActionSummary,
            models::agent_follows::TriggerIds,
            // Discovery
            models::discovery::AgentCardResponse,
            // Health
            HealthResponse,
            // Ponder
            PonderStatusResponse,
            PonderStatusError,
            ChainSyncStatus,
            // A2A Protocol
            models::a2a::JsonRpcRequest,
            models::a2a::JsonRpcResponse<serde_json::Value>,
            models::a2a::JsonRpcError,
            models::a2a::TaskStatus,
            models::a2a::TaskDefinition,
            models::a2a::TaskSendParams,
            models::a2a::TaskGetParams,
            models::a2a::TaskCancelParams,
            models::a2a::TaskSendResult,
            models::a2a::TaskGetResult,
            models::a2a::TaskCancelResult,
        )
    )
)]
pub struct ApiDoc;

/// Security scheme modifier for adding authentication options
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);

        // JWT Bearer token authentication (Layer 2)
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "JWT token obtained from /api/v1/auth/login. Valid for 1 hour.",
                    ))
                    .build(),
            ),
        );

        // API Key authentication (Layer 1)
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-API-Key",
                "API key in format: sk_live_xxx or sk_test_xxx. Create via /api/v1/api-keys.",
            ))),
        );

        // Organization ID header (required for multi-tenant endpoints)
        components.add_security_scheme(
            "organization_id",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "X-Organization-ID",
                "Organization UUID. Required for organization-scoped operations.",
            ))),
        );
    }
}
