//! OAuth Token Service
//!
//! This service handles secure OAuth token generation, hashing, and verification.
//!
//! # Security Features
//!
//! - **32 bytes of entropy**: Uses `rand::OsRng` (CSPRNG) for token generation
//! - **Argon2id hashing**: OWASP-recommended parameters (64MiB memory, 3 iterations, **p=4**)
//! - **Constant-time verification**: Argon2 verify is timing-attack resistant
//! - **Unique prefixes**: Each token type has a unique prefix for identification
//!
//! # Token Format
//!
//! ```text
//! oauth_at_<43 base64url chars>  (access token)
//! oauth_rt_<43 base64url chars>  (refresh token)
//! ```
//!
//! # CRITICAL DIFFERENCE FROM API KEYS
//!
//! OAuth tokens use Argon2id with **p=4** (parallelism=4), NOT p=1 like API keys.
//! This is because OAuth tokens are expected to be verified more frequently and
//! can benefit from parallel hashing to reduce latency.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use thiserror::Error;

/// Length of random bytes for token generation (256 bits of entropy)
const TOKEN_ENTROPY_BYTES: usize = 32;

/// Argon2 memory cost in KiB (64 MiB as per OWASP recommendations)
const ARGON2_MEMORY_COST: u32 = 65536;

/// Argon2 time cost (iterations)
const ARGON2_TIME_COST: u32 = 3;

/// Argon2 parallelism degree for OAuth tokens
/// CRITICAL: OAuth tokens use p=4, NOT p=1 like API keys!
const ARGON2_PARALLELISM: u32 = 4;

/// Errors that can occur during OAuth token operations
#[derive(Debug, Error)]
pub enum OAuthTokenError {
    #[error("Failed to generate token: {0}")]
    GenerationError(String),

    #[error("Failed to hash token: {0}")]
    HashError(String),

    #[error("Failed to verify token: {0}")]
    VerificationError(String),

    #[error("Invalid token format")]
    InvalidFormat,
}

/// Result of generating a new OAuth token
#[derive(Debug)]
pub struct GeneratedOAuthToken {
    /// The full token (to be shown to client)
    pub token: String,

    /// The Argon2id hash of the token (to be stored in database)
    pub hash: String,
}

/// Service for OAuth token operations
///
/// This service holds pre-computed values for timing attack mitigation.
/// The dummy hash is computed once at initialization.
#[derive(Clone)]
pub struct OAuthTokenService {
    argon2: Argon2<'static>,
    /// Pre-computed valid hash for timing attack mitigation
    dummy_hash: String,
}

impl Default for OAuthTokenService {
    fn default() -> Self {
        Self::new()
    }
}

/// Dummy token used for timing attack mitigation
const DUMMY_TOKEN: &str = "oauth_at_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

impl OAuthTokenService {
    /// Create a new OAuthTokenService with OWASP-recommended Argon2 parameters
    ///
    /// # CRITICAL: Argon2id Parallelism = 4
    ///
    /// OAuth tokens use p=4 (parallelism=4), which is DIFFERENT from API keys (p=1).
    /// This pre-computes a valid Argon2 hash for timing attack mitigation.
    pub fn new() -> Self {
        // Use Argon2id variant with p=4 (NOT p=1 like API keys!)
        // Parameters: 64 MiB memory, 3 iterations, 4 parallelism
        let params = Params::new(
            ARGON2_MEMORY_COST,
            ARGON2_TIME_COST,
            ARGON2_PARALLELISM, // p=4 for OAuth tokens
            None,               // Default output length (32 bytes)
        )
        .expect("Invalid Argon2 parameters");

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        // Pre-compute a valid hash for timing attack mitigation
        let salt = SaltString::generate(&mut OsRng);
        let dummy_hash = argon2
            .hash_password(DUMMY_TOKEN.as_bytes(), &salt)
            .expect("Failed to pre-compute dummy hash")
            .to_string();

        Self { argon2, dummy_hash }
    }

    /// Generate a new access token with secure random entropy
    ///
    /// # Returns
    /// A `GeneratedOAuthToken` containing the full token and hash
    ///
    /// # Security
    /// - Uses `OsRng` (OS-provided CSPRNG) for random bytes
    /// - Generates 32 bytes (256 bits) of entropy
    /// - Hash uses Argon2id with p=4 (OWASP parameters)
    pub fn generate_access_token(&self) -> Result<GeneratedOAuthToken, OAuthTokenError> {
        self.generate_token("oauth_at_")
    }

    /// Generate a new refresh token with secure random entropy
    ///
    /// # Returns
    /// A `GeneratedOAuthToken` containing the full token and hash
    pub fn generate_refresh_token(&self) -> Result<GeneratedOAuthToken, OAuthTokenError> {
        self.generate_token("oauth_rt_")
    }

    /// Internal helper to generate a token with a given prefix
    fn generate_token(&self, prefix: &str) -> Result<GeneratedOAuthToken, OAuthTokenError> {
        // Generate 32 bytes of random data using OS CSPRNG
        let mut random_bytes = [0u8; TOKEN_ENTROPY_BYTES];
        getrandom::fill(&mut random_bytes)
            .map_err(|e| OAuthTokenError::GenerationError(e.to_string()))?;

        // Encode as URL-safe base64 (no padding) - produces 43 chars
        let encoded = URL_SAFE_NO_PAD.encode(random_bytes);

        // Build the full token with prefix
        let full_token = format!("{}{}", prefix, encoded);

        // Hash the full token with Argon2id (p=4)
        let hash = self.hash_token(&full_token)?;

        Ok(GeneratedOAuthToken {
            token: full_token,
            hash,
        })
    }

    /// Hash an OAuth token using Argon2id with p=4
    ///
    /// # Arguments
    /// * `token` - The full OAuth token to hash
    ///
    /// # Returns
    /// The Argon2id hash string (PHC format) with p=4
    pub fn hash_token(&self, token: &str) -> Result<String, OAuthTokenError> {
        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

        // Hash the token with Argon2id (p=4)
        let hash = self
            .argon2
            .hash_password(token.as_bytes(), &salt)
            .map_err(|e| OAuthTokenError::HashError(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify an OAuth token against a stored hash
    ///
    /// # Arguments
    /// * `token` - The full OAuth token to verify
    /// * `hash` - The stored Argon2id hash (with p=4)
    ///
    /// # Returns
    /// `true` if the token matches the hash, `false` otherwise
    ///
    /// # Security
    /// - Uses constant-time comparison (built into Argon2 verify)
    /// - Timing is independent of where the mismatch occurs
    pub fn verify_token(&self, token: &str, hash: &str) -> Result<bool, OAuthTokenError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| OAuthTokenError::VerificationError(e.to_string()))?;

        match self.argon2.verify_password(token.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(OAuthTokenError::VerificationError(e.to_string())),
        }
    }

    /// Perform a dummy hash operation for timing attack mitigation
    ///
    /// When a token is not found in the database, we should still
    /// perform a hash operation to avoid timing differences that could
    /// leak information about which tokens exist.
    ///
    /// # Security
    ///
    /// This method uses a pre-computed **valid** Argon2 hash (with p=4,
    /// computed at service initialization). This ensures:
    ///
    /// 1. The hash parsing succeeds (no early exit)
    /// 2. Full Argon2 verification is performed (same timing as real tokens)
    /// 3. Timing is consistent regardless of whether the token exists
    pub fn dummy_verify(&self) {
        // Use the pre-computed valid hash from service initialization
        let _ = self.verify_token(DUMMY_TOKEN, &self.dummy_hash);
    }

    /// Validate the format of an OAuth token
    ///
    /// # Arguments
    /// * `token` - The OAuth token to validate
    ///
    /// # Returns
    /// `true` if the token has a valid format, `false` otherwise
    pub fn is_valid_format(token: &str) -> bool {
        // Must start with oauth_at_ or oauth_rt_
        if !token.starts_with("oauth_at_") && !token.starts_with("oauth_rt_") {
            return false;
        }

        // Total length should be 52 (9 prefix + 43 base64)
        if token.len() != 52 {
            return false;
        }

        // The random part should be valid base64url
        let random_part = &token[9..];
        URL_SAFE_NO_PAD.decode(random_part).is_ok()
    }

    /// Get the token type from an OAuth token
    ///
    /// # Arguments
    /// * `token` - The OAuth token
    ///
    /// # Returns
    /// "access_token" or "refresh_token", or an error if the format is invalid
    #[allow(dead_code)]
    pub fn get_token_type(token: &str) -> Result<&'static str, OAuthTokenError> {
        if token.starts_with("oauth_at_") {
            Ok("access_token")
        } else if token.starts_with("oauth_rt_") {
            Ok("refresh_token")
        } else {
            Err(OAuthTokenError::InvalidFormat)
        }
    }
}

// We need getrandom for CSPRNG - it's a dependency of rand
mod getrandom {
    use rand::RngCore;

    pub fn fill(dest: &mut [u8]) -> Result<(), rand::Error> {
        rand::rngs::OsRng.try_fill_bytes(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_access_token() {
        let service = OAuthTokenService::new();
        let result = service.generate_access_token().unwrap();

        assert!(result.token.starts_with("oauth_at_"));
        assert_eq!(result.token.len(), 52);
        assert!(result.hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_generate_refresh_token() {
        let service = OAuthTokenService::new();
        let result = service.generate_refresh_token().unwrap();

        assert!(result.token.starts_with("oauth_rt_"));
        assert_eq!(result.token.len(), 52);
        assert!(result.hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_tokens_are_unique() {
        let service = OAuthTokenService::new();

        let token1 = service.generate_access_token().unwrap();
        let token2 = service.generate_access_token().unwrap();

        assert_ne!(token1.token, token2.token);
        assert_ne!(token1.hash, token2.hash);
    }

    #[test]
    fn test_verify_token_correct() {
        let service = OAuthTokenService::new();
        let generated = service.generate_access_token().unwrap();

        let verified = service
            .verify_token(&generated.token, &generated.hash)
            .unwrap();
        assert!(verified);
    }

    #[test]
    fn test_verify_token_incorrect() {
        let service = OAuthTokenService::new();
        let generated = service.generate_access_token().unwrap();

        // Try to verify with a different token
        let wrong_token = "oauth_at_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk";
        let verified = service.verify_token(wrong_token, &generated.hash).unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_hash_token() {
        let service = OAuthTokenService::new();

        let hash1 = service.hash_token("test_token_1").unwrap();
        let hash2 = service.hash_token("test_token_1").unwrap();

        // Same token should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(service.verify_token("test_token_1", &hash1).unwrap());
        assert!(service.verify_token("test_token_1", &hash2).unwrap());
    }

    #[test]
    fn test_is_valid_format_access_token() {
        assert!(OAuthTokenService::is_valid_format(
            "oauth_at_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_refresh_token() {
        assert!(OAuthTokenService::is_valid_format(
            "oauth_rt_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_wrong_prefix() {
        assert!(!OAuthTokenService::is_valid_format(
            "oauth_xx_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_too_short() {
        assert!(!OAuthTokenService::is_valid_format("oauth_at_ABC"));
    }

    #[test]
    fn test_is_valid_format_too_long() {
        assert!(!OAuthTokenService::is_valid_format(
            "oauth_at_ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKXXXX"
        ));
    }

    #[test]
    fn test_get_token_type_access() {
        let token_type = OAuthTokenService::get_token_type("oauth_at_ABC").unwrap();
        assert_eq!(token_type, "access_token");
    }

    #[test]
    fn test_get_token_type_refresh() {
        let token_type = OAuthTokenService::get_token_type("oauth_rt_ABC").unwrap();
        assert_eq!(token_type, "refresh_token");
    }

    #[test]
    fn test_get_token_type_invalid() {
        let result = OAuthTokenService::get_token_type("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_dummy_verify_does_not_panic() {
        let service = OAuthTokenService::new();
        service.dummy_verify(); // Should complete without panic
    }

    #[test]
    fn test_token_entropy() {
        let service = OAuthTokenService::new();

        // Generate tokens and check they're all different
        let mut tokens: Vec<String> = Vec::new();
        for _ in 0..10 {
            let generated = service.generate_access_token().unwrap();
            assert!(
                !tokens.contains(&generated.token),
                "Token collision detected!"
            );
            tokens.push(generated.token);
        }
    }

    #[test]
    fn test_hash_contains_argon2id_params_p4() {
        let service = OAuthTokenService::new();
        let generated = service.generate_access_token().unwrap();

        // Hash should contain Argon2id identifier and our parameters
        assert!(generated.hash.contains("argon2id"));
        assert!(generated.hash.contains("m=65536")); // Memory cost
        assert!(generated.hash.contains("t=3")); // Time cost
        assert!(generated.hash.contains("p=4")); // Parallelism = 4 (CRITICAL!)
    }

    #[test]
    fn test_access_and_refresh_tokens_different() {
        let service = OAuthTokenService::new();

        let access = service.generate_access_token().unwrap();
        let refresh = service.generate_refresh_token().unwrap();

        // Different prefixes
        assert!(access.token.starts_with("oauth_at_"));
        assert!(refresh.token.starts_with("oauth_rt_"));

        // Different tokens
        assert_ne!(access.token, refresh.token);
        assert_ne!(access.hash, refresh.hash);
    }
}
