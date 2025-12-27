//! OAuth Authorization Code Service
//!
//! Handles generation and exchange of temporary authorization codes for OAuth flows.
//! This implements the Authorization Code pattern to avoid exposing tokens in URLs.
//!
//! # Security Features
//!
//! - **32 bytes of entropy**: Uses CSPRNG for code generation
//! - **SHA-256 hashing**: Fast lookup with 256-bit collision resistance
//! - **Single-use**: Code is invalidated after first exchange
//! - **5-minute expiration**: Short-lived to minimize attack window

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::repositories::OAuthTempCodeRepository;
use shared::DbPool;

/// Code prefix for OAuth authorization codes
const CODE_PREFIX: &str = "oac_"; // "oauth auth code"

/// Length of random bytes for code generation (256 bits of entropy)
const CODE_ENTROPY_BYTES: usize = 32;

/// Code validity in seconds (5 minutes)
pub const CODE_VALIDITY_SECS: i64 = 300;

/// Errors that can occur during OAuth code operations
#[derive(Debug, Error)]
pub enum OAuthCodeError {
    #[error("Failed to generate code: {0}")]
    GenerationError(String),

    #[error("Invalid or expired authorization code")]
    InvalidCode,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Service for OAuth authorization code operations
#[derive(Clone, Default)]
pub struct OAuthCodeService;

impl OAuthCodeService {
    /// Create a new OAuthCodeService
    pub fn new() -> Self {
        Self
    }

    /// Generate a new authorization code (raw value, not stored yet)
    fn generate_code(&self) -> Result<String, OAuthCodeError> {
        // Generate 32 bytes of random data
        let mut random_bytes = [0u8; CODE_ENTROPY_BYTES];
        getrandom::fill(&mut random_bytes)
            .map_err(|e| OAuthCodeError::GenerationError(e.to_string()))?;

        // Encode as URL-safe base64
        let encoded = URL_SAFE_NO_PAD.encode(random_bytes);

        // Build the full code with prefix
        Ok(format!("{}{}", CODE_PREFIX, encoded))
    }

    /// Hash an authorization code using SHA-256 (hex encoded)
    fn hash_code(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Create and store a new authorization code for a user
    ///
    /// Returns the raw code to be included in the redirect URL.
    pub async fn create_code(
        &self,
        pool: &DbPool,
        user_id: &str,
    ) -> Result<String, OAuthCodeError> {
        let code = self.generate_code()?;
        let code_hash = Self::hash_code(&code);

        let expires_at = Utc::now() + Duration::seconds(CODE_VALIDITY_SECS);

        OAuthTempCodeRepository::create(pool, user_id, &code_hash, expires_at)
            .await
            .map_err(|e| OAuthCodeError::DatabaseError(e.to_string()))?;

        Ok(code)
    }

    /// Exchange an authorization code for the user_id
    ///
    /// This atomically validates the code and marks it as used.
    /// Returns the user_id if successful.
    pub async fn exchange_code(&self, pool: &DbPool, code: &str) -> Result<String, OAuthCodeError> {
        // Validate code format
        if !code.starts_with(CODE_PREFIX) {
            return Err(OAuthCodeError::InvalidCode);
        }

        // Hash the code for lookup
        let code_hash = Self::hash_code(code);

        // Atomically exchange the code with explicit error handling
        match OAuthTempCodeRepository::exchange_code(pool, &code_hash).await {
            Ok(Some(user_id)) => Ok(user_id),
            Ok(None) => Err(OAuthCodeError::InvalidCode),
            Err(e) => {
                tracing::error!(error = %e, "Database error during OAuth code exchange");
                Err(OAuthCodeError::DatabaseError(e.to_string()))
            }
        }
    }
}

/// Local getrandom wrapper using rand
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
    fn test_generate_code_format() {
        let service = OAuthCodeService::new();
        let code = service.generate_code().unwrap();

        assert!(code.starts_with("oac_"));
        // Base64 of 32 bytes = 43 chars, plus prefix
        assert_eq!(code.len(), 4 + 43);
    }

    #[test]
    fn test_hash_code_deterministic() {
        let code = "oac_test_code_12345";
        let hash1 = OAuthCodeService::hash_code(code);
        let hash2 = OAuthCodeService::hash_code(code);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_different_codes_different_hashes() {
        let service = OAuthCodeService::new();
        let code1 = service.generate_code().unwrap();
        let code2 = service.generate_code().unwrap();

        let hash1 = OAuthCodeService::hash_code(&code1);
        let hash2 = OAuthCodeService::hash_code(&code2);

        assert_ne!(code1, code2);
        assert_ne!(hash1, hash2);
    }
}
