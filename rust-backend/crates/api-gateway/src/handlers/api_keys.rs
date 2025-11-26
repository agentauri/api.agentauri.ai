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
use validator::Validate;

use crate::{
    middleware::get_user_id,
    models::{
        can_manage_org, ApiKeyListResponse, ApiKeyResponse, CreateApiKeyRequest,
        CreateApiKeyResponse, ErrorResponse, PaginationParams, RevokeApiKeyRequest,
        RotateApiKeyRequest, RotateApiKeyResponse, SuccessResponse,
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
pub async fn create_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateApiKeyRequest>,
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
                "Failed to create API key",
            ));
        }
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to create API keys",
        ));
    }

    // Generate the API key
    let api_key_service = ApiKeyService::new();
    let generated = match api_key_service.generate_key(&req.environment) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to generate API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create API key",
            ));
        }
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
    let ip_address = req_http
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());
    let user_agent = req_http
        .headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key.id),
        &org_id,
        "created",
        ip_address.as_deref(),
        user_agent.as_deref(),
        Some("/api/v1/api-keys"),
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
    let response = CreateApiKeyResponse {
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
pub async fn list_api_keys(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<ApiKeyListQuery>,
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
    let role = match MemberRepository::get_role(&pool, &query.organization_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to list API keys",
            ));
        }
    };

    // All members can view keys (masked)
    let _ = role; // Role checked above for membership

    // Get total count
    let include_revoked = query.include_revoked.unwrap_or(false);
    let total = match ApiKeyRepository::count_by_organization(
        &pool,
        &query.organization_id,
        include_revoked,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count API keys: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to list API keys",
            ));
        }
    };

    // Get keys
    let keys = match ApiKeyRepository::list_by_organization(
        &pool,
        &query.organization_id,
        include_revoked,
        pagination.limit,
        pagination.offset,
    )
    .await
    {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to list API keys: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to list API keys",
            ));
        }
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
pub async fn get_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let key_id = path.into_inner();

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

    // Get the key first to determine org_id
    let key = match ApiKeyRepository::find_by_id(&pool, &key_id).await {
        Ok(Some(k)) => k,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to fetch API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch API key",
            ));
        }
    };

    // Check membership
    match MemberRepository::get_role(&pool, &key.organization_id, &user_id).await {
        Ok(Some(_)) => {} // Any member can view
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch API key",
            ));
        }
    }

    let response = ApiKeyResponse::from(key);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Revoke an API key
///
/// DELETE /api/v1/api-keys/{id}
pub async fn revoke_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: Option<web::Json<RevokeApiKeyRequest>>,
) -> impl Responder {
    let key_id = path.into_inner();

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

    // Get the key first to determine org_id
    let key = match ApiKeyRepository::find_by_id(&pool, &key_id).await {
        Ok(Some(k)) => k,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to fetch API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to revoke API key",
            ));
        }
    };

    // Check if already revoked
    if key.revoked_at.is_some() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "API key is already revoked",
        ));
    }

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &key.organization_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to revoke API key",
            ));
        }
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to revoke API keys",
        ));
    }

    // Validate reason if provided
    let reason = match &req {
        Some(r) => {
            if let Err(e) = r.validate() {
                return HttpResponse::BadRequest().json(ErrorResponse::new(
                    "validation_error",
                    format!("Validation failed: {}", e),
                ));
            }
            r.reason.as_deref()
        }
        None => None,
    };

    // Revoke the key
    let revoked_key = match ApiKeyRepository::revoke(&pool, &key_id, &user_id, reason).await {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to revoke API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to revoke API key",
            ));
        }
    };

    // Log the revocation event
    let ip_address = req_http
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    if let Err(e) = ApiKeyAuditRepository::log(
        &pool,
        Some(&key_id),
        &key.organization_id,
        "revoked",
        ip_address.as_deref(),
        None,
        Some("/api/v1/api-keys/{id}"),
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
pub async fn rotate_api_key(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: Option<web::Json<RotateApiKeyRequest>>,
) -> impl Responder {
    let old_key_id = path.into_inner();

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

    // Validate request if provided
    if let Some(ref r) = req {
        if let Err(e) = r.validate() {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "validation_error",
                format!("Validation failed: {}", e),
            ));
        }
    }

    // Get the old key
    let old_key = match ApiKeyRepository::find_by_id(&pool, &old_key_id).await {
        Ok(Some(k)) => k,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to fetch API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to rotate API key",
            ));
        }
    };

    // Check if already revoked
    if old_key.revoked_at.is_some() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "bad_request",
            "Cannot rotate a revoked API key",
        ));
    }

    // Check membership and role
    let role = match MemberRepository::get_role(&pool, &old_key.organization_id, &user_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "API key not found"))
        }
        Err(e) => {
            tracing::error!("Failed to check membership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to rotate API key",
            ));
        }
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to rotate API keys",
        ));
    }

    // Generate new key
    let api_key_service = ApiKeyService::new();
    let generated = match api_key_service.generate_key(&old_key.environment) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to generate new API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to rotate API key",
            ));
        }
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

    // Use a transaction for atomicity
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to rotate API key",
            ));
        }
    };

    // Revoke old key
    if let Err(e) =
        ApiKeyRepository::revoke_with_executor(&mut *tx, &old_key_id, &user_id, Some("rotated"))
            .await
    {
        tracing::error!("Failed to revoke old API key: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to rotate API key",
        ));
    }

    // Create new key
    let new_key = match ApiKeyRepository::create_with_executor(
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
    .await
    {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to create new API key: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to rotate API key",
            ));
        }
    };

    // Log the rotation event
    let ip_address = req_http
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    if let Err(e) = ApiKeyAuditRepository::log_with_executor(
        &mut *tx,
        Some(&new_key.id),
        &old_key.organization_id,
        "rotated",
        ip_address.as_deref(),
        None,
        Some("/api/v1/api-keys/{id}/rotate"),
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
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to rotate API key",
        ));
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
// Query Parameter Structs
// ============================================================================

/// Query parameters for organization ID
#[derive(Debug, serde::Deserialize)]
pub struct OrgIdQuery {
    pub organization_id: String,
}

/// Query parameters for listing API keys
#[derive(Debug, serde::Deserialize)]
pub struct ApiKeyListQuery {
    pub organization_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_revoked: Option<bool>,
}
