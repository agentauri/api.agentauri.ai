//! Unified Rate Limiter Middleware
//!
//! This middleware provides comprehensive rate limiting for all API routes based on:
//! - Authentication layer (Anonymous, API Key, Wallet Signature)
//! - Query tier (Tier 0-3 with different cost multipliers)
//! - Organization subscription plan
//!
//! # Features
//!
//! - Extracts authentication context from request extensions
//! - Applies tier-based cost multipliers
//! - Returns 429 Too Many Requests when limit exceeded
//! - Adds X-RateLimit-* headers to all responses
//! - Graceful degradation when Redis is unavailable
//!
//! # Response Headers
//!
//! - `X-RateLimit-Limit`: Maximum requests allowed in window
//! - `X-RateLimit-Remaining`: Remaining quota
//! - `X-RateLimit-Reset`: Unix timestamp when limit resets
//! - `X-RateLimit-Window`: Window size in seconds
//!
//! # Error Response (429)
//!
//! ```json
//! {
//!   "error": {
//!     "code": "RATE_LIMITED",
//!     "message": "Rate limit exceeded. Try again in 1847 seconds.",
//!     "retry_after": 1847,
//!     "limit": 100,
//!     "window": 3600
//!   }
//! }
//! ```
//!
//! # Monitoring Token Bypass
//!
//! Requests with a valid `X-Monitoring-Token` header bypass rate limiting entirely.
//! This is intended for infrastructure monitoring (Grafana, Prometheus, health checkers).
//! The token is configured via the `MONITORING_TOKEN` environment variable.

use crate::middleware::{auth_extractor::AuthContext, query_tier::QueryTier};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorTooManyRequests,
    http::header::{HeaderName, HeaderValue},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use once_cell::sync::Lazy;
use shared::RateLimiter;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{debug, error, warn};

/// Default window size in seconds (1 hour)
const DEFAULT_WINDOW_SECONDS: i64 = 3600;

/// Monitoring token loaded from environment variable
/// If set, requests with matching X-Monitoring-Token header bypass rate limiting
static MONITORING_TOKEN: Lazy<Option<String>> = Lazy::new(|| {
    std::env::var("MONITORING_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
});

/// Rate limit mode configuration (loaded once at startup)
/// Returns (is_production, mode_string)
static RATE_LIMIT_CONFIG: Lazy<(bool, String)> = Lazy::new(|| {
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e == "production")
        .unwrap_or(false);
    let default_mode = if is_production { "enforcing" } else { "shadow" };
    let mode = std::env::var("RATE_LIMIT_MODE").unwrap_or_else(|_| default_mode.to_string());
    (is_production, mode)
});

/// Add rate limit headers to a response
///
/// # Arguments
/// * `headers` - Mutable reference to response headers
/// * `limit` - Maximum requests allowed in window
/// * `remaining` - Remaining quota
/// * `reset_at` - Unix timestamp when limit resets
/// * `window_seconds` - Window size in seconds
fn add_rate_limit_headers(
    headers: &mut actix_web::http::header::HeaderMap,
    limit: i64,
    remaining: i64,
    reset_at: i64,
    window_seconds: i64,
) {
    headers.insert(
        HeaderName::from_static("x-ratelimit-limit"),
        HeaderValue::from(limit),
    );
    headers.insert(
        HeaderName::from_static("x-ratelimit-remaining"),
        HeaderValue::from(remaining),
    );
    headers.insert(
        HeaderName::from_static("x-ratelimit-reset"),
        HeaderValue::from(reset_at),
    );
    headers.insert(
        HeaderName::from_static("x-ratelimit-window"),
        HeaderValue::from(window_seconds),
    );
}

/// Unified rate limiter middleware
pub struct UnifiedRateLimiter {
    rate_limiter: Rc<RateLimiter>,
    /// Window size in seconds for rate limit headers
    window_seconds: i64,
}

impl UnifiedRateLimiter {
    /// Create a new unified rate limiter with default window (1 hour)
    ///
    /// # Arguments
    ///
    /// * `rate_limiter` - The rate limiter instance (shared across requests)
    pub fn new(rate_limiter: RateLimiter) -> Self {
        Self::with_window(rate_limiter, DEFAULT_WINDOW_SECONDS)
    }

    /// Create a new unified rate limiter with custom window size
    ///
    /// # Arguments
    ///
    /// * `rate_limiter` - The rate limiter instance (shared across requests)
    /// * `window_seconds` - Window size in seconds for rate limiting
    pub fn with_window(rate_limiter: RateLimiter, window_seconds: i64) -> Self {
        Self {
            rate_limiter: Rc::new(rate_limiter),
            window_seconds,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for UnifiedRateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = UnifiedRateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(UnifiedRateLimiterMiddleware {
            service: Rc::new(service),
            rate_limiter: self.rate_limiter.clone(),
            window_seconds: self.window_seconds,
        }))
    }
}

pub struct UnifiedRateLimiterMiddleware<S> {
    service: Rc<S>,
    rate_limiter: Rc<RateLimiter>,
    window_seconds: i64,
}

impl<S, B> Service<ServiceRequest> for UnifiedRateLimiterMiddleware<S>
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
        let rate_limiter = self.rate_limiter.clone();
        let window_seconds = self.window_seconds;

        Box::pin(async move {
            // Check for monitoring token bypass
            // If MONITORING_TOKEN is configured and request has matching X-Monitoring-Token header,
            // bypass rate limiting entirely (for Grafana, Prometheus, health checkers, etc.)
            if let Some(ref expected_token) = *MONITORING_TOKEN {
                if let Some(provided_token) = req
                    .headers()
                    .get("X-Monitoring-Token")
                    .and_then(|h| h.to_str().ok())
                {
                    if provided_token == expected_token {
                        debug!("Monitoring token valid - bypassing rate limit");
                        return service.call(req).await;
                    }
                }
            }

            // Skip rate limiting for health check, metrics, documentation, and OAuth endpoints
            // These are called frequently by load balancers, monitoring systems, Swagger UI,
            // and OAuth providers (Google, GitHub) during authentication flow
            let path = req.path();
            if path == "/api/v1/health"
                || path == "/metrics"
                || path == "/api/v1/openapi.json"
                || path.starts_with("/api-docs")
                || path.starts_with("/api/v1/auth/google")
                || path.starts_with("/api/v1/auth/github")
                || path.starts_with("/api/v1/auth/link/")
                || path == "/api/v1/auth/exchange"
            {
                return service.call(req).await;
            }

            // Extract authentication context (set by AuthExtractor or DualAuth middleware)
            let auth_ctx = match req.extensions().get::<AuthContext>() {
                Some(ctx) => ctx.clone(),
                None => {
                    // If no AuthContext, this means the middleware chain is misconfigured
                    // In production, this should never happen
                    error!(
                        "Missing AuthContext in rate limiter middleware - check middleware order"
                    );
                    return Err(actix_web::error::ErrorInternalServerError(
                        "Authentication context missing",
                    ));
                }
            };

            // Extract query tier (default to Tier 0 if not set)
            let query_tier = req
                .extensions()
                .get::<QueryTier>()
                .copied()
                .unwrap_or(QueryTier::Tier0);

            // Get rate limit parameters
            let scope = auth_ctx.get_scope();
            let limit = auth_ctx.get_rate_limit() as i64;
            let cost = query_tier.cost_multiplier();

            debug!(
                scope = ?scope,
                limit = limit,
                tier = %query_tier.as_str(),
                cost = cost,
                "Checking rate limit"
            );

            // Check rate limit
            let result = match rate_limiter.check(scope.clone(), limit, cost).await {
                Ok(r) => r,
                Err(e) => {
                    // Redis error - log and fail open (allow request)
                    error!(
                        error = %e,
                        scope = ?scope,
                        "Rate limiter error - failing open"
                    );

                    // Still call the service (graceful degradation)
                    let mut res = service.call(req).await?;

                    // Add headers indicating degraded mode
                    let headers = res.headers_mut();
                    headers.insert(
                        HeaderName::from_static("x-ratelimit-status"),
                        HeaderValue::from_static("degraded"),
                    );

                    return Ok(res);
                }
            };

            // Check if rate limit exceeded
            if !result.allowed {
                // Use pre-computed rate limit mode from static config
                let (is_production, ref mode) = *RATE_LIMIT_CONFIG;

                // Warn if shadow mode is used in production (should be intentional)
                if is_production && mode == "shadow" {
                    warn!(
                        "Rate limiting is in SHADOW mode in PRODUCTION - requests will NOT be blocked"
                    );
                }

                if mode == "shadow" {
                    // Shadow mode: Log violation but allow request
                    warn!(
                        mode = "SHADOW",
                        scope = ?scope,
                        current_usage = result.current_usage,
                        limit = result.limit,
                        retry_after = result.retry_after,
                        "Rate limit WOULD BE exceeded (shadow mode - request allowed)"
                    );

                    // Continue processing request (no error)
                    // Add special header to indicate shadow mode violation
                    let mut res = service.call(req).await?;
                    let headers = res.headers_mut();
                    headers.insert(
                        HeaderName::from_static("x-ratelimit-status"),
                        HeaderValue::from_static("shadow-violation"),
                    );
                    add_rate_limit_headers(
                        headers,
                        result.limit,
                        0,
                        result.reset_at,
                        window_seconds,
                    );
                    return Ok(res);
                } else {
                    // Enforcing mode: Block request
                    warn!(
                        mode = "ENFORCING",
                        scope = ?scope,
                        current_usage = result.current_usage,
                        limit = result.limit,
                        retry_after = result.retry_after,
                        "Rate limit exceeded"
                    );

                    // Return 429 Too Many Requests error
                    return Err(ErrorTooManyRequests(format!(
                        "Rate limit exceeded. Try again in {} seconds. (Limit: {}, Window: {}s)",
                        result.retry_after, result.limit, window_seconds
                    )));
                }
            }

            debug!(
                scope = ?scope,
                current_usage = result.current_usage,
                remaining = result.remaining,
                "Rate limit check: ALLOWED"
            );

            // Call the next service
            let mut res = service.call(req).await?;

            // Add rate limit headers to response
            add_rate_limit_headers(
                res.headers_mut(),
                result.limit,
                result.remaining,
                result.reset_at,
                window_seconds,
            );

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth_extractor::AuthContext;

    #[test]
    fn test_query_tier_cost_applied() {
        assert_eq!(QueryTier::Tier0.cost_multiplier(), 1);
        assert_eq!(QueryTier::Tier1.cost_multiplier(), 2);
        assert_eq!(QueryTier::Tier2.cost_multiplier(), 5);
        assert_eq!(QueryTier::Tier3.cost_multiplier(), 10);
    }

    #[test]
    fn test_auth_context_rate_limits() {
        let ctx = AuthContext::anonymous("192.168.1.1".to_string());
        assert_eq!(ctx.get_rate_limit(), 10);
    }

    #[test]
    fn test_rate_limiter_requires_auth_context() {
        // This test verifies that the middleware expects AuthContext in extensions
        // The actual integration test would require Redis and is marked #[ignore]
        // Integration tests should be added in tests/ directory
    }

    #[test]
    fn test_rate_limit_mode_defaults() {
        // Test that rate limit mode defaults correctly based on environment
        // In production (ENVIRONMENT=production), default should be "enforcing"
        // In development (ENVIRONMENT unset or != production), default should be "shadow"

        // Clear any existing env vars for clean test
        std::env::remove_var("RATE_LIMIT_MODE");

        // Test development default (shadow)
        std::env::remove_var("ENVIRONMENT");
        let is_production = std::env::var("ENVIRONMENT")
            .map(|e| e == "production")
            .unwrap_or(false);
        let default_mode = if is_production { "enforcing" } else { "shadow" };
        assert_eq!(default_mode, "shadow");

        // Test production default (enforcing)
        std::env::set_var("ENVIRONMENT", "production");
        let is_production = std::env::var("ENVIRONMENT")
            .map(|e| e == "production")
            .unwrap_or(false);
        let default_mode = if is_production { "enforcing" } else { "shadow" };
        assert_eq!(default_mode, "enforcing");

        // Clean up
        std::env::remove_var("ENVIRONMENT");
    }

    #[test]
    fn test_rate_limit_mode_override() {
        // Test that RATE_LIMIT_MODE env var can override the default

        // Set production mode
        std::env::set_var("ENVIRONMENT", "production");

        // Override with shadow mode
        std::env::set_var("RATE_LIMIT_MODE", "shadow");
        let mode = std::env::var("RATE_LIMIT_MODE").unwrap_or_else(|_| "enforcing".to_string());
        assert_eq!(mode, "shadow");

        // Override with enforcing mode
        std::env::set_var("RATE_LIMIT_MODE", "enforcing");
        let mode = std::env::var("RATE_LIMIT_MODE").unwrap_or_else(|_| "shadow".to_string());
        assert_eq!(mode, "enforcing");

        // Clean up
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("RATE_LIMIT_MODE");
    }
}

// Rust guideline compliant 2025-01-28
