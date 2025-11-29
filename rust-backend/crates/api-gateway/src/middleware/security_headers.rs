//! Security Headers Middleware
//!
//! This middleware adds essential security headers to all HTTP responses:
//!
//! - **Strict-Transport-Security (HSTS)**: Enforces HTTPS connections
//! - **X-Content-Type-Options**: Prevents MIME type sniffing
//! - **X-Frame-Options**: Prevents clickjacking attacks
//! - **X-XSS-Protection**: Legacy XSS protection for older browsers
//! - **Content-Security-Policy**: Controls resource loading (optional)
//! - **Referrer-Policy**: Controls referrer information
//! - **Permissions-Policy**: Controls browser feature permissions
//!
//! # Usage
//!
//! ```ignore
//! use actix_web::App;
//! use api_gateway::middleware::security_headers;
//!
//! let app = App::new()
//!     .wrap(security_headers::SecurityHeaders::default())
//!     // ... routes
//! ```
//!
//! # Environment Configuration
//!
//! - `ENABLE_HSTS`: Set to "true" in production to enable HSTS (default: false in dev)
//! - `HSTS_MAX_AGE`: HSTS max-age in seconds (default: 31536000 = 1 year)
//!
//! Rust guideline compliant 2025-01-28

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    env,
    future::{ready, Ready},
    rc::Rc,
};
use tracing::debug;

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS header (should only be true in production with valid HTTPS)
    pub enable_hsts: bool,
    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload (requires careful consideration)
    pub hsts_preload: bool,
    /// X-Frame-Options value (DENY or SAMEORIGIN)
    pub frame_options: String,
    /// Content-Security-Policy value (optional, can be complex)
    pub content_security_policy: Option<String>,
    /// Referrer-Policy value
    pub referrer_policy: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        // Check environment for HSTS settings
        let enable_hsts = env::var("ENABLE_HSTS")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or_else(|_| !cfg!(debug_assertions)); // Enable in release builds

        let hsts_max_age = env::var("HSTS_MAX_AGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(31_536_000); // 1 year

        Self {
            enable_hsts,
            hsts_max_age,
            hsts_include_subdomains: true,
            hsts_preload: false, // Requires careful consideration
            frame_options: "DENY".to_string(),
            content_security_policy: Some("default-src 'self'".to_string()),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
        }
    }
}

/// Security headers middleware
///
/// Adds security headers to all HTTP responses.
pub struct SecurityHeaders {
    config: Rc<SecurityHeadersConfig>,
}

impl SecurityHeaders {
    /// Create a new security headers middleware with custom configuration
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self {
            config: Rc::new(config),
        }
    }

    /// Create with API-friendly defaults (no CSP that might break JSON responses)
    pub fn for_api() -> Self {
        let mut config = SecurityHeadersConfig::default();
        // APIs typically don't need CSP (no HTML/JS)
        config.content_security_policy = None;
        Self::new(config)
    }
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self::new(SecurityHeadersConfig::default())
    }
}

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
        }))
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: Rc<S>,
    config: Rc<SecurityHeadersConfig>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
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
        let config = self.config.clone();

        Box::pin(async move {
            // Call the next service
            let mut res = service.call(req).await?;

            // Add security headers
            let headers = res.headers_mut();

            // X-Content-Type-Options: Prevent MIME type sniffing
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );

            // X-Frame-Options: Prevent clickjacking
            if let Ok(value) = HeaderValue::try_from(config.frame_options.as_str()) {
                headers.insert(HeaderName::from_static("x-frame-options"), value);
            }

            // X-XSS-Protection: Legacy XSS protection (some older browsers)
            // Note: Modern browsers deprecate this in favor of CSP
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("1; mode=block"),
            );

            // Referrer-Policy: Control referrer information
            if let Ok(value) = HeaderValue::try_from(config.referrer_policy.as_str()) {
                headers.insert(HeaderName::from_static("referrer-policy"), value);
            }

            // Permissions-Policy: Restrict browser features
            headers.insert(
                HeaderName::from_static("permissions-policy"),
                HeaderValue::from_static(
                    "accelerometer=(), camera=(), geolocation=(), gyroscope=(), \
                     magnetometer=(), microphone=(), payment=(), usb=()",
                ),
            );

            // Strict-Transport-Security (HSTS) - only in production with HTTPS
            if config.enable_hsts {
                let mut hsts_value = format!("max-age={}", config.hsts_max_age);
                if config.hsts_include_subdomains {
                    hsts_value.push_str("; includeSubDomains");
                }
                if config.hsts_preload {
                    hsts_value.push_str("; preload");
                }

                if let Ok(value) = HeaderValue::try_from(hsts_value) {
                    headers.insert(HeaderName::from_static("strict-transport-security"), value);
                }

                debug!("HSTS header added");
            }

            // Content-Security-Policy (optional)
            if let Some(csp) = &config.content_security_policy {
                if let Ok(value) = HeaderValue::try_from(csp.as_str()) {
                    headers.insert(HeaderName::from_static("content-security-policy"), value);
                }
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
    async fn test_security_headers_added() {
        let app = test::init_service(
            App::new()
                .wrap(SecurityHeaders::default())
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;

        // Check security headers are present
        assert!(resp.headers().contains_key("x-content-type-options"));
        assert_eq!(
            resp.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );

        assert!(resp.headers().contains_key("x-frame-options"));
        assert_eq!(resp.headers().get("x-frame-options").unwrap(), "DENY");

        assert!(resp.headers().contains_key("x-xss-protection"));
        assert_eq!(
            resp.headers().get("x-xss-protection").unwrap(),
            "1; mode=block"
        );

        assert!(resp.headers().contains_key("referrer-policy"));
        assert!(resp.headers().contains_key("permissions-policy"));
    }

    #[actix_web::test]
    async fn test_api_config_no_csp() {
        let app = test::init_service(
            App::new()
                .wrap(SecurityHeaders::for_api())
                .route("/api/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/test").to_request();
        let resp = test::call_service(&app, req).await;

        // API config should not have CSP
        assert!(!resp.headers().contains_key("content-security-policy"));

        // But should have other security headers
        assert!(resp.headers().contains_key("x-content-type-options"));
        assert!(resp.headers().contains_key("x-frame-options"));
    }

    #[actix_web::test]
    async fn test_custom_config() {
        let config = SecurityHeadersConfig {
            enable_hsts: false,
            hsts_max_age: 3600,
            hsts_include_subdomains: false,
            hsts_preload: false,
            frame_options: "SAMEORIGIN".to_string(),
            content_security_policy: None,
            referrer_policy: "no-referrer".to_string(),
        };

        let app = test::init_service(
            App::new()
                .wrap(SecurityHeaders::new(config))
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.headers().get("x-frame-options").unwrap(), "SAMEORIGIN");
        assert_eq!(
            resp.headers().get("referrer-policy").unwrap(),
            "no-referrer"
        );
        assert!(!resp.headers().contains_key("strict-transport-security"));
    }

    #[::core::prelude::v1::test]
    fn test_default_config() {
        let config = SecurityHeadersConfig::default();

        assert_eq!(config.hsts_max_age, 31_536_000);
        assert!(config.hsts_include_subdomains);
        assert!(!config.hsts_preload);
        assert_eq!(config.frame_options, "DENY");
        assert_eq!(config.referrer_policy, "strict-origin-when-cross-origin");
    }
}
