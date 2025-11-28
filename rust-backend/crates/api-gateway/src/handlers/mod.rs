//! Request handlers for API endpoints

pub mod actions;
pub mod agents;
pub mod api_keys;
pub mod auth;
pub mod billing;
pub mod conditions;
pub mod health;
pub mod helpers;
pub mod oauth;
pub mod organizations;
pub mod triggers;

// Re-export commonly used handlers
pub use actions::*;
pub use api_keys::*;
pub use auth::*;
pub use conditions::*;
pub use health::*;
pub use organizations::*;
pub use triggers::*;

// Explicitly re-export agent handlers (avoid OrgIdQuery conflict with billing)
pub use agents::{link_agent, list_linked_agents, unlink_agent};

// Explicitly re-export billing handlers
pub use billing::{
    get_credits, get_subscription, handle_stripe_webhook, list_transactions, purchase_credits,
};

// Explicitly re-export OAuth handlers
pub use oauth::{
    create_oauth_client, delete_oauth_client, list_oauth_clients, token_endpoint,
};

// Note: helpers module is not re-exported to avoid polluting the namespace
// Import helpers directly: use crate::handlers::helpers::{...}
