//! Query Tier Extraction Middleware
//!
//! This middleware extracts the query tier from the request path or query parameters
//! and stores it in request extensions for use by the rate limiter.
//!
//! # Tier Detection
//!
//! - **Path-based**: `/api/v1/queries/tier0/...`, `/api/v1/queries/tier1/...`
//! - **Query parameter**: `?tier=2`
//! - **Default**: Tier 0 (if not specified)
//!
//! # Cost Multipliers
//!
//! - Tier 0: 1x (basic queries)
//! - Tier 1: 2x (aggregated queries)
//! - Tier 2: 5x (analysis queries)
//! - Tier 3: 10x (AI-powered queries)

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use tracing::debug;

/// Query tier for rate limiting cost calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTier {
    /// Tier 0: Basic queries (1x cost)
    Tier0,
    /// Tier 1: Aggregated queries (2x cost)
    Tier1,
    /// Tier 2: Analysis queries (5x cost)
    Tier2,
    /// Tier 3: AI-powered queries (10x cost)
    Tier3,
}

impl QueryTier {
    /// Get the cost multiplier for this tier
    pub fn cost_multiplier(self) -> i64 {
        match self {
            QueryTier::Tier0 => 1,
            QueryTier::Tier1 => 2,
            QueryTier::Tier2 => 5,
            QueryTier::Tier3 => 10,
        }
    }

    /// Get the tier name as a string
    pub fn as_str(self) -> &'static str {
        match self {
            QueryTier::Tier0 => "tier0",
            QueryTier::Tier1 => "tier1",
            QueryTier::Tier2 => "tier2",
            QueryTier::Tier3 => "tier3",
        }
    }

    /// Parse tier from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tier0" | "0" => Some(QueryTier::Tier0),
            "tier1" | "1" => Some(QueryTier::Tier1),
            "tier2" | "2" => Some(QueryTier::Tier2),
            "tier3" | "3" => Some(QueryTier::Tier3),
            _ => None,
        }
    }

    /// Extract tier from request path
    ///
    /// Looks for patterns like `/api/v1/queries/tier0/...` or `/api/v1/queries/tier1/...`
    fn from_path(path: &str) -> Option<Self> {
        // Check for tier in path segments
        let segments: Vec<&str> = path.split('/').collect();
        for (i, segment) in segments.iter().enumerate() {
            if *segment == "queries" && i + 1 < segments.len() {
                if let Some(tier) = Self::from_str(segments[i + 1]) {
                    return Some(tier);
                }
            }
        }
        None
    }

    /// Extract tier from query parameters
    ///
    /// Looks for `?tier=2` or similar
    fn from_query(query_string: &str) -> Option<Self> {
        for param in query_string.split('&') {
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 && parts[0] == "tier" {
                if let Some(tier) = Self::from_str(parts[1]) {
                    return Some(tier);
                }
            }
        }
        None
    }
}

/// Query tier extraction middleware
pub struct QueryTierExtractor;

impl QueryTierExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QueryTierExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for QueryTierExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = QueryTierExtractorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(QueryTierExtractorMiddleware {
            service: std::rc::Rc::new(service)
        }))
    }
}

pub struct QueryTierExtractorMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> Service<ServiceRequest> for QueryTierExtractorMiddleware<S>
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
            // Extract tier from path
            let path = req.uri().path();
            let tier = QueryTier::from_path(path)
                .or_else(|| {
                    // Fallback: check query parameters
                    req.uri()
                        .query()
                        .and_then(|q| QueryTier::from_query(q))
                })
                .unwrap_or(QueryTier::Tier0); // Default to Tier 0

            debug!(
                path = %path,
                tier = %tier.as_str(),
                cost = tier.cost_multiplier(),
                "Extracted query tier"
            );

            // Store tier in request extensions
            req.extensions_mut().insert(tier);

            // Continue to next service
            service.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler(req: actix_web::HttpRequest) -> HttpResponse {
        let tier = req.extensions().get::<QueryTier>().copied();
        match tier {
            Some(t) => HttpResponse::Ok().body(format!("tier:{}", t.as_str())),
            None => HttpResponse::Ok().body("no_tier"),
        }
    }

    #[actix_web::test]
    async fn test_query_tier_cost_multipliers() {
        assert_eq!(QueryTier::Tier0.cost_multiplier(), 1);
        assert_eq!(QueryTier::Tier1.cost_multiplier(), 2);
        assert_eq!(QueryTier::Tier2.cost_multiplier(), 5);
        assert_eq!(QueryTier::Tier3.cost_multiplier(), 10);
    }

    #[actix_web::test]
    async fn test_query_tier_from_str() {
        assert_eq!(QueryTier::from_str("tier0"), Some(QueryTier::Tier0));
        assert_eq!(QueryTier::from_str("tier1"), Some(QueryTier::Tier1));
        assert_eq!(QueryTier::from_str("tier2"), Some(QueryTier::Tier2));
        assert_eq!(QueryTier::from_str("tier3"), Some(QueryTier::Tier3));
        assert_eq!(QueryTier::from_str("0"), Some(QueryTier::Tier0));
        assert_eq!(QueryTier::from_str("1"), Some(QueryTier::Tier1));
        assert_eq!(QueryTier::from_str("2"), Some(QueryTier::Tier2));
        assert_eq!(QueryTier::from_str("3"), Some(QueryTier::Tier3));
        assert_eq!(QueryTier::from_str("TIER0"), Some(QueryTier::Tier0));
        assert_eq!(QueryTier::from_str("invalid"), None);
    }

    #[actix_web::test]
    async fn test_query_tier_from_path() {
        assert_eq!(
            QueryTier::from_path("/api/v1/queries/tier0/feedbacks"),
            Some(QueryTier::Tier0)
        );
        assert_eq!(
            QueryTier::from_path("/api/v1/queries/tier1/summary"),
            Some(QueryTier::Tier1)
        );
        assert_eq!(
            QueryTier::from_path("/api/v1/queries/tier2/analysis"),
            Some(QueryTier::Tier2)
        );
        assert_eq!(
            QueryTier::from_path("/api/v1/queries/tier3/report"),
            Some(QueryTier::Tier3)
        );
        assert_eq!(QueryTier::from_path("/api/v1/triggers"), None);
    }

    #[actix_web::test]
    async fn test_query_tier_from_query() {
        assert_eq!(QueryTier::from_query("tier=0"), Some(QueryTier::Tier0));
        assert_eq!(QueryTier::from_query("tier=1"), Some(QueryTier::Tier1));
        assert_eq!(QueryTier::from_query("tier=2"), Some(QueryTier::Tier2));
        assert_eq!(QueryTier::from_query("tier=3"), Some(QueryTier::Tier3));
        assert_eq!(
            QueryTier::from_query("foo=bar&tier=2&baz=qux"),
            Some(QueryTier::Tier2)
        );
        assert_eq!(QueryTier::from_query("foo=bar"), None);
    }

    #[actix_web::test]
    async fn test_middleware_extracts_tier_from_path() {
        let app = test::init_service(
            App::new()
                .wrap(QueryTierExtractor::new())
                .route("/api/v1/queries/tier2/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/queries/tier2/test")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        assert_eq!(body, "tier:tier2");
    }

    #[actix_web::test]
    async fn test_middleware_extracts_tier_from_query() {
        let app = test::init_service(
            App::new()
                .wrap(QueryTierExtractor::new())
                .route("/api/v1/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/test?tier=3")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        assert_eq!(body, "tier:tier3");
    }

    #[actix_web::test]
    async fn test_middleware_defaults_to_tier0() {
        let app = test::init_service(
            App::new()
                .wrap(QueryTierExtractor::new())
                .route("/api/v1/triggers", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/triggers")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        assert_eq!(body, "tier:tier0");
    }
}

// Rust guideline compliant 2025-01-28
