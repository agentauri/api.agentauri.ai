//! API Key DTOs and Permission Management
//!
//! This module provides:
//! - Request/Response DTOs for API key CRUD operations
//! - Permission validation helpers
//! - Key format and type constants
//!
//! # Security Model
//!
//! API keys are used for Layer 1 authentication. The full key is shown only once
//! at creation time. After that, only the prefix is visible.
//!
//! # Key Format
//!
//! ```text
//! sk_live_<43 base64 chars>  (production)
//! sk_test_<43 base64 chars>  (testing)
//! ```
//!
//! # Permission Model
//!
//! Only organization owners and admins can create, rotate, and revoke keys.
//! All members can view (masked) key information.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ============================================================================
// Constants
// ============================================================================

/// Live environment key prefix
#[allow(dead_code)]
pub const KEY_PREFIX_LIVE: &str = "sk_live_";

/// Test environment key prefix
#[allow(dead_code)]
pub const KEY_PREFIX_TEST: &str = "sk_test_";

/// Valid key environments
pub const VALID_ENVIRONMENTS: [&str; 2] = ["live", "test"];

/// Valid key types
pub const VALID_KEY_TYPES: [&str; 3] = ["standard", "restricted", "admin"];

/// Valid permissions
pub const VALID_PERMISSIONS: [&str; 4] = ["read", "write", "delete", "admin"];

/// Length of the prefix stored in database (for lookup)
/// Format: sk_live_XXXXXXXX (16 chars total)
#[allow(dead_code)]
pub const KEY_PREFIX_LENGTH: usize = 16;

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to create a new API key
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"name": "Production Key", "environment": "live", "key_type": "standard", "permissions": ["read", "write"]}))]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the key
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    /// Environment: "live" or "test"
    #[validate(custom(function = "validate_environment"))]
    pub environment: String,

    /// Key type: "standard", "restricted", or "admin"
    #[validate(custom(function = "validate_key_type"))]
    #[serde(default = "default_key_type")]
    pub key_type: String,

    /// List of permissions: ["read"], ["read", "write"], etc.
    #[validate(custom(function = "validate_permissions"))]
    #[serde(default = "default_permissions")]
    pub permissions: Vec<String>,

    /// Optional custom rate limit (requests per hour)
    #[validate(range(min = 1, max = 100000))]
    pub rate_limit_override: Option<i32>,

    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_key_type() -> String {
    "standard".to_string()
}

fn default_permissions() -> Vec<String> {
    vec!["read".to_string()]
}

/// Request to rotate an API key
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"name": "Rotated Key"}))]
pub struct RotateApiKeyRequest {
    /// Optional new name for the rotated key
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    /// Optional new expiration
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to revoke an API key
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RevokeApiKeyRequest {
    /// Reason for revocation (for audit purposes)
    #[validate(length(max = 1000))]
    pub reason: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Response when creating a new API key
/// This is the ONLY time the full key is returned
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyCreatedResponse {
    /// The API key ID
    pub id: String,

    /// The full API key - ONLY returned at creation time
    /// Client MUST save this immediately, it will never be shown again
    pub key: String,

    /// Human-readable name
    pub name: String,

    /// Key prefix (first 16 chars) for identification
    pub prefix: String,

    /// Environment: "live" or "test"
    pub environment: String,

    /// Key type
    pub key_type: String,

    /// Granted permissions
    pub permissions: Vec<String>,

    /// When the key was created
    pub created_at: DateTime<Utc>,

    /// When the key expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response for API key details (masked - never shows full key)
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub environment: String,
    pub key_type: String,
    pub permissions: Vec<String>,
    pub rate_limit_override: Option<i32>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub is_revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl From<shared::models::ApiKey> for ApiKeyResponse {
    fn from(key: shared::models::ApiKey) -> Self {
        // Parse permissions from JSONB
        let permissions: Vec<String> = serde_json::from_value(key.permissions.clone())
            .unwrap_or_else(|_| vec!["read".to_string()]);

        Self {
            id: key.id,
            name: key.name,
            prefix: key.prefix,
            environment: key.environment,
            key_type: key.key_type,
            permissions,
            rate_limit_override: key.rate_limit_override,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            created_at: key.created_at,
            created_by: key.created_by,
            is_revoked: key.revoked_at.is_some(),
            revoked_at: key.revoked_at,
        }
    }
}

/// Response after rotating a key
#[derive(Debug, Serialize, ToSchema)]
pub struct RotateApiKeyResponse {
    /// The new API key ID
    pub id: String,

    /// The new full API key - ONLY returned at rotation time
    pub key: String,

    /// New key prefix
    pub prefix: String,

    /// ID of the old key that was revoked
    pub old_key_id: String,

    /// When the old key was revoked
    pub old_key_revoked_at: DateTime<Utc>,
}

/// Paginated list of API keys
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyListResponse {
    pub items: Vec<ApiKeyResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

// ============================================================================
// Validators
// ============================================================================

/// Validate environment field
fn validate_environment(env: &str) -> Result<(), validator::ValidationError> {
    if !VALID_ENVIRONMENTS.contains(&env) {
        let mut err = validator::ValidationError::new("invalid_environment");
        err.message = Some(
            format!(
                "Environment must be one of: {}",
                VALID_ENVIRONMENTS.join(", ")
            )
            .into(),
        );
        return Err(err);
    }
    Ok(())
}

/// Validate key type field
fn validate_key_type(key_type: &str) -> Result<(), validator::ValidationError> {
    if !VALID_KEY_TYPES.contains(&key_type) {
        let mut err = validator::ValidationError::new("invalid_key_type");
        err.message =
            Some(format!("Key type must be one of: {}", VALID_KEY_TYPES.join(", ")).into());
        return Err(err);
    }
    Ok(())
}

/// Validate permissions array
fn validate_permissions(permissions: &[String]) -> Result<(), validator::ValidationError> {
    if permissions.is_empty() {
        let mut err = validator::ValidationError::new("empty_permissions");
        err.message = Some("At least one permission is required".into());
        return Err(err);
    }

    for perm in permissions {
        if !VALID_PERMISSIONS.contains(&perm.as_str()) {
            let mut err = validator::ValidationError::new("invalid_permission");
            err.message = Some(
                format!(
                    "Invalid permission '{}'. Must be one of: {}",
                    perm,
                    VALID_PERMISSIONS.join(", ")
                )
                .into(),
            );
            return Err(err);
        }
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if an API key has a specific permission
#[allow(dead_code)]
pub fn api_key_has_permission(permissions: &[String], required: &str) -> bool {
    // Admin permission grants all access
    if permissions.contains(&"admin".to_string()) {
        return true;
    }
    permissions.contains(&required.to_string())
}

/// Check if an API key can perform write operations
#[allow(dead_code)]
pub fn api_key_can_write(permissions: &[String]) -> bool {
    api_key_has_permission(permissions, "write") || api_key_has_permission(permissions, "admin")
}

/// Check if an API key can perform delete operations
#[allow(dead_code)]
pub fn api_key_can_delete(permissions: &[String]) -> bool {
    api_key_has_permission(permissions, "delete") || api_key_has_permission(permissions, "admin")
}

/// Get the prefix for a given environment
#[allow(dead_code)]
pub fn get_prefix_for_environment(environment: &str) -> &'static str {
    match environment {
        "live" => KEY_PREFIX_LIVE,
        "test" => KEY_PREFIX_TEST,
        _ => KEY_PREFIX_TEST, // Default to test for safety
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreateApiKeyRequest validation tests
    // ========================================================================

    #[test]
    fn test_create_api_key_request_valid() {
        let req = CreateApiKeyRequest {
            name: "Production Key".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            rate_limit_override: Some(1000),
            expires_at: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_api_key_request_minimal() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "test".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_api_key_request_empty_name() {
        let req = CreateApiKeyRequest {
            name: "".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_create_api_key_request_name_too_long() {
        let req = CreateApiKeyRequest {
            name: "a".repeat(256),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_api_key_request_invalid_environment() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "production".to_string(), // Invalid
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("environment"));
    }

    #[test]
    fn test_create_api_key_request_invalid_key_type() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "live".to_string(),
            key_type: "superadmin".to_string(), // Invalid
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("key_type"));
    }

    #[test]
    fn test_create_api_key_request_empty_permissions() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec![],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("permissions"));
    }

    #[test]
    fn test_create_api_key_request_invalid_permission() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string(), "superwrite".to_string()],
            rate_limit_override: None,
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("permissions"));
    }

    #[test]
    fn test_create_api_key_request_rate_limit_too_low() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: Some(0),
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_api_key_request_rate_limit_too_high() {
        let req = CreateApiKeyRequest {
            name: "Test Key".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: Some(100001),
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // RotateApiKeyRequest validation tests
    // ========================================================================

    #[test]
    fn test_rotate_api_key_request_valid() {
        let req = RotateApiKeyRequest {
            name: Some("Rotated Key".to_string()),
            expires_at: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_rotate_api_key_request_empty() {
        let req = RotateApiKeyRequest {
            name: None,
            expires_at: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_rotate_api_key_request_name_too_long() {
        let req = RotateApiKeyRequest {
            name: Some("a".repeat(256)),
            expires_at: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // RevokeApiKeyRequest validation tests
    // ========================================================================

    #[test]
    fn test_revoke_api_key_request_valid() {
        let req = RevokeApiKeyRequest {
            reason: Some("No longer needed".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_revoke_api_key_request_empty() {
        let req = RevokeApiKeyRequest { reason: None };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_revoke_api_key_request_reason_too_long() {
        let req = RevokeApiKeyRequest {
            reason: Some("a".repeat(1001)),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // Validator function tests
    // ========================================================================

    #[test]
    fn test_validate_environment_valid() {
        assert!(validate_environment("live").is_ok());
        assert!(validate_environment("test").is_ok());
    }

    #[test]
    fn test_validate_environment_invalid() {
        assert!(validate_environment("production").is_err());
        assert!(validate_environment("staging").is_err());
        assert!(validate_environment("").is_err());
    }

    #[test]
    fn test_validate_key_type_valid() {
        assert!(validate_key_type("standard").is_ok());
        assert!(validate_key_type("restricted").is_ok());
        assert!(validate_key_type("admin").is_ok());
    }

    #[test]
    fn test_validate_key_type_invalid() {
        assert!(validate_key_type("superadmin").is_err());
        assert!(validate_key_type("").is_err());
        assert!(validate_key_type("Standard").is_err()); // case-sensitive
    }

    #[test]
    fn test_validate_permissions_valid() {
        assert!(validate_permissions(&["read".to_string()]).is_ok());
        assert!(validate_permissions(&["read".to_string(), "write".to_string()]).is_ok());
        assert!(validate_permissions(&[
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
            "admin".to_string()
        ])
        .is_ok());
    }

    #[test]
    fn test_validate_permissions_invalid() {
        assert!(validate_permissions(&[]).is_err()); // empty
        assert!(validate_permissions(&["superread".to_string()]).is_err()); // invalid
        assert!(validate_permissions(&["read".to_string(), "invalid".to_string()]).is_err());
        // one invalid
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_api_key_has_permission() {
        let perms = vec!["read".to_string(), "write".to_string()];
        assert!(api_key_has_permission(&perms, "read"));
        assert!(api_key_has_permission(&perms, "write"));
        assert!(!api_key_has_permission(&perms, "delete"));
        assert!(!api_key_has_permission(&perms, "admin"));
    }

    #[test]
    fn test_api_key_has_permission_admin_grants_all() {
        let perms = vec!["admin".to_string()];
        assert!(api_key_has_permission(&perms, "read"));
        assert!(api_key_has_permission(&perms, "write"));
        assert!(api_key_has_permission(&perms, "delete"));
        assert!(api_key_has_permission(&perms, "admin"));
    }

    #[test]
    fn test_api_key_can_write() {
        assert!(api_key_can_write(&["write".to_string()]));
        assert!(api_key_can_write(&["admin".to_string()]));
        assert!(api_key_can_write(&[
            "read".to_string(),
            "write".to_string()
        ]));
        assert!(!api_key_can_write(&["read".to_string()]));
    }

    #[test]
    fn test_api_key_can_delete() {
        assert!(api_key_can_delete(&["delete".to_string()]));
        assert!(api_key_can_delete(&["admin".to_string()]));
        assert!(api_key_can_delete(&[
            "read".to_string(),
            "delete".to_string()
        ]));
        assert!(!api_key_can_delete(&[
            "read".to_string(),
            "write".to_string()
        ]));
    }

    #[test]
    fn test_get_prefix_for_environment() {
        assert_eq!(get_prefix_for_environment("live"), "sk_live_");
        assert_eq!(get_prefix_for_environment("test"), "sk_test_");
        assert_eq!(get_prefix_for_environment("invalid"), "sk_test_"); // defaults to test
    }

    // ========================================================================
    // Constants tests
    // ========================================================================

    #[test]
    fn test_key_prefix_length() {
        // Verify prefix length matches our format
        let sample_prefix = "sk_live_XXXXXXXX";
        assert_eq!(sample_prefix.len(), KEY_PREFIX_LENGTH);
    }

    #[test]
    fn test_valid_environments_contains_expected() {
        assert!(VALID_ENVIRONMENTS.contains(&"live"));
        assert!(VALID_ENVIRONMENTS.contains(&"test"));
        assert_eq!(VALID_ENVIRONMENTS.len(), 2);
    }

    #[test]
    fn test_valid_key_types_contains_expected() {
        assert!(VALID_KEY_TYPES.contains(&"standard"));
        assert!(VALID_KEY_TYPES.contains(&"restricted"));
        assert!(VALID_KEY_TYPES.contains(&"admin"));
        assert_eq!(VALID_KEY_TYPES.len(), 3);
    }

    #[test]
    fn test_valid_permissions_contains_expected() {
        assert!(VALID_PERMISSIONS.contains(&"read"));
        assert!(VALID_PERMISSIONS.contains(&"write"));
        assert!(VALID_PERMISSIONS.contains(&"delete"));
        assert!(VALID_PERMISSIONS.contains(&"admin"));
        assert_eq!(VALID_PERMISSIONS.len(), 4);
    }
}
