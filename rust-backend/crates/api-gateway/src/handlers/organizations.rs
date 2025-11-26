//! Organization and Member Management Handlers
//!
//! This module provides REST API handlers for organization CRUD operations
//! and member management (invitations, role updates, removals).
//!
//! # Endpoints
//!
//! ## Organizations
//! - `POST /api/v1/organizations` - Create a new organization
//! - `GET /api/v1/organizations` - List user's organizations (paginated)
//! - `GET /api/v1/organizations/{id}` - Get organization details
//! - `PUT /api/v1/organizations/{id}` - Update organization (admin+)
//! - `DELETE /api/v1/organizations/{id}` - Delete organization (owner only)
//! - `POST /api/v1/organizations/{id}/transfer` - Transfer ownership (owner only)
//!
//! ## Members
//! - `POST /api/v1/organizations/{id}/members` - Add a member (admin+)
//! - `GET /api/v1/organizations/{id}/members` - List members (paginated)
//! - `PUT /api/v1/organizations/{id}/members/{user_id}` - Update role (owner only)
//! - `DELETE /api/v1/organizations/{id}/members/{user_id}` - Remove member (admin+)
//!
//! # Authorization
//!
//! All endpoints require JWT authentication. Organization access is controlled
//! by membership and role:
//!
//! - **viewer**: Read-only access
//! - **member**: Can create/edit resources
//! - **admin**: Can manage settings and members
//! - **owner**: Full control including deletion and ownership transfer
//!
//! # Privacy
//!
//! Member email addresses are masked for privacy unless:
//! - The requester is an owner or admin
//! - The requester is viewing their own email
//!
//! # Security Features
//!
//! - All database operations use parameterized queries (SQL injection prevention)
//! - Ownership transfer uses database transactions for atomicity
//! - Slug uniqueness enforced by database constraints (race condition safe)
//! - Organization membership verified on all operations

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;
use validator::Validate;

use crate::{
    middleware::get_user_id,
    models::{
        can_delete_org, can_manage_members, can_manage_org, is_owner, AddMemberRequest,
        CreateOrganizationRequest, ErrorResponse, MemberResponse, OrganizationResponse,
        OrganizationWithRoleResponse, PaginatedResponse, PaginationMeta, PaginationParams,
        SuccessResponse, TransferOwnershipRequest, UpdateMemberRoleRequest,
        UpdateOrganizationRequest, ROLE_ADMIN, ROLE_OWNER,
    },
    repositories::{MemberRepository, OrganizationRepository, UserRepository},
};

// ============================================================================
// Organization CRUD Handlers
// ============================================================================

/// Create a new organization
///
/// POST /api/v1/organizations
pub async fn create_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateOrganizationRequest>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Create organization - let the database handle uniqueness via constraint
    // This avoids the check-then-insert race condition
    let org = match OrganizationRepository::create(
        &pool,
        &req.name,
        &req.slug,
        req.description.as_deref(),
        &user_id,
        false, // not a personal organization
    )
    .await
    {
        Ok(org) => org,
        Err(e) => {
            // Check if the error is a unique constraint violation (slug already exists)
            let error_string = e.to_string();
            if error_string.contains("duplicate key")
                || error_string.contains("unique constraint")
                || error_string.contains("organizations_slug_key")
            {
                return HttpResponse::Conflict().json(ErrorResponse::new(
                    "conflict",
                    "Organization slug already exists",
                ));
            }
            tracing::error!("Failed to create organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create organization",
            ));
        }
    };

    // Add creator as owner
    if let Err(e) = MemberRepository::add(&pool, &org.id, &user_id, ROLE_OWNER, None).await {
        tracing::error!("Failed to add owner to organization: {}", e);
        // Try to clean up the organization
        let _ = OrganizationRepository::delete(&pool, &org.id).await;
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to create organization",
        ));
    }

    let response = OrganizationResponse::from(org);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List organizations for authenticated user
///
/// GET /api/v1/organizations?limit=20&offset=0
pub async fn list_organizations(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Get total count
    let total = match OrganizationRepository::count_by_user(&pool, &user_id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count organizations: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch organizations",
            ));
        }
    };

    // Get organizations WITH roles in a single optimized query (no N+1)
    let orgs_with_roles = match OrganizationRepository::list_by_user_with_roles(
        &pool,
        &user_id,
        query.limit,
        query.offset,
    )
    .await
    {
        Ok(orgs) => orgs,
        Err(e) => {
            tracing::error!("Failed to list organizations: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch organizations",
            ));
        }
    };

    // Build response (no additional queries needed)
    let org_responses: Vec<OrganizationWithRoleResponse> = orgs_with_roles
        .into_iter()
        .map(|org| OrganizationWithRoleResponse {
            organization: OrganizationResponse::from_with_role(&org),
            my_role: org.my_role,
        })
        .collect();

    let response = PaginatedResponse {
        data: org_responses,
        pagination: PaginationMeta::new(total, query.limit, query.offset),
    };

    HttpResponse::Ok().json(response)
}

/// Get a single organization
///
/// GET /api/v1/organizations/{id}
pub async fn get_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Check membership
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch organization",
            ));
        }
    };

    // Get organization
    let org = match OrganizationRepository::find_by_id(&pool, &org_id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to fetch organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch organization",
            ));
        }
    };

    let response = OrganizationWithRoleResponse {
        organization: OrganizationResponse::from(org),
        my_role: role,
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update an organization
///
/// PUT /api/v1/organizations/{id}
pub async fn update_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateOrganizationRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update organization",
            ));
        }
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to update organization",
        ));
    }

    // Update organization
    let org = match OrganizationRepository::update(
        &pool,
        &org_id,
        req.name.as_deref(),
        req.description.as_ref().map(|d| Some(d.as_str())),
    )
    .await
    {
        Ok(org) => org,
        Err(e) => {
            tracing::error!("Failed to update organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update organization",
            ));
        }
    };

    let response = OrganizationResponse::from(org);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete an organization (owner only, not personal)
///
/// DELETE /api/v1/organizations/{id}
pub async fn delete_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete organization",
            ));
        }
    };

    // Only owner can delete
    if !can_delete_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only the owner can delete an organization",
        ));
    }

    // Check if it's a personal organization
    let org = match OrganizationRepository::find_by_id(&pool, &org_id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to fetch organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete organization",
            ));
        }
    };

    if org.is_personal {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Personal organizations cannot be deleted",
        ));
    }

    // Delete organization
    let deleted = match OrganizationRepository::delete(&pool, &org_id).await {
        Ok(deleted) => deleted,
        Err(e) => {
            tracing::error!("Failed to delete organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete organization",
            ));
        }
    };

    if !deleted {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Organization not found"));
    }

    HttpResponse::NoContent().finish()
}

// ============================================================================
// Member Management Handlers
// ============================================================================

/// Add a member to an organization
///
/// POST /api/v1/organizations/{id}/members
pub async fn add_member(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<AddMemberRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Cannot add as owner
    if is_owner(&req.role) {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot add a member as owner",
        ));
    }

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "Failed to add member"));
        }
    };

    // Check if user can manage members (owner or admin)
    if !can_manage_members(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to add members",
        ));
    }

    // Check if target user exists
    let target_user = match UserRepository::find_by_id(&pool, &req.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "User not found"))
        }
        Err(e) => {
            tracing::error!("Failed to find user: {}", e);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "Failed to add member"));
        }
    };

    // Check if already a member
    match MemberRepository::is_member(&pool, &org_id, &req.user_id).await {
        Ok(true) => {
            return HttpResponse::Conflict().json(ErrorResponse::new(
                "conflict",
                "User is already a member of this organization",
            ))
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "Failed to add member"));
        }
    }

    // Add member
    let member = match MemberRepository::add(
        &pool,
        &org_id,
        &req.user_id,
        &req.role,
        Some(&user_id),
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to add member: {}", e);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "Failed to add member"));
        }
    };

    let response = MemberResponse {
        id: member.id,
        user_id: member.user_id,
        username: target_user.username,
        email: target_user.email,
        role: member.role,
        created_at: member.created_at,
    };

    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List members of an organization
///
/// GET /api/v1/organizations/{id}/members?limit=20&offset=0
pub async fn list_members(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Check membership and get role (needed for email masking decision)
    let requester_role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(role)) => role,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to list members",
            ));
        }
    };

    // Get total count
    let total = match MemberRepository::count(&pool, &org_id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count members: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to list members",
            ));
        }
    };

    // Get members WITH user info in a single optimized query (no N+1)
    let members_with_users =
        match MemberRepository::list_with_users(&pool, &org_id, query.limit, query.offset).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to list members: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to list members",
                ));
            }
        };

    // Determine if emails should be shown (only owner/admin can see full emails)
    let can_see_emails = is_owner(&requester_role) || requester_role == ROLE_ADMIN;

    // Build response with email masking
    let member_responses: Vec<MemberResponse> = members_with_users
        .into_iter()
        .map(|m| MemberResponse {
            id: m.id,
            user_id: m.user_id.clone(),
            username: m.username,
            email: if can_see_emails || m.user_id == user_id {
                m.email // Show full email to admins/owners or to self
            } else {
                mask_email(&m.email) // Mask email for regular members/viewers
            },
            role: m.role,
            created_at: m.created_at,
        })
        .collect();

    let response = PaginatedResponse {
        data: member_responses,
        pagination: PaginationMeta::new(total, query.limit, query.offset),
    };

    HttpResponse::Ok().json(response)
}

/// Mask an email address for privacy (e.g., "john@example.com" -> "j***@e***.com")
fn mask_email(email: &str) -> String {
    if let Some((local, domain)) = email.split_once('@') {
        let masked_local = if local.len() <= 1 {
            "*".to_string()
        } else {
            format!("{}***", local.chars().next().unwrap_or('*'))
        };

        let masked_domain = if let Some((name, ext)) = domain.rsplit_once('.') {
            if name.len() <= 1 {
                format!("***.{}", ext)
            } else {
                format!("{}***.{}", name.chars().next().unwrap_or('*'), ext)
            }
        } else {
            "***".to_string()
        };

        format!("{}@{}", masked_local, masked_domain)
    } else {
        "***@***".to_string()
    }
}

/// Path parameters for member operations
#[derive(Debug, serde::Deserialize)]
pub struct MemberPath {
    pub id: String,
    pub user_id: String,
}

/// Update a member's role (owner only)
///
/// PUT /api/v1/organizations/{id}/members/{user_id}
pub async fn update_member_role(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<MemberPath>,
    req: web::Json<UpdateMemberRoleRequest>,
) -> impl Responder {
    let MemberPath {
        id: org_id,
        user_id: target_user_id,
    } = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Cannot set role to owner
    if is_owner(&req.role) {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot change role to owner",
        ));
    }

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update member",
            ));
        }
    };

    // Only owner can update roles
    if !is_owner(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only the owner can update member roles",
        ));
    }

    // Check target member exists
    let target_role = match MemberRepository::get_role(&pool, &org_id, &target_user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Member not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check target membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update member",
            ));
        }
    };

    // Cannot change owner's role
    if is_owner(&target_role) {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot change the owner's role",
        ));
    }

    // Update role
    let member =
        match MemberRepository::update_role(&pool, &org_id, &target_user_id, &req.role).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to update member role: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to update member",
                ));
            }
        };

    // Get user info
    let target_user = match UserRepository::find_by_id(&pool, &target_user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "User not found"))
        }
        Err(e) => {
            tracing::error!("Failed to find user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update member",
            ));
        }
    };

    let response = MemberResponse {
        id: member.id,
        user_id: member.user_id,
        username: target_user.username,
        email: target_user.email,
        role: member.role,
        created_at: member.created_at,
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Remove a member from an organization (admin+, cannot remove owner)
///
/// DELETE /api/v1/organizations/{id}/members/{user_id}
pub async fn remove_member(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<MemberPath>,
) -> impl Responder {
    let MemberPath {
        id: org_id,
        user_id: target_user_id,
    } = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to remove member",
            ));
        }
    };

    // Check if user can manage members (owner or admin)
    if !can_manage_members(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to remove members",
        ));
    }

    // Check target member exists and get their role
    let target_role = match MemberRepository::get_role(&pool, &org_id, &target_user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Member not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check target membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to remove member",
            ));
        }
    };

    // Cannot remove owner
    if is_owner(&target_role) {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot remove the owner from the organization",
        ));
    }

    // Remove member
    let removed = match MemberRepository::remove(&pool, &org_id, &target_user_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to remove member: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to remove member",
            ));
        }
    };

    if !removed {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Member not found"));
    }

    HttpResponse::NoContent().finish()
}

// ============================================================================
// Ownership Transfer Handler
// ============================================================================

/// Transfer organization ownership to another member (owner only)
///
/// POST /api/v1/organizations/{id}/transfer
///
/// This endpoint allows the current owner to transfer ownership to another
/// member of the organization. The current owner becomes an admin after transfer.
///
/// # Constraints
/// - Only the current owner can initiate a transfer
/// - Personal organizations cannot be transferred
/// - The new owner must already be a member of the organization
pub async fn transfer_ownership(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<TransferOwnershipRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Cannot transfer to yourself
    if req.new_owner_id == user_id {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot transfer ownership to yourself",
        ));
    }

    // Check membership and role - must be owner
    let role = match MemberRepository::get_role(&pool, &org_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to transfer ownership",
            ));
        }
    };

    // Only owner can transfer ownership
    if !is_owner(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only the owner can transfer ownership",
        ));
    }

    // Check that the new owner is a member
    let new_owner_role = match MemberRepository::get_role(&pool, &org_id, &req.new_owner_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "bad_request",
                "New owner must be a member of the organization",
            ))
        }
        Err(e) => {
            tracing::error!("Failed to check new owner membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to transfer ownership",
            ));
        }
    };

    // Warn if transferring to a viewer (unusual but allowed)
    if new_owner_role == "viewer" {
        tracing::info!(
            "Ownership transfer from {} to viewer {} in org {}",
            user_id,
            req.new_owner_id,
            org_id
        );
    }

    // Perform the transfer
    let org = match OrganizationRepository::transfer_ownership(
        &pool,
        &org_id,
        &user_id,
        &req.new_owner_id,
    )
    .await
    {
        Ok(org) => org,
        Err(e) => {
            let error_string = e.to_string();
            if error_string.contains("personal organization") {
                return HttpResponse::BadRequest().json(ErrorResponse::new(
                    "bad_request",
                    "Personal organizations cannot be transferred",
                ));
            }
            tracing::error!("Failed to transfer ownership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to transfer ownership",
            ));
        }
    };

    tracing::info!(
        "Ownership transferred: org={}, from={}, to={}",
        org_id,
        user_id,
        req.new_owner_id
    );

    let response = OrganizationResponse::from(org);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email_standard() {
        assert_eq!(mask_email("john@example.com"), "j***@e***.com");
        assert_eq!(mask_email("alice@domain.org"), "a***@d***.org");
    }

    #[test]
    fn test_mask_email_short_local() {
        assert_eq!(mask_email("j@example.com"), "*@e***.com");
    }

    #[test]
    fn test_mask_email_short_domain() {
        assert_eq!(mask_email("john@e.com"), "j***@***.com");
    }

    #[test]
    fn test_mask_email_invalid_no_at() {
        assert_eq!(mask_email("invalid-email"), "***@***");
    }

    #[test]
    fn test_mask_email_no_domain_extension() {
        assert_eq!(mask_email("john@localhost"), "j***@***");
    }
}
