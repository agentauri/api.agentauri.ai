//! Business logic services for the API gateway
//!
//! This module contains services that encapsulate business logic
//! separate from HTTP handlers and database access.

pub mod a2a_task_processor;
pub mod api_key_service;
pub mod auth_rate_limiter;
pub mod oauth_client_service;
pub mod oauth_token_service;
pub mod query_executor;
pub mod social_auth_service;
pub mod stripe_service;
pub mod wallet_service;

pub use a2a_task_processor::{start_a2a_task_processor, A2aTaskProcessor, A2aTaskProcessorConfig};
pub use api_key_service::ApiKeyService;
pub use auth_rate_limiter::AuthRateLimiter;
pub use oauth_client_service::OAuthClientService;
pub use oauth_token_service::OAuthTokenService;
pub use query_executor::QueryExecutor;
pub use social_auth_service::{OAuthUserProfile, SocialAuthError, SocialAuthService};
pub use stripe_service::{StripeConfig, StripeService, WebhookEvent};
#[allow(unused_imports)] // ChainConfig used in main.rs
pub use wallet_service::{ChainConfig, WalletService};
