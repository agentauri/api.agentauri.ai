//! Data models matching the PostgreSQL database schema

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// User account
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>, // Optional for social-only users
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub primary_auth_provider: Option<String>, // 'email', 'google', 'github', 'wallet'
    pub avatar_url: Option<String>,
    pub display_name: Option<String>, // Human-friendly name from OAuth or user-set
    // Account lockout fields for brute-force protection
    pub failed_login_attempts: i32,
    #[serde(skip_serializing)] // Don't expose lockout info to client
    pub locked_until: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub last_failed_login: Option<DateTime<Utc>>,
}

/// Organization for multi-tenant account model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub plan: String,
    pub is_personal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization member with role-based access
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrganizationMember {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    pub role: String,
    pub invited_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Agent follow relationship for simplified multi-registry monitoring.
///
/// Creates 3 underlying triggers (identity, reputation, validation) to
/// monitor all activities of a specific ERC-8004 agent.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentFollow {
    pub id: String,
    /// ERC-8004 token ID being followed
    pub agent_id: i64,
    /// Blockchain chain ID
    pub chain_id: i32,
    pub organization_id: String,
    pub user_id: String,
    /// Auto-managed trigger for identity registry events
    pub trigger_identity_id: String,
    /// Auto-managed trigger for reputation registry events
    pub trigger_reputation_id: String,
    /// Auto-managed trigger for validation registry events
    pub trigger_validation_id: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trigger configuration
///
/// Triggers can have a specific `chain_id` or `None` for wildcard matching
/// (matches events from any chain).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trigger {
    pub id: String,
    pub user_id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Chain ID to match, or None for wildcard (matches all chains)
    pub chain_id: Option<i32>,
    pub registry: String,
    pub enabled: bool,
    pub is_stateful: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trigger condition
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerCondition {
    pub id: String, // TEXT in production (UUID stored as text)
    pub trigger_id: String,
    pub condition_type: String,
    pub field: String,
    pub operator: String,
    #[sqlx(json)]
    pub value: serde_json::Value, // JSONB in production
    #[sqlx(json(nullable))]
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Trigger action
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerAction {
    pub id: i32,
    pub trigger_id: String,
    pub action_type: String,
    pub priority: i32,
    #[sqlx(json)]
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Trigger state for stateful triggers
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriggerState {
    pub trigger_id: String,
    #[sqlx(json)]
    pub state_data: serde_json::Value,
    pub last_updated: DateTime<Utc>,
}

/// Blockchain event
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: String,
    pub chain_id: i32,
    pub block_number: i64,
    pub block_hash: String,
    pub transaction_hash: String,
    pub log_index: i32,
    pub registry: String,
    pub event_type: String,
    pub agent_id: Option<i64>,
    pub timestamp: i64,
    // Identity Registry fields
    pub owner: Option<String>,
    pub token_uri: Option<String>,
    pub metadata_key: Option<String>,
    pub metadata_value: Option<String>,
    // Reputation Registry fields
    pub client_address: Option<String>,
    pub feedback_index: Option<i64>,
    pub score: Option<i32>,
    pub tag1: Option<String>,
    pub tag2: Option<String>,
    pub file_uri: Option<String>,
    pub file_hash: Option<String>,
    // Validation Registry fields
    pub validator_address: Option<String>,
    pub request_hash: Option<String>,
    pub response: Option<i32>,
    pub response_uri: Option<String>,
    pub response_hash: Option<String>,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Checkpoint for tracking last processed block per chain
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Checkpoint {
    pub chain_id: i32,
    pub last_block_number: i64,
    pub last_block_hash: String,
    pub updated_at: DateTime<Utc>,
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionResult {
    pub id: String,
    pub job_id: String,
    pub trigger_id: Option<String>,
    pub event_id: Option<String>,
    pub action_type: String,
    pub status: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    #[sqlx(json(nullable))]
    pub response_data: Option<serde_json::Value>,
    pub retry_count: i32,
}

/// API Key for Layer 1 authentication
/// Note: key_hash is intentionally excluded from serialization for security
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: String,
    pub organization_id: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub name: String,
    pub prefix: String,
    pub environment: String,
    pub key_type: String,
    #[sqlx(json)]
    pub permissions: serde_json::Value,
    pub rate_limit_override: Option<i32>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_used_ip: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
    pub revocation_reason: Option<String>,
}

/// API Key environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyEnvironment {
    Live,
    Test,
}

impl ApiKeyEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyEnvironment::Live => "live",
            ApiKeyEnvironment::Test => "test",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            ApiKeyEnvironment::Live => "sk_live_",
            ApiKeyEnvironment::Test => "sk_test_",
        }
    }
}

impl std::str::FromStr for ApiKeyEnvironment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "live" => Ok(ApiKeyEnvironment::Live),
            "test" => Ok(ApiKeyEnvironment::Test),
            _ => Err(format!("Invalid environment: {}", s)),
        }
    }
}

/// API Key type (permission level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyType {
    Standard,
    Restricted,
    Admin,
}

impl ApiKeyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyType::Standard => "standard",
            ApiKeyType::Restricted => "restricted",
            ApiKeyType::Admin => "admin",
        }
    }
}

impl std::str::FromStr for ApiKeyType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(ApiKeyType::Standard),
            "restricted" => Ok(ApiKeyType::Restricted),
            "admin" => Ok(ApiKeyType::Admin),
            _ => Err(format!("Invalid key type: {}", s)),
        }
    }
}

/// API Key audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKeyAuditLog {
    pub id: i64,
    pub api_key_id: Option<String>,
    pub organization_id: String,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub endpoint: Option<String>,
    pub actor_user_id: Option<String>,
    #[sqlx(json(nullable))]
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// API Key audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyAuditEventType {
    Created,
    Used,
    Rotated,
    Revoked,
    AuthFailed,
    RateLimited,
}

impl ApiKeyAuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyAuditEventType::Created => "created",
            ApiKeyAuditEventType::Used => "used",
            ApiKeyAuditEventType::Rotated => "rotated",
            ApiKeyAuditEventType::Revoked => "revoked",
            ApiKeyAuditEventType::AuthFailed => "auth_failed",
            ApiKeyAuditEventType::RateLimited => "rate_limited",
        }
    }
}

/// Authentication failure log entry (without organization context)
///
/// Used for logging authentication failures where we cannot determine
/// the organization, such as invalid key formats or unknown prefixes.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthFailure {
    pub id: i64,
    pub failure_type: String,
    pub key_prefix: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub endpoint: Option<String>,
    #[sqlx(json(nullable))]
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Authentication failure type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthFailureType {
    InvalidFormat,
    PrefixNotFound,
    RateLimited,
    InvalidKey,
}

impl AuthFailureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthFailureType::InvalidFormat => "invalid_format",
            AuthFailureType::PrefixNotFound => "prefix_not_found",
            AuthFailureType::RateLimited => "rate_limited",
            AuthFailureType::InvalidKey => "invalid_key",
        }
    }
}

/// OAuth Client for OAuth 2.0 authentication
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthClient {
    pub id: String,
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret_hash: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub owner_organization_id: String,
    pub grant_types: Vec<String>,
    pub is_trusted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// OAuth Token for OAuth 2.0 authentication
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthToken {
    pub id: String,
    #[serde(skip_serializing)]
    pub access_token_hash: String,
    #[serde(skip_serializing)]
    pub refresh_token_hash: Option<String>,
    pub client_id: String,
    pub user_id: String,
    pub organization_id: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub refresh_token_expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

/// User identity for multi-provider authentication
///
/// Links multiple authentication providers (email, Google, GitHub, wallet)
/// to a single user account, enabling account linking.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserIdentity {
    pub id: String,
    pub user_id: String,

    /// Authentication provider: 'email', 'google', 'github', 'wallet'
    pub provider: String,

    /// Unique identifier from the provider (e.g., Google sub, GitHub id)
    pub provider_user_id: String,

    /// Email from the provider (if available)
    pub email: Option<String>,

    /// Display name from the provider
    pub display_name: Option<String>,

    /// Avatar URL from the provider
    pub avatar_url: Option<String>,

    /// Wallet address (only for provider='wallet')
    pub wallet_address: Option<String>,

    /// Chain ID (only for provider='wallet')
    pub chain_id: Option<i32>,

    /// Encrypted OAuth access token
    #[serde(skip_serializing)]
    pub access_token_encrypted: Option<String>,

    /// Encrypted OAuth refresh token
    #[serde(skip_serializing)]
    pub refresh_token_encrypted: Option<String>,

    /// Token expiration time
    pub token_expires_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Authentication provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthProvider {
    Email,
    Google,
    GitHub,
    Wallet,
}

impl AuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthProvider::Email => "email",
            AuthProvider::Google => "google",
            AuthProvider::GitHub => "github",
            AuthProvider::Wallet => "wallet",
        }
    }
}

impl std::str::FromStr for AuthProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(AuthProvider::Email),
            "google" => Ok(AuthProvider::Google),
            "github" => Ok(AuthProvider::GitHub),
            "wallet" => Ok(AuthProvider::Wallet),
            _ => Err(format!("Invalid auth provider: {}", s)),
        }
    }
}

impl std::fmt::Display for AuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Request/Response DTOs for API
// ============================================================================

/// Request to create a new trigger
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTriggerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub chain_id: i32,
    #[validate(custom(function = "validate_registry"))]
    pub registry: String,
    pub enabled: Option<bool>,
    pub is_stateful: Option<bool>,
    #[validate(length(min = 1))]
    pub conditions: Vec<CreateConditionRequest>,
    #[validate(length(min = 1))]
    pub actions: Vec<CreateActionRequest>,
}

/// Request to create a condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConditionRequest {
    pub condition_type: String,
    pub field: String,
    pub operator: String,
    pub value: String,
    pub config: Option<serde_json::Value>,
}

/// Request to create an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActionRequest {
    pub action_type: String,
    pub priority: Option<i32>,
    pub config: serde_json::Value,
}

/// Custom validator for registry field
fn validate_registry(registry: &str) -> Result<(), validator::ValidationError> {
    if !["identity", "reputation", "validation"].contains(&registry) {
        return Err(validator::ValidationError::new("invalid_registry"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_registry_valid() {
        assert!(validate_registry("identity").is_ok());
        assert!(validate_registry("reputation").is_ok());
        assert!(validate_registry("validation").is_ok());
    }

    #[test]
    fn test_validate_registry_invalid() {
        assert!(validate_registry("invalid").is_err());
        assert!(validate_registry("").is_err());
    }
}
