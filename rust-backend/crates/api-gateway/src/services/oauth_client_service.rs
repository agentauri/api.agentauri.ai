//! OAuth Client Service
//!
//! This service handles secure OAuth client ID and secret generation, hashing, and verification.
//!
//! # Security Features
//!
//! - **32 bytes of entropy**: Uses `rand::OsRng` (CSPRNG) for secret generation
//! - **Argon2id hashing**: OWASP-recommended parameters (64MiB memory, 3 iterations, **p=4**)
//! - **Constant-time verification**: Argon2 verify is timing-attack resistant
//! - **Unique client IDs**: Each client has a unique identifier
//!
//! # Format
//!
//! ```text
//! Client ID:     client_<random_uuid>
//! Client Secret: cs_<43 base64url chars>
//! ```

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use thiserror::Error;
use uuid::Uuid;

/// Length of random bytes for secret generation (256 bits of entropy)
const SECRET_ENTROPY_BYTES: usize = 32;

/// Argon2 memory cost in KiB (64 MiB as per OWASP recommendations)
const ARGON2_MEMORY_COST: u32 = 65536;

/// Argon2 time cost (iterations)
const ARGON2_TIME_COST: u32 = 3;

/// Argon2 parallelism degree for OAuth (p=4)
const ARGON2_PARALLELISM: u32 = 4;

/// Errors that can occur during OAuth client operations
#[derive(Debug, Error)]
pub enum OAuthClientError {
    #[error("Failed to generate client ID: {0}")]
    GenerationError(String),

    #[error("Failed to hash client secret: {0}")]
    HashError(String),

    #[error("Failed to verify client secret: {0}")]
    VerificationError(String),

    #[error("Invalid client format")]
    InvalidFormat,
}

/// Result of generating a new OAuth client
#[derive(Debug)]
pub struct GeneratedOAuthClient {
    /// The client ID (public identifier)
    pub client_id: String,

    /// The client secret (to be shown to user ONLY ONCE)
    pub client_secret: String,

    /// The Argon2id hash of the client secret (to be stored in database)
    pub client_secret_hash: String,
}

/// Service for OAuth client operations
///
/// This service holds pre-computed values for timing attack mitigation.
#[derive(Clone)]
pub struct OAuthClientService {
    argon2: Argon2<'static>,
    /// Pre-computed valid hash for timing attack mitigation
    dummy_hash: String,
}

impl Default for OAuthClientService {
    fn default() -> Self {
        Self::new()
    }
}

/// Dummy secret used for timing attack mitigation
const DUMMY_SECRET: &str = "cs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

impl OAuthClientService {
    /// Create a new OAuthClientService with OWASP-recommended Argon2 parameters
    ///
    /// # CRITICAL: Argon2id Parallelism = 4
    ///
    /// OAuth client secrets use p=4 (parallelism=4), same as OAuth tokens.
    pub fn new() -> Self {
        // Use Argon2id variant with p=4
        // Parameters: 64 MiB memory, 3 iterations, 4 parallelism
        let params = Params::new(
            ARGON2_MEMORY_COST,
            ARGON2_TIME_COST,
            ARGON2_PARALLELISM, // p=4 for OAuth
            None,               // Default output length (32 bytes)
        )
        .expect("Invalid Argon2 parameters");

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        // Pre-compute a valid hash for timing attack mitigation
        let salt = SaltString::generate(&mut OsRng);
        let dummy_hash = argon2
            .hash_password(DUMMY_SECRET.as_bytes(), &salt)
            .expect("Failed to pre-compute dummy hash")
            .to_string();

        Self { argon2, dummy_hash }
    }

    /// Generate a new OAuth client with ID and secret
    ///
    /// # Returns
    /// A `GeneratedOAuthClient` containing the client ID, secret, and hash
    ///
    /// # Security
    /// - Client ID uses UUID v4 (122 bits of entropy)
    /// - Client secret uses OsRng (256 bits of entropy)
    /// - Hash uses Argon2id with p=4
    pub fn generate_client(&self) -> Result<GeneratedOAuthClient, OAuthClientError> {
        // Generate client ID using UUID
        let client_id = self.generate_client_id();

        // Generate client secret
        let client_secret = self.generate_client_secret()?;

        // Hash the client secret
        let client_secret_hash = self.hash_client_secret(&client_secret)?;

        Ok(GeneratedOAuthClient {
            client_id,
            client_secret,
            client_secret_hash,
        })
    }

    /// Generate a new client ID
    ///
    /// # Format
    /// `client_<uuid>` (e.g., `client_550e8400-e29b-41d4-a716-446655440000`)
    pub fn generate_client_id(&self) -> String {
        format!("client_{}", Uuid::new_v4())
    }

    /// Generate a new client secret
    ///
    /// # Format
    /// `cs_<43 base64url chars>`
    ///
    /// # Security
    /// - Uses OsRng (OS-provided CSPRNG)
    /// - 32 bytes (256 bits) of entropy
    pub fn generate_client_secret(&self) -> Result<String, OAuthClientError> {
        // Generate 32 bytes of random data using OS CSPRNG
        let mut random_bytes = [0u8; SECRET_ENTROPY_BYTES];
        getrandom::fill(&mut random_bytes)
            .map_err(|e| OAuthClientError::GenerationError(e.to_string()))?;

        // Encode as URL-safe base64 (no padding) - produces 43 chars
        let encoded = URL_SAFE_NO_PAD.encode(random_bytes);

        // Build the full secret with prefix
        let client_secret = format!("cs_{}", encoded);

        Ok(client_secret)
    }

    /// Hash a client secret using Argon2id with p=4
    ///
    /// # Arguments
    /// * `secret` - The full client secret to hash
    ///
    /// # Returns
    /// The Argon2id hash string (PHC format) with p=4
    pub fn hash_client_secret(&self, secret: &str) -> Result<String, OAuthClientError> {
        // Generate a random salt
        let salt = SaltString::generate(&mut OsRng);

        // Hash the secret with Argon2id (p=4)
        let hash = self
            .argon2
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|e| OAuthClientError::HashError(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a client secret against a stored hash
    ///
    /// # Arguments
    /// * `secret` - The full client secret to verify
    /// * `hash` - The stored Argon2id hash (with p=4)
    ///
    /// # Returns
    /// `true` if the secret matches the hash, `false` otherwise
    ///
    /// # Security
    /// - Uses constant-time comparison (built into Argon2 verify)
    pub fn verify_client_secret(&self, secret: &str, hash: &str) -> Result<bool, OAuthClientError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| OAuthClientError::VerificationError(e.to_string()))?;

        match self.argon2.verify_password(secret.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(OAuthClientError::VerificationError(e.to_string())),
        }
    }

    /// Perform a dummy hash operation for timing attack mitigation
    ///
    /// # Security
    ///
    /// Uses a pre-computed valid Argon2 hash (with p=4) to ensure
    /// timing is consistent regardless of whether the client exists.
    pub fn dummy_verify(&self) {
        let _ = self.verify_client_secret(DUMMY_SECRET, &self.dummy_hash);
    }

    /// Validate the format of a client ID
    ///
    /// # Arguments
    /// * `client_id` - The client ID to validate
    ///
    /// # Returns
    /// `true` if the client ID has a valid format, `false` otherwise
    pub fn is_valid_client_id_format(client_id: &str) -> bool {
        // Must start with "client_"
        if !client_id.starts_with("client_") {
            return false;
        }

        // The UUID part should be valid
        let uuid_part = &client_id[7..];
        Uuid::parse_str(uuid_part).is_ok()
    }

    /// Validate the format of a client secret
    ///
    /// # Arguments
    /// * `secret` - The client secret to validate
    ///
    /// # Returns
    /// `true` if the secret has a valid format, `false` otherwise
    pub fn is_valid_client_secret_format(secret: &str) -> bool {
        // Must start with "cs_"
        if !secret.starts_with("cs_") {
            return false;
        }

        // Total length should be 46 (3 prefix + 43 base64)
        if secret.len() != 46 {
            return false;
        }

        // The random part should be valid base64url
        let random_part = &secret[3..];
        URL_SAFE_NO_PAD.decode(random_part).is_ok()
    }
}

// We need getrandom for CSPRNG
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
    fn test_generate_client() {
        let service = OAuthClientService::new();
        let result = service.generate_client().unwrap();

        assert!(result.client_id.starts_with("client_"));
        assert!(result.client_secret.starts_with("cs_"));
        assert_eq!(result.client_secret.len(), 46);
        assert!(result.client_secret_hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_generate_client_id() {
        let service = OAuthClientService::new();

        let id1 = service.generate_client_id();
        let id2 = service.generate_client_id();

        assert!(id1.starts_with("client_"));
        assert!(id2.starts_with("client_"));
        assert_ne!(id1, id2); // UUIDs should be unique
    }

    #[test]
    fn test_generate_client_secret() {
        let service = OAuthClientService::new();

        let secret1 = service.generate_client_secret().unwrap();
        let secret2 = service.generate_client_secret().unwrap();

        assert!(secret1.starts_with("cs_"));
        assert!(secret2.starts_with("cs_"));
        assert_eq!(secret1.len(), 46);
        assert_ne!(secret1, secret2); // Secrets should be unique
    }

    #[test]
    fn test_verify_client_secret_correct() {
        let service = OAuthClientService::new();
        let generated = service.generate_client().unwrap();

        let verified = service
            .verify_client_secret(&generated.client_secret, &generated.client_secret_hash)
            .unwrap();
        assert!(verified);
    }

    #[test]
    fn test_verify_client_secret_incorrect() {
        let service = OAuthClientService::new();
        let generated = service.generate_client().unwrap();

        let wrong_secret = "cs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk";
        let verified = service
            .verify_client_secret(wrong_secret, &generated.client_secret_hash)
            .unwrap();
        assert!(!verified);
    }

    #[test]
    fn test_hash_client_secret() {
        let service = OAuthClientService::new();

        let hash1 = service.hash_client_secret("test_secret").unwrap();
        let hash2 = service.hash_client_secret("test_secret").unwrap();

        // Same secret should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(service.verify_client_secret("test_secret", &hash1).unwrap());
        assert!(service.verify_client_secret("test_secret", &hash2).unwrap());
    }

    #[test]
    fn test_is_valid_client_id_format() {
        let service = OAuthClientService::new();
        let client_id = service.generate_client_id();

        assert!(OAuthClientService::is_valid_client_id_format(&client_id));
        assert!(!OAuthClientService::is_valid_client_id_format("invalid"));
        assert!(!OAuthClientService::is_valid_client_id_format(
            "client_not-a-uuid"
        ));
    }

    #[test]
    fn test_is_valid_client_secret_format() {
        assert!(OAuthClientService::is_valid_client_secret_format(
            "cs_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
        assert!(!OAuthClientService::is_valid_client_secret_format(
            "invalid"
        ));
        assert!(!OAuthClientService::is_valid_client_secret_format(
            "cs_short"
        ));
    }

    #[test]
    fn test_dummy_verify_does_not_panic() {
        let service = OAuthClientService::new();
        service.dummy_verify(); // Should complete without panic
    }

    #[test]
    fn test_hash_contains_argon2id_params_p4() {
        let service = OAuthClientService::new();
        let generated = service.generate_client().unwrap();

        // Hash should contain Argon2id identifier and p=4
        assert!(generated.client_secret_hash.contains("argon2id"));
        assert!(generated.client_secret_hash.contains("m=65536"));
        assert!(generated.client_secret_hash.contains("t=3"));
        assert!(generated.client_secret_hash.contains("p=4")); // CRITICAL!
    }

    #[test]
    fn test_client_entropy() {
        let service = OAuthClientService::new();

        // Generate multiple clients and verify uniqueness
        let mut ids: Vec<String> = Vec::new();
        let mut secrets: Vec<String> = Vec::new();

        for _ in 0..10 {
            let generated = service.generate_client().unwrap();
            assert!(!ids.contains(&generated.client_id), "Client ID collision!");
            assert!(
                !secrets.contains(&generated.client_secret),
                "Client secret collision!"
            );
            ids.push(generated.client_id);
            secrets.push(generated.client_secret);
        }
    }
}
