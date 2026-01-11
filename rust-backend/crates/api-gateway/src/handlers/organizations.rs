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

use crate::{
    handlers::helpers::{
        bad_request, extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
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
/// Creates a new organization and adds the authenticated user as owner.
#[utoipa::path(
    post,
    path = "/api/v1/organizations",
    tag = "Organizations",
    request_body = CreateOrganizationRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Organization created", body = SuccessResponse<OrganizationResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 409, description = "Slug already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateOrganizationRequest>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Start transaction for atomic organization + member creation
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create organization",
            ));
        }
    };

    // Create organization - let the database handle uniqueness via constraint
    // This avoids the check-then-insert race condition
    let org = match OrganizationRepository::create_with_executor(
        &mut *tx,
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
            // Use SQLx database error codes for robustness (PostgreSQL code 23505 = unique_violation)
            let is_unique_violation = e
                .downcast_ref::<sqlx::Error>()
                .and_then(|sqlx_err| sqlx_err.as_database_error())
                .and_then(|db_err| db_err.code())
                .map(|code| code == "23505")
                .unwrap_or(false);

            if is_unique_violation {
                // Slug is unique per user, so this means the user already has an org with this slug
                tracing::info!(
                    slug = %req.slug,
                    user_id = %user_id,
                    "Organization creation failed - user already has org with this slug"
                );
                return HttpResponse::BadRequest().json(ErrorResponse::new(
                    "slug_exists",
                    "You already have an organization with this slug. Please choose a different one.",
                ));
            }
            tracing::error!("Failed to create organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create organization",
            ));
        }
    };

    // Add creator as owner (within the same transaction)
    if let Err(e) =
        MemberRepository::add_with_executor(&mut *tx, &org.id, &user_id, ROLE_OWNER, None).await
    {
        tracing::error!("Failed to add owner to organization: {}", e);
        // Transaction will be rolled back automatically on drop
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to create organization",
        ));
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit organization creation: {}", e);
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
/// Returns paginated list of organizations the user is a member of.
#[utoipa::path(
    get,
    path = "/api/v1/organizations",
    tag = "Organizations",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum items per page (default: 20)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (default: 0)")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of organizations", body = PaginatedResponse<OrganizationWithRoleResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn list_organizations(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Execute count and list in parallel for better performance
    let (total_result, orgs_result) = tokio::join!(
        OrganizationRepository::count_by_user(&pool, &user_id),
        OrganizationRepository::list_by_user_with_roles(&pool, &user_id, query.limit, query.offset)
    );

    // Handle count result
    let total = match handle_db_error(total_result, "count organizations") {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Handle list result (optimized query with roles - no N+1)
    let orgs_with_roles = match handle_db_error(orgs_result, "list organizations") {
        Ok(orgs) => orgs,
        Err(resp) => return resp,
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
/// Returns organization details including the user's role.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organization details", body = SuccessResponse<OrganizationWithRoleResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Execute membership check and organization fetch in parallel for better performance
    let (role_result, org_result) = tokio::join!(
        MemberRepository::get_role(&pool, &org_id, &user_id),
        OrganizationRepository::find_by_id(&pool, &org_id)
    );

    // Handle role result
    let role = match handle_db_error(role_result, "check membership") {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Handle organization result
    let org = match handle_db_error(org_result, "fetch organization") {
        Ok(Some(org)) => org,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    let response = OrganizationWithRoleResponse {
        organization: OrganizationResponse::from(org),
        my_role: role,
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update an organization
///
/// Updates organization details. Requires admin or owner role.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organization updated", body = SuccessResponse<OrganizationResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateOrganizationRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to update organization");
    }

    // Update organization
    let org = match handle_db_error(
        OrganizationRepository::update(
            &pool,
            &org_id,
            req.name.as_deref(),
            req.description.as_ref().map(|d| Some(d.as_str())),
        )
        .await,
        "update organization",
    ) {
        Ok(org) => org,
        Err(resp) => return resp,
    };

    let response = OrganizationResponse::from(org);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete an organization (owner only, not personal)
///
/// Permanently deletes an organization. Personal organizations cannot be deleted.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Organization deleted"),
        (status = 400, description = "Cannot delete personal organization", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Only owner can delete", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn delete_organization(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Only owner can delete
    if !can_delete_org(&role) {
        return forbidden("Only the owner can delete an organization");
    }

    // Check if it's a personal organization
    let org = match handle_db_error(
        OrganizationRepository::find_by_id(&pool, &org_id).await,
        "fetch organization",
    ) {
        Ok(Some(org)) => org,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    if org.is_personal {
        return bad_request("Personal organizations cannot be deleted");
    }

    // Delete organization
    let deleted = match handle_db_error(
        OrganizationRepository::delete(&pool, &org_id).await,
        "delete organization",
    ) {
        Ok(deleted) => deleted,
        Err(resp) => return resp,
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
/// Adds a user to the organization. Requires admin or owner role.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/members",
    tag = "Members",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = AddMemberRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Member added", body = SuccessResponse<MemberResponse>),
        (status = 400, description = "Validation error or invalid role", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Organization or user not found", body = ErrorResponse),
        (status = 409, description = "User already a member", body = ErrorResponse)
    )
)]
pub async fn add_member(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<AddMemberRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Cannot add as owner
    if is_owner(&req.role) {
        return bad_request("Cannot add a member as owner");
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Check if user can manage members (owner or admin)
    if !can_manage_members(&role) {
        return forbidden("Insufficient permissions to add members");
    }

    // Check if target user exists
    let target_user = match handle_db_error(
        UserRepository::find_by_id(&pool, &req.user_id).await,
        "find user",
    ) {
        Ok(Some(u)) => u,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "User not found"))
        }
        Err(resp) => return resp,
    };

    // Check if already a member
    match handle_db_error(
        MemberRepository::is_member(&pool, &org_id, &req.user_id).await,
        "check membership",
    ) {
        Ok(true) => {
            return HttpResponse::Conflict().json(ErrorResponse::new(
                "conflict",
                "User is already a member of this organization",
            ))
        }
        Ok(false) => {}
        Err(resp) => return resp,
    }

    // Add member
    let member = match handle_db_error(
        MemberRepository::add(&pool, &org_id, &req.user_id, &req.role, Some(&user_id)).await,
        "add member",
    ) {
        Ok(m) => m,
        Err(resp) => return resp,
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
/// Returns paginated list of organization members. Emails are masked for privacy.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/members",
    tag = "Members",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of members", body = PaginatedResponse<MemberResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_members(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Execute membership check, count, and list in parallel for better performance
    let (role_result, total_result, members_result) = tokio::join!(
        MemberRepository::get_role(&pool, &org_id, &user_id),
        MemberRepository::count(&pool, &org_id),
        MemberRepository::list_with_users(&pool, &org_id, query.limit, query.offset)
    );

    // Handle role result (needed for email masking decision)
    let requester_role = match handle_db_error(role_result, "check membership") {
        Ok(Some(role)) => role,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Handle count result
    let total = match handle_db_error(total_result, "count members") {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Handle list result (optimized query with user info - no N+1)
    let members_with_users = match handle_db_error(members_result, "list members") {
        Ok(m) => m,
        Err(resp) => return resp,
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
/// Changes a member's role within the organization. Only the owner can update roles.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/members/{user_id}",
    tag = "Members",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to update")
    ),
    request_body = UpdateMemberRoleRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role updated", body = SuccessResponse<MemberResponse>),
        (status = 400, description = "Invalid role change", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Only owner can update roles", body = ErrorResponse),
        (status = 404, description = "Organization or member not found", body = ErrorResponse)
    )
)]
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
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Cannot set role to owner
    if is_owner(&req.role) {
        return bad_request("Cannot change role to owner");
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Only owner can update roles
    if !is_owner(&role) {
        return forbidden("Only the owner can update member roles");
    }

    // Check target member exists
    let target_role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &target_user_id).await,
        "check target membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Member not found"))
        }
        Err(resp) => return resp,
    };

    // Cannot change owner's role
    if is_owner(&target_role) {
        return bad_request("Cannot change the owner's role");
    }

    // Update role
    let member = match handle_db_error(
        MemberRepository::update_role(&pool, &org_id, &target_user_id, &req.role).await,
        "update member role",
    ) {
        Ok(m) => m,
        Err(resp) => return resp,
    };

    // Get user info
    let target_user = match handle_db_error(
        UserRepository::find_by_id(&pool, &target_user_id).await,
        "find user",
    ) {
        Ok(Some(u)) => u,
        Ok(None) => {
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "User not found"))
        }
        Err(resp) => return resp,
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
/// Removes a member from the organization. The owner cannot be removed.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{id}/members/{user_id}",
    tag = "Members",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to remove")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Member removed"),
        (status = 400, description = "Cannot remove owner", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Organization or member not found", body = ErrorResponse)
    )
)]
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
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Check if user can manage members (owner or admin)
    if !can_manage_members(&role) {
        return forbidden("Insufficient permissions to remove members");
    }

    // Check target member exists and get their role
    let target_role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &target_user_id).await,
        "check target membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Member not found"))
        }
        Err(resp) => return resp,
    };

    // Cannot remove owner
    if is_owner(&target_role) {
        return bad_request("Cannot remove the owner from the organization");
    }

    // Remove member
    let removed = match handle_db_error(
        MemberRepository::remove(&pool, &org_id, &target_user_id).await,
        "remove member",
    ) {
        Ok(r) => r,
        Err(resp) => return resp,
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
/// Transfers ownership to another member. Personal organizations cannot be transferred.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/transfer",
    tag = "Organizations",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = TransferOwnershipRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Ownership transferred", body = SuccessResponse<OrganizationResponse>),
        (status = 400, description = "Cannot transfer personal organization or to self", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Only owner can transfer ownership", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn transfer_ownership(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<TransferOwnershipRequest>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Cannot transfer to yourself
    if req.new_owner_id == user_id {
        return bad_request("Cannot transfer ownership to yourself");
    }

    // Check membership and role - must be owner
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Only owner can transfer ownership
    if !is_owner(&role) {
        return forbidden("Only the owner can transfer ownership");
    }

    // Check that the new owner is a member
    let new_owner_role = match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &req.new_owner_id).await,
        "check new owner membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "bad_request",
                "New owner must be a member of the organization",
            ))
        }
        Err(resp) => return resp,
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
            // Check for personal organization constraint violation
            // The database function raises an exception with this specific message
            let is_personal_org_error = e
                .downcast_ref::<sqlx::Error>()
                .and_then(|sqlx_err| sqlx_err.as_database_error())
                .map(|db_err| db_err.message().contains("personal organization"))
                .unwrap_or(false);

            if is_personal_org_error {
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
