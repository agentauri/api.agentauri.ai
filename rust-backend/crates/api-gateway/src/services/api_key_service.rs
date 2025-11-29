//! API Key Service
//!
//! This service handles secure API key generation, hashing, and verification.
//!
//! # Security Features
//!
//! - **32 bytes of entropy**: Uses `rand::OsRng` (CSPRNG) for key generation
//! - **Argon2id hashing**: OWASP-recommended parameters (64MiB memory, 3 iterations)
//! - **Constant-time verification**: Argon2 verify is timing-attack resistant
//! - **Unique prefixes**: Each key has a unique prefix for database lookup
//!
//! # Key Format
//!
//! ```text
//! sk_live_<43 base64url chars>  (51 total chars)
//! sk_test_<43 base64url chars>  (51 total chars)
//! ```
//!
//! The prefix stored in the database is the first 16 characters (e.g., `sk_live_XXXXXXXX`)
//! for efficient lookup without exposing the full key.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use thiserror::Error;

/// Length of random bytes for key generation (256 bits of entropy)
const KEY_ENTROPY_BYTES: usize = 32;

/// Length of the stored prefix (for database lookup)
const PREFIX_LENGTH: usize = 16;

/// Argon2 memory cost in KiB (64 MiB as per OWASP recommendations)
const ARGON2_MEMORY_COST: u32 = 65536;

/// Argon2 time cost (iterations)
const ARGON2_TIME_COST: u32 = 3;

/// Argon2 parallelism degree
const ARGON2_PARALLELISM: u32 = 1;

/// Dummy key used for timing attack mitigation
const DUMMY_KEY: &str = "sk_test_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// Pre-computed dummy hash for timing attack mitigation
///
/// This hash is computed ONCE for the entire process lifetime, not once per
/// ApiKeyService instance. This ensures:
/// 1. Consistent timing across all dummy_verify() calls
/// 2. Reduced memory usage (one hash for entire process)
/// 3. Better performance (no repeated hash computation)
/// 4. More stable CI tests (less variance in timing)
static DUMMY_HASH: Lazy<String> = Lazy::new(|| {
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        None,
    )
    .expect("Invalid Argon2 parameters");

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);

    argon2
        .hash_password(DUMMY_KEY.as_bytes(), &salt)
        .expect("Failed to pre-compute dummy hash")
        .to_string()
});

/// Errors that can occur during API key operations
#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error("Failed to generate key: {0}")]
    GenerationError(String),

    #[error("Failed to hash key: {0}")]
    HashError(String),

    #[error("Failed to verify key: {0}")]
    VerificationError(String),

    #[error("Invalid key format")]
    InvalidFormat,
}

/// Result of generating a new API key
#[derive(Debug)]
pub struct GeneratedApiKey {
    /// The full API key (to be shown to user ONLY ONCE)
    pub key: String,

    /// The Argon2id hash of the key (to be stored in database)
    pub hash: String,

    /// The prefix for database lookup (first 16 chars of key)
    pub prefix: String,
}

/// Service for API key operations
///
/// This service uses a globally pre-computed dummy hash (DUMMY_HASH static)
/// for timing attack mitigation, ensuring consistent timing across all instances.
#[derive(Clone)]
pub struct ApiKeyService {
    argon2: Argon2<'static>,
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyService {
    /// Create a new ApiKeyService with OWASP-recommended Argon2 parameters
    ///
    /// The dummy hash for timing attack mitigation is pre-computed globally
    /// (DUMMY_HASH static) once for the entire process, not per instance.
    pub fn new() -> Self {
        // Use Argon2id variant (recommended for password hashing)
        // Parameters: 64 MiB memory, 3 iterations, 1 parallelism
        let params = Params::new(
            ARGON2_MEMORY_COST,
            ARGON2_TIME_COST,
            ARGON2_PARALLELISM,
            None, // Default output length (32 bytes)
        )
        .expect("Invalid Argon2 parameters");

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        Self { argon2 }
    }

    /// Generate a new API key with secure random entropy
    ///
    /// # Arguments
    /// * `environment` - "live" or "test"
    ///
    /// # Returns
    /// A `GeneratedApiKey` containing the full key, hash, and prefix
    ///
    /// # Security
    /// - Uses `OsRng` (OS-provided CSPRNG) for random bytes
    /// - Generates 32 bytes (256 bits) of entropy
    /// - Hash uses Argon2id with OWASP-recommended parameters
    pub fn generate_key(&self, environment: &str) -> Result<GeneratedApiKey, ApiKeyError> {
        // Generate 32 bytes of random data using OS CSPRNG
        let mut random_bytes = [0u8; KEY_ENTROPY_BYTES];
        getrandom::fill(&mut random_bytes)
            .map_err(|e| ApiKeyError::GenerationError(e.to_string()))?;

        // Encode as URL-safe base64 (no padding) - produces 43 chars
        let encoded = URL_SAFE_NO_PAD.encode(random_bytes);

        // Build the full key with environment prefix
        let prefix_str = match environment {
            "live" => "sk_live_",
            "test" => "sk_test_",
            _ => {
                return Err(ApiKeyError::GenerationError(
                    "Invalid environment".to_string(),
                ))
            }
        };

        let full_key = format!("{}{}", prefix_str, encoded);

        // Extract the prefix for database lookup (first 16 chars)
        let prefix = full_key.chars().take(PREFIX_LENGTH).collect::<String>();

        // Hash the full key with Argon2id
        let hash = self.hash_key(&full_key)?;

        Ok(GeneratedApiKey {
            key: full_key,
            hash,
            prefix,
        })
    }

    /// Hash an API key using Argon2id
    ///
    /// # Arguments
    /// * `key` - The full API key to hash
    ///
    /// # Returns
    /// The Argon2id hash string (PHC format)
    pub fn hash_key(&self, key: &str) -> Result<String, ApiKeyError> {
        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

        // Hash the key
        let hash = self
            .argon2
            .hash_password(key.as_bytes(), &salt)
            .map_err(|e| ApiKeyError::HashError(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify an API key against a stored hash
    ///
    /// # Arguments
    /// * `key` - The full API key to verify
    /// * `hash` - The stored Argon2id hash
    ///
    /// # Returns
    /// `true` if the key matches the hash, `false` otherwise
    ///
    /// # Security
    /// - Uses constant-time comparison (built into Argon2 verify)
    /// - Timing is independent of where the mismatch occurs
    pub fn verify_key(&self, key: &str, hash: &str) -> Result<bool, ApiKeyError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| ApiKeyError::VerificationError(e.to_string()))?;

        match self.argon2.verify_password(key.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(ApiKeyError::VerificationError(e.to_string())),
        }
    }

    /// Perform a dummy hash operation for timing attack mitigation
    ///
    /// When a key prefix is not found in the database, we should still
    /// perform a hash operation to avoid timing differences that could
    /// leak information about which prefixes exist.
    ///
    /// # Security
    ///
    /// This method uses a globally pre-computed **valid** Argon2 hash
    /// (DUMMY_HASH static, computed once for the entire process). This ensures:
    ///
    /// 1. The hash parsing succeeds (no early exit)
    /// 2. Full Argon2 verification is performed (same 300-500ms as real keys)
    /// 3. Timing is consistent regardless of whether the key exists
    /// 4. Minimal variance in CI environments (same hash used across all calls)
    ///
    /// **Previous vulnerability**: The old implementation used an invalid
    /// hash format that could fail during parsing, causing early exit and
    /// revealing key existence via timing differences.
    pub fn dummy_verify(&self) {
        // Use the globally pre-computed valid hash (initialized once per process)
        // This ensures full Argon2 verification is performed with consistent timing
        let _ = self.verify_key(DUMMY_KEY, &DUMMY_HASH);
    }

    /// Extract the prefix from a full API key
    ///
    /// # Arguments
    /// * `key` - The full API key
    ///
    /// # Returns
    /// The prefix (first 16 characters) or an error if the key is too short
    pub fn extract_prefix(key: &str) -> Result<String, ApiKeyError> {
        if key.len() < PREFIX_LENGTH {
            return Err(ApiKeyError::InvalidFormat);
        }

        Ok(key.chars().take(PREFIX_LENGTH).collect())
    }

    /// Validate the format of an API key
    ///
    /// # Arguments
    /// * `key` - The API key to validate
    ///
    /// # Returns
    /// `true` if the key has a valid format, `false` otherwise
    pub fn is_valid_format(key: &str) -> bool {
        // Must start with sk_live_ or sk_test_
        if !key.starts_with("sk_live_") && !key.starts_with("sk_test_") {
            return false;
        }

        // Total length should be 51 (8 prefix + 43 base64)
        if key.len() != 51 {
            return false;
        }

        // The random part should be valid base64url
        let random_part = &key[8..];
        URL_SAFE_NO_PAD.decode(random_part).is_ok()
    }

    /// Get the environment from an API key
    ///
    /// # Arguments
    /// * `key` - The API key
    ///
    /// # Returns
    /// "live", "test", or an error if the format is invalid
    #[allow(dead_code)]
    pub fn get_environment(key: &str) -> Result<&'static str, ApiKeyError> {
        if key.starts_with("sk_live_") {
            Ok("live")
        } else if key.starts_with("sk_test_") {
            Ok("test")
        } else {
            Err(ApiKeyError::InvalidFormat)
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
    fn test_generate_key_live() {
        let service = ApiKeyService::new();
        let result = service.generate_key("live").unwrap();

        assert!(result.key.starts_with("sk_live_"));
        assert_eq!(result.key.len(), 51);
        assert_eq!(result.prefix.len(), PREFIX_LENGTH);
        assert!(result.prefix.starts_with("sk_live_"));
        assert!(result.hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_generate_key_test() {
        let service = ApiKeyService::new();
        let result = service.generate_key("test").unwrap();

        assert!(result.key.starts_with("sk_test_"));
        assert_eq!(result.key.len(), 51);
        assert!(result.hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_generate_key_invalid_environment() {
        let service = ApiKeyService::new();
        let result = service.generate_key("production");

        assert!(result.is_err());
    }

    #[test]
    fn test_keys_are_unique() {
        let service = ApiKeyService::new();

        let key1 = service.generate_key("live").unwrap();
        let key2 = service.generate_key("live").unwrap();

        assert_ne!(key1.key, key2.key);
        assert_ne!(key1.prefix, key2.prefix);
        assert_ne!(key1.hash, key2.hash);
    }

    #[test]
    fn test_verify_key_correct() {
        let service = ApiKeyService::new();
        let generated = service.generate_key("live").unwrap();

        let verified = service.verify_key(&generated.key, &generated.hash).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_verify_key_incorrect() {
        let service = ApiKeyService::new();
        let generated = service.generate_key("live").unwrap();

        // Try to verify with a different key
        let wrong_key = "sk_live_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk";
        let verified = service.verify_key(wrong_key, &generated.hash).unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_hash_key() {
        let service = ApiKeyService::new();

        let hash1 = service.hash_key("test_key_1").unwrap();
        let hash2 = service.hash_key("test_key_1").unwrap();

        // Same key should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(service.verify_key("test_key_1", &hash1).unwrap());
        assert!(service.verify_key("test_key_1", &hash2).unwrap());
    }

    #[test]
    fn test_extract_prefix() {
        let prefix =
            ApiKeyService::extract_prefix("sk_live_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk").unwrap();
        assert_eq!(prefix, "sk_live_ABCDEFGH");
    }

    #[test]
    fn test_extract_prefix_short_key() {
        let result = ApiKeyService::extract_prefix("sk_live");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_format_live() {
        // 43-char valid base64url string (represents 32 bytes)
        assert!(ApiKeyService::is_valid_format(
            "sk_live_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_test() {
        // 43-char valid base64url string (represents 32 bytes)
        assert!(ApiKeyService::is_valid_format(
            "sk_test_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_wrong_prefix() {
        assert!(!ApiKeyService::is_valid_format(
            "sk_prod_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
    }

    #[test]
    fn test_is_valid_format_too_short() {
        assert!(!ApiKeyService::is_valid_format("sk_live_ABC"));
    }

    #[test]
    fn test_is_valid_format_too_long() {
        assert!(!ApiKeyService::is_valid_format(
            "sk_live_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijkXXXXXXXXX"
        ));
    }

    #[test]
    fn test_get_environment_live() {
        let env = ApiKeyService::get_environment("sk_live_ABC").unwrap();
        assert_eq!(env, "live");
    }

    #[test]
    fn test_get_environment_test() {
        let env = ApiKeyService::get_environment("sk_test_ABC").unwrap();
        assert_eq!(env, "test");
    }

    #[test]
    fn test_get_environment_invalid() {
        let result = ApiKeyService::get_environment("sk_prod_ABC");
        assert!(result.is_err());
    }

    #[test]
    fn test_dummy_verify_does_not_panic() {
        let service = ApiKeyService::new();
        service.dummy_verify(); // Should complete without panic
    }

    #[test]
    fn test_key_entropy() {
        let service = ApiKeyService::new();

        // Generate keys and check they're all different
        // 10 iterations is sufficient - with 256 bits of entropy, collision probability
        // is astronomically low (~1 in 2^256). More iterations would slow down CI
        // due to Argon2id hashing cost (~0.6s per key).
        let mut keys: Vec<String> = Vec::new();
        for _ in 0..10 {
            let generated = service.generate_key("live").unwrap();
            assert!(!keys.contains(&generated.key), "Key collision detected!");
            keys.push(generated.key);
        }
    }

    #[test]
    fn test_hash_contains_argon2id_params() {
        let service = ApiKeyService::new();
        let generated = service.generate_key("live").unwrap();

        // Hash should contain Argon2id identifier and our parameters
        assert!(generated.hash.contains("argon2id"));
        assert!(generated.hash.contains("m=65536")); // Memory cost
        assert!(generated.hash.contains("t=3")); // Time cost
        assert!(generated.hash.contains("p=1")); // Parallelism
    }
}
