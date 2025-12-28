//! API Key Management Handlers
//!
//! This module provides REST API handlers for API key CRUD operations
//! including creation, listing, revocation, and rotation.
//!
//! # Endpoints
//!
//! - `POST /api/v1/api-keys` - Create a new API key (admin+)
//! - `GET /api/v1/api-keys` - List organization's API keys (paginated)
//! - `GET /api/v1/api-keys/{id}` - Get API key details (masked)
//! - `DELETE /api/v1/api-keys/{id}` - Revoke an API key (admin+)
//! - `POST /api/v1/api-keys/{id}/rotate` - Rotate an API key (admin+)
//!
//! # Authorization
//!
//! All endpoints require JWT authentication. Organization access is controlled
//! by membership and role:
//!
//! - **viewer**: Can view (masked) key information
//! - **member**: Can view (masked) key information
//! - **admin**: Can create, revoke, and rotate keys
//! - **owner**: Full control over keys
//!
//! # Security
//!
//! - Full API key is shown ONLY ONCE at creation time
//! - Keys are stored as Argon2id hashes (never plaintext)
//! - All operations are logged to audit trail
//! - Revoked keys are kept for audit purposes

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use shared::DbPool;

use crate::{
    handlers::helpers::{
        bad_request, extract_request_context, extract_user_id_or_unauthorized, forbidden,
        handle_db_error, handle_error, require_found, validate_request,
    },
    models::{
        can_manage_org, ApiKeyCreatedResponse, ApiKeyListResponse, ApiKeyResponse,
        CreateApiKeyRequest, ErrorResponse, PaginationParams, RevokeApiKeyRequest,
        RotateApiKeyRequest, RotateApiKeyResponse, SuccessResponse, UpdateApiKeyRequest,
    },
    repositories::{ApiKeyAuditRepository, ApiKeyRepository, MemberRepository},
    services::ApiKeyService,
};

// ============================================================================
// API Key Handlers
// ============================================================================

/// Create a new API key
///
/// POST /api/v1/api-keys
///
/// Returns the full API key ONLY at creation time. The key will never be
/// shown again - the client MUST save it immediately.
#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    params(
        ("organization_id" = String, Query, description = "Organization ID")
    ),
    request_body = CreateApiKeyRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "API key created - full key shown once", body = SuccessResponse<ApiKeyCreatedResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn create_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateApiKeyRequest>,
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

    // Get organization_id from query param or request body
    // For now, we'll require it in the query string
    let org_id = match req_http.match_info().get("org_id") {
        Some(id) => id.to_string(),
        None => {
            // Try to get from query string
            match web::Query::<OrgIdQuery>::from_query(req_http.query_string()) {
                Ok(q) => q.organization_id.clone(),
                Err(_) => {
                    return HttpResponse::BadRequest().json(ErrorResponse::new(
                        "bad_request",
                        "organization_id query parameter is required",
                    ))
                }
            }
        }
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

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to create API keys");
    }

    // Generate the API key
    let api_key_service = ApiKeyService::new();
    let generated = match handle_error(
        api_key_service.generate_key(&req.environment),
        "generate API key",
    ) {
        Ok(g) => g,
        Err(resp) => return resp,
    };

    // Store the key in database
    let key = match ApiKeyRepository::create(
        &pool,
        &org_id,
        &generated.hash,
        &req.name,
        &generated.prefix,
        &req.environment,
        &req.key_type,
        &req.permissions,
        req.rate_limit_override,
        req.expires_at,
        &user_id,
    )
    .await
    {
        Ok(k) => k,
        Err(e) => {
            let error_string = e.to_string();
            if error_string.contains("duplicate key") || error_string.contains("unique constraint")
            {
                // Extremely rare: prefix collision. Retry would be needed in production.
                tracing::error!("API key prefix collision: {}", generated.prefix);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Key generation failed, please retry",
                ));
            }
            tracing::error!("Failed to store API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create API key",
            ));
        }
    };

    // Log the creation event
    let ctx = extract_request_context(&req_http);
    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key.id),
        &org_id,
        "created",
        ctx.ip_str(),
        ctx.user_agent_str(),
        Some(ctx.endpoint_str()),
        Some(&user_id),
        Some(serde_json::json!({
            "name": req.name,
            "environment": req.environment,
            "key_type": req.key_type,
        })),
    )
    .await
    {
        tracing::warn!("Failed to log API key creation: {}", e);
        // Don't fail the request for audit log failures
    }

    // Return the full key - THIS IS THE ONLY TIME IT WILL BE SHOWN
    let response = ApiKeyCreatedResponse {
        id: key.id,
        key: generated.key, // Full key - never shown again
        name: key.name,
        prefix: key.prefix,
        environment: key.environment,
        key_type: key.key_type,
        permissions: req.permissions.clone(),
        created_at: key.created_at,
        expires_at: key.expires_at,
    };

    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List API keys for an organization
///
/// GET /api/v1/api-keys?organization_id=xxx&limit=20&offset=0
#[utoipa::path(
    get,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    params(
        ("organization_id" = String, Query, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip"),
        ("include_revoked" = Option<bool>, Query, description = "Include revoked keys")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of API keys (masked)", body = ApiKeyListResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_api_keys(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<ApiKeyListQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate pagination
    let pagination = PaginationParams {
        limit: query.limit.unwrap_or(20),
        offset: query.offset.unwrap_or(0),
    };
    if let Err(e) = pagination.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Check membership
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &query.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // All members can view keys (masked)
    let _ = role; // Role checked above for membership

    // Get total count
    let include_revoked = query.include_revoked.unwrap_or(false);
    let total = match handle_db_error(
        ApiKeyRepository::count_by_organization(&pool, &query.organization_id, include_revoked)
            .await,
        "count API keys",
    ) {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Get keys
    let keys = match handle_db_error(
        ApiKeyRepository::list_by_organization(
            &pool,
            &query.organization_id,
            include_revoked,
            pagination.limit,
            pagination.offset,
        )
        .await,
        "list API keys",
    ) {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Convert to response (masked - no key_hash exposed)
    let key_responses: Vec<ApiKeyResponse> = keys.into_iter().map(ApiKeyResponse::from).collect();

    let response = ApiKeyListResponse {
        items: key_responses,
        total,
        page: (pagination.offset / pagination.limit) + 1,
        page_size: pagination.limit,
        total_pages: (total + pagination.limit - 1) / pagination.limit,
    };

    HttpResponse::Ok().json(response)
}

/// Get API key details
///
/// GET /api/v1/api-keys/{id}
#[utoipa::path(
    get,
    path = "/api/v1/api-keys/{id}",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "API Key ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "API key details (masked)", body = SuccessResponse<ApiKeyResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "API key not found", body = ErrorResponse)
    )
)]
pub async fn get_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let key_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get the key first to determine org_id
    let key = match handle_db_error(
        ApiKeyRepository::find_by_id(&pool, &key_id).await,
        "fetch API key",
    ) {
        Ok(Some(k)) => k,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check membership
    match handle_db_error(
        MemberRepository::get_role(&pool, &key.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    }

    let response = ApiKeyResponse::from(key);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Revoke an API key
///
/// DELETE /api/v1/api-keys/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/api-keys/{id}",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "API Key ID")
    ),
    request_body(content = Option<RevokeApiKeyRequest>, description = "Optional revocation reason"),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "API key revoked", body = SuccessResponse<ApiKeyResponse>),
        (status = 400, description = "API key already revoked", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "API key not found", body = ErrorResponse)
    )
)]
pub async fn revoke_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: Option<web::Json<RevokeApiKeyRequest>>,
) -> impl Responder {
    let key_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get the key first to determine org_id
    let key = match handle_db_error(
        ApiKeyRepository::find_by_id(&pool, &key_id).await,
        "fetch API key",
    ) {
        Ok(Some(k)) => k,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if already revoked
    if key.revoked_at.is_some() {
        return bad_request("API key is already revoked");
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &key.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to revoke API keys");
    }

    // Validate reason if provided
    let reason = match &req {
        Some(r) => {
            if let Err(resp) = validate_request(&**r) {
                return resp;
            }
            r.reason.as_deref()
        }
        None => None,
    };

    // Revoke the key
    let revoked_key = match handle_db_error(
        ApiKeyRepository::revoke(&pool, &key_id, &user_id, reason).await,
        "revoke API key",
    ) {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Log the revocation event
    let ctx = extract_request_context(&req_http);
    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key_id),
        &key.organization_id,
        "revoked",
        ctx.ip_str(),
        ctx.user_agent_str(),
        Some(ctx.endpoint_str()),
        Some(&user_id),
        Some(serde_json::json!({
            "reason": reason,
        })),
    )
    .await
    {
        tracing::warn!("Failed to log API key revocation: {}", e);
    }

    let response = ApiKeyResponse::from(revoked_key);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Rotate an API key (revoke old, create new)
///
/// POST /api/v1/api-keys/{id}/rotate
#[utoipa::path(
    post,
    path = "/api/v1/api-keys/{id}/rotate",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "API Key ID to rotate")
    ),
    request_body(content = Option<RotateApiKeyRequest>, description = "Optional new name and expiration"),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "API key rotated - new key shown once", body = SuccessResponse<RotateApiKeyResponse>),
        (status = 400, description = "Cannot rotate revoked key", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "API key not found", body = ErrorResponse)
    )
)]
pub async fn rotate_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: Option<web::Json<RotateApiKeyRequest>>,
) -> impl Responder {
    let old_key_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate request if provided
    if let Some(ref r) = req {
        if let Err(resp) = validate_request(&**r) {
            return resp;
        }
    }

    // SECURITY: Start transaction FIRST, then use SELECT FOR UPDATE
    // This prevents TOCTOU race condition where two concurrent requests
    // could both pass the "is revoked" check before either completes
    let mut tx = match handle_db_error(pool.begin().await, "start transaction") {
        Ok(tx) => tx,
        Err(resp) => return resp,
    };

    // Get the old key WITH ROW LOCK to prevent concurrent modifications
    let old_key = match handle_db_error(
        ApiKeyRepository::find_by_id_for_update(&mut *tx, &old_key_id).await,
        "fetch API key",
    ) {
        Ok(Some(k)) => k,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if already revoked (now safe - we have an exclusive lock)
    if old_key.revoked_at.is_some() {
        return bad_request("Cannot rotate a revoked API key");
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &old_key.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to rotate API keys");
    }

    // Generate new key
    let api_key_service = ApiKeyService::new();
    let generated = match handle_error(
        api_key_service.generate_key(&old_key.environment),
        "generate API key",
    ) {
        Ok(g) => g,
        Err(resp) => return resp,
    };

    // Get permissions from old key
    let permissions: Vec<String> = serde_json::from_value(old_key.permissions.clone())
        .unwrap_or_else(|_| vec!["read".to_string()]);

    // Determine new name
    let new_name = match &req {
        Some(r) => r.name.clone().unwrap_or_else(|| old_key.name.clone()),
        None => old_key.name.clone(),
    };

    // Determine new expiration
    let new_expires_at = match &req {
        Some(r) => r.expires_at,
        None => old_key.expires_at,
    };

    // Revoke old key (within the same transaction that holds the lock)
    if let Err(resp) = handle_db_error(
        ApiKeyRepository::revoke_with_executor(&mut *tx, &old_key_id, &user_id, Some("rotated"))
            .await,
        "revoke old API key",
    ) {
        return resp;
    }

    // Create new key
    let new_key = match handle_db_error(
        ApiKeyRepository::create_with_executor(
            &mut *tx,
            &old_key.organization_id,
            &generated.hash,
            &new_name,
            &generated.prefix,
            &old_key.environment,
            &old_key.key_type,
            &permissions,
            old_key.rate_limit_override,
            new_expires_at,
            &user_id,
        )
        .await,
        "create new API key",
    ) {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Log the rotation event
    let ctx = extract_request_context(&req_http);
    if let Err(e) = ApiKeyAuditRepository::log_with_executor(
        &mut *tx,
        Some(&new_key.id),
        &old_key.organization_id,
        "rotated",
        ctx.ip_str(),
        ctx.user_agent_str(),
        Some(ctx.endpoint_str()),
        Some(&user_id),
        Some(serde_json::json!({
            "old_key_id": old_key_id,
            "new_key_id": new_key.id,
        })),
    )
    .await
    {
        tracing::warn!("Failed to log API key rotation: {}", e);
        // Don't fail for audit log issues
    }

    // Commit transaction
    if let Err(resp) = handle_db_error(tx.commit().await, "commit transaction") {
        return resp;
    }

    // Return the new key
    let response = RotateApiKeyResponse {
        id: new_key.id,
        key: generated.key, // Full key - never shown again
        prefix: new_key.prefix,
        old_key_id,
        old_key_revoked_at: Utc::now(),
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

// ============================================================================
// Organization-scoped API Key Handlers
// ============================================================================

/// List API keys for an organization (nested under /organizations/{id}/api-keys)
///
/// GET /api/v1/organizations/{id}/api-keys
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/api-keys",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip"),
        ("include_revoked" = Option<bool>, Query, description = "Include revoked keys")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of API keys (masked)", body = ApiKeyListResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_org_api_keys(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    query: web::Query<OrgApiKeyListQuery>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate pagination
    let pagination = PaginationParams {
        limit: query.limit.unwrap_or(20),
        offset: query.offset.unwrap_or(0),
    };
    if let Err(e) = pagination.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Check membership
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

    // All members can view keys (masked)
    let _ = role; // Role checked above for membership

    // Get total count
    let include_revoked = query.include_revoked.unwrap_or(false);
    let total = match handle_db_error(
        ApiKeyRepository::count_by_organization(&pool, &org_id, include_revoked).await,
        "count API keys",
    ) {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Get keys
    let keys = match handle_db_error(
        ApiKeyRepository::list_by_organization(
            &pool,
            &org_id,
            include_revoked,
            pagination.limit,
            pagination.offset,
        )
        .await,
        "list API keys",
    ) {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Convert to response (masked - no key_hash exposed)
    let key_responses: Vec<ApiKeyResponse> = keys.into_iter().map(ApiKeyResponse::from).collect();

    let response = ApiKeyListResponse {
        items: key_responses,
        total,
        page: (pagination.offset / pagination.limit) + 1,
        page_size: pagination.limit,
        total_pages: (total + pagination.limit - 1) / pagination.limit,
    };

    HttpResponse::Ok().json(response)
}

/// Create API key for an organization (nested under /organizations/{id}/api-keys)
///
/// POST /api/v1/organizations/{id}/api-keys
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/api-keys",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    request_body = CreateApiKeyRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "API key created - full key shown once", body = SuccessResponse<ApiKeyCreatedResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn create_org_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<CreateApiKeyRequest>,
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
        return forbidden("Insufficient permissions to create API keys");
    }

    // Generate the API key
    let api_key_service = ApiKeyService::new();
    let generated = match handle_error(
        api_key_service.generate_key(&req.environment),
        "generate API key",
    ) {
        Ok(g) => g,
        Err(resp) => return resp,
    };

    // Store the key in database
    let key = match ApiKeyRepository::create(
        &pool,
        &org_id,
        &generated.hash,
        &req.name,
        &generated.prefix,
        &req.environment,
        &req.key_type,
        &req.permissions,
        req.rate_limit_override,
        req.expires_at,
        &user_id,
    )
    .await
    {
        Ok(k) => k,
        Err(e) => {
            let error_string = e.to_string();
            if error_string.contains("duplicate key") || error_string.contains("unique constraint")
            {
                tracing::error!("API key prefix collision: {}", generated.prefix);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Key generation failed, please retry",
                ));
            }
            tracing::error!("Failed to store API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create API key",
            ));
        }
    };

    // Log the creation event
    let ctx = extract_request_context(&req_http);
    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key.id),
        &org_id,
        "created",
        ctx.ip_str(),
        ctx.user_agent_str(),
        Some(ctx.endpoint_str()),
        Some(&user_id),
        Some(serde_json::json!({
            "name": req.name,
            "environment": req.environment,
            "key_type": req.key_type,
        })),
    )
    .await
    {
        tracing::warn!("Failed to log API key creation: {}", e);
    }

    // Return the full key - THIS IS THE ONLY TIME IT WILL BE SHOWN
    let response = ApiKeyCreatedResponse {
        id: key.id,
        key: generated.key,
        name: key.name,
        prefix: key.prefix,
        environment: key.environment,
        key_type: key.key_type,
        permissions: req.permissions.clone(),
        created_at: key.created_at,
        expires_at: key.expires_at,
    };

    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// Update an API key (name, expiration)
///
/// PATCH /api/v1/api-keys/{id}
#[utoipa::path(
    patch,
    path = "/api/v1/api-keys/{id}",
    tag = "API Keys",
    params(
        ("id" = String, Path, description = "API Key ID")
    ),
    request_body = UpdateApiKeyRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "API key updated", body = SuccessResponse<ApiKeyResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "API key not found", body = ErrorResponse)
    )
)]
pub async fn update_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateApiKeyRequest>,
) -> impl Responder {
    let key_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get the key first to determine org_id
    let key = match handle_db_error(
        ApiKeyRepository::find_by_id(&pool, &key_id).await,
        "fetch API key",
    ) {
        Ok(Some(k)) => k,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if revoked
    if key.revoked_at.is_some() {
        return bad_request("Cannot update a revoked API key");
    }

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &key.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => return require_found::<()>(None, "API key").unwrap_err(),
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to update API keys");
    }

    // Update the key
    let updated_key = match handle_db_error(
        ApiKeyRepository::update(&pool, &key_id, req.name.as_deref(), req.expires_at).await,
        "update API key",
    ) {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Log the update event
    let ctx = extract_request_context(&req_http);
    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key_id),
        &key.organization_id,
        "updated",
        ctx.ip_str(),
        ctx.user_agent_str(),
        Some(ctx.endpoint_str()),
        Some(&user_id),
        Some(serde_json::json!({
            "name": req.name,
            "expires_at": req.expires_at,
        })),
    )
    .await
    {
        tracing::warn!("Failed to log API key update: {}", e);
    }

    let response = ApiKeyResponse::from(updated_key);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

// ============================================================================
// Query Parameter Structs
// ============================================================================

/// Query parameters for organization ID
#[derive(Debug, serde::Deserialize)]
pub struct OrgIdQuery {
    pub organization_id: String,
}

/// Query parameters for listing API keys (org-scoped, org_id from path)
#[derive(Debug, serde::Deserialize)]
pub struct OrgApiKeyListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_revoked: Option<bool>,
}

/// Query parameters for listing API keys
#[derive(Debug, serde::Deserialize)]
pub struct ApiKeyListQuery {
    pub organization_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_revoked: Option<bool>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ApiKeyCreatedResponse, ApiKeyListResponse, ApiKeyResponse, CreateApiKeyRequest,
        RevokeApiKeyRequest, RotateApiKeyRequest, RotateApiKeyResponse,
    };
    use chrono::Utc;

    // ========================================================================
    // Query Parameter Tests
    // ========================================================================

    #[test]
    fn test_org_id_query_deserialize() {
        let query_string = "organization_id=org_123";
        let query: OrgIdQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert_eq!(query.organization_id, "org_123");
    }

    #[test]
    fn test_api_key_list_query_deserialize_minimal() {
        let query_string = "organization_id=org_123";
        let query: ApiKeyListQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert_eq!(query.organization_id, "org_123");
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
        assert!(query.include_revoked.is_none());
    }

    #[test]
    fn test_api_key_list_query_deserialize_full() {
        let query_string = "organization_id=org_456&limit=50&offset=100&include_revoked=true";
        let query: ApiKeyListQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert_eq!(query.organization_id, "org_456");
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(100));
        assert_eq!(query.include_revoked, Some(true));
    }

    #[test]
    fn test_api_key_list_query_deserialize_partial() {
        let query_string = "organization_id=org_789&limit=25";
        let query: ApiKeyListQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert_eq!(query.organization_id, "org_789");
        assert_eq!(query.limit, Some(25));
        assert!(query.offset.is_none());
        assert!(query.include_revoked.is_none());
    }

    // ========================================================================
    // Response Serialization Tests
    // ========================================================================

    #[test]
    fn test_api_key_created_response_serialize() {
        let response = ApiKeyCreatedResponse {
            id: "key_123".to_string(),
            key: "sk_live_abc123xyz456".to_string(),
            name: "Production Key".to_string(),
            prefix: "sk_live_abc123".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string(), "write".to_string()],
            created_at: Utc::now(),
            expires_at: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"key_123\""));
        assert!(json.contains("\"key\":\"sk_live_abc123xyz456\""));
        assert!(json.contains("\"environment\":\"live\""));
        assert!(json.contains("\"permissions\":[\"read\",\"write\"]"));
    }

    #[test]
    fn test_api_key_response_serialize() {
        let response = ApiKeyResponse {
            id: "key_456".to_string(),
            name: "Test Key".to_string(),
            prefix: "sk_test_xyz789".to_string(),
            environment: "test".to_string(),
            key_type: "restricted".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: Some(500),
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            created_by: "user_123".to_string(),
            is_revoked: false,
            revoked_at: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"key_456\""));
        assert!(json.contains("\"prefix\":\"sk_test_xyz789\""));
        assert!(json.contains("\"rate_limit_override\":500"));
        assert!(json.contains("\"is_revoked\":false"));
        // Ensure the full key is NOT in the response (security check)
        assert!(!json.contains("\"key\":"));
    }

    #[test]
    fn test_api_key_response_revoked_serialize() {
        let now = Utc::now();
        let response = ApiKeyResponse {
            id: "key_revoked".to_string(),
            name: "Revoked Key".to_string(),
            prefix: "sk_live_revoked".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            last_used_at: Some(now),
            expires_at: None,
            created_at: now,
            created_by: "user_456".to_string(),
            is_revoked: true,
            revoked_at: Some(now),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"is_revoked\":true"));
        assert!(json.contains("\"revoked_at\":"));
    }

    #[test]
    fn test_rotate_api_key_response_serialize() {
        let response = RotateApiKeyResponse {
            id: "new_key_id".to_string(),
            key: "sk_live_new_secret_key".to_string(),
            prefix: "sk_live_new_secr".to_string(),
            old_key_id: "old_key_id".to_string(),
            old_key_revoked_at: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"new_key_id\""));
        assert!(json.contains("\"key\":\"sk_live_new_secret_key\""));
        assert!(json.contains("\"old_key_id\":\"old_key_id\""));
    }

    #[test]
    fn test_api_key_list_response_serialize() {
        let response = ApiKeyListResponse {
            items: vec![
                ApiKeyResponse {
                    id: "key_1".to_string(),
                    name: "Key 1".to_string(),
                    prefix: "sk_live_key1pre".to_string(),
                    environment: "live".to_string(),
                    key_type: "standard".to_string(),
                    permissions: vec!["read".to_string()],
                    rate_limit_override: None,
                    last_used_at: None,
                    expires_at: None,
                    created_at: Utc::now(),
                    created_by: "user_1".to_string(),
                    is_revoked: false,
                    revoked_at: None,
                },
                ApiKeyResponse {
                    id: "key_2".to_string(),
                    name: "Key 2".to_string(),
                    prefix: "sk_test_key2pre".to_string(),
                    environment: "test".to_string(),
                    key_type: "admin".to_string(),
                    permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
                    rate_limit_override: Some(1000),
                    last_used_at: Some(Utc::now()),
                    expires_at: None,
                    created_at: Utc::now(),
                    created_by: "user_2".to_string(),
                    is_revoked: false,
                    revoked_at: None,
                },
            ],
            total: 2,
            page: 1,
            page_size: 20,
            total_pages: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"page_size\":20"));
        assert!(json.contains("\"total_pages\":1"));
        assert!(json.contains("\"key_1\""));
        assert!(json.contains("\"key_2\""));
    }

    // ========================================================================
    // Request Deserialization Tests
    // ========================================================================

    #[test]
    fn test_create_api_key_request_deserialize() {
        let json = r#"{
            "name": "My API Key",
            "environment": "live",
            "key_type": "standard",
            "permissions": ["read", "write"],
            "rate_limit_override": 500
        }"#;

        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "My API Key");
        assert_eq!(req.environment, "live");
        assert_eq!(req.key_type, "standard");
        assert_eq!(req.permissions, vec!["read", "write"]);
        assert_eq!(req.rate_limit_override, Some(500));
    }

    #[test]
    fn test_create_api_key_request_deserialize_with_expiry() {
        let json = r#"{
            "name": "Expiring Key",
            "environment": "test",
            "key_type": "restricted",
            "permissions": ["read"],
            "expires_at": "2025-12-31T23:59:59Z"
        }"#;

        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Expiring Key");
        assert!(req.expires_at.is_some());
    }

    #[test]
    fn test_rotate_api_key_request_deserialize() {
        let json = r#"{
            "name": "Rotated Key Name",
            "expires_at": "2026-01-01T00:00:00Z"
        }"#;

        let req: RotateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("Rotated Key Name".to_string()));
        assert!(req.expires_at.is_some());
    }

    #[test]
    fn test_rotate_api_key_request_deserialize_empty() {
        let json = r#"{}"#;

        let req: RotateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.expires_at.is_none());
    }

    #[test]
    fn test_revoke_api_key_request_deserialize() {
        let json = r#"{
            "reason": "Compromised key"
        }"#;

        let req: RevokeApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.reason, Some("Compromised key".to_string()));
    }

    #[test]
    fn test_revoke_api_key_request_deserialize_empty() {
        let json = r#"{}"#;

        let req: RevokeApiKeyRequest = serde_json::from_str(json).unwrap();
        assert!(req.reason.is_none());
    }

    // ========================================================================
    // Permission Logic Tests
    // ========================================================================

    #[test]
    fn test_can_manage_org_owner() {
        assert!(can_manage_org("owner"));
    }

    #[test]
    fn test_can_manage_org_admin() {
        assert!(can_manage_org("admin"));
    }

    #[test]
    fn test_can_manage_org_member() {
        assert!(!can_manage_org("member"));
    }

    #[test]
    fn test_can_manage_org_viewer() {
        assert!(!can_manage_org("viewer"));
    }

    #[test]
    fn test_can_manage_org_unknown_role() {
        assert!(!can_manage_org("superuser"));
        assert!(!can_manage_org(""));
    }

    // ========================================================================
    // Pagination Tests
    // ========================================================================

    #[test]
    fn test_pagination_calculation() {
        // Test page calculation logic from list_api_keys
        let offset = 0i64;
        let limit = 20i64;
        let page = (offset / limit) + 1;
        assert_eq!(page, 1);

        let offset = 20i64;
        let page = (offset / limit) + 1;
        assert_eq!(page, 2);

        let offset = 40i64;
        let page = (offset / limit) + 1;
        assert_eq!(page, 3);
    }

    #[test]
    fn test_total_pages_calculation() {
        // Test total pages calculation logic from list_api_keys
        let limit = 20i64;

        // 0 items = 0 pages
        let total = 0i64;
        let total_pages = (total + limit - 1) / limit;
        assert_eq!(total_pages, 0);

        // 1 item = 1 page
        let total = 1i64;
        let total_pages = (total + limit - 1) / limit;
        assert_eq!(total_pages, 1);

        // 20 items = 1 page
        let total = 20i64;
        let total_pages = (total + limit - 1) / limit;
        assert_eq!(total_pages, 1);

        // 21 items = 2 pages
        let total = 21i64;
        let total_pages = (total + limit - 1) / limit;
        assert_eq!(total_pages, 2);

        // 100 items = 5 pages
        let total = 100i64;
        let total_pages = (total + limit - 1) / limit;
        assert_eq!(total_pages, 5);
    }

    // ========================================================================
    // Error Response Tests
    // ========================================================================

    #[test]
    fn test_error_response_format() {
        use crate::models::ErrorResponse;

        let error = ErrorResponse::new("unauthorized", "Authentication required");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"error\":\"unauthorized\""));
        assert!(json.contains("\"message\":\"Authentication required\""));
    }

    #[test]
    fn test_error_response_validation() {
        use crate::models::ErrorResponse;

        let error = ErrorResponse::new(
            "validation_error",
            "Validation failed: name must be at least 1 character",
        );
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"error\":\"validation_error\""));
        assert!(json.contains("Validation failed"));
    }

    #[test]
    fn test_error_response_forbidden() {
        use crate::models::ErrorResponse;

        let error = ErrorResponse::new("forbidden", "Insufficient permissions to create API keys");
        let json = serde_json::to_string(&error).unwrap();

        assert!(json.contains("\"error\":\"forbidden\""));
        assert!(json.contains("Insufficient permissions"));
    }

    // ========================================================================
    // Success Response Tests
    // ========================================================================

    #[test]
    fn test_success_response_format() {
        use crate::models::SuccessResponse;

        let response = SuccessResponse::new(ApiKeyResponse {
            id: "test_key".to_string(),
            name: "Test".to_string(),
            prefix: "sk_test_prefix".to_string(),
            environment: "test".to_string(),
            key_type: "standard".to_string(),
            permissions: vec!["read".to_string()],
            rate_limit_override: None,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            created_by: "user".to_string(),
            is_revoked: false,
            revoked_at: None,
        });

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"data\":{"));
        assert!(json.contains("\"id\":\"test_key\""));
    }

    // ========================================================================
    // API Key Service Integration Tests
    // ========================================================================

    #[test]
    fn test_api_key_service_generate_live_key() {
        let service = ApiKeyService::new();
        let result = service.generate_key("live").unwrap();

        assert!(result.key.starts_with("sk_live_"));
        assert!(result.prefix.starts_with("sk_live_"));
        assert_eq!(result.prefix.len(), 16);
        assert!(!result.hash.is_empty());
    }

    #[test]
    fn test_api_key_service_generate_test_key() {
        let service = ApiKeyService::new();
        let result = service.generate_key("test").unwrap();

        assert!(result.key.starts_with("sk_test_"));
        assert!(result.prefix.starts_with("sk_test_"));
        assert_eq!(result.prefix.len(), 16);
        assert!(!result.hash.is_empty());
    }

    #[test]
    fn test_api_key_service_key_format() {
        let service = ApiKeyService::new();
        let result = service.generate_key("live").unwrap();

        // Total length should be 8 (prefix) + 43 (base64) = 51
        assert_eq!(result.key.len(), 51);
    }

    #[test]
    fn test_api_key_service_key_uniqueness() {
        let service = ApiKeyService::new();
        let key1 = service.generate_key("live").unwrap();
        let key2 = service.generate_key("live").unwrap();

        // Keys should be unique
        assert_ne!(key1.key, key2.key);
        assert_ne!(key1.prefix, key2.prefix);
        assert_ne!(key1.hash, key2.hash);
    }

    #[test]
    fn test_api_key_service_verify_key() {
        let service = ApiKeyService::new();
        let generated = service.generate_key("live").unwrap();

        // Verify correct key
        assert!(service.verify_key(&generated.key, &generated.hash).unwrap());

        // Verify wrong key
        assert!(!service
            .verify_key("sk_live_wrongkey12345678901234567890123", &generated.hash)
            .unwrap());
    }

    #[test]
    fn test_api_key_service_invalid_environment() {
        let service = ApiKeyService::new();
        let result = service.generate_key("invalid");

        // Invalid environment returns an error
        assert!(result.is_err());
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_api_key_response_from_shared_model() {
        use shared::models::ApiKey;

        let api_key = ApiKey {
            id: "key_from_db".to_string(),
            organization_id: "org_123".to_string(),
            key_hash: "hashed_key".to_string(),
            name: "DB Key".to_string(),
            prefix: "sk_live_dbprefi".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: serde_json::json!(["read", "write"]),
            rate_limit_override: Some(100),
            last_used_at: Some(Utc::now()),
            last_used_ip: Some("192.168.1.1".to_string()),
            expires_at: None,
            created_by: "creator_user".to_string(),
            created_at: Utc::now(),
            revoked_at: None,
            revoked_by: None,
            revocation_reason: None,
        };

        let response = ApiKeyResponse::from(api_key);

        assert_eq!(response.id, "key_from_db");
        assert_eq!(response.name, "DB Key");
        assert_eq!(response.prefix, "sk_live_dbprefi");
        assert_eq!(response.environment, "live");
        assert_eq!(response.permissions, vec!["read", "write"]);
        assert_eq!(response.rate_limit_override, Some(100));
        assert!(!response.is_revoked);
    }

    #[test]
    fn test_api_key_response_from_revoked_model() {
        use shared::models::ApiKey;

        let now = Utc::now();
        let api_key = ApiKey {
            id: "revoked_key".to_string(),
            organization_id: "org_123".to_string(),
            key_hash: "hash".to_string(),
            name: "Revoked Key".to_string(),
            prefix: "sk_live_revoked".to_string(),
            environment: "live".to_string(),
            key_type: "standard".to_string(),
            permissions: serde_json::json!(["read"]),
            rate_limit_override: None,
            last_used_at: None,
            last_used_ip: None,
            expires_at: None,
            created_by: "user".to_string(),
            created_at: now,
            revoked_at: Some(now),
            revoked_by: Some("admin_user".to_string()),
            revocation_reason: Some("Security concern".to_string()),
        };

        let response = ApiKeyResponse::from(api_key);

        assert!(response.is_revoked);
        assert!(response.revoked_at.is_some());
    }

    #[test]
    fn test_api_key_response_permissions_parsing_fallback() {
        use shared::models::ApiKey;

        let api_key = ApiKey {
            id: "bad_perms_key".to_string(),
            organization_id: "org_123".to_string(),
            key_hash: "hash".to_string(),
            name: "Bad Perms".to_string(),
            prefix: "sk_test_badperm".to_string(),
            environment: "test".to_string(),
            key_type: "standard".to_string(),
            permissions: serde_json::json!("invalid_not_array"), // Invalid format
            rate_limit_override: None,
            last_used_at: None,
            last_used_ip: None,
            expires_at: None,
            created_by: "user".to_string(),
            created_at: Utc::now(),
            revoked_at: None,
            revoked_by: None,
            revocation_reason: None,
        };

        let response = ApiKeyResponse::from(api_key);

        // Should default to ["read"] when parsing fails
        assert_eq!(response.permissions, vec!["read"]);
    }
}
