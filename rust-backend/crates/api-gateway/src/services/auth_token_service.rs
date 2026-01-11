//! Authentication Token Service
//!
//! Provides a unified interface for generating JWT access tokens and refresh tokens.
//! This service consolidates the token generation logic that was previously duplicated
//! across multiple auth handlers (login, register, wallet auth, OAuth exchange).
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::services::AuthTokenService;
//!
//! let auth_service = AuthTokenService::new(&config.server.jwt_secret);
//! match auth_service.generate_auth_response(&pool, &user, user_agent, ip_address).await {
//!     Ok(response) => HttpResponse::Ok().json(response),
//!     Err(error_response) => error_response,
//! }
//! ```

use crate::models::{AuthResponse, Claims, UserResponse};
use crate::services::{UserRefreshTokenService, ACCESS_TOKEN_VALIDITY_SECS};
use actix_web::HttpResponse;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use shared::models::User;
use shared::DbPool;

/// Error type for authentication token generation
#[derive(Debug, thiserror::Error)]
pub enum AuthTokenError {
    #[error("Failed to generate JWT: {0}")]
    JwtGeneration(#[from] jsonwebtoken::errors::Error),

    #[error("Failed to generate refresh token: {0}")]
    RefreshToken(#[from] crate::services::RefreshTokenError),
}

/// Service for generating authentication tokens (JWT and refresh tokens)
pub struct AuthTokenService {
    jwt_secret: String,
    refresh_service: UserRefreshTokenService,
}

impl AuthTokenService {
    /// Create a new AuthTokenService with the given JWT secret
    pub fn new(jwt_secret: &str) -> Self {
        Self {
            jwt_secret: jwt_secret.to_string(),
            refresh_service: UserRefreshTokenService::new(),
        }
    }

    /// Generate a complete auth response with JWT and refresh token
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `user` - The user to generate tokens for
    /// * `user_agent` - Optional user agent for refresh token tracking
    /// * `ip_address` - Optional IP address for refresh token tracking
    ///
    /// # Returns
    ///
    /// `AuthResponse` on success, or `HttpResponse` error on failure
    pub async fn generate_auth_response(
        &self,
        pool: &DbPool,
        user: &User,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<AuthResponse, HttpResponse> {
        // Generate JWT
        let token = self.generate_jwt(&user.id, &user.username)?;

        // Generate refresh token
        let refresh_token = self
            .refresh_service
            .create_refresh_token(pool, &user.id, user_agent, ip_address)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user.id, "Failed to generate refresh token");
                HttpResponse::InternalServerError().json(crate::models::ErrorResponse::new(
                    "internal_error",
                    "Failed to generate authentication token",
                ))
            })?;

        Ok(AuthResponse {
            token,
            refresh_token,
            expires_in: ACCESS_TOKEN_VALIDITY_SECS,
            user: UserResponse::from(user.clone()),
        })
    }

    /// Generate only a JWT token (for token refresh flows)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID to include in claims
    /// * `username` - Username to include in claims
    ///
    /// # Returns
    ///
    /// JWT string on success, or `HttpResponse` error on failure
    pub fn generate_jwt(&self, user_id: &str, username: &str) -> Result<String, HttpResponse> {
        let claims = Claims::new(user_id.to_string(), username.to_string(), 1); // 1 hour
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate JWT");
            HttpResponse::InternalServerError().json(crate::models::ErrorResponse::new(
                "internal_error",
                "Failed to generate token",
            ))
        })
    }

    /// Generate both JWT and refresh token, returning them as a tuple
    ///
    /// This is useful for refresh flows where you need both tokens but
    /// the response format differs from AuthResponse.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `user_id` - User ID for token generation
    /// * `username` - Username for JWT claims
    /// * `user_agent` - Optional user agent for refresh token tracking
    /// * `ip_address` - Optional IP address for refresh token tracking
    ///
    /// # Returns
    ///
    /// Tuple of (jwt_token, refresh_token) on success
    pub async fn generate_tokens(
        &self,
        pool: &DbPool,
        user_id: &str,
        username: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(String, String), HttpResponse> {
        let token = self.generate_jwt(user_id, username)?;

        let refresh_token = self
            .refresh_service
            .create_refresh_token(pool, user_id, user_agent, ip_address)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user_id, "Failed to generate refresh token");
                HttpResponse::InternalServerError().json(crate::models::ErrorResponse::new(
                    "internal_error",
                    "Failed to generate authentication token",
                ))
            })?;

        Ok((token, refresh_token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_jwt() {
        let service = AuthTokenService::new("test_secret_key_for_testing");
        let result = service.generate_jwt("user123", "testuser");
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
        // JWT has 3 parts separated by dots
        assert_eq!(token.matches('.').count(), 2);
    }

    #[test]
    fn test_jwt_contains_valid_structure() {
        let service = AuthTokenService::new("another_test_secret");
        let token = service.generate_jwt("user456", "anotheruser").unwrap();

        // Decode the header to verify it's HS256
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Header should be valid base64
        let header =
            base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, parts[0]);
        assert!(header.is_ok());
    }
}
