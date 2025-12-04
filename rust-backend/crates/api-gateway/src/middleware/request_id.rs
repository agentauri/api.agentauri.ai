//! Request ID Middleware
//!
//! This middleware adds a unique request ID to all HTTP requests and responses.
//! It enables request tracing and correlation across logs and services.
//!
//! # Features
//!
//! - Generates UUID v4 for each request (if not provided)
//! - Accepts existing X-Request-ID from clients (for distributed tracing)
//! - Adds X-Request-ID to all responses
//! - Stores request ID in request extensions for handler access
//!
//! # Usage
//!
//! ```ignore
//! use actix_web::App;
//! use api_gateway::middleware::request_id::RequestId;
//!
//! let app = App::new()
//!     .wrap(RequestId::new())
//!     // ... routes
//! ```
//!
//! # Response Header
//!
//! - `X-Request-ID`: Unique identifier for the request (UUID v4)

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{debug, Span};
use uuid::Uuid;

/// Request ID stored in request extensions
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestIdExt(pub String);

/// Request ID middleware
///
/// Adds a unique request ID to all requests and responses.
pub struct RequestId;

impl RequestId {
    /// Create a new request ID middleware
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
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
            // Try to get existing request ID from header (for distributed tracing)
            let request_id = req
                .headers()
                .get("x-request-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            // Store request ID in request extensions for handler access
            req.extensions_mut()
                .insert(RequestIdExt(request_id.clone()));

            // Add request ID to tracing span
            Span::current().record("request_id", &request_id);

            debug!(request_id = %request_id, "Processing request");

            // Call the next service
            let mut res = service.call(req).await?;

            // Add request ID header to response
            if let Ok(value) = HeaderValue::try_from(&request_id) {
                res.headers_mut()
                    .insert(HeaderName::from_static("x-request-id"), value);
            }

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().body("test")
    }

    #[actix_web::test]
    async fn test_request_id_generated() {
        let app = test::init_service(
            App::new()
                .wrap(RequestId::new())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;

        // Request ID should be present in response
        assert!(resp.headers().contains_key("x-request-id"));

        // Should be a valid UUID
        let request_id = resp
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(Uuid::parse_str(request_id).is_ok());
    }

    #[actix_web::test]
    async fn test_request_id_preserved() {
        let app = test::init_service(
            App::new()
                .wrap(RequestId::new())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let custom_id = "custom-request-id-123";
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-Request-ID", custom_id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        // Custom request ID should be preserved
        assert_eq!(
            resp.headers()
                .get("x-request-id")
                .unwrap()
                .to_str()
                .unwrap(),
            custom_id
        );
    }

    #[actix_web::test]
    async fn test_request_id_in_extensions() {
        async fn handler(req: actix_web::HttpRequest) -> HttpResponse {
            let request_id = req.extensions().get::<RequestIdExt>().map(|r| r.0.clone());
            match request_id {
                Some(id) => HttpResponse::Ok().body(id),
                None => HttpResponse::InternalServerError().body("No request ID"),
            }
        }

        let app = test::init_service(
            App::new()
                .wrap(RequestId::new())
                .route("/test", web::get().to(handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_request_id_unique_per_request() {
        let app = test::init_service(
            App::new()
                .wrap(RequestId::new())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req1 = test::TestRequest::get().uri("/test").to_request();
        let resp1 = test::call_service(&app, req1).await;
        let id1 = resp1
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let req2 = test::TestRequest::get().uri("/test").to_request();
        let resp2 = test::call_service(&app, req2).await;
        let id2 = resp2
            .headers()
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Each request should have a unique ID
        assert_ne!(id1, id2);
    }
}
