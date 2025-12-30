//! Billing Handlers
//!
//! This module provides REST API handlers for billing operations
//! including credit balance management, transactions, and Stripe integration.
//!
//! # Endpoints
//!
//! - `GET /api/v1/billing/credits` - Get credit balance for organization
//! - `GET /api/v1/billing/transactions` - List credit transactions
//! - `POST /api/v1/billing/credits/purchase` - Purchase credits via Stripe
//! - `GET /api/v1/billing/subscription` - Get subscription details
//! - `POST /api/v1/billing/webhook` - Handle Stripe webhook events
//!
//! # Authorization
//!
//! All endpoints except webhook require JWT authentication and organization membership.
//!
//! # Security
//!
//! The Stripe webhook endpoint validates:
//! 1. Webhook signature (cryptographic verification)
//! 2. Source IP address (optional whitelist for defense in depth)

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::{Config, DbPool};
use std::net::IpAddr;
use std::str::FromStr;
use tracing::{debug, info, warn};

use crate::{
    handlers::helpers::{extract_user_id_or_unauthorized, handle_db_error, validate_request},
    models::{
        billing::{
            CreditBalanceResponse, CreditTransactionResponse, PurchaseCreditsResponse,
            SubscriptionResponse, TransactionListQuery,
        },
        ErrorResponse,
    },
    repositories::{
        billing::{CreditRepository, SubscriptionRepository, TransactionRepository},
        MemberRepository,
    },
    services::{StripeConfig, StripeService},
};

// ============================================================================
// Query DTOs
// ============================================================================

/// Query parameter for organization ID
#[derive(Debug, serde::Deserialize)]
pub struct OrgIdQuery {
    pub organization_id: String,
}

// ============================================================================
// Credit Balance Handlers
// ============================================================================

/// Get credit balance for an organization
///
/// GET /api/v1/billing/credits?organization_id=xxx
#[utoipa::path(
    get,
    path = "/api/v1/billing/credits",
    tag = "Billing",
    params(
        ("organization_id" = String, Query, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Credit balance", body = CreditBalanceResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization or credits not found", body = ErrorResponse)
    )
)]
pub async fn get_credits(
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
        Ok(Some(_)) => {} // Any member can view balance
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get credit balance
    let credit = match handle_db_error(
        CreditRepository::get_balance(&pool, &query.organization_id).await,
        "get credit balance",
    ) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Credits not initialized for this organization",
            ))
        }
        Err(resp) => return resp,
    };

    HttpResponse::Ok().json(CreditBalanceResponse::new(credit.balance))
}

/// List credit transactions for an organization
///
/// GET /api/v1/billing/transactions?organization_id=xxx&limit=20&offset=0
#[utoipa::path(
    get,
    path = "/api/v1/billing/transactions",
    tag = "Billing",
    params(
        ("organization_id" = String, Query, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items to return"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip"),
        ("transaction_type" = Option<String>, Query, description = "Filter by transaction type")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of transactions", body = Vec<CreditTransactionResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_transactions(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<OrgIdQuery>,
    list_query: web::Query<TransactionListQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate query parameters
    if let Err(resp) = validate_request(&*list_query) {
        return resp;
    }

    // Check organization membership
    match handle_db_error(
        MemberRepository::get_role(&pool, &query.organization_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view transactions
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get transactions
    let transactions = match handle_db_error(
        TransactionRepository::list(
            &pool,
            &query.organization_id,
            list_query.limit,
            list_query.offset,
            list_query.transaction_type.as_deref(),
        )
        .await,
        "list transactions",
    ) {
        Ok(txs) => txs,
        Err(resp) => return resp,
    };

    let responses: Vec<CreditTransactionResponse> =
        transactions.into_iter().map(|tx| tx.into()).collect();

    HttpResponse::Ok().json(responses)
}

// =============================================================================
// Organization-scoped endpoints (path parameter for org_id)
// =============================================================================

/// Get credit balance for organization (path-based)
///
/// Returns the credit balance for the specified organization.
/// Organization ID is taken from the URL path.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/credits/balance",
    tag = "Billing",
    params(
        ("id" = String, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Credit balance", body = CreditBalanceResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_org_credits(
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
        Ok(Some(_)) => {} // Any member can view balance
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get credit balance
    let credit = match handle_db_error(
        CreditRepository::get_balance(&pool, &org_id).await,
        "get credit balance",
    ) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Credits not initialized for this organization",
            ))
        }
        Err(resp) => return resp,
    };

    HttpResponse::Ok().json(CreditBalanceResponse::new(credit.balance))
}

/// List transactions for organization (path-based)
///
/// Returns paginated list of credit transactions for the specified organization.
/// Organization ID is taken from the URL path.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/credits/transactions",
    tag = "Billing",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip"),
        ("transaction_type" = Option<String>, Query, description = "Filter by transaction type")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of transactions", body = Vec<CreditTransactionResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_org_transactions(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    list_query: web::Query<TransactionListQuery>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate query parameters
    if let Err(resp) = validate_request(&*list_query) {
        return resp;
    }

    // Check organization membership
    match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // Any member can view transactions
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get transactions
    let transactions = match handle_db_error(
        TransactionRepository::list(
            &pool,
            &org_id,
            list_query.limit,
            list_query.offset,
            list_query.transaction_type.as_deref(),
        )
        .await,
        "list transactions",
    ) {
        Ok(txs) => txs,
        Err(resp) => return resp,
    };

    let responses: Vec<CreditTransactionResponse> =
        transactions.into_iter().map(|tx| tx.into()).collect();

    HttpResponse::Ok().json(responses)
}

// ============================================================================
// Purchase Credits Handler
// ============================================================================

/// Request body wrapper for purchase that includes organization_id
#[derive(Debug, serde::Deserialize, validator::Validate, utoipa::ToSchema)]
pub struct PurchaseCreditsRequestWithOrg {
    /// The organization purchasing credits
    pub organization_id: String,
    /// Amount in USDC (whole units)
    #[validate(range(min = 1, max = 10000))]
    pub amount: i64,
    /// Success URL for redirect
    #[validate(url)]
    pub success_url: String,
    /// Cancel URL for redirect
    #[validate(url)]
    pub cancel_url: String,
}

/// Purchase credits via Stripe Checkout
///
/// POST /api/v1/billing/credits/purchase
///
/// Creates a Stripe Checkout session for credit purchase.
/// Returns checkout URL for client redirect.
#[utoipa::path(
    post,
    path = "/api/v1/billing/credits/purchase",
    tag = "Billing",
    request_body = PurchaseCreditsRequestWithOrg,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Checkout session created", body = PurchaseCreditsResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - admin required", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse),
        (status = 503, description = "Payment service unavailable", body = ErrorResponse)
    )
)]
pub async fn purchase_credits(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req_http: HttpRequest,
    req: web::Json<PurchaseCreditsRequestWithOrg>,
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

    // Check organization membership (admin required for purchases)
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

    // Must be admin or owner to purchase credits
    if role != "admin" && role != "owner" {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Only admins can purchase credits",
        ));
    }

    // Get Stripe configuration from environment
    let stripe_config = match get_stripe_config(&config) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!("Stripe not configured: {}", e);
            return HttpResponse::ServiceUnavailable().json(ErrorResponse::new(
                "service_unavailable",
                "Payment service not configured",
            ));
        }
    };

    // Initialize Stripe service
    let stripe_service = match StripeService::new(stripe_config) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to initialize Stripe service: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to initialize payment service",
            ));
        }
    };

    // Create checkout session
    let checkout = match stripe_service
        .create_checkout_session(
            &req.organization_id,
            req.amount,
            &req.success_url,
            &req.cancel_url,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create checkout session: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "checkout_failed",
                format!("Failed to create checkout session: {}", e),
            ));
        }
    };

    info!(
        organization_id = %req.organization_id,
        amount = req.amount,
        session_id = %checkout.session_id,
        "Created Stripe checkout session"
    );

    HttpResponse::Ok().json(PurchaseCreditsResponse {
        checkout_url: checkout.checkout_url,
        session_id: checkout.session_id,
    })
}

// ============================================================================
// Subscription Handler
// ============================================================================

/// Get subscription details for an organization
///
/// GET /api/v1/billing/subscription?organization_id=xxx
#[utoipa::path(
    get,
    path = "/api/v1/billing/subscription",
    tag = "Billing",
    params(
        ("organization_id" = String, Query, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subscription details", body = SubscriptionResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization or subscription not found", body = ErrorResponse)
    )
)]
pub async fn get_subscription(
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
        Ok(Some(_)) => {} // Any member can view subscription
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "Organization not found or you are not a member",
            ))
        }
        Err(resp) => return resp,
    }

    // Get subscription
    let subscription = match handle_db_error(
        SubscriptionRepository::find_by_organization(&pool, &query.organization_id).await,
        "get subscription",
    ) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse::new(
                "not_found",
                "No subscription found for this organization",
            ))
        }
        Err(resp) => return resp,
    };

    HttpResponse::Ok().json(SubscriptionResponse::from(subscription))
}

// ============================================================================
// Stripe IP Whitelist (Defense in Depth)
// ============================================================================

/// Stripe webhook IP ranges (from https://stripe.com/docs/ips)
/// These are the IP addresses Stripe uses to send webhook events.
/// This is an additional security layer on top of signature verification.
///
/// Note: This list should be periodically updated from Stripe's documentation.
/// Last updated: 2025-01-28
const STRIPE_WEBHOOK_IP_RANGES: &[&str] = &[
    // Stripe webhook IPs (CIDR ranges)
    "3.18.12.63/32",
    "3.130.192.231/32",
    "13.235.14.237/32",
    "13.235.122.149/32",
    "18.211.135.69/32",
    "35.154.171.200/32",
    "52.15.183.38/32",
    "54.88.130.119/32",
    "54.88.130.237/32",
    "54.187.174.169/32",
    "54.187.205.235/32",
    "54.187.216.72/32",
];

/// Check if an IP address is from Stripe's webhook servers
///
/// # Arguments
/// * `ip` - The IP address string to check
///
/// # Returns
/// * `true` if the IP is in Stripe's known webhook IP ranges
/// * `false` otherwise
fn is_stripe_ip(ip: &str) -> bool {
    let ip_addr = match IpAddr::from_str(ip) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    for cidr in STRIPE_WEBHOOK_IP_RANGES {
        if ip_in_cidr(&ip_addr, cidr) {
            return true;
        }
    }

    false
}

/// Check if an IP is within a CIDR range
fn ip_in_cidr(ip: &IpAddr, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network_addr = match IpAddr::from_str(parts[0]) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    let prefix_len: u8 = match parts[1].parse() {
        Ok(len) => len,
        Err(_) => return false,
    };

    match (ip, network_addr) {
        (IpAddr::V4(ip_v4), IpAddr::V4(net_v4)) => {
            if prefix_len > 32 {
                return false;
            }
            let ip_u32 = u32::from(*ip_v4);
            let net_u32 = u32::from(net_v4);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u32 << (32 - prefix_len)
            };
            (ip_u32 & mask) == (net_u32 & mask)
        }
        (IpAddr::V6(ip_v6), IpAddr::V6(net_v6)) => {
            if prefix_len > 128 {
                return false;
            }
            let ip_u128 = u128::from(*ip_v6);
            let net_u128 = u128::from(net_v6);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u128 << (128 - prefix_len)
            };
            (ip_u128 & mask) == (net_u128 & mask)
        }
        _ => false,
    }
}

/// Extract client IP from request (simplified version)
fn extract_client_ip(req: &HttpRequest) -> String {
    // Check X-Forwarded-For first (from trusted proxies)
    if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = forwarded.to_str() {
            // Take the first IP (client IP)
            if let Some(first_ip) = value.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Fall back to peer address
    req.connection_info()
        .peer_addr()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// ============================================================================
// Stripe Webhook Handler
// ============================================================================

/// Handle Stripe webhook events
///
/// POST /api/v1/billing/webhook
///
/// Verifies webhook signature and processes events:
/// - checkout.session.completed: Add credits
/// - checkout.session.async_payment_failed: Log failure
/// - customer.subscription.created/updated/deleted: Update subscription
///
/// # Security
///
/// 1. Validates source IP against Stripe's known webhook IPs (configurable)
/// 2. Verifies webhook signature cryptographically
#[utoipa::path(
    post,
    path = "/api/v1/billing/webhook",
    tag = "Billing",
    request_body(content = String, description = "Raw webhook payload", content_type = "application/json"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid signature or payload", body = ErrorResponse),
        (status = 403, description = "IP not in Stripe whitelist", body = ErrorResponse),
        (status = 503, description = "Payment service unavailable", body = ErrorResponse)
    )
)]
pub async fn handle_stripe_webhook(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req_http: HttpRequest,
    payload: web::Bytes,
) -> impl Responder {
    // SECURITY: Validate source IP (defense in depth)
    // This is configurable via STRIPE_IP_WHITELIST_ENABLED env var (default: true in production)
    let ip_whitelist_enabled = std::env::var("STRIPE_IP_WHITELIST_ENABLED")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or_else(|_| !cfg!(debug_assertions)); // Enabled by default in release builds

    if ip_whitelist_enabled {
        let client_ip = extract_client_ip(&req_http);

        // Strip port if present (e.g., "127.0.0.1:8080" -> "127.0.0.1")
        let ip_only = client_ip.split(':').next().unwrap_or(&client_ip);

        if !is_stripe_ip(ip_only) {
            warn!(
                client_ip = %client_ip,
                "Stripe webhook rejected: IP not in whitelist"
            );
            return HttpResponse::Forbidden().json(ErrorResponse::new(
                "ip_not_allowed",
                "Request IP not in allowed webhook sources",
            ));
        }

        debug!(client_ip = %client_ip, "Stripe webhook IP validated");
    }

    // Get Stripe-Signature header
    let signature = match req_http
        .headers()
        .get("Stripe-Signature")
        .and_then(|h| h.to_str().ok())
    {
        Some(s) => s,
        None => {
            warn!("Missing Stripe-Signature header");
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_signature",
                "Missing Stripe-Signature header",
            ));
        }
    };

    // Get Stripe configuration
    let stripe_config = match get_stripe_config(&config) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!("Stripe not configured: {}", e);
            return HttpResponse::ServiceUnavailable().json(ErrorResponse::new(
                "service_unavailable",
                "Payment service not configured",
            ));
        }
    };

    // Initialize Stripe service
    let stripe_service = match StripeService::new(stripe_config) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to initialize Stripe service: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to initialize payment service",
            ));
        }
    };

    // Convert payload to string
    let payload_str = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_payload",
                "Invalid webhook payload encoding",
            ))
        }
    };

    // Verify and parse webhook event
    let event = match stripe_service.verify_webhook(payload_str, signature) {
        Ok(e) => e,
        Err(e) => {
            warn!("Webhook verification failed: {}", e);
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_signature",
                "Webhook signature verification failed",
            ));
        }
    };

    // Process event
    use crate::services::WebhookEvent;

    match event {
        WebhookEvent::CheckoutCompleted {
            session_id,
            amount_paid,
            organization_id,
            ..
        } => {
            info!(
                session_id = %session_id,
                amount_paid = amount_paid,
                organization_id = %organization_id,
                "Processing checkout completed"
            );

            // Convert cents to micro-USDC
            let micro_usdc = StripeService::cents_to_micro_usdc(amount_paid);

            // SECURITY: Use atomic transaction with idempotency guarantee
            // This prevents race conditions where two concurrent webhook requests
            // could both pass an EXISTS check before either records the transaction
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to begin transaction: {}", e);
                    return HttpResponse::InternalServerError()
                        .json(ErrorResponse::new("internal_error", "Database error"));
                }
            };

            // Add credits first (will be rolled back if transaction insert fails)
            let new_balance =
                match CreditRepository::add_credits(&mut *tx, &organization_id, micro_usdc).await {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("Failed to add credits: {}", e);
                        return HttpResponse::InternalServerError().json(ErrorResponse::new(
                            "internal_error",
                            "Failed to add credits",
                        ));
                    }
                };

            // SECURITY: Atomic idempotency check using INSERT ... ON CONFLICT DO NOTHING
            // This prevents the TOCTOU race condition in webhook replay attacks
            let tx_result = match TransactionRepository::create_idempotent(
                &mut *tx,
                crate::repositories::billing::CreateTransactionParams {
                    organization_id: &organization_id,
                    amount: micro_usdc,
                    transaction_type: "purchase",
                    description: Some("Stripe checkout purchase"),
                    reference_id: Some(&session_id),
                    balance_after: new_balance,
                    metadata: None,
                },
            )
            .await
            {
                Ok(Some(created_tx)) => created_tx,
                Ok(None) => {
                    // Transaction already exists - this is a duplicate webhook
                    // Rollback the credit addition (it was a duplicate)
                    info!(
                        session_id = %session_id,
                        "Webhook already processed, rolling back (atomic idempotency)"
                    );
                    // tx is automatically rolled back when dropped
                    return HttpResponse::Ok().json(serde_json::json!({
                        "received": true,
                        "status": "duplicate"
                    }));
                }
                Err(e) => {
                    warn!("Failed to record transaction: {}", e);
                    return HttpResponse::InternalServerError().json(ErrorResponse::new(
                        "internal_error",
                        "Failed to record transaction",
                    ));
                }
            };

            // Commit transaction - credits and transaction record are now persisted atomically
            if let Err(e) = tx.commit().await {
                warn!("Failed to commit transaction: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to complete transaction",
                ));
            }

            info!(
                organization_id = %organization_id,
                amount = micro_usdc,
                new_balance = new_balance,
                transaction_id = %tx_result.id,
                "Credits added successfully (atomic)"
            );
        }

        WebhookEvent::PaymentFailed {
            session_id,
            organization_id,
            error_message,
        } => {
            warn!(
                session_id = %session_id,
                organization_id = %organization_id,
                error = ?error_message,
                "Payment failed"
            );
            // Log the failure but don't modify credits
        }

        WebhookEvent::SubscriptionCreated {
            subscription_id,
            customer_id,
            plan,
            organization_id,
        } => {
            info!(
                subscription_id = %subscription_id,
                organization_id = %organization_id,
                plan = %plan,
                "Subscription created"
            );

            if let Err(e) = SubscriptionRepository::upsert(
                &**pool,
                &organization_id,
                &subscription_id,
                &customer_id,
                &plan,
                "active",
            )
            .await
            {
                warn!("Failed to create subscription record: {}", e);
            }
        }

        WebhookEvent::SubscriptionUpdated {
            subscription_id,
            status,
            ..
        } => {
            info!(
                subscription_id = %subscription_id,
                status = %status,
                "Subscription updated"
            );

            if let Err(e) =
                SubscriptionRepository::update_status(&**pool, &subscription_id, &status).await
            {
                warn!("Failed to update subscription status: {}", e);
            }
        }

        WebhookEvent::SubscriptionCanceled {
            subscription_id, ..
        } => {
            info!(subscription_id = %subscription_id, "Subscription canceled");

            if let Err(e) = SubscriptionRepository::cancel(&**pool, &subscription_id).await {
                warn!("Failed to cancel subscription: {}", e);
            }
        }

        WebhookEvent::Unknown(event_type) => {
            info!(event_type = %event_type, "Ignoring unknown webhook event");
        }
    }

    HttpResponse::Ok().json(serde_json::json!({"received": true}))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get Stripe configuration from environment with format validation
///
/// # Security
/// Validates that Stripe keys have the correct format to catch misconfiguration early.
fn get_stripe_config(_config: &Config) -> Result<StripeConfig, &'static str> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|_| "STRIPE_SECRET_KEY not set")?;

    // SECURITY: Validate secret key format
    if !secret_key.starts_with("sk_test_") && !secret_key.starts_with("sk_live_") {
        return Err("STRIPE_SECRET_KEY must start with sk_test_ or sk_live_");
    }

    let webhook_secret =
        std::env::var("STRIPE_WEBHOOK_SECRET").map_err(|_| "STRIPE_WEBHOOK_SECRET not set")?;

    // SECURITY: Validate webhook secret format
    if !webhook_secret.starts_with("whsec_") {
        return Err("STRIPE_WEBHOOK_SECRET must start with whsec_");
    }

    Ok(StripeConfig {
        secret_key,
        webhook_secret,
        currency: "usd".to_string(),
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // IP Whitelist Tests
    // ========================================================================

    #[test]
    fn test_is_stripe_ip_valid_stripe_ip() {
        // All IPs from STRIPE_WEBHOOK_IP_RANGES should be valid
        assert!(is_stripe_ip("3.18.12.63"));
        assert!(is_stripe_ip("3.130.192.231"));
        assert!(is_stripe_ip("13.235.14.237"));
        assert!(is_stripe_ip("54.88.130.119"));
        assert!(is_stripe_ip("54.187.216.72"));
    }

    #[test]
    fn test_is_stripe_ip_invalid_ip() {
        // Random IPs should not be in whitelist
        assert!(!is_stripe_ip("1.2.3.4"));
        assert!(!is_stripe_ip("192.168.1.1"));
        assert!(!is_stripe_ip("10.0.0.1"));
        assert!(!is_stripe_ip("8.8.8.8"));
    }

    #[test]
    fn test_is_stripe_ip_localhost() {
        // Localhost should not be in whitelist
        assert!(!is_stripe_ip("127.0.0.1"));
        assert!(!is_stripe_ip("::1"));
    }

    #[test]
    fn test_is_stripe_ip_malformed() {
        // Malformed IPs should return false, not panic
        assert!(!is_stripe_ip("not-an-ip"));
        assert!(!is_stripe_ip(""));
        assert!(!is_stripe_ip("256.256.256.256"));
        assert!(!is_stripe_ip("1.2.3.4.5"));
    }

    #[test]
    fn test_ip_in_cidr_exact_match() {
        let ip = IpAddr::from_str("3.18.12.63").unwrap();
        assert!(ip_in_cidr(&ip, "3.18.12.63/32"));
    }

    #[test]
    fn test_ip_in_cidr_network_range() {
        let ip1 = IpAddr::from_str("10.0.0.1").unwrap();
        let ip2 = IpAddr::from_str("10.0.0.254").unwrap();
        let ip_outside = IpAddr::from_str("10.0.1.1").unwrap();

        assert!(ip_in_cidr(&ip1, "10.0.0.0/24"));
        assert!(ip_in_cidr(&ip2, "10.0.0.0/24"));
        assert!(!ip_in_cidr(&ip_outside, "10.0.0.0/24"));
    }

    #[test]
    fn test_ip_in_cidr_wide_range() {
        let ip = IpAddr::from_str("192.168.100.50").unwrap();
        assert!(ip_in_cidr(&ip, "192.168.0.0/16"));

        let ip_outside = IpAddr::from_str("192.169.0.1").unwrap();
        assert!(!ip_in_cidr(&ip_outside, "192.168.0.0/16"));
    }

    #[test]
    fn test_ip_in_cidr_ipv6() {
        let ip = IpAddr::from_str("2001:db8::1").unwrap();
        assert!(ip_in_cidr(&ip, "2001:db8::/32"));

        let ip_outside = IpAddr::from_str("2001:db9::1").unwrap();
        assert!(!ip_in_cidr(&ip_outside, "2001:db8::/32"));
    }

    #[test]
    fn test_ip_in_cidr_invalid_cidr() {
        let ip = IpAddr::from_str("10.0.0.1").unwrap();

        // Missing prefix length
        assert!(!ip_in_cidr(&ip, "10.0.0.0"));

        // Invalid prefix length
        assert!(!ip_in_cidr(&ip, "10.0.0.0/33"));

        // Malformed CIDR
        assert!(!ip_in_cidr(&ip, "not-a-cidr/24"));
    }

    #[test]
    fn test_ip_in_cidr_type_mismatch() {
        // IPv4 address against IPv6 CIDR
        let ipv4 = IpAddr::from_str("10.0.0.1").unwrap();
        assert!(!ip_in_cidr(&ipv4, "2001:db8::/32"));

        // IPv6 address against IPv4 CIDR
        let ipv6 = IpAddr::from_str("2001:db8::1").unwrap();
        assert!(!ip_in_cidr(&ipv6, "10.0.0.0/8"));
    }

    #[test]
    fn test_ip_in_cidr_zero_prefix() {
        // /0 should match everything (of the same IP family)
        let ipv4 = IpAddr::from_str("1.2.3.4").unwrap();
        assert!(ip_in_cidr(&ipv4, "0.0.0.0/0"));

        let ipv6 = IpAddr::from_str("2001:db8::1").unwrap();
        assert!(ip_in_cidr(&ipv6, "::/0"));
    }
}
