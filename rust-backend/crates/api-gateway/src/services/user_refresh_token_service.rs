//! User Refresh Token Service
//!
//! Handles secure generation and verification of user refresh tokens.
//!
//! # Security Features
//!
//! - **32 bytes of entropy**: Uses `getrandom` (CSPRNG) for token generation
//! - **SHA-256 hashing**: Fast lookup with 256-bit collision resistance
//! - **Token rotation**: Old token invalidated when new one is issued
//! - **30-day expiration**: Refresh tokens expire after 30 days
//!
//! # Why SHA-256 instead of Argon2?
//!
//! Argon2 is designed for password hashing where the input has low entropy.
//! Our tokens have 256 bits of entropy, making brute-force infeasible.
//! SHA-256 provides fast O(1) lookup while maintaining security.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::repositories::RefreshTokenRepository;
use shared::DbPool;

/// Token prefix for user refresh tokens
const TOKEN_PREFIX: &str = "urt_"; // "user refresh token"

/// Length of random bytes for token generation (256 bits of entropy)
const TOKEN_ENTROPY_BYTES: usize = 32;

/// Refresh token validity in days
pub const REFRESH_TOKEN_VALIDITY_DAYS: i64 = 30;

/// Access token validity in seconds (1 hour)
pub const ACCESS_TOKEN_VALIDITY_SECS: i64 = 3600;

/// Maximum number of concurrent sessions per user
const MAX_SESSIONS_PER_USER: i64 = 10;

/// Errors that can occur during refresh token operations
#[derive(Debug, Error)]
pub enum RefreshTokenError {
    #[error("Failed to generate token: {0}")]
    GenerationError(String),

    #[error("Invalid or expired refresh token")]
    InvalidToken,

    #[error("Token has been revoked")]
    TokenRevoked,

    #[error("User not found")]
    UserNotFound,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Service for user refresh token operations
#[derive(Clone, Default)]
pub struct UserRefreshTokenService;

impl UserRefreshTokenService {
    /// Create a new UserRefreshTokenService
    pub fn new() -> Self {
        Self
    }

    /// Generate a new refresh token (raw value, not stored yet)
    fn generate_token(&self) -> Result<String, RefreshTokenError> {
        // Generate 32 bytes of random data
        let mut random_bytes = [0u8; TOKEN_ENTROPY_BYTES];
        getrandom::fill(&mut random_bytes)
            .map_err(|e| RefreshTokenError::GenerationError(e.to_string()))?;

        // Encode as URL-safe base64
        let encoded = URL_SAFE_NO_PAD.encode(random_bytes);

        // Build the full token with prefix
        Ok(format!("{}{}", TOKEN_PREFIX, encoded))
    }

    /// Hash a refresh token using SHA-256 (hex encoded)
    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Create and store a new refresh token for a user
    ///
    /// Also enforces a maximum session limit per user for security.
    pub async fn create_refresh_token(
        &self,
        pool: &DbPool,
        user_id: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<String, RefreshTokenError> {
        let token = self.generate_token()?;
        let token_hash = Self::hash_token(&token);

        let expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_VALIDITY_DAYS);

        RefreshTokenRepository::create(
            pool,
            user_id,
            &token_hash,
            expires_at,
            user_agent,
            ip_address,
        )
        .await
        .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))?;

        // Enforce session limit: revoke oldest sessions if over limit
        if let Err(e) =
            RefreshTokenRepository::enforce_session_limit(pool, user_id, MAX_SESSIONS_PER_USER)
                .await
        {
            tracing::warn!(
                "Failed to enforce session limit for user {}: {}",
                user_id,
                e
            );
        }

        Ok(token)
    }

    /// Validate a refresh token and return the user_id
    ///
    /// Returns the user_id if the token is valid, not expired, and not revoked.
    pub async fn validate_token(
        &self,
        pool: &DbPool,
        refresh_token: &str,
    ) -> Result<String, RefreshTokenError> {
        // Validate token format
        if !refresh_token.starts_with(TOKEN_PREFIX) {
            return Err(RefreshTokenError::InvalidToken);
        }

        // Hash the token for lookup
        let token_hash = Self::hash_token(refresh_token);

        // Look up the token
        let record = RefreshTokenRepository::find_valid_by_hash(pool, &token_hash)
            .await
            .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))?
            .ok_or(RefreshTokenError::InvalidToken)?;

        // Check if revoked
        if record.revoked_at.is_some() {
            return Err(RefreshTokenError::TokenRevoked);
        }

        Ok(record.user_id)
    }

    /// Validate and rotate a refresh token
    ///
    /// This atomically validates the token, revokes it, and creates a new one.
    /// Uses database row locking (SELECT ... FOR UPDATE) to prevent race conditions
    /// where two concurrent requests could both validate the same token.
    ///
    /// Returns (user_id, new_refresh_token).
    pub async fn validate_and_rotate(
        &self,
        pool: &DbPool,
        refresh_token: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(String, String), RefreshTokenError> {
        // Validate token format
        if !refresh_token.starts_with(TOKEN_PREFIX) {
            return Err(RefreshTokenError::InvalidToken);
        }

        // Generate new token before the atomic operation
        let new_token = self.generate_token()?;
        let old_token_hash = Self::hash_token(refresh_token);
        let new_token_hash = Self::hash_token(&new_token);
        let new_expires_at = Utc::now() + Duration::days(REFRESH_TOKEN_VALIDITY_DAYS);

        // Atomically rotate: validate, revoke old, create new (with row locking)
        let result = RefreshTokenRepository::atomic_rotate(
            pool,
            &old_token_hash,
            &new_token_hash,
            new_expires_at,
            user_agent,
            ip_address,
        )
        .await
        .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))?;

        let (user_id, _new_token_id) = result.ok_or(RefreshTokenError::InvalidToken)?;

        // Enforce session limit after creating new token
        if let Err(e) =
            RefreshTokenRepository::enforce_session_limit(pool, &user_id, MAX_SESSIONS_PER_USER)
                .await
        {
            tracing::warn!(
                "Failed to enforce session limit for user {}: {}",
                user_id,
                e
            );
        }

        Ok((user_id, new_token))
    }

    /// Revoke a specific refresh token
    pub async fn revoke_token(
        &self,
        pool: &DbPool,
        refresh_token: &str,
    ) -> Result<bool, RefreshTokenError> {
        let token_hash = Self::hash_token(refresh_token);

        let record = RefreshTokenRepository::find_valid_by_hash(pool, &token_hash)
            .await
            .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))?;

        if let Some(record) = record {
            RefreshTokenRepository::revoke(pool, &record.id)
                .await
                .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Revoke all refresh tokens for a user (logout from all devices)
    pub async fn revoke_all_for_user(
        &self,
        pool: &DbPool,
        user_id: &str,
    ) -> Result<u64, RefreshTokenError> {
        RefreshTokenRepository::revoke_all_for_user(pool, user_id)
            .await
            .map_err(|e| RefreshTokenError::DatabaseError(e.to_string()))
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
    fn test_generate_token_format() {
        let service = UserRefreshTokenService::new();
        let token = service.generate_token().unwrap();

        assert!(token.starts_with("urt_"));
        // Base64 of 32 bytes = 43 chars, plus prefix
        assert_eq!(token.len(), 4 + 43);
    }

    #[test]
    fn test_hash_token_deterministic() {
        let token = "urt_test_token_12345";
        let hash1 = UserRefreshTokenService::hash_token(token);
        let hash2 = UserRefreshTokenService::hash_token(token);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_different_tokens_different_hashes() {
        let service = UserRefreshTokenService::new();
        let token1 = service.generate_token().unwrap();
        let token2 = service.generate_token().unwrap();

        let hash1 = UserRefreshTokenService::hash_token(&token1);
        let hash2 = UserRefreshTokenService::hash_token(&token2);

        assert_ne!(token1, token2);
        assert_ne!(hash1, hash2);
    }
}
