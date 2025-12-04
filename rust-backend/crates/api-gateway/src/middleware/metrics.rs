//! Prometheus Metrics Middleware
//!
//! This middleware collects HTTP request metrics for Prometheus scraping.
//! It tracks:
//! - Request count by method, path, and status code
//! - Request duration in seconds
//! - In-flight requests (concurrent requests)
//!
//! # Metrics Exposed
//!
//! - `http_requests_total` - Counter of HTTP requests
//! - `http_request_duration_seconds` - Histogram of request durations
//! - `http_requests_in_flight` - Gauge of concurrent requests
//!
//! # Usage
//!
//! ```rust
//! use api_gateway::middleware::metrics::{PrometheusMetrics, metrics_handler};
//!
//! // In your main.rs:
//! let metrics = PrometheusMetrics::new();
//!
//! App::new()
//!     .wrap(metrics.clone())
//!     .route("/metrics", web::get().to(metrics_handler))
//! ```

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use std::{
    future::{ready, Ready},
    sync::Arc,
    time::Instant,
};

/// Global Prometheus handle for the /metrics endpoint
static PROMETHEUS_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

/// Initialize the Prometheus metrics recorder
///
/// This should be called once at application startup, before any metrics are recorded.
/// Returns the PrometheusHandle for rendering metrics.
pub fn init_metrics() -> PrometheusHandle {
    PROMETHEUS_HANDLE
        .get_or_init(|| {
            // Build and install the Prometheus recorder
            let builder = PrometheusBuilder::new();
            let handle = builder
                .install_recorder()
                .expect("Failed to install Prometheus recorder");

            // Describe metrics for better documentation in /metrics output
            describe_counter!(
                "http_requests_total",
                "Total number of HTTP requests processed"
            );
            describe_histogram!(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            );
            describe_gauge!(
                "http_requests_in_flight",
                "Number of HTTP requests currently being processed"
            );

            handle
        })
        .clone()
}

/// Get the global Prometheus handle
///
/// Panics if `init_metrics()` hasn't been called.
pub fn get_prometheus_handle() -> &'static PrometheusHandle {
    PROMETHEUS_HANDLE
        .get()
        .expect("Prometheus metrics not initialized. Call init_metrics() first.")
}

/// Handler for the /metrics endpoint
///
/// Returns Prometheus metrics in text format for scraping.
pub async fn metrics_handler() -> HttpResponse {
    let handle = get_prometheus_handle();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(handle.render())
}

/// Prometheus metrics middleware for Actix-web
///
/// Collects metrics for every HTTP request:
/// - Increments request counter with labels
/// - Records request duration histogram
/// - Tracks in-flight requests gauge
#[derive(Clone)]
pub struct PrometheusMetrics {
    /// Paths to exclude from metrics (e.g., /metrics, /health)
    excluded_paths: Arc<Vec<String>>,
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusMetrics {
    /// Create new PrometheusMetrics middleware
    pub fn new() -> Self {
        Self {
            excluded_paths: Arc::new(vec!["/metrics".to_string(), "/api/v1/health".to_string()]),
        }
    }

    /// Create PrometheusMetrics with custom excluded paths
    #[allow(dead_code)]
    pub fn with_excluded_paths(paths: Vec<String>) -> Self {
        Self {
            excluded_paths: Arc::new(paths),
        }
    }

    /// Check if a path should be excluded from metrics
    #[allow(dead_code)]
    fn is_excluded(&self, path: &str) -> bool {
        self.excluded_paths.iter().any(|p| path.starts_with(p))
    }
}

impl<S, B> Transform<S, ServiceRequest> for PrometheusMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = PrometheusMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PrometheusMetricsMiddleware {
            service,
            excluded_paths: self.excluded_paths.clone(),
        }))
    }
}

pub struct PrometheusMetricsMiddleware<S> {
    service: S,
    excluded_paths: Arc<Vec<String>>,
}

impl<S> PrometheusMetricsMiddleware<S> {
    /// Check if a path should be excluded from metrics
    fn is_excluded(&self, path: &str) -> bool {
        self.excluded_paths.iter().any(|p| path.starts_with(p))
    }
}

impl<S, B> Service<ServiceRequest> for PrometheusMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = normalize_path(req.path());
        let is_excluded = self.is_excluded(req.path());

        // Increment in-flight requests (only for non-excluded paths)
        if !is_excluded {
            gauge!("http_requests_in_flight").increment(1.0);
        }

        let fut = self.service.call(req);
        let excluded_for_metric = is_excluded;

        Box::pin(async move {
            let result = fut.await;

            // Decrement in-flight requests
            if !excluded_for_metric {
                gauge!("http_requests_in_flight").decrement(1.0);
            }

            match result {
                Ok(response) => {
                    if !excluded_for_metric {
                        let status = response.status().as_u16().to_string();
                        let duration = start.elapsed().as_secs_f64();

                        // Record request count
                        counter!(
                            "http_requests_total",
                            "method" => method.clone(),
                            "path" => path.clone(),
                            "status" => status.clone()
                        )
                        .increment(1);

                        // Record request duration
                        histogram!(
                            "http_request_duration_seconds",
                            "method" => method,
                            "path" => path,
                            "status" => status
                        )
                        .record(duration);
                    }

                    Ok(response.map_into_left_body())
                }
                Err(e) => {
                    if !excluded_for_metric {
                        // Record error
                        let status = e.as_response_error().status_code().as_u16().to_string();
                        let duration = start.elapsed().as_secs_f64();

                        counter!(
                            "http_requests_total",
                            "method" => method.clone(),
                            "path" => path.clone(),
                            "status" => status.clone()
                        )
                        .increment(1);

                        histogram!(
                            "http_request_duration_seconds",
                            "method" => method,
                            "path" => path,
                            "status" => status
                        )
                        .record(duration);
                    }

                    Err(e)
                }
            }
        })
    }
}

/// Normalize path for metrics labels
///
/// Replaces dynamic path segments (UUIDs, numbers) with placeholders
/// to prevent high-cardinality labels.
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = parts
        .iter()
        .map(|part| {
            // Check if it's a UUID (36 chars with dashes) or a numeric ID
            let is_uuid = part.len() == 36 && part.chars().filter(|c| *c == '-').count() == 4;
            let is_numeric = !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());

            if is_uuid || is_numeric {
                "{id}".to_string()
            }
            // Check if it's a hex string (could be a hash or transaction ID)
            else if part.len() > 16
                && part
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c == 'x' || c == 'X')
            {
                "{hash}".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();

    normalized.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().body("ok")
    }

    async fn test_error_handler() -> HttpResponse {
        HttpResponse::InternalServerError().body("error")
    }

    #[actix_web::test]
    async fn test_normalize_path_uuid() {
        let path = "/api/v1/triggers/123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(normalize_path(path), "/api/v1/triggers/{id}");
    }

    #[actix_web::test]
    async fn test_normalize_path_numeric_id() {
        let path = "/api/v1/users/12345";
        assert_eq!(normalize_path(path), "/api/v1/users/{id}");
    }

    #[actix_web::test]
    async fn test_normalize_path_no_dynamic_segments() {
        let path = "/api/v1/health";
        assert_eq!(normalize_path(path), "/api/v1/health");
    }

    #[actix_web::test]
    async fn test_normalize_path_hex_hash() {
        let path = "/api/v1/transactions/0x1234567890abcdef1234567890abcdef12345678";
        assert_eq!(normalize_path(path), "/api/v1/transactions/{hash}");
    }

    #[actix_web::test]
    async fn test_normalize_path_multiple_ids() {
        let path = "/api/v1/orgs/123e4567-e89b-12d3-a456-426614174000/triggers/456";
        assert_eq!(normalize_path(path), "/api/v1/orgs/{id}/triggers/{id}");
    }

    #[actix_web::test]
    async fn test_excluded_paths() {
        let metrics = PrometheusMetrics::new();
        assert!(metrics.is_excluded("/metrics"));
        assert!(metrics.is_excluded("/api/v1/health"));
        assert!(!metrics.is_excluded("/api/v1/triggers"));
    }

    #[actix_web::test]
    async fn test_custom_excluded_paths() {
        let metrics = PrometheusMetrics::with_excluded_paths(vec![
            "/custom".to_string(),
            "/test".to_string(),
        ]);
        assert!(metrics.is_excluded("/custom"));
        assert!(metrics.is_excluded("/test/something"));
        assert!(!metrics.is_excluded("/api/v1/triggers"));
    }

    #[actix_web::test]
    async fn test_metrics_middleware_records_success() {
        // Note: This test verifies the middleware compiles and runs
        // Full integration testing requires a test recorder

        let app = test::init_service(
            App::new()
                .wrap(PrometheusMetrics::new())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_metrics_middleware_records_error() {
        let app = test::init_service(
            App::new()
                .wrap(PrometheusMetrics::new())
                .route("/error", web::get().to(test_error_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/error").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
