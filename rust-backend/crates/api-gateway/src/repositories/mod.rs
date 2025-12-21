//! Repository layer for database access

pub mod a2a_tasks;
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
pub use a2a_tasks::A2aTaskRepository;
pub use actions::ActionRepository;
pub use agent_links::AgentLinkRepository;
pub use api_keys::{ApiKeyAuditRepository, ApiKeyRepository, AuthFailureRepository};
pub use billing::CreditRepository;
pub use conditions::ConditionRepository;
pub use oauth::{OAuthClientRepository, OAuthTokenRepository};
pub use organizations::{MemberRepository, OrganizationRepository, OrganizationWithRole};
pub use triggers::TriggerRepository;
pub use user_identities::UserIdentityRepository;
pub use users::UserRepository;
