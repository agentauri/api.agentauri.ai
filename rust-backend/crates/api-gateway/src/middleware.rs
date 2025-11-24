//! Middleware for the API Gateway

use actix_cors::Cors;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    http, Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::{
    env,
    future::{ready, Ready},
    rc::Rc,
};

use crate::models::Claims;

/// Configure CORS middleware
pub fn cors() -> Cors {
    // Get allowed origins from environment variable
    // Format: comma-separated list of origins
    // Example: ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com
    let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| String::new());

    let origins: Vec<String> = allowed_origins
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    Cors::default()
        .allowed_origin_fn(move |origin, _req_head| {
            let origin_str = origin.to_str().unwrap_or("");

            if cfg!(debug_assertions) {
                // Development mode: Allow localhost
                origin_str.starts_with("http://localhost")
                    || origin_str.starts_with("http://127.0.0.1")
            } else {
                // Production mode: Whitelist only
                if origins.is_empty() {
                    tracing::warn!(
                        "ALLOWED_ORIGINS not set. Denying all CORS requests in production."
                    );
                    false
                } else {
                    origins.iter().any(|allowed| origin_str == allowed)
                }
            }
        })
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ])
        .max_age(3600)
}

// ============================================================================
// JWT Authentication Middleware
// ============================================================================

/// JWT authentication middleware
pub struct JwtAuth {
    jwt_secret: Rc<String>,
}

impl JwtAuth {
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
            // Extract Authorization header
            let auth_header = req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok());

            let token = match auth_header {
                Some(header) => {
                    if let Some(token) = header.strip_prefix("Bearer ") {
                        token
                    } else {
                        return Err(ErrorUnauthorized("Invalid authorization header format"));
                    }
                }
                None => return Err(ErrorUnauthorized("Missing authorization header")),
            };

            // Validate JWT token
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = true; // Explicitly enable expiration validation
            validation.leeway = 60; // 60 seconds clock skew tolerance

            let token_data = decode::<Claims>(
                token,
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
