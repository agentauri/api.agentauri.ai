//! Agent Linking Handlers
//!
//! This module provides REST API handlers for agent linking operations
//! using wallet signature verification (Layer 2 authentication).
//!
//! # Endpoints
//!
//! - `POST /api/v1/agents/link` - Link an agent NFT to an organization
//! - `GET /api/v1/agents/linked` - List linked agents for an organization
//! - `DELETE /api/v1/agents/{agent_id}/link` - Unlink an agent
//!
//! # Authorization
//!
//! All endpoints require JWT authentication. The linking process requires:
//! 1. Wallet signature verification (EIP-191)
//! 2. On-chain ownership verification (IdentityRegistry.ownerOf)
//! 3. Organization membership (admin+ role)

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::DbPool;
use tracing::info;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    handlers::helpers::{extract_user_id_or_unauthorized, handle_db_error, validate_request},
    models::{can_manage_org, ErrorResponse, SuccessResponse},
    repositories::{wallet::NonceRepository, AgentLinkRepository, MemberRepository},
    services::WalletService,
};

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to link an agent to an organization
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LinkAgentRequest {
    /// The agent token ID from IdentityRegistry
    pub agent_id: i64,
    /// The chain ID where the agent is registered
    pub chain_id: i32,
    /// The organization ID to link to
    #[validate(length(min = 1))]
    pub organization_id: String,
    /// The wallet address claiming ownership
    #[validate(length(min = 42, max = 42))]
    pub wallet_address: String,
    /// The challenge message that was signed
    #[validate(length(min = 1))]
    pub challenge: String,
    /// The EIP-191 signature of the challenge
    #[validate(length(min = 130, max = 132))]
    pub signature: String,
}

/// Response for linked agent
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentLinkResponse {
    pub id: String,
    pub agent_id: i64,
    pub chain_id: i32,
    pub organization_id: String,
    pub wallet_address: String,
    pub status: String,
    pub created_at: String,
}

/// Query for organization filter
#[derive(Debug, Deserialize)]
pub struct OrgIdQuery {
    pub organization_id: String,
}

/// Path parameter for agent ID
#[derive(Debug, Deserialize)]
pub struct AgentIdPath {
    pub agent_id: i64,
}

/// Query for chain ID
#[derive(Debug, Deserialize)]
pub struct ChainIdQuery {
    pub chain_id: i32,
    pub organization_id: String,
}

// ============================================================================
// Agent Linking Handlers
// ============================================================================

/// Link an agent NFT to an organization
///
/// POST /api/v1/agents/link
///
/// This endpoint verifies wallet signature and on-chain ownership before
/// creating the link. The wallet address must own the agent NFT.
#[utoipa::path(
    post,
    path = "/api/v1/agents/link",
    tag = "Agents",
    request_body = LinkAgentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Agent linked successfully", body = AgentLinkResponse),
        (status = 400, description = "Invalid request or signature", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not owner or not admin", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse),
        (status = 409, description = "Agent already linked", body = ErrorResponse)
    )
)]
pub async fn link_agent(
    pool: web::Data<DbPool>,
    wallet_service: web::Data<WalletService>,
    req_http: HttpRequest,
    req: web::Json<LinkAgentRequest>,
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

    // Check organization membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &req.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    };

    // Must be admin or owner to link agents
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only admins can link agents to the organization",
        ));
    }

    // WalletService is now injected from app state (created once at startup)
    // This provides connection pooling for RPC calls

    // SECURITY: Extract and validate nonce from challenge (prevents replay attacks)
    let nonce = match extract_nonce_from_challenge(&req.challenge) {
        Ok(n) => n,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("invalid_challenge", e))
        }
    };

    // SECURITY: Check if nonce was already used (replay attack prevention)
    match handle_db_error(
        NonceRepository::is_nonce_used(&pool, &nonce).await,
        "check nonce",
    ) {
        Ok(true) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "nonce_reused",
                "This challenge has already been used. Please request a new one.",
            ))
        }
        Ok(false) => {} // Nonce is fresh, continue
        Err(resp) => return resp,
    }

    // SECURITY: Extract and validate expiration from challenge
    let expires_at = match extract_expiration_from_challenge(&req.challenge) {
        Ok(exp) => exp,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("invalid_challenge", e))
        }
    };

    // SECURITY: Check if challenge has expired
    if wallet_service
        .validate_challenge_expiration(expires_at)
        .is_err()
    {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "challenge_expired",
            "Challenge has expired. Please request a new one.",
        ));
    }

    // Verify signature and recover signer
    let _recovered = match wallet_service.verify_signature(
        &req.challenge,
        &req.signature,
        &req.wallet_address,
    ) {
        Ok(addr) => addr,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_signature",
                format!("Signature verification failed: {}", e),
            ))
        }
    };

    // SECURITY: Mark nonce as used AFTER successful signature verification
    // This prevents attackers from burning valid nonces with fake signatures
    if let Err(resp) = handle_db_error(
        NonceRepository::store_nonce(&**pool, &nonce, &req.wallet_address, expires_at).await,
        "store nonce",
    ) {
        return resp;
    }

    info!(
        nonce = %nonce,
        wallet = %req.wallet_address,
        agent_id = req.agent_id,
        "Nonce consumed successfully"
    );

    // Verify on-chain ownership (returns Ok(()) if owner, Err if not)
    if let Err(e) = wallet_service
        .verify_agent_ownership(&req.wallet_address, req.agent_id, req.chain_id)
        .await
    {
        // Check if it's specifically a "not owner" error
        let error_msg = e.to_string();
        if error_msg.contains("not own") || error_msg.contains("ownership") {
            return HttpResponse::Forbidden().json(ErrorResponse::new(
                "not_owner",
                "Wallet address does not own this agent NFT",
            ));
        }
        tracing::warn!("On-chain verification failed: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "verification_error",
            "Could not verify on-chain ownership",
        ));
    }

    // Check if agent is already linked
    match handle_db_error(
        AgentLinkRepository::is_agent_linked(&pool, req.agent_id, req.chain_id).await,
        "check existing link",
    ) {
        Ok(true) => {
            return HttpResponse::Conflict().json(ErrorResponse::new(
                "already_linked",
                "This agent is already linked to an organization",
            ))
        }
        Ok(false) => {}
        Err(resp) => return resp,
    }

    // Create the link
    let link = match handle_db_error(
        AgentLinkRepository::create(
            &**pool,
            req.agent_id,
            req.chain_id,
            &req.organization_id,
            &req.wallet_address,
            &user_id,
            &req.signature,
        )
        .await,
        "create agent link",
    ) {
        Ok(l) => l,
        Err(resp) => return resp,
    };

    HttpResponse::Created().json(AgentLinkResponse {
        id: link.id,
        agent_id: link.agent_id,
        chain_id: link.chain_id,
        organization_id: link.organization_id,
        wallet_address: link.wallet_address,
        status: link.status,
        created_at: link.created_at.to_rfc3339(),
    })
}

/// List linked agents for an organization
///
/// GET /api/v1/agents/linked?organization_id=xxx
#[utoipa::path(
    get,
    path = "/api/v1/agents/linked",
    tag = "Agents",
    params(
        ("organization_id" = String, Query, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of linked agents", body = Vec<AgentLinkResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_linked_agents(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<OrgIdQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Check organization membership
    match handle_db_error(
        MemberRepository::get_role(&pool, &query.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get linked agents
    let links = match handle_db_error(
        AgentLinkRepository::find_by_organization(&pool, &query.organization_id).await,
        "list agent links",
    ) {
        Ok(l) => l,
        Err(resp) => return resp,
    };

    let responses: Vec<AgentLinkResponse> = links
        .into_iter()
        .map(|l| AgentLinkResponse {
            id: l.id,
            agent_id: l.agent_id,
            chain_id: l.chain_id,
            organization_id: l.organization_id,
            wallet_address: l.wallet_address,
            status: l.status,
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    HttpResponse::Ok().json(responses)
}

/// Unlink an agent from an organization
///
/// DELETE /api/v1/agents/{agent_id}/link?chain_id=xxx&organization_id=xxx
#[utoipa::path(
    delete,
    path = "/api/v1/agents/{agent_id}/link",
    tag = "Agents",
    params(
        ("agent_id" = i64, Path, description = "Agent token ID"),
        ("chain_id" = i32, Query, description = "Chain ID"),
        ("organization_id" = String, Query, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Agent unlinked successfully", body = SuccessResponse<String>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not admin or wrong organization", body = ErrorResponse),
        (status = 404, description = "Agent link not found", body = ErrorResponse)
    )
)]
pub async fn unlink_agent(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<AgentIdPath>,
    query: web::Query<ChainIdQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Check organization membership and role
    let role = match handle_db_error(
        MemberRepository::get_role(&pool, &query.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    };

    // Must be admin or owner to unlink agents
    if !can_manage_org(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only admins can unlink agents from the organization",
        ));
    }

    // Check that the agent is linked to THIS organization
    match handle_db_error(
        AgentLinkRepository::get_organization_for_agent(&pool, path.agent_id, query.chain_id).await,
        "get agent link",
    ) {
        Ok(Some(org_id)) if org_id == query.organization_id => {}
        Ok(Some(_)) => {
            return HttpResponse::Forbidden().json(ErrorResponse::new(
                "forbidden",
                "This agent is not linked to your organization",
            ))
        }
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Agent link not found"))
        }
        Err(resp) => return resp,
    }

    // Revoke the link
    match handle_db_error(
        AgentLinkRepository::revoke(&**pool, path.agent_id, query.chain_id).await,
        "revoke agent link",
    ) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Agent link not found or already revoked",
            ))
        }
        Err(resp) => return resp,
    }

    HttpResponse::Ok().json(SuccessResponse::new("Agent unlinked successfully"))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract nonce from challenge message
///
/// Challenge format:
/// "Sign this message to authenticate with ERC-8004 API
///
/// Wallet: 0x...
/// Nonce: <hex string>
/// Expires: 2024-11-27T12:00:00Z"
fn extract_nonce_from_challenge(challenge: &str) -> Result<String, &'static str> {
    for line in challenge.lines() {
        if line.starts_with("Nonce: ") {
            let nonce = line.trim_start_matches("Nonce: ").trim().to_string();
            if nonce.is_empty() {
                return Err("Nonce value is empty");
            }
            return Ok(nonce);
        }
    }
    Err("Nonce not found in challenge message")
}

/// Extract expiration timestamp from challenge message
fn extract_expiration_from_challenge(challenge: &str) -> Result<DateTime<Utc>, &'static str> {
    for line in challenge.lines() {
        if line.starts_with("Expires: ") {
            let ts_str = line.trim_start_matches("Expires: ").trim();
            return DateTime::parse_from_rfc3339(ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| "Invalid expiration timestamp format");
        }
    }
    Err("Expiration not found in challenge message")
}

// =============================================================================
// Organization-scoped endpoints (path parameter for org_id)
// =============================================================================

/// List linked agents for organization (path-based)
///
/// Returns list of agents linked to the specified organization.
/// Organization ID is taken from the URL path.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/agents",
    tag = "Agents",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of linked agents", body = Vec<AgentLinkResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_org_agents(
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

    // Check organization membership
    match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get linked agents
    let links = match handle_db_error(
        AgentLinkRepository::find_by_organization(&pool, &org_id).await,
        "list agent links",
    ) {
        Ok(l) => l,
        Err(resp) => return resp,
    };

    let responses: Vec<AgentLinkResponse> = links
        .into_iter()
        .map(|l| AgentLinkResponse {
            id: l.id,
            agent_id: l.agent_id,
            chain_id: l.chain_id,
            organization_id: l.organization_id,
            wallet_address: l.wallet_address,
            status: l.status,
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    HttpResponse::Ok().json(responses)
}
