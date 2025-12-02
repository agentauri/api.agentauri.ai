//! Repository layer for database access

pub mod actions;
pub mod agent_links;
pub mod api_keys;
pub mod billing;
pub mod conditions;
pub mod oauth;
pub mod organizations;
pub mod triggers;
pub mod user_identities;
pub mod users;
pub mod wallet;

// Re-exports for commonly used repositories
pub use actions::ActionRepository;
pub use agent_links::AgentLinkRepository;
pub use api_keys::{ApiKeyAuditRepository, ApiKeyRepository, AuthFailureRepository};
pub use conditions::ConditionRepository;
pub use oauth::{OAuthClientRepository, OAuthTokenRepository};
pub use organizations::{MemberRepository, OrganizationRepository, OrganizationWithRole};
pub use triggers::TriggerRepository;
pub use user_identities::UserIdentityRepository;
pub use users::UserRepository;

// Billing and wallet repositories are accessed via their modules
// when needed (e.g., crate::repositories::billing::CreditRepository)
