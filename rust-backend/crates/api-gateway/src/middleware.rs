//! Middleware for the API Gateway
//!
//! This module provides security middleware for the API Gateway:
//!
//! # CORS Configuration
//!
//! - [`cors()`] - Configures CORS with environment-based origin whitelist
//!
//! # JWT Authentication
//!
//! - [`JwtAuth`] - Actix-web middleware for JWT validation
//! - [`get_user_id()`] - Extract authenticated user ID from request
//! - [`get_claims()`] - Extract full JWT claims from request
//!
//! # Organization Verification
//!
//! These functions verify that a user has access to the organization specified
//! in the `X-Organization-ID` header:
//!
//! - [`get_verified_organization_id()`] - Verify membership, return org_id
//! - [`get_verified_organization_id_with_role()`] - Verify membership, return (org_id, role)
//!
//! # API Key Authentication
//!
//! - [`DualAuth`] - Middleware supporting both JWT and API Key authentication
//! - [`ApiKeyAuth`] - API key authentication context
//! - [`get_api_key_auth()`] - Extract API key auth from request
//!
//! # Rate Limiting
//!
//! - [`auth_extractor`] - Extract authentication context for rate limiting
//! - [`ip_extractor`] - Extract client IP address with proxy support
//! - [`query_tier`] - Extract query tier for cost calculation
//! - [`unified_rate_limiter`] - Unified rate limiting middleware
//!
//! # Security Headers
//!
//! - [`security_headers`] - Adds security headers (HSTS, X-Frame-Options, etc.)
//!
//! # Security Notes
//!
//! - JWT tokens are validated using HS256 algorithm
//! - The `X-Organization-ID` header is untrusted and always verified against membership
//! - All verification functions return appropriate HTTP error responses on failure
//! - Security headers protect against common web vulnerabilities

pub mod auth_extractor;
pub mod cors;
pub mod ip_extractor;
pub mod metrics;
pub mod query_tier;
pub mod request_id;
pub mod security_headers;
pub mod unified_rate_limiter;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    http, web, Error, HttpMessage, HttpRequest, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use shared::models::ApiKey;
use shared::DbPool;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use crate::models::{Claims, ErrorResponse};
use crate::repositories::{
    ApiKeyAuditRepository, ApiKeyRepository, AuthFailureRepository, MemberRepository,
};
use crate::services::{ApiKeyService, AuthRateLimiter};

// Re-export middleware components
#[allow(unused_imports)] // Used in integration tests
pub use auth_extractor::{AuthContext, AuthLayer};
pub use cors::cors;
#[allow(unused_imports)] // Used in integration tests
pub use query_tier::{QueryTier, QueryTierExtractor};
#[allow(unused_imports)] // Used in integration tests
pub use unified_rate_limiter::UnifiedRateLimiter;

// ============================================================================
// JWT Token Extraction Helper
// ============================================================================

/// Extract JWT token from cookie or Authorization header
///
/// Checks sources in order of preference:
/// 1. `auth-token` cookie (preferred for browser clients)
/// 2. `Authorization: Bearer <token>` header (fallback for API clients)
fn extract_jwt_from_request(req: &ServiceRequest) -> Option<String> {
    // 1. Try auth-token cookie first (preferred for browser clients)
    if let Some(cookie) = req.cookie("auth-token") {
        return Some(cookie.value().to_string());
    }

    // 2. Fallback to Authorization header (for API clients)
    if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}

// ============================================================================
// JWT Authentication Middleware
// ============================================================================

/// JWT authentication middleware (kept for JWT-only routes)
#[allow(dead_code)]
pub struct JwtAuth {
    jwt_secret: Rc<String>,
}

impl JwtAuth {
    #[allow(dead_code)]
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret: Rc::new(jwt_secret),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: Rc::new(service),
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

#[allow(dead_code)]
pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
    jwt_secret: Rc<String>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
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
        let jwt_secret = self.jwt_secret.clone();

        Box::pin(async move {
            // Extract token from cookie or Authorization header
            let token = match extract_jwt_from_request(&req) {
                Some(t) => t,
                None => return Err(ErrorUnauthorized("Missing authentication")),
            };

            // Validate JWT token with explicit algorithm restriction
            // SECURITY: Explicitly set allowed algorithms to prevent algorithm confusion attacks
            let mut validation = Validation::new(Algorithm::HS256);
            validation.algorithms = vec![Algorithm::HS256]; // Only allow HS256, reject 'none' and others
            validation.validate_exp = true; // Explicitly enable expiration validation
            validation.leeway = 60; // 60 seconds clock skew tolerance

            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &validation,
            )
            .map_err(|e| {
                tracing::warn!("JWT validation failed: {}", e);
                ErrorUnauthorized("Invalid or expired token")
            })?;

            // Store claims in request extensions for handlers to access
            req.extensions_mut().insert(token_data.claims);

            // Continue to the next service
            service.call(req).await
        })
    }
}

/// Helper to extract user_id from request extensions
pub fn get_user_id(req: &actix_web::HttpRequest) -> Result<String, Error> {
    req.extensions()
        .get::<Claims>()
        .map(|claims| claims.sub.clone())
        .ok_or_else(|| ErrorUnauthorized("User not authenticated"))
}

/// Helper to extract and verify X-Organization-ID header
///
/// This function extracts the organization ID from the request header AND
/// verifies that the authenticated user is a member of that organization.
/// This prevents horizontal privilege escalation via header spoofing.
///
/// Uses Redis caching when EntityCache is available in app state to avoid
/// database lookups on every request. Cache TTL is 5 minutes.
///
/// # Arguments
/// * `req` - The HTTP request containing the X-Organization-ID header
/// * `pool` - Database connection pool
/// * `user_id` - The authenticated user's ID
///
/// # Returns
/// * `Ok(String)` - The verified organization ID
/// * `Err(HttpResponse)` - Error response if header missing, invalid, or user not a member
pub async fn get_verified_organization_id(
    req: &HttpRequest,
    pool: &DbPool,
    user_id: &str,
) -> Result<String, HttpResponse> {
    // Extract X-Organization-ID header
    let org_id = req
        .headers()
        .get("X-Organization-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(ErrorResponse::new(
                "missing_organization",
                "X-Organization-ID header is required",
            ))
        })?;

    // CRITICAL: Verify user belongs to the organization
    // Try to use cached version if EntityCache is available
    let is_member = if let Some(cache) =
        req.app_data::<actix_web::web::Data<shared::redis::cache::EntityCache>>()
    {
        MemberRepository::is_member_cached(pool, cache.get_ref(), &org_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify organization membership: {}", e);
                HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to verify organization access",
                ))
            })?
    } else {
        // Fallback to non-cached version if cache not available
        MemberRepository::is_member(pool, &org_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify organization membership: {}", e);
                HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to verify organization access",
                ))
            })?
    };

    if !is_member {
        return Err(HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Not a member of the specified organization",
        )));
    }

    Ok(org_id)
}

/// Helper to extract and verify X-Organization-ID header with role check
///
/// Same as `get_verified_organization_id` but also returns the user's role
/// in the organization for permission checks.
///
/// Uses Redis caching when EntityCache is available in app state.
///
/// # Returns
/// * `Ok((String, String))` - Tuple of (organization_id, user_role)
pub async fn get_verified_organization_id_with_role(
    req: &HttpRequest,
    pool: &DbPool,
    user_id: &str,
) -> Result<(String, String), HttpResponse> {
    // Extract X-Organization-ID header
    let org_id = req
        .headers()
        .get("X-Organization-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(ErrorResponse::new(
                "missing_organization",
                "X-Organization-ID header is required",
            ))
        })?;

    // Get user's role in the organization (also verifies membership)
    // Try to use cached version if EntityCache is available
    let role = if let Some(cache) =
        req.app_data::<actix_web::web::Data<shared::redis::cache::EntityCache>>()
    {
        MemberRepository::get_role_cached(pool, cache.get_ref(), &org_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get organization role: {}", e);
                HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to verify organization access",
                ))
            })?
            .ok_or_else(|| {
                HttpResponse::Forbidden().json(ErrorResponse::new(
                    "forbidden",
                    "Not a member of the specified organization",
                ))
            })?
    } else {
        // Fallback to non-cached version if cache not available
        MemberRepository::get_role(pool, &org_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get organization role: {}", e);
                HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to verify organization access",
                ))
            })?
            .ok_or_else(|| {
                HttpResponse::Forbidden().json(ErrorResponse::new(
                    "forbidden",
                    "Not a member of the specified organization",
                ))
            })?
    };

    Ok((org_id, role))
}

// ============================================================================
// API Key Authentication
// ============================================================================

/// Authentication context from API key validation
///
/// This struct is stored in request extensions when API key auth succeeds.
/// Handlers can use `get_api_key_auth()` to retrieve it.
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    /// The API key record from database
    pub api_key: ApiKey,
    /// Parsed permissions list
    pub permissions: Vec<String>,
}

impl ApiKeyAuth {
    /// Check if this API key has a specific permission
    #[allow(dead_code)]
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"admin".to_string())
    }

    /// Check if this API key has write permission
    #[allow(dead_code)]
    pub fn can_write(&self) -> bool {
        self.has_permission("write") || self.has_permission("admin")
    }

    /// Check if this API key has delete permission
    #[allow(dead_code)]
    pub fn can_delete(&self) -> bool {
        self.has_permission("delete") || self.has_permission("admin")
    }
}

/// Helper to extract API key auth from request extensions
#[allow(dead_code)]
pub fn get_api_key_auth(req: &HttpRequest) -> Option<ApiKeyAuth> {
    req.extensions().get::<ApiKeyAuth>().cloned()
}

/// Helper to get the authenticated organization ID
///
/// Works with both JWT auth (via X-Organization-ID header with membership verification)
/// and API key auth (organization from the key itself).
///
/// # Arguments
/// * `req` - The HTTP request
/// * `pool` - Database connection pool
///
/// # Returns
/// * `Ok(String)` - The verified organization ID
/// * `Err(HttpResponse)` - Error if no valid auth context found
#[allow(dead_code)]
pub async fn get_authenticated_organization_id(
    req: &HttpRequest,
    pool: &DbPool,
) -> Result<String, HttpResponse> {
    // First check for API key auth (takes precedence, no header verification needed)
    if let Some(api_key_auth) = get_api_key_auth(req) {
        return Ok(api_key_auth.api_key.organization_id);
    }

    // Fall back to JWT auth with header verification
    let user_id = get_user_id(req).map_err(|_| {
        HttpResponse::Unauthorized().json(ErrorResponse::new(
            "unauthorized",
            "Authentication required",
        ))
    })?;

    get_verified_organization_id(req, pool, &user_id).await
}

/// Dual authentication middleware supporting both JWT tokens and API keys
///
/// # Authentication Methods
///
/// 1. **API Key** (checked first):
///    - Header: `X-API-Key: sk_live_xxx` or `X-API-Key: sk_test_xxx`
///    - Or in Authorization header: `Authorization: sk_live_xxx`
///
/// 2. **JWT Token** (fallback):
///    - Header: `Authorization: Bearer <jwt-token>`
///
/// # Security Features
///
/// - Constant-time key verification using Argon2id
/// - Timing attack mitigation via dummy verification when key not found
/// - Audit logging for all authentication attempts
/// - Key expiration and revocation checks
pub struct DualAuth {
    jwt_secret: Rc<String>,
}

impl DualAuth {
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret: Rc::new(jwt_secret),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for DualAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DualAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(DualAuthMiddleware {
            service: Rc::new(service),
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

pub struct DualAuthMiddleware<S> {
    service: Rc<S>,
    jwt_secret: Rc<String>,
}

impl<S, B> Service<ServiceRequest> for DualAuthMiddleware<S>
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
        let jwt_secret = self.jwt_secret.clone();

        Box::pin(async move {
            // Try to get the database pool from app_data
            let pool = req.app_data::<web::Data<DbPool>>().cloned();

            // Try to get the rate limiter from app_data (optional)
            let rate_limiter = req.app_data::<web::Data<AuthRateLimiter>>().cloned();

            // Check for X-API-Key header first
            let api_key_header = req
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // Check Authorization header for API key or JWT
            let auth_header = req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // Determine auth method
            let api_key = api_key_header.or_else(|| {
                auth_header.as_ref().and_then(|h| {
                    // If it starts with sk_live_ or sk_test_, it's an API key
                    if h.starts_with("sk_live_") || h.starts_with("sk_test_") {
                        Some(h.clone())
                    } else {
                        None
                    }
                })
            });

            // Try API key authentication first
            if let Some(key) = api_key {
                if let Some(pool) = pool {
                    let ip_address = req
                        .connection_info()
                        .realip_remote_addr()
                        .map(|s| s.to_string());
                    let user_agent = req
                        .headers()
                        .get(http::header::USER_AGENT)
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());
                    let endpoint = Some(req.uri().path().to_string());

                    match validate_api_key(
                        &pool,
                        &key,
                        ip_address.as_deref(),
                        user_agent.as_deref(),
                        endpoint.as_deref(),
                        rate_limiter.as_deref().map(|r| r.as_ref()),
                    )
                    .await
                    {
                        Ok(api_key_auth) => {
                            // Store API key auth in request extensions
                            req.extensions_mut().insert(api_key_auth);
                            return service.call(req).await;
                        }
                        Err(e) => {
                            tracing::warn!("API key authentication failed: {}", e);
                            return Err(ErrorUnauthorized("Invalid or expired API key"));
                        }
                    }
                }
            }

            // Fall back to JWT authentication (cookie or Authorization header)
            // Note: If an API key was provided but couldn't be validated, we still try JWT
            let token = match extract_jwt_from_request(&req) {
                Some(t) => t,
                None => {
                    // Check if an API key was provided but we couldn't validate it
                    if auth_header.as_ref().is_some_and(|h| h.starts_with("sk_")) {
                        return Err(ErrorUnauthorized("API key authentication not available"));
                    }
                    return Err(ErrorUnauthorized("Missing authentication"));
                }
            };

            // Validate JWT token with explicit algorithm restriction
            // SECURITY: Explicitly set allowed algorithms to prevent algorithm confusion attacks
            let mut validation = Validation::new(Algorithm::HS256);
            validation.algorithms = vec![Algorithm::HS256]; // Only allow HS256, reject 'none' and others
            validation.validate_exp = true;
            validation.leeway = 60;

            let token_data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &validation,
            )
            .map_err(|e| {
                tracing::warn!("JWT validation failed: {}", e);
                ErrorUnauthorized("Invalid or expired token")
            })?;

            // Store claims in request extensions for handlers to access
            req.extensions_mut().insert(token_data.claims);

            // Continue to the next service
            service.call(req).await
        })
    }
}

/// Validate an API key and return the authentication context
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `key` - The full API key (e.g., "sk_live_xxx")
/// * `ip_address` - Client IP for audit logging and rate limiting
/// * `user_agent` - Client user agent for audit logging
/// * `endpoint` - Requested endpoint for audit logging
/// * `rate_limiter` - Optional rate limiter for brute-force protection
///
/// # Returns
/// * `Ok(ApiKeyAuth)` - Authentication context if key is valid
/// * `Err(String)` - Error message if validation fails
///
/// # Security
///
/// - Rate limiting is enforced before any database or crypto operations
/// - This prevents timing attacks from revealing rate limit status
async fn validate_api_key(
    pool: &DbPool,
    key: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    endpoint: Option<&str>,
    rate_limiter: Option<&AuthRateLimiter>,
) -> Result<ApiKeyAuth, String> {
    // Check rate limit FIRST (before any DB or crypto operations)
    // This prevents brute-force attacks
    if let Some(limiter) = rate_limiter {
        let ip = ip_address.unwrap_or("unknown");
        if let Err(e) = limiter.check(ip) {
            tracing::warn!(ip = ip, "API key auth rate limited: {}", e.message);

            // Log to auth_failures table (no org context for rate limits)
            if let Err(log_err) = AuthFailureRepository::log(
                pool,
                "rate_limited",
                None,
                ip_address,
                user_agent,
                endpoint,
                Some(serde_json::json!({"message": e.message})),
            )
            .await
            {
                // SECURITY: Log audit failure but don't expose to client
                tracing::error!(
                    error = %log_err,
                    ip = ?ip_address,
                    "CRITICAL: Failed to log rate limit event to audit trail"
                );
            }

            return Err(e.message);
        }
    }

    let api_key_service = ApiKeyService::new();

    // Validate key format
    if !ApiKeyService::is_valid_format(key) {
        // Log to auth_failures table
        if let Err(log_err) = AuthFailureRepository::log(
            pool,
            "invalid_format",
            key.chars().take(16).collect::<String>().as_str().into(),
            ip_address,
            user_agent,
            endpoint,
            Some(serde_json::json!({"key_length": key.len()})),
        )
        .await
        {
            tracing::error!(
                error = %log_err,
                ip = ?ip_address,
                "CRITICAL: Failed to log invalid format event to audit trail"
            );
        }

        return Err("Invalid API key format".to_string());
    }

    // Extract prefix for database lookup
    let prefix = ApiKeyService::extract_prefix(key).map_err(|e| e.to_string())?;

    // Look up key by prefix
    let api_key = match ApiKeyRepository::find_by_prefix(pool, &prefix).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            // CRITICAL: Perform dummy verification FIRST for timing attack mitigation
            // This ensures constant-time behavior regardless of whether the key exists.
            // Any I/O operations (database writes, logging) MUST happen AFTER this.
            api_key_service.dummy_verify();

            // Now safe to log (timing attack already mitigated)
            // Note: This is async and adds latency, but that's acceptable since
            // we've already maintained constant-time behavior above
            if let Err(log_err) = AuthFailureRepository::log(
                pool,
                "prefix_not_found",
                Some(&prefix),
                ip_address,
                user_agent,
                endpoint,
                None,
            )
            .await
            {
                tracing::error!(
                    error = %log_err,
                    prefix = %prefix,
                    ip = ?ip_address,
                    "CRITICAL: Failed to log prefix not found event to audit trail"
                );
            }

            tracing::warn!("API key not found for prefix: {}", prefix);
            return Err("Invalid API key".to_string());
        }
        Err(e) => {
            tracing::error!("Database error looking up API key: {}", e);
            return Err("Authentication error".to_string());
        }
    };

    // Verify the key hash
    let valid = api_key_service
        .verify_key(key, &api_key.key_hash)
        .map_err(|e| {
            tracing::error!("Error verifying API key: {}", e);
            "Authentication error".to_string()
        })?;

    if !valid {
        // Log failed auth attempt
        if let Err(log_err) = ApiKeyAuditRepository::log(
            pool,
            Some(&api_key.id),
            &api_key.organization_id,
            "auth_failed",
            ip_address,
            user_agent,
            endpoint,
            None,
            Some(serde_json::json!({"reason": "invalid_key"})),
        )
        .await
        {
            tracing::error!(
                error = %log_err,
                api_key_id = %api_key.id,
                org_id = %api_key.organization_id,
                "CRITICAL: Failed to log auth failure to audit trail"
            );
        }

        return Err("Invalid API key".to_string());
    }

    // Check if key is active (not revoked, not expired)
    if !ApiKeyRepository::is_active(&api_key) {
        let reason = if ApiKeyRepository::is_revoked(&api_key) {
            "key_revoked"
        } else {
            "key_expired"
        };

        // Log failed auth attempt
        if let Err(log_err) = ApiKeyAuditRepository::log(
            pool,
            Some(&api_key.id),
            &api_key.organization_id,
            "auth_failed",
            ip_address,
            user_agent,
            endpoint,
            None,
            Some(serde_json::json!({"reason": reason})),
        )
        .await
        {
            tracing::error!(
                error = %log_err,
                api_key_id = %api_key.id,
                org_id = %api_key.organization_id,
                reason = %reason,
                "CRITICAL: Failed to log key revoked/expired to audit trail"
            );
        }

        return Err(format!("API key is {}", reason.replace('_', " ")));
    }

    // Parse permissions
    let permissions: Vec<String> = serde_json::from_value(api_key.permissions.clone())
        .unwrap_or_else(|_| vec!["read".to_string()]);

    // Log successful auth
    if let Err(log_err) = ApiKeyAuditRepository::log(
        pool,
        Some(&api_key.id),
        &api_key.organization_id,
        "used",
        ip_address,
        user_agent,
        endpoint,
        None,
        None,
    )
    .await
    {
        tracing::error!(
            error = %log_err,
            api_key_id = %api_key.id,
            org_id = %api_key.organization_id,
            "CRITICAL: Failed to log successful auth to audit trail"
        );
    }

    // Update last used timestamp (fire and forget with error logging)
    let pool_clone = pool.clone();
    let key_id = api_key.id.clone();
    let ip = ip_address.map(|s| s.to_string());
    tokio::spawn(async move {
        if let Err(e) =
            ApiKeyRepository::update_last_used(&pool_clone, &key_id, ip.as_deref()).await
        {
            tracing::warn!(
                error = %e,
                api_key_id = %key_id,
                "Failed to update API key last_used timestamp"
            );
        }
    });

    Ok(ApiKeyAuth {
        api_key,
        permissions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};
    use jsonwebtoken::{encode, EncodingKey, Header};

    const TEST_SECRET: &str = "test-jwt-secret-for-testing-purposes";

    /// Create a valid JWT token for testing
    fn create_test_token(user_id: &str, username: &str, expired: bool) -> String {
        let now = chrono::Utc::now().timestamp();
        let exp = if expired {
            now - 3600 // 1 hour in the past
        } else {
            now + 3600 // 1 hour in the future
        };

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap()
    }

    /// Test handler that just returns OK
    async fn protected_handler() -> HttpResponse {
        HttpResponse::Ok().body("success")
    }

    #[actix_web::test]
    async fn test_jwt_validation_valid_token() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        let token = create_test_token("user-123", "testuser", false);
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_jwt_validation_expired_token() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        let token = create_test_token("user-123", "testuser", true);
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected unauthorized error for expired token"
        );
    }

    #[actix_web::test]
    async fn test_jwt_validation_invalid_signature() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        // Create token with a different secret
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: "user-123".to_string(),
            username: "testuser".to_string(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"wrong-secret"),
        )
        .unwrap();

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected unauthorized error for invalid signature"
        );
    }

    #[actix_web::test]
    async fn test_jwt_validation_missing_header() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/protected").to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected unauthorized error for missing header"
        );
    }

    #[actix_web::test]
    async fn test_jwt_validation_malformed_header() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        // Missing "Bearer " prefix
        let token = create_test_token("user-123", "testuser", false);
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", token))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected unauthorized error for malformed header"
        );
    }

    #[actix_web::test]
    async fn test_jwt_validation_basic_auth_header() {
        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(protected_handler)),
        )
        .await;

        // Wrong auth type
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", "Basic dXNlcjpwYXNz"))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected unauthorized error for basic auth header"
        );
    }

    #[actix_web::test]
    async fn test_jwt_claims_available_in_request() {
        async fn handler_with_claims(req: actix_web::HttpRequest) -> HttpResponse {
            if let Some(claims) = req.extensions().get::<Claims>() {
                HttpResponse::Ok().body(format!("user:{}", claims.sub))
            } else {
                HttpResponse::InternalServerError().body("no claims")
            }
        }

        let app = test::init_service(
            App::new()
                .wrap(JwtAuth::new(TEST_SECRET.to_string()))
                .route("/protected", web::get().to(handler_with_claims)),
        )
        .await;

        let token = create_test_token("user-456", "testuser", false);
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body = test::read_body(resp).await;
        assert_eq!(body, "user:user-456");
    }

    #[actix_web::test]
    async fn test_claims_new() {
        let claims = Claims::new("user-123".to_string(), "testuser".to_string(), 1);

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "testuser");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 3600); // 1 hour
    }
}
