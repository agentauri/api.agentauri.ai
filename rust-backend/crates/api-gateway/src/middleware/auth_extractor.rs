//! Authentication Context Extractor
//!
//! This module provides utilities for extracting authentication context from
//! HTTP requests to determine the rate limiting scope and tier.
//!
//! # Authentication Layers (Precedence Order)
//!
//! 1. **Layer 2 (Wallet Signature)**: Agent-based authentication via EIP-191
//!    - Inherits organization limits from linked organization
//!    - Additional agent-specific operations allowed
//!
//! 2. **Layer 1 (API Key)**: Organization-based authentication
//!    - API Key determines organization and plan
//!    - Rate limits based on subscription plan
//!
//! 3. **Layer 0 (Anonymous)**: IP-based authentication
//!    - No authentication required
//!    - Strict rate limits (10 requests/hour)
//!    - Tier 0-1 queries only

use crate::middleware::{ip_extractor, ApiKeyAuth};
use crate::models::Claims;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpRequest,
};
use futures_util::future::LocalBoxFuture;
use shared::models::Organization;
use shared::DbPool;
use std::future::{ready, Ready};
use tracing::debug;

/// Authentication layer (for rate limiting scope)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLayer {
    /// Layer 0: Anonymous (IP-based)
    Anonymous,
    /// Layer 1: API Key (Organization-based)
    ApiKey,
    /// Layer 2: Wallet Signature (Agent-based, inherits from org)
    #[allow(dead_code)] // Future feature: Layer 2 Wallet authentication
    WalletSignature,
}

impl AuthLayer {
    /// Get the layer name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthLayer::Anonymous => "anonymous",
            AuthLayer::ApiKey => "api_key",
            AuthLayer::WalletSignature => "wallet_signature",
        }
    }

    /// Get the layer priority (higher = takes precedence)
    #[allow(dead_code)] // Future feature: Used for Layer 2 precedence logic
    pub fn priority(&self) -> u8 {
        match self {
            AuthLayer::Anonymous => 0,
            AuthLayer::ApiKey => 1,
            AuthLayer::WalletSignature => 2,
        }
    }
}

/// Authentication context for rate limiting
///
/// This struct contains all the information needed to determine:
/// - Which rate limit scope to use (IP, Organization, Agent)
/// - What limits apply (based on plan)
/// - Who to audit log entries to
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Authentication layer used
    pub layer: AuthLayer,

    /// User ID (Layer 1 JWT, or Layer 2 wallet-linked user)
    #[allow(dead_code)] // Future feature: Layer 2 Wallet authentication
    pub user_id: Option<String>,

    /// Organization ID (Layer 1 API Key, or Layer 2 agent's org)
    pub organization_id: Option<String>,

    /// Agent ID (Layer 2 only)
    #[allow(dead_code)] // Future feature: Layer 2 Wallet authentication
    pub agent_id: Option<i64>,

    /// Client IP address (always present for Layer 0 fallback)
    pub ip_address: String,

    /// Subscription plan (determines rate limits)
    pub plan: String,

    /// Rate limit override (from API key or organization settings)
    pub rate_limit_override: Option<i32>,
}

impl AuthContext {
    /// Create a Layer 0 (Anonymous) auth context
    ///
    /// # Arguments
    ///
    /// * `ip_address` - Client IP address
    pub fn anonymous(ip_address: String) -> Self {
        Self {
            layer: AuthLayer::Anonymous,
            user_id: None,
            organization_id: None,
            agent_id: None,
            ip_address,
            plan: "anonymous".to_string(),
            rate_limit_override: Some(10), // Anonymous limit: 10/hour
        }
    }

    /// Create a Layer 1 (API Key) auth context
    ///
    /// # Arguments
    ///
    /// * `api_key_auth` - API key authentication context
    /// * `ip_address` - Client IP address
    /// * `plan` - Organization subscription plan
    pub fn api_key(api_key_auth: &ApiKeyAuth, ip_address: String, plan: String) -> Self {
        Self {
            layer: AuthLayer::ApiKey,
            user_id: None, // API keys don't have user context
            organization_id: Some(api_key_auth.api_key.organization_id.clone()),
            agent_id: None,
            ip_address,
            plan,
            rate_limit_override: api_key_auth.api_key.rate_limit_override,
        }
    }

    /// Create a Layer 2 (Wallet Signature) auth context
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID from JWT claims
    /// * `organization_id` - Organization ID (from agent linking)
    /// * `agent_id` - Agent ID (from agent linking)
    /// * `ip_address` - Client IP address
    /// * `plan` - Organization subscription plan
    #[allow(dead_code)] // Future feature: Layer 2 Wallet authentication
    pub fn wallet_signature(
        user_id: String,
        organization_id: String,
        agent_id: i64,
        ip_address: String,
        plan: String,
    ) -> Self {
        Self {
            layer: AuthLayer::WalletSignature,
            user_id: Some(user_id),
            organization_id: Some(organization_id.clone()),
            agent_id: Some(agent_id),
            ip_address,
            plan,
            rate_limit_override: None,
        }
    }

    /// Get the rate limit (requests per hour) for this context
    ///
    /// Returns the configured limit based on:
    /// 1. API key override (if present)
    /// 2. Subscription plan limits
    /// 3. Anonymous limit (Layer 0)
    pub fn get_rate_limit(&self) -> i32 {
        // Check for override first
        if let Some(override_limit) = self.rate_limit_override {
            return override_limit;
        }

        // Plan-based limits
        match self.plan.as_str() {
            "free" => 50,
            "starter" => 100,
            "pro" => 500,
            "enterprise" => 2000,
            "anonymous" => 10,
            _ => {
                tracing::warn!(plan = %self.plan, "Unknown plan, defaulting to free tier");
                50
            }
        }
    }

    /// Get the rate limit scope for Redis key
    pub fn get_scope(&self) -> shared::RateLimitScope {
        match self.layer {
            AuthLayer::Anonymous => shared::RateLimitScope::Ip(self.ip_address.clone()),
            AuthLayer::ApiKey | AuthLayer::WalletSignature => {
                if let Some(org_id) = &self.organization_id {
                    shared::RateLimitScope::Organization(org_id.clone())
                } else {
                    // Fallback to IP if org not available (shouldn't happen)
                    tracing::warn!("Organization ID missing for Layer 1/2, using IP fallback");
                    shared::RateLimitScope::Ip(self.ip_address.clone())
                }
            }
        }
    }

    /// Check if this context allows access to a specific query tier
    ///
    /// # Arguments
    ///
    /// * `tier` - Query tier (0-3)
    ///
    /// # Returns
    ///
    /// `true` if the tier is allowed, `false` otherwise
    #[allow(dead_code)] // Future feature: Tier-based query authorization
    pub fn allows_tier(&self, tier: u8) -> bool {
        match self.layer {
            AuthLayer::Anonymous => tier <= 1, // Tier 0-1 only
            AuthLayer::ApiKey | AuthLayer::WalletSignature => tier <= 3, // All tiers
        }
    }
}

/// Extract authentication context from the request
///
/// This function determines the authentication layer by checking:
/// 1. JWT claims (if present) - Could be Layer 2 if agent-linked
/// 2. API key auth (if present) - Layer 1
/// 3. IP address only - Layer 0
///
/// # Arguments
///
/// * `req` - HTTP request
/// * `pool` - Database connection pool
///
/// # Returns
///
/// `AuthContext` with the highest priority authentication method detected
pub async fn extract_auth_context(req: &HttpRequest, pool: &DbPool) -> AuthContext {
    let ip_address = ip_extractor::extract_ip(req);

    // Check for API Key auth (Layer 1) - stored in request extensions by DualAuth middleware
    // Clone the api_key_auth to avoid holding RefCell borrow across await point
    let api_key_auth_opt = req.extensions().get::<ApiKeyAuth>().cloned();

    if let Some(api_key_auth) = api_key_auth_opt {
        // Look up organization plan
        let plan = get_organization_plan(pool, &api_key_auth.api_key.organization_id)
            .await
            .unwrap_or_else(|| {
                tracing::warn!(
                    org_id = %api_key_auth.api_key.organization_id,
                    "Failed to get organization plan, defaulting to free"
                );
                "free".to_string()
            });

        debug!(
            layer = "api_key",
            org_id = %api_key_auth.api_key.organization_id,
            plan = %plan,
            ip = %ip_address,
            "Extracted Layer 1 auth context"
        );

        return AuthContext::api_key(&api_key_auth, ip_address, plan);
    }

    // Check for JWT claims (Layer 1 authentication without API key)
    // Note: Layer 2 (Wallet Signature) is handled via agent_links table
    // and WalletService, not through JWT claims directly
    if let Some(claims) = req.extensions().get::<Claims>() {
        debug!(
            user_id = %claims.sub,
            "JWT detected without API key - falling back to Layer 0 (anonymous)"
        );
    }

    // Default to Layer 0 (Anonymous)
    debug!(
        layer = "anonymous",
        ip = %ip_address,
        "Extracted Layer 0 auth context"
    );

    AuthContext::anonymous(ip_address)
}

/// Get the organization's subscription plan
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `organization_id` - Organization ID
///
/// # Returns
///
/// The plan name, or `None` if the organization doesn't exist
async fn get_organization_plan(pool: &DbPool, organization_id: &str) -> Option<String> {
    match sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
        .bind(organization_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(org)) => Some(org.plan),
        Ok(None) => None,
        Err(e) => {
            tracing::error!(
                org_id = %organization_id,
                error = %e,
                "Failed to fetch organization plan"
            );
            None
        }
    }
}

/// AuthExtractor middleware
///
/// Extracts authentication context from the request and stores it in extensions.
/// This middleware should run BEFORE UnifiedRateLimiter.
pub struct AuthExtractor;

impl AuthExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthExtractorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthExtractorMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct AuthExtractorMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthExtractorMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Extract database pool from app state
            let pool = match req.app_data::<actix_web::web::Data<DbPool>>() {
                Some(pool) => pool.get_ref().clone(),
                None => {
                    tracing::error!("Database pool not found in app state");
                    return Err(actix_web::error::ErrorInternalServerError(
                        "Database configuration error",
                    ));
                }
            };

            // Extract HTTP request reference before consuming ServiceRequest
            let http_req = req.request();

            // Extract auth context
            let auth_ctx = extract_auth_context(http_req, &pool).await;

            debug!(
                layer = %auth_ctx.layer.as_str(),
                plan = %auth_ctx.plan,
                rate_limit = auth_ctx.get_rate_limit(),
                "Auth context extracted"
            );

            // Store auth context in request extensions
            req.extensions_mut().insert(auth_ctx);

            // Continue to next service
            service.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_layer_priority() {
        assert_eq!(AuthLayer::Anonymous.priority(), 0);
        assert_eq!(AuthLayer::ApiKey.priority(), 1);
        assert_eq!(AuthLayer::WalletSignature.priority(), 2);
    }

    #[test]
    fn test_anonymous_context_rate_limit() {
        let ctx = AuthContext::anonymous("192.168.1.1".to_string());

        assert_eq!(ctx.layer, AuthLayer::Anonymous);
        assert_eq!(ctx.get_rate_limit(), 10);
        assert_eq!(ctx.plan, "anonymous");
        assert!(ctx.allows_tier(0));
        assert!(ctx.allows_tier(1));
        assert!(!ctx.allows_tier(2));
        assert!(!ctx.allows_tier(3));
    }

    #[test]
    fn test_plan_based_rate_limits() {
        let ctx = AuthContext {
            layer: AuthLayer::ApiKey,
            user_id: None,
            organization_id: Some("org_123".to_string()),
            agent_id: None,
            ip_address: "192.168.1.1".to_string(),
            plan: "pro".to_string(),
            rate_limit_override: None,
        };

        assert_eq!(ctx.get_rate_limit(), 500);
    }

    #[test]
    fn test_rate_limit_override() {
        let ctx = AuthContext {
            layer: AuthLayer::ApiKey,
            user_id: None,
            organization_id: Some("org_123".to_string()),
            agent_id: None,
            ip_address: "192.168.1.1".to_string(),
            plan: "free".to_string(),
            rate_limit_override: Some(1000), // Custom override
        };

        assert_eq!(ctx.get_rate_limit(), 1000); // Override takes precedence
    }

    #[test]
    fn test_get_scope_ip() {
        let ctx = AuthContext::anonymous("192.168.1.1".to_string());
        let scope = ctx.get_scope();

        match scope {
            shared::RateLimitScope::Ip(ip) => assert_eq!(ip, "192.168.1.1"),
            _ => panic!("Expected IP scope"),
        }
    }

    #[test]
    fn test_get_scope_organization() {
        let ctx = AuthContext {
            layer: AuthLayer::ApiKey,
            user_id: None,
            organization_id: Some("org_123".to_string()),
            agent_id: None,
            ip_address: "192.168.1.1".to_string(),
            plan: "pro".to_string(),
            rate_limit_override: None,
        };

        let scope = ctx.get_scope();

        match scope {
            shared::RateLimitScope::Organization(org_id) => assert_eq!(org_id, "org_123"),
            _ => panic!("Expected Organization scope"),
        }
    }

    #[test]
    fn test_allows_tier_layer1() {
        let ctx = AuthContext {
            layer: AuthLayer::ApiKey,
            user_id: None,
            organization_id: Some("org_123".to_string()),
            agent_id: None,
            ip_address: "192.168.1.1".to_string(),
            plan: "pro".to_string(),
            rate_limit_override: None,
        };

        assert!(ctx.allows_tier(0));
        assert!(ctx.allows_tier(1));
        assert!(ctx.allows_tier(2));
        assert!(ctx.allows_tier(3));
    }
}
