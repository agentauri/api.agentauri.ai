//! Request handlers for API endpoints

pub mod actions;
pub mod agents;
pub mod api_keys;
pub mod auth;
pub mod billing;
pub mod circuit_breaker;
pub mod conditions;
pub mod discovery;
pub mod health;
pub mod helpers;
pub mod oauth;
pub mod organizations;
pub mod social_auth;
pub mod triggers;

// Re-export commonly used handlers
pub use actions::*;
pub use api_keys::*;
pub use auth::*;
pub use circuit_breaker::*;
pub use conditions::*;
pub use discovery::*;
pub use health::*;
pub use organizations::*;
pub use triggers::*;

// Note: For utoipa to work properly with #[utoipa::path] macros, we need to use
// wildcard re-exports so the generated __path_* types are also accessible.

// Explicitly re-export agent handlers (avoid OrgIdQuery conflict with billing)
pub use agents::{
    __path_link_agent, __path_list_linked_agents, __path_unlink_agent, link_agent,
    list_linked_agents, unlink_agent,
};

// Explicitly re-export billing handlers
pub use billing::{
    __path_get_credits, __path_get_subscription, __path_handle_stripe_webhook,
    __path_list_transactions, __path_purchase_credits, get_credits, get_subscription,
    handle_stripe_webhook, list_transactions, purchase_credits,
};

// Explicitly re-export OAuth handlers
pub use oauth::{
    __path_create_oauth_client, __path_delete_oauth_client, __path_list_oauth_clients,
    __path_token_endpoint, create_oauth_client, delete_oauth_client, list_oauth_clients,
    token_endpoint,
};

// Explicitly re-export social auth handlers
pub use social_auth::{
    __path_github_auth, __path_github_callback, __path_google_auth, __path_google_callback,
    github_auth, github_callback, google_auth, google_callback,
};

// Note: helpers module is not re-exported to avoid polluting the namespace
// Import helpers directly: use crate::handlers::helpers::{...}
