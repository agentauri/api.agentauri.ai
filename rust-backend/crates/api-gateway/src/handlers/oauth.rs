//! OAuth 2.0 Client and Token Management Handlers
//!
//! This module provides REST API handlers for OAuth 2.0 operations including:
//! - Client registration and management
//! - Token issuance (authorization_code, client_credentials, refresh_token)
//! - Token refresh
//!
//! # Endpoints
//!
//! ## Client Management (JWT Auth Required)
//! - `POST /api/v1/oauth/clients` - Register a new OAuth client
//! - `GET /api/v1/oauth/clients` - List organization's OAuth clients
//! - `DELETE /api/v1/oauth/clients/:id` - Delete an OAuth client
//!
//! ## Token Endpoints (Public - Client Credentials Auth)
//! - `POST /api/v1/oauth/token` - Issue access and refresh tokens
//! - `POST /api/v1/oauth/token/refresh` - Refresh an access token
//!
//! # Security Features
//!
//! - Client secrets use Argon2id with p=4 (parallelism=4)
//! - Access/refresh tokens use Argon2id with p=4
//! - Client secrets shown ONLY ONCE at creation
//! - Tokens expire after configurable duration (default: 1 hour for access, 7 days for refresh)
//! - All secrets are hashed before storage (never stored in plaintext)
//!
//! # OAuth 2.0 Grant Types
//!
//! 1. **authorization_code** - Standard OAuth flow with authorization
//! 2. **client_credentials** - Machine-to-machine authentication
//! 3. **refresh_token** - Refresh an expired access token

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use shared::DbPool;

use crate::{
    handlers::helpers::{
        bad_request, extract_user_id_or_unauthorized, forbidden, handle_db_error, unauthorized,
        validate_request,
    },
    models::{
        can_manage_org, CreateOAuthClientRequest, CreateOAuthClientResponse, ErrorResponse,
        OAuthClientListResponse, OAuthClientResponse, SuccessResponse, TokenRequest, TokenResponse,
    },
    repositories::{
        MemberRepository, OAuthClientRepository, OAuthTokenRepository, OrganizationRepository,
    },
    services::{OAuthClientService, OAuthTokenService},
};

// ============================================================================
// Constants
// ============================================================================

/// Access token expiration (1 hour)
const ACCESS_TOKEN_EXPIRES_IN_SECONDS: i64 = 3600;

/// Refresh token expiration (7 days)
const REFRESH_TOKEN_EXPIRES_IN_SECONDS: i64 = 604800;

// ============================================================================
// Client Management Handlers (JWT Auth Required)
// ============================================================================

/// Create a new OAuth client
///
/// Registers a new OAuth 2.0 client. The client secret is shown ONLY once at creation.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/clients",
    tag = "OAuth Clients",
    request_body = CreateOAuthClientRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "OAuth client created", body = SuccessResponse<CreateOAuthClientResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse),
        (status = 500, description = "Failed to create client", body = ErrorResponse)
    )
)]
pub async fn create_oauth_client(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateOAuthClientRequest>,
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

    // Get user's organization (for now, use their personal organization)
    // TODO: Allow specifying organization_id in request body or query param
    let org = match handle_db_error(
        OrganizationRepository::find_personal_by_user(&pool, &user_id).await,
        "get personal organization",
    ) {
        Ok(Some(o)) => o,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Personal organization not found",
            ))
        }
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &org.id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::Forbidden().json(ErrorResponse::new(
                "forbidden",
                "You are not a member of this organization",
            ))
        }
        Err(resp) => return resp,
    };

    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to create OAuth clients");
    }

    // Generate client ID and secret
    let oauth_client_service = OAuthClientService::new();
    let generated = match oauth_client_service.generate_client() {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to generate OAuth client: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate OAuth client credentials",
            ));
        }
    };

    // Store client in database
    let client = match OAuthClientRepository::create(
        &pool,
        &generated.client_id,
        &generated.client_secret_hash,
        &req.client_name,
        &req.redirect_uris,
        &req.scopes,
        &org.id,
        &req.grant_types,
        req.is_trusted,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to store OAuth client: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create OAuth client",
            ));
        }
    };

    // Return response with FULL client_secret (shown only once!)
    let response = CreateOAuthClientResponse {
        client_id: client.client_id,
        client_secret: generated.client_secret, // ONLY shown at creation
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
        scopes: client.scopes,
        grant_types: client.grant_types,
        is_trusted: client.is_trusted,
        created_at: client.created_at,
    };

    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List OAuth clients for authenticated user's organization
///
/// Returns list of OAuth clients. Client secrets are never exposed.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/clients",
    tag = "OAuth Clients",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of OAuth clients", body = SuccessResponse<OAuthClientListResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_oauth_clients(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<ListClientsQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get user's organization
    let org = match handle_db_error(
        OrganizationRepository::find_personal_by_user(&pool, &user_id).await,
        "get personal organization",
    ) {
        Ok(Some(o)) => o,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Personal organization not found",
            ))
        }
        Err(resp) => return resp,
    };

    // Check membership (any member can view clients)
    match handle_db_error(
        MemberRepository::get_role(&pool, &org.id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view
        Ok(None) => {
            return HttpResponse::Forbidden().json(ErrorResponse::new(
                "forbidden",
                "Not a member of this organization",
            ))
        }
        Err(resp) => return resp,
    }

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    // Get total count
    let total = match handle_db_error(
        OAuthClientRepository::count_by_organization(&pool, &org.id).await,
        "count OAuth clients",
    ) {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Get clients (secrets are never returned)
    let clients = match handle_db_error(
        OAuthClientRepository::list_by_organization(&pool, &org.id, limit, offset).await,
        "list OAuth clients",
    ) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // Convert to response DTOs (no secrets exposed)
    let client_responses: Vec<OAuthClientResponse> =
        clients.into_iter().map(OAuthClientResponse::from).collect();

    let response = OAuthClientListResponse {
        clients: client_responses,
        total,
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete an OAuth client
///
/// Deletes an OAuth client and all associated tokens.
#[utoipa::path(
    delete,
    path = "/api/v1/oauth/clients/{id}",
    tag = "OAuth Clients",
    params(
        ("id" = String, Path, description = "Client ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Client deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse)
    )
)]
pub async fn delete_oauth_client(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let client_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get the client first to determine organization
    let client = match handle_db_error(
        OAuthClientRepository::find_by_client_id(&pool, &client_id).await,
        "fetch OAuth client",
    ) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "OAuth client not found"))
        }
        Err(resp) => return resp,
    };

    // Check membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &client.owner_organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "OAuth client not found"))
        }
        Err(resp) => return resp,
    };

    // Check if user can manage org (owner or admin)
    if !can_manage_org(&role) {
        return forbidden("Insufficient permissions to delete OAuth clients");
    }

    // Delete the client
    if let Err(e) = OAuthClientRepository::delete(&pool, &client_id).await {
        tracing::error!("Failed to delete OAuth client: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to delete OAuth client",
        ));
    }

    HttpResponse::NoContent().finish()
}

// ============================================================================
// Token Endpoints (Public - Client Credentials Auth)
// ============================================================================

/// OAuth 2.0 token endpoint
///
/// Issues access and refresh tokens. Supports client_credentials and refresh_token grants.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/token",
    tag = "OAuth Clients",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token issued", body = TokenResponse),
        (status = 400, description = "Invalid request or unsupported grant type", body = ErrorResponse),
        (status = 401, description = "Invalid client credentials", body = ErrorResponse),
        (status = 500, description = "Failed to generate token", body = ErrorResponse),
        (status = 501, description = "Grant type not implemented", body = ErrorResponse)
    )
)]
pub async fn token_endpoint(
    pool: web::Data<DbPool>,
    req: web::Json<TokenRequest>,
) -> impl Responder {
    // Validate grant type
    match req.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(&pool, &req).await,
        "client_credentials" => handle_client_credentials_grant(&pool, &req).await,
        "refresh_token" => handle_refresh_token_grant(&pool, &req).await,
        _ => bad_request("Unsupported grant_type"),
    }
}

/// Handle authorization_code grant
///
/// This is the standard OAuth 2.0 authorization code flow.
/// For now, we return an error as the full flow requires an authorization server.
async fn handle_authorization_code_grant(_pool: &DbPool, _req: &TokenRequest) -> HttpResponse {
    // TODO: Implement authorization code flow with PKCE
    // This requires:
    // 1. Authorization endpoint to get the code
    // 2. Code verification and exchange
    // 3. PKCE code_challenge verification
    HttpResponse::NotImplemented().json(ErrorResponse::new(
        "not_implemented",
        "Authorization code grant is not yet implemented",
    ))
}

/// Handle client_credentials grant
///
/// This grant type is for machine-to-machine authentication.
/// The client authenticates with client_id and client_secret.
async fn handle_client_credentials_grant(pool: &DbPool, req: &TokenRequest) -> HttpResponse {
    // Verify client_id is provided
    if req.client_id.is_empty() {
        return bad_request("client_id is required");
    }

    // Verify client_secret is provided
    let client_secret = match &req.client_secret {
        Some(s) if !s.is_empty() => s,
        _ => return bad_request("client_secret is required for client_credentials grant"),
    };

    // Verify client credentials
    let client = match verify_client_credentials(pool, &req.client_id, client_secret).await {
        Ok(Some(c)) => c,
        Ok(None) => return unauthorized("Invalid client credentials"),
        Err(resp) => return resp,
    };

    // Verify grant_type is allowed
    if !client
        .grant_types
        .contains(&"client_credentials".to_string())
    {
        return bad_request("client_credentials grant type not allowed for this client");
    }

    // Parse requested scopes
    let requested_scopes = match &req.scope {
        Some(s) => crate::models::parse_scopes(s),
        None => client.scopes.clone(), // Use all client scopes if none specified
    };

    // Verify requested scopes are allowed
    for scope in &requested_scopes {
        if !client.scopes.contains(scope) {
            return bad_request("Scope not allowed for this client");
        }
    }

    // Generate tokens
    let oauth_token_service = OAuthTokenService::new();

    let access_token = match oauth_token_service.generate_access_token() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate access token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate access token",
            ));
        }
    };

    let refresh_token = match oauth_token_service.generate_refresh_token() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate refresh token",
            ));
        }
    };

    // Calculate expiration times
    let access_token_expires_at = Utc::now() + Duration::seconds(ACCESS_TOKEN_EXPIRES_IN_SECONDS);
    let refresh_token_expires_at = Utc::now() + Duration::seconds(REFRESH_TOKEN_EXPIRES_IN_SECONDS);

    // For client_credentials grant, we use the client's organization as the user
    // This is a special case - typically there would be a user_id from the authorization flow
    let user_id = client.owner_organization_id.clone(); // Use org_id as "user_id" for M2M

    // Store token in database
    if let Err(e) = OAuthTokenRepository::create(
        pool,
        &access_token.hash,
        Some(&refresh_token.hash),
        &client.client_id,
        &user_id,
        &client.owner_organization_id,
        &requested_scopes,
        access_token_expires_at,
        Some(refresh_token_expires_at),
    )
    .await
    {
        tracing::error!("Failed to store OAuth token: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to create token",
        ));
    }

    // Return token response
    let response = TokenResponse {
        access_token: access_token.token,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECONDS,
        refresh_token: Some(refresh_token.token),
        scope: crate::models::scopes_to_string(&requested_scopes),
    };

    HttpResponse::Ok().json(response)
}

/// Handle refresh_token grant
///
/// This grant type is used to refresh an expired access token using a valid refresh token.
async fn handle_refresh_token_grant(pool: &DbPool, req: &TokenRequest) -> HttpResponse {
    // Verify refresh_token is provided
    let refresh_token_str = match &req.refresh_token {
        Some(t) if !t.is_empty() => t,
        _ => return bad_request("refresh_token is required for refresh_token grant"),
    };

    // Verify client credentials
    let client_secret = match &req.client_secret {
        Some(s) => s,
        None => return bad_request("client_secret is required"),
    };

    let client = match verify_client_credentials(pool, &req.client_id, client_secret).await {
        Ok(Some(c)) => c,
        Ok(None) => return unauthorized("Invalid client credentials"),
        Err(resp) => return resp,
    };

    // Hash the refresh token to look it up
    let oauth_token_service = OAuthTokenService::new();
    let refresh_token_hash = match oauth_token_service.hash_token(refresh_token_str) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process refresh token",
            ));
        }
    };

    // Find the token in database and verify it's still valid
    let stored_token =
        match OAuthTokenRepository::find_by_refresh_token_hash(pool, &refresh_token_hash).await {
            Ok(Some(t)) => {
                // Verify the token with constant-time comparison
                match oauth_token_service.verify_token(
                    refresh_token_str,
                    &t.refresh_token_hash.clone().unwrap_or_default(),
                ) {
                    Ok(true) => t,
                    Ok(false) => {
                        oauth_token_service.dummy_verify(); // Timing attack mitigation
                        return unauthorized("Invalid refresh token");
                    }
                    Err(e) => {
                        tracing::error!("Failed to verify refresh token: {}", e);
                        return HttpResponse::InternalServerError().json(ErrorResponse::new(
                            "internal_error",
                            "Failed to verify refresh token",
                        ));
                    }
                }
            }
            Ok(None) => {
                oauth_token_service.dummy_verify(); // Timing attack mitigation
                return unauthorized("Invalid refresh token");
            }
            Err(e) => {
                tracing::error!("Failed to find refresh token: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to process refresh token",
                ));
            }
        };

    // Verify the token belongs to the client
    if stored_token.client_id != client.client_id {
        return unauthorized("Refresh token does not belong to this client");
    }

    // Revoke the old token
    if let Err(e) = OAuthTokenRepository::revoke(pool, &stored_token.id).await {
        tracing::error!("Failed to revoke old token: {}", e);
        // Continue anyway - better to issue new token than fail
    }

    // Generate new tokens
    let new_access_token = match oauth_token_service.generate_access_token() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate access token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate access token",
            ));
        }
    };

    let new_refresh_token = match oauth_token_service.generate_refresh_token() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate refresh token",
            ));
        }
    };

    // Calculate expiration times
    let access_token_expires_at = Utc::now() + Duration::seconds(ACCESS_TOKEN_EXPIRES_IN_SECONDS);
    let refresh_token_expires_at = Utc::now() + Duration::seconds(REFRESH_TOKEN_EXPIRES_IN_SECONDS);

    // Store new token in database
    if let Err(e) = OAuthTokenRepository::create(
        pool,
        &new_access_token.hash,
        Some(&new_refresh_token.hash),
        &client.client_id,
        &stored_token.user_id,
        &stored_token.organization_id,
        &stored_token.scopes,
        access_token_expires_at,
        Some(refresh_token_expires_at),
    )
    .await
    {
        tracing::error!("Failed to store new OAuth token: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to create token",
        ));
    }

    // Return token response
    let response = TokenResponse {
        access_token: new_access_token.token,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECONDS,
        refresh_token: Some(new_refresh_token.token),
        scope: crate::models::scopes_to_string(&stored_token.scopes),
    };

    HttpResponse::Ok().json(response)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify client credentials (client_id + client_secret)
///
/// Returns the client if credentials are valid, None if invalid.
/// Uses timing-attack resistant verification.
async fn verify_client_credentials(
    pool: &DbPool,
    client_id: &str,
    client_secret: &str,
) -> Result<Option<shared::models::OAuthClient>, HttpResponse> {
    let oauth_client_service = OAuthClientService::new();

    // Find client by client_id
    let client = match OAuthClientRepository::find_by_client_id(pool, client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            // Client not found - perform dummy verification for timing attack resistance
            oauth_client_service.dummy_verify();
            return Ok(None);
        }
        Err(e) => {
            tracing::error!("Failed to find OAuth client: {}", e);
            return Err(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to verify client credentials",
            )));
        }
    };

    // Verify client secret
    match oauth_client_service.verify_client_secret(client_secret, &client.client_secret_hash) {
        Ok(true) => Ok(Some(client)),
        Ok(false) => Ok(None),
        Err(e) => {
            tracing::error!("Failed to verify client secret: {}", e);
            Err(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to verify client credentials",
            )))
        }
    }
}

// ============================================================================
// Query Parameter Structs
// ============================================================================

/// Query parameters for listing OAuth clients
#[derive(Debug, serde::Deserialize)]
pub struct ListClientsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_clients_query_deserialize_minimal() {
        let query_string = "";
        let query: ListClientsQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_list_clients_query_deserialize_full() {
        let query_string = "limit=50&offset=100";
        let query: ListClientsQuery = serde_urlencoded::from_str(query_string).unwrap();
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(100));
    }

    #[test]
    fn test_access_token_expiration_constant() {
        assert_eq!(ACCESS_TOKEN_EXPIRES_IN_SECONDS, 3600); // 1 hour
    }

    #[test]
    fn test_refresh_token_expiration_constant() {
        assert_eq!(REFRESH_TOKEN_EXPIRES_IN_SECONDS, 604800); // 7 days
    }
}
