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
/// This helper converts database/repository errors into appropriate HTTP responses
/// while logging the error for debugging. The user-facing message is generic
/// to avoid leaking internal details.
///
/// # Arguments
///
/// * `result` - A Result from a database operation
/// * `action` - A description of the action for logging and error messages
///
/// # Returns
///
/// * `Ok(T)` - The successful result
/// * `Err(HttpResponse)` - 500 Internal Server Error with generic message
///
/// # Example
///
/// ```ignore
/// let key = handle_db_error(
///     ApiKeyRepository::find_by_id(&pool, &key_id).await,
///     "fetch API key"
/// )?;
/// ```
pub fn handle_db_error<T, E: std::fmt::Display>(
    result: Result<T, E>,
    action: &str,
) -> Result<T, HttpResponse> {
    result.map_err(|e| {
        tracing::error!("Database error during {}: {}", action, e);
        HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            format!("Failed to {}", action),
        ))
    })
}

/// Handle any error with consistent logging and response
///
/// Similar to `handle_db_error` but works with any error type that implements
/// `std::fmt::Display`. Useful for service-level errors.
///
/// # Example
///
/// ```ignore
/// let generated = handle_error(
///     api_key_service.generate_key(&req.environment),
///     "generate API key"
/// )?;
/// ```
pub fn handle_error<T, E: std::fmt::Display>(
    result: Result<T, E>,
    action: &str,
) -> Result<T, HttpResponse> {
    result.map_err(|e| {
        tracing::error!("Error during {}: {}", action, e);
        HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            format!("Failed to {}", action),
        ))
    })
}

/// Convert Option<T> to T or return 404 Not Found
///
/// This helper handles the common pattern of checking if a database query
/// returned a result and returning a 404 if not found.
///
/// # Arguments
///
/// * `option` - An Option from a database lookup
/// * `resource` - The name of the resource for the error message
///
/// # Returns
///
/// * `Ok(T)` - The found value
/// * `Err(HttpResponse)` - 404 Not Found
///
/// # Example
///
/// ```ignore
/// let trigger = require_found(
///     TriggerRepository::find_by_id(&pool, &id).await?,
///     "Trigger"
/// )?;
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

// ============================================================================
// Request Context
// ============================================================================

/// Structured request context for audit logging
///
/// This struct captures common request metadata that is useful for audit
/// logging, including IP address, user agent, and the requested endpoint.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Client IP address (from X-Forwarded-For or direct connection)
    pub ip_address: Option<String>,
    /// Client User-Agent header
    pub user_agent: Option<String>,
    /// Requested endpoint path
    pub endpoint: String,
}

impl RequestContext {
    /// Get IP address as a string slice for logging functions
    pub fn ip_str(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }

    /// Get user agent as a string slice for logging functions
    pub fn user_agent_str(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Get endpoint as a string slice for logging functions
    pub fn endpoint_str(&self) -> &str {
        &self.endpoint
    }
}

/// Extract request context for audit logging
///
/// This helper extracts common request metadata that is useful for audit
/// logging, including the client IP, user agent, and endpoint.
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
/// ).await;
/// ```
pub fn extract_request_context(req: &HttpRequest) -> RequestContext {
    RequestContext {
        ip_address: req
            .connection_info()
            .realip_remote_addr()
            .map(|s| s.to_string()),
        user_agent: req
            .headers()
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        endpoint: req.uri().path().to_string(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Request Context Tests
    // ========================================================================

    #[test]
    fn test_request_context_ip_str() {
        let ctx = RequestContext {
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            endpoint: "/test".to_string(),
        };
        assert_eq!(ctx.ip_str(), Some("192.168.1.1"));
    }

    #[test]
    fn test_request_context_ip_str_none() {
        let ctx = RequestContext {
            ip_address: None,
            user_agent: None,
            endpoint: "/test".to_string(),
        };
        assert!(ctx.ip_str().is_none());
    }

    #[test]
    fn test_request_context_user_agent_str() {
        let ctx = RequestContext {
            ip_address: None,
            user_agent: Some("Mozilla/5.0".to_string()),
            endpoint: "/test".to_string(),
        };
        assert_eq!(ctx.user_agent_str(), Some("Mozilla/5.0"));
    }

    #[test]
    fn test_request_context_endpoint_str() {
        let ctx = RequestContext {
            ip_address: None,
            user_agent: None,
            endpoint: "/api/v1/test".to_string(),
        };
        assert_eq!(ctx.endpoint_str(), "/api/v1/test");
    }

    // ========================================================================
    // Validation Helper Tests
    // ========================================================================

    #[derive(Debug)]
    struct TestValidatable {
        valid: bool,
    }

    impl Validate for TestValidatable {
        fn validate(&self) -> Result<(), validator::ValidationErrors> {
            if self.valid {
                Ok(())
            } else {
                let mut errors = validator::ValidationErrors::new();
                errors.add("field", validator::ValidationError::new("test error"));
                Err(errors)
            }
        }
    }

    #[test]
    fn test_validate_request_success() {
        let req = TestValidatable { valid: true };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn test_validate_request_failure() {
        let req = TestValidatable { valid: false };
        let result = validate_request(&req);
        assert!(result.is_err());
    }

    // ========================================================================
    // Option Helper Tests
    // ========================================================================

    #[test]
    fn test_require_found_some() {
        let option = Some("value");
        let result = require_found(option, "Resource");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "value");
    }

    #[test]
    fn test_require_found_none() {
        let option: Option<&str> = None;
        let result = require_found(option, "Resource");
        assert!(result.is_err());
    }

    // ========================================================================
    // Error Response Helper Tests
    // ========================================================================

    #[test]
    fn test_forbidden_response() {
        let resp = forbidden("Access denied");
        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_bad_request_response() {
        let resp = bad_request("Invalid input");
        assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }

    // ========================================================================
    // DB Error Handler Tests
    // ========================================================================

    #[test]
    fn test_handle_db_error_ok() {
        let result: Result<i32, sqlx::Error> = Ok(42);
        let handled = handle_db_error(result, "test operation");
        assert!(handled.is_ok());
        assert_eq!(handled.unwrap(), 42);
    }

    // ========================================================================
    // Generic Error Handler Tests
    // ========================================================================

    #[test]
    fn test_handle_error_ok() {
        let result: Result<i32, String> = Ok(42);
        let handled = handle_error(result, "test operation");
        assert!(handled.is_ok());
        assert_eq!(handled.unwrap(), 42);
    }

    #[test]
    fn test_handle_error_err() {
        let result: Result<i32, &str> = Err("something went wrong");
        let handled = handle_error(result, "test operation");
        assert!(handled.is_err());
    }
}
