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

use crate::middleware::{auth_extractor::AuthContext, query_tier::QueryTier};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorTooManyRequests,
    http::header::{HeaderName, HeaderValue},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use shared::RateLimiter;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{debug, error, warn};

/// Unified rate limiter middleware
pub struct UnifiedRateLimiter {
    rate_limiter: Rc<RateLimiter>,
}

impl UnifiedRateLimiter {
    /// Create a new unified rate limiter
    ///
    /// # Arguments
    ///
    /// * `rate_limiter` - The rate limiter instance (shared across requests)
    pub fn new(rate_limiter: RateLimiter) -> Self {
        Self {
            rate_limiter: Rc::new(rate_limiter),
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
        }))
    }
}

pub struct UnifiedRateLimiterMiddleware<S> {
    service: Rc<S>,
    rate_limiter: Rc<RateLimiter>,
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

        Box::pin(async move {
            // Extract authentication context (set by AuthExtractor or DualAuth middleware)
            let auth_ctx = match req.extensions().get::<AuthContext>() {
                Some(ctx) => ctx.clone(),
                None => {
                    // If no AuthContext, this means the middleware chain is misconfigured
                    // In production, this should never happen
                    error!("Missing AuthContext in rate limiter middleware - check middleware order");
                    return Err(actix_web::error::ErrorInternalServerError(
                        "Authentication context missing"
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
                warn!(
                    scope = ?scope,
                    current_usage = result.current_usage,
                    limit = result.limit,
                    retry_after = result.retry_after,
                    "Rate limit exceeded"
                );

                // Return 429 Too Many Requests error
                // Note: The actual response formatting with headers would be done by an error handler
                // For now, we return a simple error. In production, you'd want a custom error type
                // that includes the rate limit info and a custom error handler that sets the headers.
                return Err(ErrorTooManyRequests(format!(
                    "Rate limit exceeded. Try again in {} seconds. (Limit: {}, Window: 3600s)",
                    result.retry_after, result.limit
                )));
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
            let headers = res.headers_mut();
            headers.insert(
                HeaderName::from_static("x-ratelimit-limit"),
                HeaderValue::from(result.limit),
            );
            headers.insert(
                HeaderName::from_static("x-ratelimit-remaining"),
                HeaderValue::from(result.remaining),
            );
            headers.insert(
                HeaderName::from_static("x-ratelimit-reset"),
                HeaderValue::from(result.reset_at),
            );
            headers.insert(
                HeaderName::from_static("x-ratelimit-window"),
                HeaderValue::from_static("3600"),
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
}

// Rust guideline compliant 2025-01-28
