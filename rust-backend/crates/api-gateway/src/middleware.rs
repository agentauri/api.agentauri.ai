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
//! # Security Notes
//!
//! - JWT tokens are validated using HS256 algorithm
//! - The `X-Organization-ID` header is untrusted and always verified against membership
//! - All verification functions return appropriate HTTP error responses on failure

use actix_cors::Cors;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    http, Error, HttpMessage, HttpRequest, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use shared::DbPool;
use std::{
    env,
    future::{ready, Ready},
    rc::Rc,
};

use crate::models::{Claims, ErrorResponse};
use crate::repositories::MemberRepository;

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

/// Helper to extract and verify X-Organization-ID header
///
/// This function extracts the organization ID from the request header AND
/// verifies that the authenticated user is a member of that organization.
/// This prevents horizontal privilege escalation via header spoofing.
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
    let is_member = MemberRepository::is_member(pool, &org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify organization membership: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to verify organization access",
            ))
        })?;

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
    let role = MemberRepository::get_role(pool, &org_id, user_id)
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
        })?;

    Ok((org_id, role))
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
