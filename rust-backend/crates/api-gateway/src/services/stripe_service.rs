//! Stripe Payment Service
//!
//! This service handles Stripe integration for credit purchases and subscriptions.
//!
//! # Features
//!
//! - **Checkout Session creation**: Creates Stripe Checkout sessions for credit purchases
//! - **Webhook verification**: Verifies and processes Stripe webhook events
//! - **Subscription management**: Manages subscription lifecycle events
//!
//! # Security
//!
//! - Webhook signatures are verified using the Stripe-Signature header
//! - All amounts are in micro-USDC (6 decimals) internally, converted to cents for Stripe

use chrono::{DateTime, Utc};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, Currency, EventType, Webhook,
};
use thiserror::Error;

/// Micro-USDC per whole USDC (6 decimals)
const MICRO_USDC_MULTIPLIER: i64 = 1_000_000;

/// Cents per dollar (for Stripe)
const CENTS_PER_DOLLAR: i64 = 100;

/// Errors that can occur during Stripe operations
#[derive(Debug, Error)]
pub enum StripeError {
    #[error("Stripe API error: {0}")]
    ApiError(String),

    #[error("Invalid webhook signature")]
    InvalidSignature,

    #[error("Webhook processing failed: {0}")]
    WebhookError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result of creating a checkout session
#[derive(Debug, Clone)]
pub struct CheckoutResult {
    /// Stripe session ID
    pub session_id: String,
    /// Checkout URL for redirect
    pub checkout_url: String,
}

/// Parsed webhook event
#[derive(Debug)]
pub enum WebhookEvent {
    /// Checkout session completed successfully
    CheckoutCompleted {
        session_id: String,
        customer_id: Option<String>,
        amount_paid: i64,
        organization_id: String,
    },
    /// Payment failed
    PaymentFailed {
        session_id: String,
        organization_id: String,
        error_message: Option<String>,
    },
    /// Subscription created
    SubscriptionCreated {
        subscription_id: String,
        customer_id: String,
        plan: String,
        organization_id: String,
    },
    /// Subscription updated
    SubscriptionUpdated {
        subscription_id: String,
        status: String,
        current_period_end: DateTime<Utc>,
    },
    /// Subscription canceled
    SubscriptionCanceled {
        subscription_id: String,
        canceled_at: DateTime<Utc>,
    },
    /// Unknown event type (ignored)
    Unknown(String),
}

/// Configuration for the Stripe service
#[derive(Debug, Clone)]
pub struct StripeConfig {
    /// Stripe secret key
    pub secret_key: String,
    /// Webhook signing secret
    pub webhook_secret: String,
    /// Default currency (e.g., "usd")
    pub currency: String,
}

/// Service for Stripe payment operations
#[derive(Clone)]
pub struct StripeService {
    client: Client,
    webhook_secret: String,
    currency: Currency,
}

impl StripeService {
    /// Create a new StripeService with the given configuration
    pub fn new(config: StripeConfig) -> Result<Self, StripeError> {
        let client = Client::new(config.secret_key);

        let currency = match config.currency.to_lowercase().as_str() {
            "usd" => Currency::USD,
            "eur" => Currency::EUR,
            "gbp" => Currency::GBP,
            _ => {
                return Err(StripeError::ConfigError(format!(
                    "Unsupported currency: {}",
                    config.currency
                )))
            }
        };

        Ok(Self {
            client,
            webhook_secret: config.webhook_secret,
            currency,
        })
    }

    /// Create a checkout session for credit purchase
    ///
    /// # Arguments
    /// * `organization_id` - The organization purchasing credits
    /// * `amount_usdc` - Amount in whole USDC (e.g., 100 for $100)
    /// * `success_url` - URL to redirect after successful payment
    /// * `cancel_url` - URL to redirect after canceled payment
    ///
    /// # Returns
    /// A `CheckoutResult` with session ID and checkout URL
    pub async fn create_checkout_session(
        &self,
        organization_id: &str,
        amount_usdc: i64,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<CheckoutResult, StripeError> {
        if amount_usdc <= 0 {
            return Err(StripeError::InvalidAmount(
                "Amount must be positive".to_string(),
            ));
        }

        // Convert USDC to cents (1 USDC = 100 cents)
        let amount_cents = amount_usdc * CENTS_PER_DOLLAR;

        // Create line item for credit purchase
        let line_items = vec![CreateCheckoutSessionLineItems {
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency: self.currency,
                unit_amount: Some(amount_cents),
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: format!("{} USDC Credits", amount_usdc),
                    description: Some(format!(
                        "API credits for ERC-8004 platform ({} USDC)",
                        amount_usdc
                    )),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            quantity: Some(1),
            ..Default::default()
        }];

        // Build checkout session params
        let mut params = CreateCheckoutSession::new();
        params.mode = Some(CheckoutSessionMode::Payment);
        params.line_items = Some(line_items);
        params.success_url = Some(success_url);
        params.cancel_url = Some(cancel_url);
        params.client_reference_id = Some(organization_id);

        // Add metadata for tracking
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("organization_id".to_string(), organization_id.to_string());
        metadata.insert("amount_usdc".to_string(), amount_usdc.to_string());
        metadata.insert(
            "amount_micro_usdc".to_string(),
            (amount_usdc * MICRO_USDC_MULTIPLIER).to_string(),
        );
        params.metadata = Some(metadata);

        // Create the session
        let session = CheckoutSession::create(&self.client, params)
            .await
            .map_err(|e| StripeError::ApiError(e.to_string()))?;

        let session_id = session.id.to_string();
        let checkout_url = session
            .url
            .ok_or_else(|| StripeError::MissingField("checkout URL".to_string()))?;

        Ok(CheckoutResult {
            session_id,
            checkout_url,
        })
    }

    /// Verify and parse a Stripe webhook event
    ///
    /// # Arguments
    /// * `payload` - Raw webhook payload body
    /// * `signature` - Stripe-Signature header value
    ///
    /// # Returns
    /// A parsed `WebhookEvent`
    pub fn verify_webhook(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<WebhookEvent, StripeError> {
        // Verify signature
        let event = Webhook::construct_event(payload, signature, &self.webhook_secret)
            .map_err(|_| StripeError::InvalidSignature)?;

        // Parse event type
        match event.type_ {
            EventType::CheckoutSessionCompleted => self.parse_checkout_completed(&event),
            EventType::CheckoutSessionAsyncPaymentFailed => self.parse_payment_failed(&event),
            EventType::CustomerSubscriptionCreated => self.parse_subscription_created(&event),
            EventType::CustomerSubscriptionUpdated => self.parse_subscription_updated(&event),
            EventType::CustomerSubscriptionDeleted => self.parse_subscription_canceled(&event),
            other => Ok(WebhookEvent::Unknown(format!("{:?}", other))),
        }
    }

    /// Parse a checkout.session.completed event
    fn parse_checkout_completed(&self, event: &stripe::Event) -> Result<WebhookEvent, StripeError> {
        // Extract session from event object
        let session = match &event.data.object {
            stripe::EventObject::CheckoutSession(s) => s,
            _ => {
                return Err(StripeError::WebhookError(
                    "Expected CheckoutSession object".to_string(),
                ))
            }
        };

        let session_id = session.id.to_string();
        let customer_id = session.customer.as_ref().map(|c| c.id().to_string());
        let amount_paid = session.amount_total.unwrap_or(0);

        // Get organization_id from metadata or client_reference_id
        let organization_id = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("organization_id"))
            .map(|s| s.to_string())
            .or_else(|| session.client_reference_id.clone())
            .ok_or_else(|| {
                StripeError::WebhookError("Missing organization_id in session".to_string())
            })?;

        Ok(WebhookEvent::CheckoutCompleted {
            session_id,
            customer_id,
            amount_paid,
            organization_id,
        })
    }

    /// Parse a checkout.session.async_payment_failed event
    fn parse_payment_failed(&self, event: &stripe::Event) -> Result<WebhookEvent, StripeError> {
        let session = match &event.data.object {
            stripe::EventObject::CheckoutSession(s) => s,
            _ => {
                return Err(StripeError::WebhookError(
                    "Expected CheckoutSession object".to_string(),
                ))
            }
        };

        let session_id = session.id.to_string();
        let organization_id = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("organization_id"))
            .map(|s| s.to_string())
            .or_else(|| session.client_reference_id.clone())
            .ok_or_else(|| {
                StripeError::WebhookError("Missing organization_id in session".to_string())
            })?;

        Ok(WebhookEvent::PaymentFailed {
            session_id,
            organization_id,
            error_message: None, // Detailed error would be in payment_intent
        })
    }

    /// Parse a customer.subscription.created event
    fn parse_subscription_created(
        &self,
        event: &stripe::Event,
    ) -> Result<WebhookEvent, StripeError> {
        let subscription = match &event.data.object {
            stripe::EventObject::Subscription(s) => s,
            _ => {
                return Err(StripeError::WebhookError(
                    "Expected Subscription object".to_string(),
                ))
            }
        };

        let subscription_id = subscription.id.to_string();
        let customer_id = subscription.customer.id().to_string();

        // Get plan from items (first item's price)
        let plan = subscription
            .items
            .data
            .first()
            .and_then(|item| item.price.as_ref())
            .and_then(|price| price.nickname.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let organization_id = subscription
            .metadata
            .get("organization_id")
            .map(|s| s.to_string())
            .ok_or_else(|| {
                StripeError::WebhookError("Missing organization_id in subscription".to_string())
            })?;

        Ok(WebhookEvent::SubscriptionCreated {
            subscription_id,
            customer_id,
            plan,
            organization_id,
        })
    }

    /// Parse a customer.subscription.updated event
    fn parse_subscription_updated(
        &self,
        event: &stripe::Event,
    ) -> Result<WebhookEvent, StripeError> {
        let subscription = match &event.data.object {
            stripe::EventObject::Subscription(s) => s,
            _ => {
                return Err(StripeError::WebhookError(
                    "Expected Subscription object".to_string(),
                ))
            }
        };

        let subscription_id = subscription.id.to_string();
        let status = format!("{:?}", subscription.status);

        // Convert timestamp to DateTime
        let current_period_end = DateTime::from_timestamp(subscription.current_period_end, 0)
            .ok_or_else(|| StripeError::WebhookError("Invalid timestamp".to_string()))?;

        Ok(WebhookEvent::SubscriptionUpdated {
            subscription_id,
            status,
            current_period_end,
        })
    }

    /// Parse a customer.subscription.deleted event
    fn parse_subscription_canceled(
        &self,
        event: &stripe::Event,
    ) -> Result<WebhookEvent, StripeError> {
        let subscription = match &event.data.object {
            stripe::EventObject::Subscription(s) => s,
            _ => {
                return Err(StripeError::WebhookError(
                    "Expected Subscription object".to_string(),
                ))
            }
        };

        let subscription_id = subscription.id.to_string();
        let canceled_at = subscription
            .canceled_at
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .unwrap_or_else(Utc::now);

        Ok(WebhookEvent::SubscriptionCanceled {
            subscription_id,
            canceled_at,
        })
    }

    /// Convert cents to micro-USDC
    ///
    /// # Arguments
    /// * `cents` - Amount in cents (e.g., 10000 for $100)
    ///
    /// # Returns
    /// Amount in micro-USDC (e.g., 100_000_000 for $100)
    pub fn cents_to_micro_usdc(cents: i64) -> i64 {
        // 1 dollar = 100 cents = 1 USDC = 1_000_000 micro-USDC
        // So 1 cent = 10_000 micro-USDC
        cents * (MICRO_USDC_MULTIPLIER / CENTS_PER_DOLLAR)
    }

    /// Convert micro-USDC to cents
    ///
    /// # Arguments
    /// * `micro_usdc` - Amount in micro-USDC
    ///
    /// # Returns
    /// Amount in cents
    #[allow(dead_code)]
    pub fn micro_usdc_to_cents(micro_usdc: i64) -> i64 {
        micro_usdc / (MICRO_USDC_MULTIPLIER / CENTS_PER_DOLLAR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Conversion tests
    // ========================================================================

    #[test]
    fn test_cents_to_micro_usdc() {
        // $1 = 100 cents = 1_000_000 micro-USDC
        assert_eq!(StripeService::cents_to_micro_usdc(100), 1_000_000);

        // $10 = 1000 cents = 10_000_000 micro-USDC
        assert_eq!(StripeService::cents_to_micro_usdc(1000), 10_000_000);

        // $100 = 10000 cents = 100_000_000 micro-USDC
        assert_eq!(StripeService::cents_to_micro_usdc(10000), 100_000_000);

        // 1 cent = 10_000 micro-USDC
        assert_eq!(StripeService::cents_to_micro_usdc(1), 10_000);
    }

    #[test]
    fn test_micro_usdc_to_cents() {
        // 1_000_000 micro-USDC = $1 = 100 cents
        assert_eq!(StripeService::micro_usdc_to_cents(1_000_000), 100);

        // 10_000_000 micro-USDC = $10 = 1000 cents
        assert_eq!(StripeService::micro_usdc_to_cents(10_000_000), 1000);

        // 10_000 micro-USDC = 1 cent
        assert_eq!(StripeService::micro_usdc_to_cents(10_000), 1);
    }

    #[test]
    fn test_conversion_roundtrip() {
        for cents in [1, 10, 100, 1000, 10000] {
            let micro = StripeService::cents_to_micro_usdc(cents);
            let back = StripeService::micro_usdc_to_cents(micro);
            assert_eq!(cents, back, "Roundtrip failed for {} cents", cents);
        }
    }

    // ========================================================================
    // Webhook event parsing tests (mocked)
    // ========================================================================

    #[test]
    fn test_webhook_event_unknown() {
        // Unknown events should be handled gracefully
        let event = WebhookEvent::Unknown("some.unknown.event".to_string());
        match event {
            WebhookEvent::Unknown(name) => {
                assert!(name.contains("unknown"));
            }
            _ => panic!("Expected Unknown event"),
        }
    }

    // ========================================================================
    // Config tests
    // ========================================================================

    #[test]
    fn test_stripe_config() {
        let config = StripeConfig {
            secret_key: "sk_test_xxx".to_string(),
            webhook_secret: "whsec_xxx".to_string(),
            currency: "usd".to_string(),
        };

        assert_eq!(config.secret_key, "sk_test_xxx");
        assert_eq!(config.webhook_secret, "whsec_xxx");
        assert_eq!(config.currency, "usd");
    }
}
