//! OAuth DTOs and Models
//!
//! This module provides:
//! - Request/Response DTOs for OAuth endpoints
//! - Validation helpers
//! - Re-exports of database models from shared crate
//!
//! # OAuth 2.0 Flow
//!
//! This implementation supports the following OAuth 2.0 grant types:
//! - **Authorization Code** (with PKCE)
//! - **Client Credentials** (for machine-to-machine)
//! - **Refresh Token**
//!
//! # Security Model
//!
//! - Client secrets are hashed with Argon2id (p=4)
//! - Access tokens are hashed with Argon2id (p=4)
//! - Refresh tokens are hashed with Argon2id (p=4)
//! - All secrets are shown only once at creation time

use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use validator::Validate;

// Re-export database models from shared crate
pub use shared::models::{OAuthClient, OAuthToken};

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to create a new OAuth client
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOAuthClientRequest {
    /// Display name for the OAuth application
    #[validate(length(min = 1, max = 255))]
    pub client_name: String,

    /// List of allowed redirect URIs
    #[validate(length(min = 1))]
    #[validate(custom(function = "validate_redirect_uris"))]
    pub redirect_uris: Vec<String>,

    /// List of allowed scopes
    #[validate(length(min = 1))]
    #[validate(custom(function = "validate_scopes"))]
    pub scopes: Vec<String>,

    /// List of allowed grant types
    #[validate(length(min = 1))]
    #[validate(custom(function = "validate_grant_types"))]
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,

    /// Whether this is a trusted first-party application
    #[serde(default)]
    pub is_trusted: bool,
}

/// OAuth token request (RFC 6749 Section 4.1.3)
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// Grant type: "authorization_code", "refresh_token", "client_credentials"
    pub grant_type: String,

    /// Client ID (required)
    pub client_id: String,

    /// Client secret (required for confidential clients)
    pub client_secret: Option<String>,

    /// Authorization code (required for authorization_code grant)
    pub code: Option<String>,

    /// Redirect URI (required for authorization_code grant)
    pub redirect_uri: Option<String>,

    /// Refresh token (required for refresh_token grant)
    pub refresh_token: Option<String>,

    /// Scope (optional, space-separated)
    pub scope: Option<String>,

    /// PKCE code verifier (optional)
    pub code_verifier: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Response after creating an OAuth client
#[derive(Debug, Serialize)]
pub struct CreateOAuthClientResponse {
    /// The client ID (public identifier)
    pub client_id: String,

    /// The client secret (SHOWN ONLY ONCE!)
    pub client_secret: String,

    /// Display name
    pub client_name: String,

    /// Allowed redirect URIs
    pub redirect_uris: Vec<String>,

    /// Allowed scopes
    pub scopes: Vec<String>,

    /// Allowed grant types
    pub grant_types: Vec<String>,

    /// Whether this is a trusted application
    pub is_trusted: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Response for listing OAuth clients (masked)
#[derive(Debug, Serialize)]
pub struct OAuthClientResponse {
    /// Internal ID
    pub id: String,

    /// Client ID (public identifier)
    pub client_id: String,

    /// Display name
    pub client_name: String,

    /// Allowed redirect URIs
    pub redirect_uris: Vec<String>,

    /// Allowed scopes
    pub scopes: Vec<String>,

    /// Allowed grant types
    pub grant_types: Vec<String>,

    /// Whether this is a trusted application
    pub is_trusted: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl From<OAuthClient> for OAuthClientResponse {
    fn from(client: OAuthClient) -> Self {
        Self {
            id: client.id,
            client_id: client.client_id,
            client_name: client.client_name,
            redirect_uris: client.redirect_uris,
            scopes: client.scopes,
            grant_types: client.grant_types,
            is_trusted: client.is_trusted,
            created_at: client.created_at,
            updated_at: client.updated_at,
        }
    }
}

/// OAuth token response (RFC 6749 Section 5.1)
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// The access token
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Expiration time in seconds
    pub expires_in: i64,

    /// Refresh token (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Granted scopes (space-separated)
    pub scope: String,
}

/// Response for listing OAuth clients
#[derive(Debug, Serialize)]
pub struct OAuthClientListResponse {
    pub clients: Vec<OAuthClientResponse>,
    pub total: i64,
}

// ============================================================================
// Validation Functions
// ============================================================================

fn validate_redirect_uris(uris: &[String]) -> Result<(), validator::ValidationError> {
    if uris.is_empty() {
        return Err(validator::ValidationError::new("redirect_uris_empty"));
    }

    for uri in uris {
        // Must be a valid URL
        if Url::parse(uri).is_err() {
            return Err(validator::ValidationError::new("invalid_redirect_uri"));
        }

        // Must use HTTPS (except localhost for development)
        let url = Url::parse(uri).unwrap();
        if url.scheme() != "https" && url.host_str() != Some("localhost") && url.host_str() != Some("127.0.0.1") {
            return Err(validator::ValidationError::new("redirect_uri_must_use_https"));
        }
    }

    Ok(())
}

fn validate_scopes(scopes: &[String]) -> Result<(), validator::ValidationError> {
    if scopes.is_empty() {
        return Err(validator::ValidationError::new("scopes_empty"));
    }

    // Valid scope pattern: <resource>:<action> (e.g., "read:triggers", "write:billing")
    let valid_scopes = [
        "read:triggers",
        "write:triggers",
        "delete:triggers",
        "read:billing",
        "write:billing",
        "read:api-keys",
        "write:api-keys",
        "delete:api-keys",
        "read:agents",
        "write:agents",
        "delete:agents",
        "read:organizations",
        "write:organizations",
        "admin:all",
    ];

    for scope in scopes {
        if !valid_scopes.contains(&scope.as_str()) {
            return Err(validator::ValidationError::new("invalid_scope"));
        }
    }

    Ok(())
}

fn validate_grant_types(grant_types: &[String]) -> Result<(), validator::ValidationError> {
    if grant_types.is_empty() {
        return Err(validator::ValidationError::new("grant_types_empty"));
    }

    let valid_grant_types = ["authorization_code", "refresh_token", "client_credentials"];

    for grant_type in grant_types {
        if !valid_grant_types.contains(&grant_type.as_str()) {
            return Err(validator::ValidationError::new("invalid_grant_type"));
        }
    }

    Ok(())
}

fn default_grant_types() -> Vec<String> {
    vec![
        "authorization_code".to_string(),
        "refresh_token".to_string(),
    ]
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse scope string (space-separated) into Vec<String>
pub fn parse_scopes(scope_str: &str) -> Vec<String> {
    scope_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Convert Vec<String> to scope string (space-separated)
pub fn scopes_to_string(scopes: &[String]) -> String {
    scopes.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_uris_https() {
        let uris = vec!["https://example.com/callback".to_string()];
        assert!(validate_redirect_uris(&uris).is_ok());
    }

    #[test]
    fn test_validate_redirect_uris_localhost() {
        let uris = vec!["http://localhost:3000/callback".to_string()];
        assert!(validate_redirect_uris(&uris).is_ok());
    }

    #[test]
    fn test_validate_redirect_uris_http_rejected() {
        let uris = vec!["http://example.com/callback".to_string()];
        assert!(validate_redirect_uris(&uris).is_err());
    }

    #[test]
    fn test_validate_redirect_uris_empty() {
        let uris: Vec<String> = vec![];
        assert!(validate_redirect_uris(&uris).is_err());
    }

    #[test]
    fn test_validate_scopes_valid() {
        let scopes = vec!["read:triggers".to_string(), "write:triggers".to_string()];
        assert!(validate_scopes(&scopes).is_ok());
    }

    #[test]
    fn test_validate_scopes_invalid() {
        let scopes = vec!["invalid:scope".to_string()];
        assert!(validate_scopes(&scopes).is_err());
    }

    #[test]
    fn test_validate_grant_types_valid() {
        let grant_types = vec!["authorization_code".to_string(), "refresh_token".to_string()];
        assert!(validate_grant_types(&grant_types).is_ok());
    }

    #[test]
    fn test_validate_grant_types_invalid() {
        let grant_types = vec!["invalid_grant".to_string()];
        assert!(validate_grant_types(&grant_types).is_err());
    }

    #[test]
    fn test_parse_scopes() {
        let scope_str = "read:triggers write:triggers";
        let scopes = parse_scopes(scope_str);
        assert_eq!(scopes, vec!["read:triggers", "write:triggers"]);
    }

    #[test]
    fn test_scopes_to_string() {
        let scopes = vec!["read:triggers".to_string(), "write:triggers".to_string()];
        let scope_str = scopes_to_string(&scopes);
        assert_eq!(scope_str, "read:triggers write:triggers");
    }
}
