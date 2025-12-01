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

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::{Config, DbPool};
use tracing::{info, warn};

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

// ============================================================================
// Purchase Credits Handler
// ============================================================================

/// Request body wrapper for purchase that includes organization_id
#[derive(Debug, serde::Deserialize, validator::Validate)]
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
pub async fn handle_stripe_webhook(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req_http: HttpRequest,
    payload: web::Bytes,
) -> impl Responder {
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

            // SECURITY: Check if this session was already processed (idempotency)
            match TransactionRepository::exists_by_reference_id(&pool, &session_id).await {
                Ok(true) => {
                    info!(
                        session_id = %session_id,
                        "Webhook already processed, skipping (idempotency check)"
                    );
                    return HttpResponse::Ok().json(serde_json::json!({
                        "received": true,
                        "status": "duplicate"
                    }));
                }
                Ok(false) => {} // Continue processing
                Err(e) => {
                    warn!("Failed to check transaction existence: {}", e);
                    return HttpResponse::InternalServerError().json(ErrorResponse::new(
                        "internal_error",
                        "Failed to verify webhook idempotency",
                    ));
                }
            }

            // Convert cents to micro-USDC
            let micro_usdc = StripeService::cents_to_micro_usdc(amount_paid);

            // Add credits in a transaction
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to begin transaction: {}", e);
                    return HttpResponse::InternalServerError()
                        .json(ErrorResponse::new("internal_error", "Database error"));
                }
            };

            // Add credits
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

            // Record transaction
            if let Err(e) = TransactionRepository::create(
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
                warn!("Failed to record transaction: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to record transaction",
                ));
            }

            // Commit transaction
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
                "Credits added successfully"
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
