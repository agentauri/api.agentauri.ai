//! Business logic services for the API gateway
//!
//! This module contains services that encapsulate business logic
//! separate from HTTP handlers and database access.

pub mod api_key_service;
pub mod auth_rate_limiter;
pub mod oauth_client_service;
pub mod oauth_token_service;
pub mod stripe_service;
pub mod wallet_service;

pub use api_key_service::ApiKeyService;
pub use auth_rate_limiter::AuthRateLimiter;
pub use oauth_client_service::OAuthClientService;
pub use oauth_token_service::OAuthTokenService;
pub use stripe_service::{StripeConfig, StripeService, WebhookEvent};
#[allow(unused_imports)] // ChainConfig used in main.rs
pub use wallet_service::{ChainConfig, WalletService};
