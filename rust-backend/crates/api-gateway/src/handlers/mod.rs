//! Request handlers for API endpoints

pub mod a2a;
pub mod actions;
pub mod agent_follows;
pub mod agents;
pub mod api_keys;
pub mod auth;
pub mod billing;
pub mod circuit_breaker;
pub mod conditions;
pub mod discovery;
pub mod events;
pub mod health;
pub mod helpers;
pub mod oauth;
pub mod organizations;
pub mod ponder;
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
    __path_link_agent, __path_list_linked_agents, __path_list_org_agents, __path_unlink_agent,
    link_agent, list_linked_agents, list_org_agents, unlink_agent,
};

// Explicitly re-export agent follow handlers
pub use agent_follows::{
    __path_follow_agent, __path_list_following, __path_unfollow_agent, __path_update_follow,
    follow_agent, list_following, unfollow_agent, update_follow,
};

// Explicitly re-export billing handlers
pub use billing::{
    __path_get_credits, __path_get_org_credits, __path_get_subscription,
    __path_handle_stripe_webhook, __path_list_org_transactions, __path_list_transactions,
    __path_purchase_credits, get_credits, get_org_credits, get_subscription, handle_stripe_webhook,
    list_org_transactions, list_transactions, purchase_credits,
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
    __path_link_github, __path_link_google, github_auth, github_callback, google_auth,
    google_callback, link_github, link_google,
};

// Explicitly re-export Ponder handlers
pub use ponder::{
    __path_get_ponder_events, __path_get_ponder_status, get_ponder_events, get_ponder_status,
};

// Explicitly re-export A2A Protocol handlers
pub use a2a::{
    __path_a2a_rpc, __path_get_task_status, __path_stream_task_progress, a2a_rpc, get_task_status,
    stream_task_progress,
};

// Explicitly re-export Events handlers
pub use events::{__path_list_events, list_events};

// Note: helpers module is not re-exported to avoid polluting the namespace
// Import helpers directly: use crate::handlers::helpers::{...}
