//! Wallet authentication and agent linking DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::validators::ETH_ADDRESS_REGEX;

/// Custom validator for Ethereum addresses
fn validate_eth_address(address: &str) -> Result<(), validator::ValidationError> {
    if ETH_ADDRESS_REGEX.is_match(address) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_eth_address"))
    }
}

// ============================================================================
// Wallet Challenge DTOs
// ============================================================================

/// Request a wallet challenge for EIP-191 signature authentication
#[derive(Debug, Deserialize, Validate)]
pub struct WalletChallengeRequest {
    /// Ethereum wallet address (0x + 40 hex chars)
    #[validate(length(equal = 42), custom(function = "validate_eth_address"))]
    pub wallet_address: String,
}

/// Response containing the challenge to sign
#[derive(Debug, Serialize)]
pub struct WalletChallengeResponse {
    /// The message to sign with your wallet
    pub challenge: String,
    /// Unique nonce (expires in 5 minutes)
    pub nonce: String,
    /// Challenge expiration time
    pub expires_at: DateTime<Utc>,
}

// ============================================================================
// Wallet Verify DTOs
// ============================================================================

/// Submit signed challenge for verification
#[derive(Debug, Deserialize, Validate)]
pub struct WalletVerifyRequest {
    /// Ethereum wallet address
    #[validate(length(equal = 42), custom(function = "validate_eth_address"))]
    pub wallet_address: String,

    /// EIP-191 signature (0x + 130 hex chars)
    #[validate(length(equal = 132))]
    pub signature: String,

    /// Nonce from challenge response
    #[validate(length(min = 1))]
    pub nonce: String,
}

/// Response containing JWT token after successful verification
#[derive(Debug, Serialize)]
pub struct WalletVerifyResponse {
    /// JWT access token
    pub token: String,
    /// Token expiration time in seconds
    pub expires_in: i64,
}

// ============================================================================
// Agent Linking DTOs
// ============================================================================

/// Request to link an ERC-8004 agent to an organization
#[derive(Debug, Deserialize, Validate)]
pub struct LinkAgentRequest {
    /// ERC-8004 agent token ID
    #[validate(range(min = 0))]
    pub agent_id: i64,

    /// Blockchain chain ID (e.g., 11155111 for Sepolia)
    #[validate(range(min = 1))]
    pub chain_id: i32,

    /// Wallet address that owns the agent
    #[validate(length(equal = 42), custom(function = "validate_eth_address"))]
    pub wallet_address: String,

    /// EIP-191 signature proving ownership
    #[validate(length(equal = 132))]
    pub signature: String,

    /// Nonce from challenge (for replay protection)
    #[validate(length(min = 1))]
    pub nonce: String,
}

/// Response after successfully linking an agent
#[derive(Debug, Serialize)]
pub struct LinkedAgentResponse {
    /// Link record ID
    pub id: String,
    /// Agent token ID
    pub agent_id: i64,
    /// Chain ID
    pub chain_id: i32,
    /// Organization ID
    pub organization_id: String,
    /// Wallet address
    pub wallet_address: String,
    /// Link status
    pub status: String,
    /// When the link was created
    pub linked_at: DateTime<Utc>,
}

/// Query parameters for unlink agent endpoint
#[derive(Debug, Deserialize)]
pub struct UnlinkAgentQuery {
    /// Chain ID (required to identify the correct link)
    pub chain_id: i32,
}

// ============================================================================
// Database Models (for repository layer)
// ============================================================================

/// Database model for used_nonces table
#[derive(Debug, sqlx::FromRow)]
pub struct UsedNonce {
    pub nonce: String,
    pub wallet_address: String,
    pub used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Database model for agent_links table
#[derive(Debug, sqlx::FromRow)]
pub struct AgentLink {
    pub id: String,
    pub agent_id: i64,
    pub chain_id: i32,
    pub organization_id: String,
    pub wallet_address: String,
    pub linked_by: String,
    pub signature: String,
    pub status: String,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<AgentLink> for LinkedAgentResponse {
    fn from(link: AgentLink) -> Self {
        Self {
            id: link.id,
            agent_id: link.agent_id,
            chain_id: link.chain_id,
            organization_id: link.organization_id,
            wallet_address: link.wallet_address,
            status: link.status,
            linked_at: link.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // WalletChallengeRequest tests
    // ========================================================================

    #[test]
    fn test_wallet_challenge_request_valid() {
        let req = WalletChallengeRequest {
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_wallet_challenge_request_invalid_length() {
        let req = WalletChallengeRequest {
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e75".to_string(), // too short
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_challenge_request_missing_prefix() {
        let req = WalletChallengeRequest {
            wallet_address: "742d35Cc6634C0532925a3b844Bc9e7595f0bEb412".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // WalletVerifyRequest tests
    // ========================================================================

    #[test]
    fn test_wallet_verify_request_valid() {
        let req = WalletVerifyRequest {
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: format!("0x{}", "a".repeat(130)),
            nonce: "abc123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_wallet_verify_request_invalid_signature_length() {
        let req = WalletVerifyRequest {
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: "0xshort".to_string(),
            nonce: "abc123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_verify_request_empty_nonce() {
        let req = WalletVerifyRequest {
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: format!("0x{}", "a".repeat(130)),
            nonce: "".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // LinkAgentRequest tests
    // ========================================================================

    #[test]
    fn test_link_agent_request_valid() {
        let req = LinkAgentRequest {
            agent_id: 42,
            chain_id: 11155111,
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: format!("0x{}", "a".repeat(130)),
            nonce: "abc123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_link_agent_request_negative_agent_id() {
        let req = LinkAgentRequest {
            agent_id: -1,
            chain_id: 11155111,
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: format!("0x{}", "a".repeat(130)),
            nonce: "abc123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_link_agent_request_invalid_chain_id() {
        let req = LinkAgentRequest {
            agent_id: 42,
            chain_id: 0,
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            signature: format!("0x{}", "a".repeat(130)),
            nonce: "abc123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ========================================================================
    // Response serialization tests
    // ========================================================================

    #[test]
    fn test_wallet_challenge_response_serialization() {
        let response = WalletChallengeResponse {
            challenge: "Sign this message".to_string(),
            nonce: "nonce123".to_string(),
            expires_at: Utc::now(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Sign this message"));
        assert!(json.contains("nonce123"));
    }

    #[test]
    fn test_linked_agent_response_serialization() {
        let response = LinkedAgentResponse {
            id: "link-123".to_string(),
            agent_id: 42,
            chain_id: 11155111,
            organization_id: "org-456".to_string(),
            wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4".to_string(),
            status: "active".to_string(),
            linked_at: Utc::now(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("link-123"));
        assert!(json.contains("42"));
        assert!(json.contains("active"));
    }
}
