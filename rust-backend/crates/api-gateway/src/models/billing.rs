//! Billing and credits DTOs
//!
//! **Note**: Stripe integration and x402 crypto payment features are partially implemented.
//! Some DTOs and database models are defined but not yet fully integrated with handlers.
//! These will be completed in Phase 4-5.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ============================================================================
// Credit Balance DTOs
// ============================================================================

/// Response containing current credit balance
#[derive(Debug, Serialize)]
pub struct CreditBalanceResponse {
    /// Current balance in micro-USDC (6 decimals)
    pub balance: i64,
    /// Currency (always "USDC")
    pub currency: String,
    /// Balance in human-readable format
    pub balance_formatted: String,
}

impl CreditBalanceResponse {
    /// Create a new credit balance response
    pub fn new(balance: i64) -> Self {
        Self {
            balance,
            currency: "USDC".to_string(),
            balance_formatted: format_micro_usdc(balance),
        }
    }
}

/// Format micro-USDC to human-readable string (e.g., 1000000 -> "1.00 USDC")
fn format_micro_usdc(micro: i64) -> String {
    let whole = micro / 1_000_000;
    let frac = (micro % 1_000_000).abs();
    format!("{}.{:06} USDC", whole, frac)
}

// ============================================================================
// Credit Transaction DTOs
// ============================================================================

/// Response containing a credit transaction
#[derive(Debug, Serialize)]
pub struct CreditTransactionResponse {
    /// Transaction ID
    pub id: i64,
    /// Amount in micro-USDC (positive for additions, negative for deductions)
    pub amount: i64,
    /// Transaction type
    pub transaction_type: String,
    /// Optional description
    pub description: Option<String>,
    /// External reference ID (Stripe payment ID, etc.)
    pub reference_id: Option<String>,
    /// Balance after transaction
    pub balance_after: i64,
    /// Transaction timestamp
    pub created_at: DateTime<Utc>,
}

/// Query parameters for listing transactions
#[derive(Debug, Deserialize, Validate)]
pub struct TransactionListQuery {
    /// Maximum number of transactions to return
    #[validate(range(min = 1, max = 100))]
    #[serde(default = "default_limit")]
    pub limit: i64,

    /// Number of transactions to skip
    #[validate(range(min = 0))]
    #[serde(default)]
    pub offset: i64,

    /// Filter by transaction type
    pub transaction_type: Option<String>,
}

fn default_limit() -> i64 {
    20
}

// ============================================================================
// Purchase Credits DTOs
// ============================================================================

/// Request to purchase credits via Stripe
#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)] // Future feature: Stripe checkout integration
pub struct PurchaseCreditsRequest {
    /// Amount in USDC (whole units, e.g., 10 for $10)
    #[validate(range(min = 1, max = 10000))]
    pub amount: i64,

    /// URL to redirect after successful payment
    #[validate(url)]
    pub success_url: String,

    /// URL to redirect after canceled payment
    #[validate(url)]
    pub cancel_url: String,
}

/// Response containing Stripe checkout session info
#[derive(Debug, Serialize)]
pub struct PurchaseCreditsResponse {
    /// Stripe checkout session URL
    pub checkout_url: String,
    /// Stripe session ID for tracking
    pub session_id: String,
}

// ============================================================================
// Subscription DTOs
// ============================================================================

/// Response containing subscription details
#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    /// Subscription ID
    pub id: String,
    /// Current plan
    pub plan: String,
    /// Subscription status
    pub status: String,
    /// Current period start
    pub current_period_start: Option<DateTime<Utc>>,
    /// Current period end
    pub current_period_end: Option<DateTime<Utc>>,
    /// Cancelation timestamp (if canceled)
    pub canceled_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Database Models (for repository layer)
// ============================================================================

/// Database model for credits table
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)] // Fields used by SQLx FromRow and in SQL queries
pub struct Credit {
    pub id: String,
    pub organization_id: String,
    pub balance: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database model for credit_transactions table
#[derive(Debug, sqlx::FromRow)]
pub struct CreditTransaction {
    pub id: i64,
    #[allow(dead_code)] // Used in SQL queries and database operations
    pub organization_id: String,
    pub amount: i64,
    pub transaction_type: String,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub balance_after: i64,
    #[allow(dead_code)] // Used in SQL queries and database operations
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<CreditTransaction> for CreditTransactionResponse {
    fn from(tx: CreditTransaction) -> Self {
        Self {
            id: tx.id,
            amount: tx.amount,
            transaction_type: tx.transaction_type,
            description: tx.description,
            reference_id: tx.reference_id,
            balance_after: tx.balance_after,
            created_at: tx.created_at,
        }
    }
}

/// Database model for subscriptions table
#[derive(Debug, sqlx::FromRow)]
pub struct Subscription {
    pub id: String,
    #[allow(dead_code)] // Used in SQL queries and database operations
    pub organization_id: String,
    #[allow(dead_code)] // Used in Stripe webhook integration
    pub stripe_subscription_id: Option<String>,
    #[allow(dead_code)] // Used in Stripe webhook integration
    pub stripe_customer_id: Option<String>,
    pub plan: String,
    pub status: String,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    #[allow(dead_code)] // Used in SQL queries
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)] // Used in SQL queries
    pub updated_at: DateTime<Utc>,
}

impl From<Subscription> for SubscriptionResponse {
    fn from(sub: Subscription) -> Self {
        Self {
            id: sub.id,
            plan: sub.plan,
            status: sub.status,
            current_period_start: sub.current_period_start,
            current_period_end: sub.current_period_end,
            canceled_at: sub.canceled_at,
        }
    }
}

/// Database model for payment_nonces table
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)] // Future feature: x402 crypto payment integration
pub struct PaymentNonce {
    pub id: String,
    pub organization_id: String,
    pub nonce: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub payment_method: String,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // CreditBalanceResponse tests
    // ========================================================================

    #[test]
    fn test_credit_balance_response_formatting() {
        let response = CreditBalanceResponse::new(1_500_000); // 1.5 USDC
        assert_eq!(response.balance, 1_500_000);
        assert_eq!(response.currency, "USDC");
        assert_eq!(response.balance_formatted, "1.500000 USDC");
    }

    #[test]
    fn test_credit_balance_response_zero() {
        let response = CreditBalanceResponse::new(0);
        assert_eq!(response.balance_formatted, "0.000000 USDC");
    }

    #[test]
    fn test_credit_balance_response_large() {
        let response = CreditBalanceResponse::new(1_000_000_000_000); // 1M USDC
        assert_eq!(response.balance_formatted, "1000000.000000 USDC");
    }

    // ========================================================================
    // TransactionListQuery tests
    // ========================================================================

    #[test]
    fn test_transaction_list_query_valid() {
        let query = TransactionListQuery {
            limit: 50,
            offset: 0,
            transaction_type: Some("purchase".to_string()),
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_transaction_list_query_limit_too_large() {
        let query = TransactionListQuery {
            limit: 200,
            offset: 0,
            transaction_type: None,
        };
        let result = query.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_list_query_limit_zero() {
        let query = TransactionListQuery {
            limit: 0,
            offset: 0,
            transaction_type: None,
        };
        let result = query.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_list_query_negative_offset() {
        let query = TransactionListQuery {
            limit: 20,
            offset: -1,
            transaction_type: None,
        };
        let result = query.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // PurchaseCreditsRequest tests
    // ========================================================================

    #[test]
    fn test_purchase_credits_request_valid() {
        let req = PurchaseCreditsRequest {
            amount: 100,
            success_url: "https://example.com/success".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_purchase_credits_request_amount_too_small() {
        let req = PurchaseCreditsRequest {
            amount: 0,
            success_url: "https://example.com/success".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_purchase_credits_request_amount_too_large() {
        let req = PurchaseCreditsRequest {
            amount: 100000,
            success_url: "https://example.com/success".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_purchase_credits_request_invalid_url() {
        let req = PurchaseCreditsRequest {
            amount: 100,
            success_url: "not-a-url".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // Response serialization tests
    // ========================================================================

    #[test]
    fn test_credit_transaction_response_serialization() {
        let response = CreditTransactionResponse {
            id: 1,
            amount: 1_000_000,
            transaction_type: "purchase".to_string(),
            description: Some("Credit purchase".to_string()),
            reference_id: Some("pi_123".to_string()),
            balance_after: 5_000_000,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("purchase"));
        assert!(json.contains("pi_123"));
    }

    #[test]
    fn test_subscription_response_serialization() {
        let response = SubscriptionResponse {
            id: "sub-123".to_string(),
            plan: "pro".to_string(),
            status: "active".to_string(),
            current_period_start: Some(Utc::now()),
            current_period_end: Some(Utc::now()),
            canceled_at: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("pro"));
        assert!(json.contains("active"));
    }
}
