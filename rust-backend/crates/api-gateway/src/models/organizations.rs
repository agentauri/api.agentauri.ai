//! Organization DTOs and Role Management
//!
//! This module provides:
//! - Request/Response DTOs for organization CRUD operations
//! - Member management DTOs
//! - Role constants and permission helpers
//!
//! # Role Hierarchy
//!
//! The organization role system follows a strict hierarchy:
//!
//! ```text
//! owner (4) > admin (3) > member (2) > viewer (1)
//! ```
//!
//! | Role    | Level | Permissions                                      |
//! |---------|-------|--------------------------------------------------|
//! | owner   | 4     | Full control, can delete org, transfer ownership |
//! | admin   | 3     | Manage settings, members, and all resources      |
//! | member  | 2     | Create/edit triggers, actions, conditions        |
//! | viewer  | 1     | Read-only access to organization resources       |
//!
//! # Permission Checks
//!
//! Use the provided helper functions for permission checks:
//! - [`can_manage_org`] - owner, admin
//! - [`can_write`] - owner, admin, member
//! - [`can_delete_org`] - owner only
//! - [`can_manage_members`] - owner, admin
//! - [`has_permission`] - generic hierarchy check

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ============================================================================
// Role Constants
// ============================================================================

/// Organization owner - full control including deletion and ownership transfer
pub const ROLE_OWNER: &str = "owner";

/// Organization admin - can manage settings, members, and resources
pub const ROLE_ADMIN: &str = "admin";

/// Organization member - can create and edit resources (triggers, etc.)
pub const ROLE_MEMBER: &str = "member";

/// Organization viewer - read-only access
pub const ROLE_VIEWER: &str = "viewer";

/// All valid roles in order of increasing permission level
#[allow(dead_code)]
pub const ROLE_HIERARCHY: [&str; 4] = [ROLE_VIEWER, ROLE_MEMBER, ROLE_ADMIN, ROLE_OWNER];

/// All valid role names (for validation)
pub const VALID_ROLES: [&str; 4] = [ROLE_OWNER, ROLE_ADMIN, ROLE_MEMBER, ROLE_VIEWER];

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to create a new organization
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOrganizationRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(length(min = 1, max = 100))]
    #[validate(custom(function = "validate_slug"))]
    pub slug: String,

    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

/// Request to update an organization
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOrganizationRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

/// Request to add a member to an organization
#[derive(Debug, Deserialize, Validate)]
pub struct AddMemberRequest {
    #[validate(length(min = 1))]
    pub user_id: String,

    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

/// Request to update a member's role
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMemberRoleRequest {
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

/// Request to transfer organization ownership
///
/// The new owner must already be a member of the organization.
/// Personal organizations cannot have ownership transferred.
#[derive(Debug, Deserialize, Validate)]
pub struct TransferOwnershipRequest {
    /// ID of the user to transfer ownership to
    #[validate(length(min = 1))]
    pub new_owner_id: String,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Organization response
#[derive(Debug, Serialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub plan: String,
    pub is_personal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<shared::models::Organization> for OrganizationResponse {
    fn from(org: shared::models::Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            slug: org.slug,
            description: org.description,
            owner_id: org.owner_id,
            plan: org.plan,
            is_personal: org.is_personal,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

impl OrganizationResponse {
    /// Create from an OrganizationWithRole (from JOIN query)
    pub fn from_with_role(org: &crate::repositories::OrganizationWithRole) -> Self {
        Self {
            id: org.id.clone(),
            name: org.name.clone(),
            slug: org.slug.clone(),
            description: org.description.clone(),
            owner_id: org.owner_id.clone(),
            plan: org.plan.clone(),
            is_personal: org.is_personal,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

/// Organization response with the user's role
#[derive(Debug, Serialize)]
pub struct OrganizationWithRoleResponse {
    #[serde(flatten)]
    pub organization: OrganizationResponse,
    pub my_role: String,
}

/// Member response
#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Validators
// ============================================================================

/// Custom validator for slug field
/// Slug must be lowercase alphanumeric with hyphens, starting and ending with alphanumeric
fn validate_slug(slug: &str) -> Result<(), validator::ValidationError> {
    let re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$").unwrap();
    if !re.is_match(slug) {
        let mut err = validator::ValidationError::new("invalid_slug");
        err.message = Some("Slug must be lowercase alphanumeric with hyphens".into());
        return Err(err);
    }
    Ok(())
}

/// Custom validator for role field
///
/// Validates that the role is one of the valid organization roles.
/// See [`VALID_ROLES`] for the list of accepted values.
fn validate_role(role: &str) -> Result<(), validator::ValidationError> {
    if !VALID_ROLES.contains(&role) {
        let mut err = validator::ValidationError::new("invalid_role");
        err.message = Some(format!("Role must be one of: {}", VALID_ROLES.join(", ")).into());
        return Err(err);
    }
    Ok(())
}

// ============================================================================
// Role Helpers
// ============================================================================

/// Check if a role can manage organization settings (update name, description, etc.)
///
/// # Authorized Roles
/// - `owner` ✓
/// - `admin` ✓
/// - `member` ✗
/// - `viewer` ✗
///
/// # Example
/// ```
/// use api_gateway::models::organizations::{can_manage_org, ROLE_OWNER, ROLE_ADMIN};
/// assert!(can_manage_org(ROLE_OWNER));
/// assert!(can_manage_org(ROLE_ADMIN));
/// ```
pub fn can_manage_org(role: &str) -> bool {
    matches!(role, ROLE_OWNER | ROLE_ADMIN)
}

/// Check if a role can write resources (create/edit triggers, actions, conditions)
///
/// # Authorized Roles
/// - `owner` ✓
/// - `admin` ✓
/// - `member` ✓
/// - `viewer` ✗
///
/// # Example
/// ```
/// use api_gateway::models::organizations::{can_write, ROLE_MEMBER, ROLE_VIEWER};
/// assert!(can_write(ROLE_MEMBER));
/// assert!(!can_write(ROLE_VIEWER));
/// ```
pub fn can_write(role: &str) -> bool {
    matches!(role, ROLE_OWNER | ROLE_ADMIN | ROLE_MEMBER)
}

/// Check if a role can delete the organization
///
/// Only the owner can delete an organization. This is a destructive operation
/// that cannot be undone.
///
/// # Authorized Roles
/// - `owner` ✓
/// - `admin` ✗
/// - `member` ✗
/// - `viewer` ✗
pub fn can_delete_org(role: &str) -> bool {
    role == ROLE_OWNER
}

/// Check if a role can manage members (add, remove, update roles)
///
/// # Authorized Roles
/// - `owner` ✓
/// - `admin` ✓
/// - `member` ✗
/// - `viewer` ✗
///
/// Note: Only owners can change roles to/from admin, and owners cannot be removed.
pub fn can_manage_members(role: &str) -> bool {
    matches!(role, ROLE_OWNER | ROLE_ADMIN)
}

/// Check if a role is the owner role
///
/// Convenience function to check for owner without string comparison.
pub fn is_owner(role: &str) -> bool {
    role == ROLE_OWNER
}

/// Check if the user's role has equal or higher permission than required
///
/// Uses the role hierarchy: viewer < member < admin < owner
///
/// # Arguments
/// * `user_role` - The user's current role
/// * `required_role` - The minimum role required for the operation
///
/// # Returns
/// `true` if user_role >= required_role in the hierarchy, `false` otherwise.
/// Returns `false` if either role is invalid.
///
/// # Example
/// ```
/// use api_gateway::models::organizations::{has_permission, ROLE_ADMIN, ROLE_MEMBER};
/// assert!(has_permission(ROLE_ADMIN, ROLE_MEMBER)); // admin >= member
/// assert!(!has_permission(ROLE_MEMBER, ROLE_ADMIN)); // member < admin
/// ```
#[allow(dead_code)]
pub fn has_permission(user_role: &str, required_role: &str) -> bool {
    let user_level = ROLE_HIERARCHY.iter().position(|r| *r == user_role);
    let required_level = ROLE_HIERARCHY.iter().position(|r| *r == required_role);

    match (user_level, required_level) {
        (Some(u), Some(r)) => u >= r,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreateOrganizationRequest validation tests
    // ========================================================================

    #[test]
    fn test_create_organization_request_valid() {
        let req = CreateOrganizationRequest {
            name: "My Organization".to_string(),
            slug: "my-org".to_string(),
            description: Some("A test organization".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_organization_request_minimal() {
        let req = CreateOrganizationRequest {
            name: "Org".to_string(),
            slug: "o".to_string(),
            description: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_organization_request_empty_name() {
        let req = CreateOrganizationRequest {
            name: "".to_string(),
            slug: "my-org".to_string(),
            description: None,
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("name"));
    }

    #[test]
    fn test_create_organization_request_name_too_long() {
        let req = CreateOrganizationRequest {
            name: "a".repeat(256),
            slug: "my-org".to_string(),
            description: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_organization_request_slug_too_short() {
        let req = CreateOrganizationRequest {
            name: "My Org".to_string(),
            slug: "a".to_string(), // 1 char is ok with our regex
            description: None,
        };
        // Single char is valid per our regex
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_organization_request_description_too_long() {
        let req = CreateOrganizationRequest {
            name: "My Org".to_string(),
            slug: "my-org".to_string(),
            description: Some("a".repeat(1001)),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // UpdateOrganizationRequest validation tests
    // ========================================================================

    #[test]
    fn test_update_organization_request_valid() {
        let req = UpdateOrganizationRequest {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_organization_request_all_none() {
        let req = UpdateOrganizationRequest {
            name: None,
            description: None,
        };
        assert!(req.validate().is_ok());
    }

    // ========================================================================
    // AddMemberRequest validation tests
    // ========================================================================

    #[test]
    fn test_add_member_request_valid() {
        let req = AddMemberRequest {
            user_id: "user-123".to_string(),
            role: ROLE_MEMBER.to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_add_member_request_invalid_role() {
        let req = AddMemberRequest {
            user_id: "user-123".to_string(),
            role: "superadmin".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("role"));
    }

    #[test]
    fn test_add_member_request_empty_user_id() {
        let req = AddMemberRequest {
            user_id: "".to_string(),
            role: ROLE_MEMBER.to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // TransferOwnershipRequest validation tests
    // ========================================================================

    #[test]
    fn test_transfer_ownership_request_valid() {
        let req = TransferOwnershipRequest {
            new_owner_id: "user-456".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transfer_ownership_request_empty_new_owner() {
        let req = TransferOwnershipRequest {
            new_owner_id: "".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("new_owner_id"));
    }

    // ========================================================================
    // validate_slug tests
    // ========================================================================

    #[test]
    fn test_validate_slug_valid() {
        assert!(validate_slug("my-org").is_ok());
        assert!(validate_slug("org123").is_ok());
        assert!(validate_slug("a").is_ok());
        assert!(validate_slug("my-awesome-org").is_ok());
        assert!(validate_slug("a1").is_ok());
    }

    #[test]
    fn test_validate_slug_invalid() {
        assert!(validate_slug("My-Org").is_err()); // uppercase
        assert!(validate_slug("-my-org").is_err()); // starts with hyphen
        assert!(validate_slug("my-org-").is_err()); // ends with hyphen
        assert!(validate_slug("my_org").is_err()); // underscore
        assert!(validate_slug("my org").is_err()); // space
    }

    // ========================================================================
    // validate_role tests
    // ========================================================================

    #[test]
    fn test_validate_role_valid() {
        assert!(validate_role(ROLE_OWNER).is_ok());
        assert!(validate_role(ROLE_ADMIN).is_ok());
        assert!(validate_role(ROLE_MEMBER).is_ok());
        assert!(validate_role(ROLE_VIEWER).is_ok());
    }

    #[test]
    fn test_validate_role_invalid() {
        assert!(validate_role("superadmin").is_err());
        assert!(validate_role("").is_err());
        assert!(validate_role("Owner").is_err()); // case-sensitive
    }

    // ========================================================================
    // Role helper tests
    // ========================================================================

    #[test]
    fn test_can_manage_org() {
        assert!(can_manage_org(ROLE_OWNER));
        assert!(can_manage_org(ROLE_ADMIN));
        assert!(!can_manage_org(ROLE_MEMBER));
        assert!(!can_manage_org(ROLE_VIEWER));
    }

    #[test]
    fn test_can_write() {
        assert!(can_write(ROLE_OWNER));
        assert!(can_write(ROLE_ADMIN));
        assert!(can_write(ROLE_MEMBER));
        assert!(!can_write(ROLE_VIEWER));
    }

    #[test]
    fn test_can_delete_org() {
        assert!(can_delete_org(ROLE_OWNER));
        assert!(!can_delete_org(ROLE_ADMIN));
        assert!(!can_delete_org(ROLE_MEMBER));
        assert!(!can_delete_org(ROLE_VIEWER));
    }

    #[test]
    fn test_can_manage_members() {
        assert!(can_manage_members(ROLE_OWNER));
        assert!(can_manage_members(ROLE_ADMIN));
        assert!(!can_manage_members(ROLE_MEMBER));
        assert!(!can_manage_members(ROLE_VIEWER));
    }

    #[test]
    fn test_is_owner() {
        assert!(is_owner(ROLE_OWNER));
        assert!(!is_owner(ROLE_ADMIN));
        assert!(!is_owner(ROLE_MEMBER));
        assert!(!is_owner(ROLE_VIEWER));
    }

    #[test]
    fn test_has_permission() {
        // Owner has all permissions
        assert!(has_permission(ROLE_OWNER, ROLE_OWNER));
        assert!(has_permission(ROLE_OWNER, ROLE_ADMIN));
        assert!(has_permission(ROLE_OWNER, ROLE_MEMBER));
        assert!(has_permission(ROLE_OWNER, ROLE_VIEWER));

        // Admin has admin and below
        assert!(!has_permission(ROLE_ADMIN, ROLE_OWNER));
        assert!(has_permission(ROLE_ADMIN, ROLE_ADMIN));
        assert!(has_permission(ROLE_ADMIN, ROLE_MEMBER));
        assert!(has_permission(ROLE_ADMIN, ROLE_VIEWER));

        // Member has member and below
        assert!(!has_permission(ROLE_MEMBER, ROLE_OWNER));
        assert!(!has_permission(ROLE_MEMBER, ROLE_ADMIN));
        assert!(has_permission(ROLE_MEMBER, ROLE_MEMBER));
        assert!(has_permission(ROLE_MEMBER, ROLE_VIEWER));

        // Viewer only has viewer
        assert!(!has_permission(ROLE_VIEWER, ROLE_OWNER));
        assert!(!has_permission(ROLE_VIEWER, ROLE_ADMIN));
        assert!(!has_permission(ROLE_VIEWER, ROLE_MEMBER));
        assert!(has_permission(ROLE_VIEWER, ROLE_VIEWER));
    }

    #[test]
    fn test_has_permission_invalid_role() {
        assert!(!has_permission("invalid", ROLE_OWNER));
        assert!(!has_permission(ROLE_OWNER, "invalid"));
    }

    #[test]
    fn test_role_hierarchy_order() {
        // Verify the hierarchy is in the correct order
        assert_eq!(ROLE_HIERARCHY[0], ROLE_VIEWER);
        assert_eq!(ROLE_HIERARCHY[1], ROLE_MEMBER);
        assert_eq!(ROLE_HIERARCHY[2], ROLE_ADMIN);
        assert_eq!(ROLE_HIERARCHY[3], ROLE_OWNER);
    }
}
