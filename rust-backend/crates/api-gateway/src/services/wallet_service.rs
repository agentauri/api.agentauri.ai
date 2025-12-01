//! Wallet Authentication Service (Layer 2 - Future Feature)
//!
//! This service handles EIP-191 signature verification for wallet-based authentication
//! and on-chain ownership verification for agent NFTs.
//!
//! **Note**: This module is implemented but not yet fully integrated with the API Gateway.
//! It will be completed in Phase 4 when wallet signature authentication (Layer 2) is
//! fully enabled. Some items have `#[allow(dead_code)]` annotations.
//!
//! # Features
//!
//! - **Challenge generation**: Creates unique, time-limited challenges for signing
//! - **EIP-191 signature verification**: Verifies personal_sign signatures
//! - **On-chain ownership verification**: Checks IdentityRegistry.ownerOf() for agent NFTs
//! - **Nonce management**: Prevents replay attacks with single-use nonces
//!
//! # Security
//!
//! - Challenges expire after 5 minutes
//! - Nonces are single-use and stored in the database
//! - Signatures are verified using recovered public key matching

use alloy::primitives::{PrimitiveSignature, B256};
use alloy::signers::k256::ecdsa::VerifyingKey;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use thiserror::Error;

/// Challenge expiration time in minutes
#[allow(dead_code)] // Future feature: Layer 2 wallet authentication
const CHALLENGE_EXPIRATION_MINUTES: i64 = 5;

/// Nonce length in bytes
#[allow(dead_code)] // Future feature: Layer 2 wallet authentication
const NONCE_LENGTH: usize = 32;

/// Errors that can occur during wallet operations
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Invalid signature format: {0}")]
    InvalidSignature(String),

    #[error("Signature verification failed: signer does not match expected address")]
    SignerMismatch,

    #[error("Challenge expired")]
    ChallengeExpired,

    #[error("Invalid nonce")]
    #[allow(dead_code)] // Future feature: Layer 2 wallet authentication
    InvalidNonce,

    #[error("Nonce already used")]
    #[allow(dead_code)] // Future feature: Layer 2 wallet authentication
    NonceReused,

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(i64),

    #[error("Agent ownership verification failed: expected {expected}, got {actual}")]
    OwnershipMismatch { expected: String, actual: String },

    #[error("On-chain verification failed: {0}")]
    OnChainError(String),

    #[error("Internal error: {0}")]
    #[allow(dead_code)] // Future feature: Layer 2 wallet authentication
    Internal(String),
}

/// Result of generating a new challenge
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future feature: Layer 2 wallet authentication
pub struct GeneratedChallenge {
    /// The message to sign
    pub message: String,
    /// Unique nonce (hex-encoded)
    pub nonce: String,
    /// Challenge expiration time
    pub expires_at: DateTime<Utc>,
}

/// Configuration for on-chain verification
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Chain ID
    pub chain_id: i32,
    /// RPC URL for the chain
    pub rpc_url: String,
    /// IdentityRegistry contract address
    pub identity_registry_address: String,
}

/// Service for wallet authentication operations
///
/// This service is designed to be created once at startup and shared across
/// all request handlers via app state. It maintains an HTTP client with
/// connection pooling for efficient RPC calls.
#[derive(Clone)]
pub struct WalletService {
    /// Chain configurations for on-chain verification
    chain_configs: Vec<ChainConfig>,
    /// HTTP client with connection pooling for RPC calls
    http_client: reqwest::Client,
}

impl Default for WalletService {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl WalletService {
    /// Create a new WalletService with chain configurations
    ///
    /// This constructor creates a shared HTTP client with connection pooling.
    /// The service should be created once at startup and shared via app state.
    pub fn new(chain_configs: Vec<ChainConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            chain_configs,
            http_client,
        }
    }

    /// Load chain configurations from environment variables
    ///
    /// This function reads RPC URLs and contract addresses from environment
    /// variables for each supported chain.
    pub fn load_chain_configs_from_env() -> Vec<ChainConfig> {
        let mut configs = Vec::new();

        // Ethereum Sepolia (chain ID 11155111)
        if let Ok(rpc_url) = std::env::var("ETHEREUM_SEPOLIA_RPC_URL") {
            if let Ok(identity_addr) = std::env::var("ETHEREUM_SEPOLIA_IDENTITY_ADDRESS") {
                configs.push(ChainConfig {
                    chain_id: 11155111,
                    rpc_url,
                    identity_registry_address: identity_addr,
                });
            }
        }

        // Base Sepolia (chain ID 84532)
        if let Ok(rpc_url) = std::env::var("BASE_SEPOLIA_RPC_URL") {
            if let Ok(identity_addr) = std::env::var("BASE_SEPOLIA_IDENTITY_ADDRESS") {
                configs.push(ChainConfig {
                    chain_id: 84532,
                    rpc_url,
                    identity_registry_address: identity_addr,
                });
            }
        }

        // Linea Sepolia (chain ID 59141)
        if let Ok(rpc_url) = std::env::var("LINEA_SEPOLIA_RPC_URL") {
            if let Ok(identity_addr) = std::env::var("LINEA_SEPOLIA_IDENTITY_ADDRESS") {
                configs.push(ChainConfig {
                    chain_id: 59141,
                    rpc_url,
                    identity_registry_address: identity_addr,
                });
            }
        }

        configs
    }

    /// Generate a new challenge for a wallet address to sign
    ///
    /// # Arguments
    /// * `wallet_address` - The Ethereum address requesting authentication
    ///
    /// # Returns
    /// A `GeneratedChallenge` containing the message, nonce, and expiration time
    #[allow(dead_code)] // Future feature: Layer 2 wallet authentication
    pub fn generate_challenge(
        &self,
        wallet_address: &str,
    ) -> Result<GeneratedChallenge, WalletError> {
        // Validate address format
        self.validate_address(wallet_address)?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = hex::encode(nonce_bytes);

        // Calculate expiration
        let expires_at = Utc::now() + Duration::minutes(CHALLENGE_EXPIRATION_MINUTES);

        // Create challenge message following EIP-191 convention
        let message = format!(
            "Sign this message to authenticate with ERC-8004 API\n\nWallet: {}\nNonce: {}\nExpires: {}",
            wallet_address,
            nonce,
            expires_at.format("%Y-%m-%dT%H:%M:%SZ")
        );

        Ok(GeneratedChallenge {
            message,
            nonce,
            expires_at,
        })
    }

    /// Verify an EIP-191 signature and recover the signer address
    ///
    /// # Arguments
    /// * `message` - The original message that was signed
    /// * `signature` - The signature in hex format (0x prefix optional, 65 bytes)
    /// * `expected_address` - The address we expect to have signed the message
    ///
    /// # Returns
    /// The recovered signer address if verification succeeds
    ///
    /// # Security
    /// Uses EIP-191 personal_sign format: "\x19Ethereum Signed Message:\n" + len + message
    pub fn verify_signature(
        &self,
        message: &str,
        signature: &str,
        expected_address: &str,
    ) -> Result<String, WalletError> {
        // Parse the expected address
        let expected = self.parse_address(expected_address)?;

        // Parse the signature (strip 0x prefix if present)
        let sig_hex = signature.strip_prefix("0x").unwrap_or(signature);
        let sig_bytes = hex::decode(sig_hex)
            .map_err(|e| WalletError::InvalidSignature(format!("Invalid hex: {}", e)))?;

        if sig_bytes.len() != 65 {
            return Err(WalletError::InvalidSignature(format!(
                "Expected 65 bytes, got {}",
                sig_bytes.len()
            )));
        }

        // Create EIP-191 prefixed message hash
        let prefixed_message = self.eip191_hash(message);

        // Parse signature components
        let r = B256::from_slice(&sig_bytes[0..32]);
        let s = B256::from_slice(&sig_bytes[32..64]);
        let v = sig_bytes[64];

        // Normalize v value (handle both legacy and EIP-155 formats)
        let v_normalized = if v >= 27 { v - 27 } else { v };
        if v_normalized > 1 {
            return Err(WalletError::InvalidSignature(format!(
                "Invalid recovery id: {}",
                v
            )));
        }

        // Create signature using the non-deprecated API
        let signature = PrimitiveSignature::new(
            alloy::primitives::U256::from_be_slice(r.as_slice()),
            alloy::primitives::U256::from_be_slice(s.as_slice()),
            v_normalized != 0, // y_parity: true if v is 28 (after normalization: 1)
        );

        // Recover the public key from signature
        let recovered_key = signature
            .recover_from_prehash(&prefixed_message)
            .map_err(|e| WalletError::InvalidSignature(format!("Recovery failed: {}", e)))?;

        // Convert recovered public key to address
        let recovered_address = self.pubkey_to_address(&recovered_key);

        // Compare addresses (case-insensitive)
        if recovered_address.to_lowercase() != expected.to_lowercase() {
            return Err(WalletError::SignerMismatch);
        }

        Ok(recovered_address)
    }

    /// Verify that a wallet address owns a specific agent NFT on-chain
    ///
    /// # Arguments
    /// * `wallet_address` - The claimed owner address
    /// * `agent_id` - The agent token ID
    /// * `chain_id` - The chain where the agent is registered
    ///
    /// # Returns
    /// Ok(()) if ownership is verified, error otherwise
    pub async fn verify_agent_ownership(
        &self,
        wallet_address: &str,
        agent_id: i64,
        chain_id: i32,
    ) -> Result<(), WalletError> {
        // Get chain config
        let config = self
            .chain_configs
            .iter()
            .find(|c| c.chain_id == chain_id)
            .ok_or_else(|| {
                WalletError::OnChainError(format!("Unsupported chain ID: {}", chain_id))
            })?;

        // Call IdentityRegistry.ownerOf(agentId)
        let owner = self
            .call_owner_of(&config.rpc_url, &config.identity_registry_address, agent_id)
            .await?;

        // Compare addresses (case-insensitive)
        if owner.to_lowercase() != wallet_address.to_lowercase() {
            return Err(WalletError::OwnershipMismatch {
                expected: wallet_address.to_string(),
                actual: owner,
            });
        }

        Ok(())
    }

    /// Validate that a challenge hasn't expired
    pub fn validate_challenge_expiration(
        &self,
        expires_at: DateTime<Utc>,
    ) -> Result<(), WalletError> {
        if Utc::now() > expires_at {
            return Err(WalletError::ChallengeExpired);
        }
        Ok(())
    }

    /// Validate Ethereum address format
    fn validate_address(&self, address: &str) -> Result<(), WalletError> {
        // Must start with 0x
        if !address.starts_with("0x") {
            return Err(WalletError::InvalidAddress(
                "Address must start with 0x".to_string(),
            ));
        }

        // Must be 42 characters (0x + 40 hex chars)
        if address.len() != 42 {
            return Err(WalletError::InvalidAddress(format!(
                "Address must be 42 characters, got {}",
                address.len()
            )));
        }

        // Must be valid hex
        let hex_part = &address[2..];
        if hex::decode(hex_part).is_err() {
            return Err(WalletError::InvalidAddress(
                "Invalid hex characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Parse an Ethereum address string
    fn parse_address(&self, address: &str) -> Result<String, WalletError> {
        self.validate_address(address)?;
        Ok(address.to_string())
    }

    /// Create EIP-191 prefixed message hash
    ///
    /// Format: "\x19Ethereum Signed Message:\n" + message.length + message
    fn eip191_hash(&self, message: &str) -> B256 {
        use alloy::primitives::keccak256;

        let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
        let prefixed = format!("{}{}", prefix, message);
        keccak256(prefixed.as_bytes())
    }

    /// Convert a public key to an Ethereum address
    fn pubkey_to_address(&self, pubkey: &VerifyingKey) -> String {
        use alloy::primitives::keccak256;

        // Get uncompressed public key bytes (65 bytes with 0x04 prefix)
        let pubkey_bytes = pubkey.to_encoded_point(false);
        let pubkey_slice = pubkey_bytes.as_bytes();

        // Skip the 0x04 prefix and hash the remaining 64 bytes
        let hash = keccak256(&pubkey_slice[1..]);

        // Take last 20 bytes as address
        format!("0x{}", hex::encode(&hash[12..]))
    }

    /// Call IdentityRegistry.ownerOf(tokenId) via JSON-RPC
    ///
    /// Uses the shared HTTP client with connection pooling for efficient
    /// repeated RPC calls.
    async fn call_owner_of(
        &self,
        rpc_url: &str,
        contract_address: &str,
        token_id: i64,
    ) -> Result<String, WalletError> {
        use alloy::primitives::keccak256;

        // ownerOf(uint256) function selector: first 4 bytes of keccak256("ownerOf(uint256)")
        let selector = &keccak256("ownerOf(uint256)".as_bytes())[0..4];

        // Encode the token ID as uint256 (32 bytes, big-endian)
        let token_bytes = {
            let mut bytes = [0u8; 32];
            bytes[24..].copy_from_slice(&(token_id as u64).to_be_bytes());
            bytes
        };

        // Build call data
        let mut call_data = Vec::with_capacity(36);
        call_data.extend_from_slice(selector);
        call_data.extend_from_slice(&token_bytes);

        // Make eth_call request using shared HTTP client (connection pooling)
        let response = self
            .http_client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": contract_address,
                    "data": format!("0x{}", hex::encode(&call_data))
                }, "latest"],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| WalletError::OnChainError(format!("RPC request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WalletError::OnChainError(format!("Failed to parse response: {}", e)))?;

        // Check for errors
        if let Some(error) = json.get("error") {
            let msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");

            // Check if the token doesn't exist (common ERC-721 revert)
            if msg.contains("ERC721") || msg.contains("nonexistent") || msg.contains("invalid") {
                return Err(WalletError::AgentNotFound(token_id));
            }

            return Err(WalletError::OnChainError(msg.to_string()));
        }

        // Parse result (should be 32 bytes address, padded with zeros)
        let result = json
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| WalletError::OnChainError("No result in response".to_string()))?;

        // Result is 0x + 64 hex chars (32 bytes)
        let result_hex = result.strip_prefix("0x").unwrap_or(result);
        if result_hex.len() != 64 {
            return Err(WalletError::OnChainError(format!(
                "Invalid result length: {}",
                result_hex.len()
            )));
        }

        // Last 20 bytes (40 hex chars) are the address
        let address = format!("0x{}", &result_hex[24..]);
        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_service() -> WalletService {
        WalletService::new(vec![])
    }

    // ========================================================================
    // Challenge generation tests
    // ========================================================================

    #[test]
    fn test_generate_challenge_valid_address() {
        let service = create_service();
        let result = service.generate_challenge("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4");

        assert!(result.is_ok());
        let challenge = result.unwrap();

        // Check nonce is 64 hex chars (32 bytes)
        assert_eq!(challenge.nonce.len(), 64);

        // Check message contains address and nonce
        assert!(challenge
            .message
            .contains("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4"));
        assert!(challenge.message.contains(&challenge.nonce));

        // Check expiration is in the future
        assert!(challenge.expires_at > Utc::now());
    }

    #[test]
    fn test_generate_challenge_invalid_address_no_prefix() {
        let service = create_service();
        let result = service.generate_challenge("742d35Cc6634C0532925a3b844Bc9e7595f0bEb4");

        assert!(matches!(result, Err(WalletError::InvalidAddress(_))));
    }

    #[test]
    fn test_generate_challenge_invalid_address_too_short() {
        let service = create_service();
        let result = service.generate_challenge("0x742d35Cc6634C0532925a3b844Bc9e75");

        assert!(matches!(result, Err(WalletError::InvalidAddress(_))));
    }

    #[test]
    fn test_generate_challenge_invalid_address_bad_hex() {
        let service = create_service();
        let result = service.generate_challenge("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEGH");

        assert!(matches!(result, Err(WalletError::InvalidAddress(_))));
    }

    // ========================================================================
    // Challenge expiration tests
    // ========================================================================

    #[test]
    fn test_validate_challenge_expiration_valid() {
        let service = create_service();
        let future = Utc::now() + Duration::minutes(1);

        assert!(service.validate_challenge_expiration(future).is_ok());
    }

    #[test]
    fn test_validate_challenge_expiration_expired() {
        let service = create_service();
        let past = Utc::now() - Duration::minutes(1);

        assert!(matches!(
            service.validate_challenge_expiration(past),
            Err(WalletError::ChallengeExpired)
        ));
    }

    // ========================================================================
    // EIP-191 hash tests
    // ========================================================================

    #[test]
    fn test_eip191_hash_deterministic() {
        let service = create_service();
        let message = "test message";

        let hash1 = service.eip191_hash(message);
        let hash2 = service.eip191_hash(message);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_eip191_hash_different_messages() {
        let service = create_service();

        let hash1 = service.eip191_hash("message 1");
        let hash2 = service.eip191_hash("message 2");

        assert_ne!(hash1, hash2);
    }

    // ========================================================================
    // Signature verification tests
    // ========================================================================

    #[test]
    fn test_verify_signature_invalid_hex() {
        let service = create_service();

        let result = service.verify_signature(
            "test message",
            "not-hex",
            "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4",
        );

        assert!(matches!(result, Err(WalletError::InvalidSignature(_))));
    }

    #[test]
    fn test_verify_signature_wrong_length() {
        let service = create_service();

        let result = service.verify_signature(
            "test message",
            "0x1234",
            "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4",
        );

        assert!(matches!(result, Err(WalletError::InvalidSignature(_))));
    }

    // ========================================================================
    // Address validation tests
    // ========================================================================

    #[test]
    fn test_validate_address_valid() {
        let service = create_service();

        assert!(service
            .validate_address("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4")
            .is_ok());
        assert!(service
            .validate_address("0x0000000000000000000000000000000000000000")
            .is_ok());
        assert!(service
            .validate_address("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
            .is_ok());
    }

    #[test]
    fn test_validate_address_lowercase() {
        let service = create_service();

        assert!(service
            .validate_address("0x742d35cc6634c0532925a3b844bc9e7595f0beb4")
            .is_ok());
    }

    // ========================================================================
    // Nonce tests
    // ========================================================================

    #[test]
    fn test_nonces_are_unique() {
        let service = create_service();
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb4";

        let challenge1 = service.generate_challenge(address).unwrap();
        let challenge2 = service.generate_challenge(address).unwrap();

        assert_ne!(challenge1.nonce, challenge2.nonce);
    }
}
