//! Common Handler Helpers
//!
//! This module provides reusable helper functions that consolidate common patterns
//! found across handlers, reducing code duplication and ensuring consistent error
//! handling and response formatting.
//!
//! # Helper Categories
//!
//! ## Authentication
//! - [`extract_user_id_or_unauthorized`] - Extract user_id from JWT or return 401
//!
//! ## Validation
//! - [`validate_request`] - Validate a request or return 400
//!
//! ## Error Handling
//! - [`handle_db_error`] - Convert database errors to HTTP responses with logging
//! - [`require_found`] - Convert Option<T> to T or return 404
//!
//! ## Request Context
//! - [`RequestContext`] - Structured request metadata for audit logging
//! - [`extract_request_context`] - Extract context from HTTP request

use actix_web::{HttpRequest, HttpResponse};
use validator::Validate;

use crate::middleware::get_user_id;
use crate::models::ErrorResponse;

// ============================================================================
// Authentication Helpers
// ============================================================================

/// Extract user_id from JWT claims or return 401 Unauthorized
///
/// This helper consolidates the common pattern of extracting the authenticated
/// user ID from request extensions and returning a consistent error response
/// when authentication is missing or invalid.
///
/// # Returns
///
/// * `Ok(String)` - The authenticated user's ID
/// * `Err(HttpResponse)` - 401 Unauthorized with standard error body
///
/// # Example
///
/// ```ignore
/// let user_id = match extract_user_id_or_unauthorized(&req_http) {
///     Ok(id) => id,
///     Err(resp) => return resp,
/// };
/// ```
pub fn extract_user_id_or_unauthorized(req: &HttpRequest) -> Result<String, HttpResponse> {
    get_user_id(req).map_err(|_| {
        HttpResponse::Unauthorized().json(ErrorResponse::new(
            "unauthorized",
            "Authentication required",
        ))
    })
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate a request struct or return 400 Bad Request
///
/// This helper consolidates the common validation pattern using the `validator`
/// crate, returning a consistent error response with validation details.
///
/// # Type Parameters
///
/// * `T` - Any type that implements the `Validate` trait
///
/// # Returns
///
/// * `Ok(())` - Validation passed
/// * `Err(HttpResponse)` - 400 Bad Request with validation errors
///
/// # Example
///
/// ```ignore
/// if let Err(resp) = validate_request(&req) {
///     return resp;
/// }
/// ```
pub fn validate_request<T: Validate>(req: &T) -> Result<(), HttpResponse> {
    req.validate().map_err(|e| {
        HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ))
    })
}

// ============================================================================
// Error Handling Helpers
// ============================================================================

/// Handle database errors with consistent logging and response
///
/// This helper consolidates the common pattern of handling database errors
/// across handlers, ensuring consistent logging and error responses.
///
/// # Arguments
///
/// * `result` - A Result from a database operation
/// * `context` - A string describing the operation (for logging)
///
/// # Returns
///
/// * `Ok(T)` - The successful value
/// * `Err(HttpResponse)` - 500 Internal Server Error with safe error message
///
/// # Example
///
/// ```ignore
/// let users = match handle_db_error(
///     UserRepository::list(&pool).await,
///     "list users",
/// ) {
///     Ok(u) => u,
///     Err(resp) => return resp,
/// };
/// ```
pub fn handle_db_error<T, E: std::fmt::Display>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, HttpResponse> {
    result.map_err(|e| {
        tracing::error!("Database error during {}: {}", context, e);
        HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            format!("Failed to {}", context),
        ))
    })
}

/// Handle general errors with consistent logging and response
///
/// Similar to handle_db_error but for non-database operations.
///
/// # Example
///
/// ```ignore
/// let generated = match handle_error(
///     api_key_service.generate_key("live"),
///     "generate API key",
/// ) {
///     Ok(g) => g,
///     Err(resp) => return resp,
/// };
/// ```
pub fn handle_error<T, E: std::fmt::Display>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, HttpResponse> {
    result.map_err(|e| {
        tracing::error!("Error during {}: {}", context, e);
        HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            format!("Failed to {}", context),
        ))
    })
}

/// Convert Option<T> to T or return 404 Not Found
///
/// This helper consolidates the common pattern of checking if a resource exists
/// and returning a consistent 404 response when it doesn't.
///
/// # Example
///
/// ```ignore
/// let trigger = match require_found(
///     TriggerRepository::find(&pool, &id).await?,
///     "Trigger",
/// ) {
///     Ok(t) => t,
///     Err(resp) => return resp,
/// };
/// ```
pub fn require_found<T>(option: Option<T>, resource: &str) -> Result<T, HttpResponse> {
    option.ok_or_else(|| {
        HttpResponse::NotFound().json(ErrorResponse::new(
            "not_found",
            format!("{} not found", resource),
        ))
    })
}

/// Return a 403 Forbidden response with a custom message
///
/// # Example
///
/// ```ignore
/// if !can_manage_org(&role) {
///     return forbidden("Insufficient permissions to create API keys");
/// }
/// ```
pub fn forbidden(message: &str) -> HttpResponse {
    HttpResponse::Forbidden().json(ErrorResponse::new("forbidden", message))
}

/// Return a 400 Bad Request response with a custom message
///
/// # Example
///
/// ```ignore
/// if key.revoked_at.is_some() {
///     return bad_request("API key is already revoked");
/// }
/// ```
pub fn bad_request(message: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(ErrorResponse::new("bad_request", message))
}

/// Return a 401 Unauthorized response with a custom message
///
/// # Example
///
/// ```ignore
/// if !valid_credentials {
///     return unauthorized("Invalid credentials");
/// }
/// ```
pub fn unauthorized(message: &str) -> HttpResponse {
    HttpResponse::Unauthorized().json(ErrorResponse::new("unauthorized", message))
}

/// Return a safe internal server error that doesn't leak implementation details
///
/// SECURITY: Use this for errors where the cause should never be exposed to clients,
/// such as database errors, configuration issues, or third-party service failures.
///
/// # Example
///
/// ```ignore
/// if let Err(e) = some_sensitive_operation().await {
///     tracing::error!("Sensitive operation failed: {}", e);
///     return safe_internal_error();
/// }
/// ```
#[allow(dead_code)] // Used in handlers for security-sensitive errors
pub fn safe_internal_error() -> HttpResponse {
    HttpResponse::InternalServerError().json(ErrorResponse::new(
        "internal_error",
        "An internal error occurred. Please try again later.",
    ))
}

/// Handle errors with strict sanitization - no dynamic content in response
///
/// SECURITY: This variant never includes any dynamic content in the error response,
/// making it suitable for handling sensitive errors where even the action name
/// might reveal too much information.
///
/// # Arguments
///
/// * `result` - A Result from any operation
/// * `context` - Context string for logging (not exposed to client)
///
/// # Example
///
/// ```ignore
/// let result = match handle_db_error_safe(
///     perform_sensitive_query(&pool).await,
///     "sensitive database query",
/// ) {
///     Ok(r) => r,
///     Err(resp) => return resp,
/// };
/// ```
#[allow(dead_code)] // Used in handlers for security-sensitive database errors
pub fn handle_db_error_safe<T, E: std::fmt::Display>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, HttpResponse> {
    result.map_err(|e| {
        tracing::error!("Database error during {}: {}", context, e);
        safe_internal_error()
    })
}

// ============================================================================
// Request Context
// ============================================================================

/// Structured request metadata for audit logging
///
/// This struct captures common request metadata in a consistent format,
/// suitable for audit logging and security analysis.
pub struct RequestContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub endpoint: String,
}

impl RequestContext {
    /// Get IP address as Option<&str> for database operations
    pub fn ip_str(&self) -> Option<&str> {
        self.ip.as_deref()
    }

    /// Get user agent as Option<&str> for database operations
    pub fn user_agent_str(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Get endpoint as &str for database operations
    pub fn endpoint_str(&self) -> &str {
        &self.endpoint
    }
}

/// Extract request context from HTTP request
///
/// # Arguments
///
/// * `req` - The HTTP request
///
/// # Returns
///
/// A `RequestContext` struct containing IP, user agent, and endpoint path
///
/// # Example
///
/// ```ignore
/// let ctx = extract_request_context(&req_http);
/// ApiKeyAuditRepository::log(
///     &pool,
///     Some(&key.id),
///     &org_id,
///     "created",
///     ctx.ip_str(),
///     ctx.user_agent_str(),
///     Some(ctx.endpoint_str()),
///     Some(&user_id),
///     None,
/// ).await?;
/// ```
pub fn extract_request_context(req: &HttpRequest) -> RequestContext {
    // Extract IP from connection info or X-Forwarded-For header
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    // Extract User-Agent header
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Get the endpoint path
    let endpoint = req.path().to_string();

    RequestContext {
        ip,
        user_agent,
        endpoint,
    }
}
