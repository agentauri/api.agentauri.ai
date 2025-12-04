//! Social Authentication Service for OAuth 2.0 providers (Google, GitHub)
//!
//! This service handles the OAuth 2.0 authorization code flow for social login,
//! including authorization URL generation, token exchange, and user profile fetching.
//!
//! # Supported Providers
//!
//! - **Google**: Uses OpenID Connect discovery for automatic configuration
//! - **GitHub**: Manual OAuth 2.0 configuration
//!
//! # Security
//!
//! - CSRF protection via state parameter (base64-encoded JSON with HMAC signature)
//! - HTTPS-only redirect URIs in production
//! - Token encryption at rest (via UserIdentityRepository)

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenUrl,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OAuth state token expiration in minutes
const STATE_EXPIRATION_MINUTES: i64 = 10;

/// Errors that can occur during social authentication
#[derive(Debug, Error)]
#[allow(dead_code)] // Some variants used for future account linking feature
pub enum SocialAuthError {
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Invalid state token")]
    InvalidState,

    #[error("State token expired")]
    StateExpired,

    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("Failed to fetch user profile: {0}")]
    ProfileFetchFailed(String),

    #[error("Email not provided by provider")]
    EmailNotProvided,

    #[error("Email not verified by provider")]
    EmailNotVerified,

    #[error("Account already linked to another user")]
    AlreadyLinked,

    #[error("Cannot unlink last authentication method")]
    CannotUnlinkLast,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// OAuth 2.0 provider configuration
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
    pub user_info_url: String,
}

/// State token payload (stored in the state parameter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePayload {
    /// CSRF token for verification
    pub csrf_token: String,
    /// Optional user_id for account linking flow
    pub user_id: Option<String>,
    /// State creation timestamp
    pub created_at: i64,
    /// PKCE code verifier (base64-encoded)
    pub pkce_verifier: String,
    /// Redirect URL after authentication
    pub redirect_after: Option<String>,
}

/// User profile from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserProfile {
    /// Unique identifier from the provider
    pub provider_user_id: String,
    /// Email address (if provided and verified)
    pub email: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Whether email is verified by provider
    pub email_verified: bool,
}

/// Google user info response
#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

/// GitHub user info response
#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
    email: Option<String>,
    name: Option<String>,
    login: String,
    avatar_url: Option<String>,
}

/// GitHub email response
#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    verified: bool,
    primary: bool,
}

/// Token response from OAuth provider
#[derive(Debug, Deserialize)]
struct TokenResponseData {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
}

/// Service for social authentication (OAuth 2.0)
#[derive(Clone)]
pub struct SocialAuthService {
    /// Google OAuth configuration (optional)
    google_config: Option<OAuthProviderConfig>,
    /// GitHub OAuth configuration (optional)
    github_config: Option<OAuthProviderConfig>,
    /// HMAC secret for state token signing
    state_secret: String,
    /// HTTP client with connection pooling
    http_client: reqwest::Client,
    /// Frontend URL for redirects
    frontend_url: String,
}

impl SocialAuthService {
    /// Create a new SocialAuthService from environment variables
    ///
    /// # Panics
    /// In production (when ENVIRONMENT != "development"), panics if:
    /// - OAUTH_STATE_SECRET is not set or is less than 32 characters
    /// - FRONTEND_URL is not set
    pub fn from_env() -> Self {
        let google_config = Self::load_google_config();
        let github_config = Self::load_github_config();

        let is_development = std::env::var("ENVIRONMENT")
            .map(|e| e == "development")
            .unwrap_or(false);

        // State secret for HMAC signing - CRITICAL for security
        let state_secret = match std::env::var("OAUTH_STATE_SECRET") {
            Ok(secret) => {
                if secret.len() < 32 && !is_development {
                    panic!(
                        "OAUTH_STATE_SECRET must be at least 32 characters in production. \
                         Generate with: openssl rand -base64 32"
                    );
                }
                secret
            }
            Err(_) if is_development => {
                tracing::warn!(
                    "OAUTH_STATE_SECRET not set, using development default. \
                     DO NOT use this in production!"
                );
                "default-dev-secret-not-for-production".to_string()
            }
            Err(_) => {
                panic!(
                    "OAUTH_STATE_SECRET must be set in production. \
                     Generate with: openssl rand -base64 32"
                );
            }
        };

        // Frontend URL for redirects
        let frontend_url = match std::env::var("FRONTEND_URL") {
            Ok(url) => url,
            Err(_) if is_development => {
                tracing::warn!("FRONTEND_URL not set, using localhost for development");
                "http://localhost:3000".to_string()
            }
            Err(_) => {
                panic!("FRONTEND_URL must be set in production");
            }
        };

        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            google_config,
            github_config,
            state_secret,
            http_client,
            frontend_url,
        }
    }

    /// Load Google OAuth configuration from environment
    fn load_google_config() -> Option<OAuthProviderConfig> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").ok()?;

        Some(OAuthProviderConfig {
            client_id,
            client_secret,
            redirect_uri,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            user_info_url: "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
        })
    }

    /// Load GitHub OAuth configuration from environment
    fn load_github_config() -> Option<OAuthProviderConfig> {
        let client_id = std::env::var("GITHUB_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GITHUB_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("GITHUB_REDIRECT_URI").ok()?;

        Some(OAuthProviderConfig {
            client_id,
            client_secret,
            redirect_uri,
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scopes: vec!["user:email".to_string(), "read:user".to_string()],
            user_info_url: "https://api.github.com/user".to_string(),
        })
    }

    /// Check if Google OAuth is configured
    pub fn is_google_configured(&self) -> bool {
        self.google_config.is_some()
    }

    /// Check if GitHub OAuth is configured
    pub fn is_github_configured(&self) -> bool {
        self.github_config.is_some()
    }

    /// Get the frontend URL for redirects
    pub fn frontend_url(&self) -> &str {
        &self.frontend_url
    }

    /// Generate Google authorization URL
    ///
    /// # Arguments
    /// * `user_id` - Optional user ID for account linking (None for login/register)
    /// * `redirect_after` - Optional URL to redirect to after authentication
    ///
    /// # Returns
    /// The authorization URL to redirect the user to
    pub fn google_auth_url(
        &self,
        user_id: Option<String>,
        redirect_after: Option<String>,
    ) -> Result<String, SocialAuthError> {
        let config = self
            .google_config
            .as_ref()
            .ok_or_else(|| SocialAuthError::ProviderNotConfigured("google".to_string()))?;

        self.generate_auth_url(config, user_id, redirect_after)
    }

    /// Generate GitHub authorization URL
    ///
    /// # Arguments
    /// * `user_id` - Optional user ID for account linking (None for login/register)
    /// * `redirect_after` - Optional URL to redirect to after authentication
    ///
    /// # Returns
    /// The authorization URL to redirect the user to
    pub fn github_auth_url(
        &self,
        user_id: Option<String>,
        redirect_after: Option<String>,
    ) -> Result<String, SocialAuthError> {
        let config = self
            .github_config
            .as_ref()
            .ok_or_else(|| SocialAuthError::ProviderNotConfigured("github".to_string()))?;

        self.generate_auth_url(config, user_id, redirect_after)
    }

    /// Generate authorization URL for a provider
    fn generate_auth_url(
        &self,
        config: &OAuthProviderConfig,
        user_id: Option<String>,
        redirect_after: Option<String>,
    ) -> Result<String, SocialAuthError> {
        let auth_url = AuthUrl::new(config.auth_url.clone())
            .map_err(|e| SocialAuthError::Internal(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new(config.token_url.clone())
            .map_err(|e| SocialAuthError::Internal(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new(config.redirect_uri.clone())
            .map_err(|e| SocialAuthError::Internal(format!("Invalid redirect URI: {}", e)))?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate CSRF token
        let csrf_token = CsrfToken::new_random();

        // Create state payload
        let state_payload = StatePayload {
            csrf_token: csrf_token.secret().clone(),
            user_id,
            created_at: Utc::now().timestamp(),
            pkce_verifier: pkce_verifier.secret().clone(),
            redirect_after,
        };

        // Encode and sign state
        let state_token = self.encode_state(&state_payload)?;

        // Build authorization URL
        let mut auth_request = client
            .authorize_url(|| CsrfToken::new(state_token))
            .set_pkce_challenge(pkce_challenge);

        // Add scopes
        for scope in &config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, _) = auth_request.url();

        Ok(auth_url.to_string())
    }

    /// Exchange Google authorization code for tokens and fetch user profile
    pub async fn google_callback(
        &self,
        code: &str,
        state: &str,
    ) -> Result<(OAuthUserProfile, StatePayload), SocialAuthError> {
        let config = self
            .google_config
            .as_ref()
            .ok_or_else(|| SocialAuthError::ProviderNotConfigured("google".to_string()))?;

        // Verify and decode state
        let state_payload = self.decode_state(state)?;

        // Exchange code for tokens using reqwest directly (simpler than oauth2 HTTP client)
        let token_response = self
            .exchange_code_with_reqwest(config, code, &state_payload.pkce_verifier)
            .await?;

        // Fetch user profile
        let profile = self
            .fetch_google_profile(&token_response.access_token)
            .await?;

        Ok((profile, state_payload))
    }

    /// Exchange GitHub authorization code for tokens and fetch user profile
    pub async fn github_callback(
        &self,
        code: &str,
        state: &str,
    ) -> Result<(OAuthUserProfile, StatePayload), SocialAuthError> {
        let config = self
            .github_config
            .as_ref()
            .ok_or_else(|| SocialAuthError::ProviderNotConfigured("github".to_string()))?;

        // Verify and decode state
        let state_payload = self.decode_state(state)?;

        // Exchange code for tokens using reqwest directly
        let token_response = self
            .exchange_code_with_reqwest(config, code, &state_payload.pkce_verifier)
            .await?;

        // Fetch user profile
        let profile = self
            .fetch_github_profile(&token_response.access_token)
            .await?;

        Ok((profile, state_payload))
    }

    /// Exchange authorization code for tokens using reqwest
    async fn exchange_code_with_reqwest(
        &self,
        config: &OAuthProviderConfig,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<TokenResponseData, SocialAuthError> {
        let response = self
            .http_client
            .post(&config.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", config.redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|e| SocialAuthError::TokenExchangeFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SocialAuthError::TokenExchangeFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let token_data: TokenResponseData = response
            .json()
            .await
            .map_err(|e| SocialAuthError::TokenExchangeFailed(format!("Invalid JSON: {}", e)))?;

        Ok(token_data)
    }

    /// Fetch user profile from Google
    async fn fetch_google_profile(
        &self,
        access_token: &str,
    ) -> Result<OAuthUserProfile, SocialAuthError> {
        let response = self
            .http_client
            .get(&self.google_config.as_ref().unwrap().user_info_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SocialAuthError::ProfileFetchFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SocialAuthError::ProfileFetchFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let user_info: GoogleUserInfo = response
            .json()
            .await
            .map_err(|e| SocialAuthError::ProfileFetchFailed(format!("Invalid JSON: {}", e)))?;

        // Verify email is provided and verified
        let email_verified = user_info.email_verified.unwrap_or(false);

        Ok(OAuthUserProfile {
            provider_user_id: user_info.sub,
            email: user_info.email,
            display_name: user_info.name,
            avatar_url: user_info.picture,
            email_verified,
        })
    }

    /// Fetch user profile from GitHub
    async fn fetch_github_profile(
        &self,
        access_token: &str,
    ) -> Result<OAuthUserProfile, SocialAuthError> {
        // Fetch basic user info
        let response = self
            .http_client
            .get(&self.github_config.as_ref().unwrap().user_info_url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "AgentAuri-Backend")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SocialAuthError::ProfileFetchFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SocialAuthError::ProfileFetchFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let user_info: GitHubUserInfo = response
            .json()
            .await
            .map_err(|e| SocialAuthError::ProfileFetchFailed(format!("Invalid JSON: {}", e)))?;

        // GitHub may not return email in user info, need to fetch from /user/emails
        let (email, email_verified) = if user_info.email.is_some() {
            (user_info.email, true) // If email is in user info, it's verified
        } else {
            self.fetch_github_primary_email(access_token).await?
        };

        Ok(OAuthUserProfile {
            provider_user_id: user_info.id.to_string(),
            email,
            display_name: user_info.name.or(Some(user_info.login)),
            avatar_url: user_info.avatar_url,
            email_verified,
        })
    }

    /// Fetch primary verified email from GitHub
    async fn fetch_github_primary_email(
        &self,
        access_token: &str,
    ) -> Result<(Option<String>, bool), SocialAuthError> {
        let response = self
            .http_client
            .get("https://api.github.com/user/emails")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "AgentAuri-Backend")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                SocialAuthError::ProfileFetchFailed(format!("Email request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            // 404 or 403 typically means user hasn't granted email scope
            if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::FORBIDDEN
            {
                tracing::info!(
                    status = %status,
                    "GitHub email endpoint returned expected non-success status (likely missing email scope)"
                );
                return Ok((None, false));
            }
            // Other errors (5xx, 401, etc.) should be logged as warnings
            tracing::warn!(
                status = %status,
                "GitHub email endpoint failed with unexpected status - proceeding without email"
            );
            return Ok((None, false));
        }

        let emails: Vec<GitHubEmail> = response.json().await.map_err(|e| {
            SocialAuthError::ProfileFetchFailed(format!("Invalid email JSON: {}", e))
        })?;

        // Find primary verified email
        let primary_email = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified));

        match primary_email {
            Some(email) => Ok((Some(email.email.clone()), email.verified)),
            None => Ok((None, false)),
        }
    }

    /// Encode state payload to a signed base64 string
    fn encode_state(&self, payload: &StatePayload) -> Result<String, SocialAuthError> {
        let json = serde_json::to_string(payload)
            .map_err(|e| SocialAuthError::Internal(format!("Failed to encode state: {}", e)))?;

        // Create HMAC signature
        let signature = self.sign_data(json.as_bytes());

        // Combine payload and signature
        let combined = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&json),
            URL_SAFE_NO_PAD.encode(&signature)
        );

        Ok(combined)
    }

    /// Decode and verify state token
    fn decode_state(&self, state: &str) -> Result<StatePayload, SocialAuthError> {
        let parts: Vec<&str> = state.split('.').collect();
        if parts.len() != 2 {
            return Err(SocialAuthError::InvalidState);
        }

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| SocialAuthError::InvalidState)?;

        let signature_bytes = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| SocialAuthError::InvalidState)?;

        // Verify signature
        let expected_signature = self.sign_data(&payload_bytes);
        if signature_bytes != expected_signature {
            return Err(SocialAuthError::InvalidState);
        }

        // Parse payload
        let payload: StatePayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| SocialAuthError::InvalidState)?;

        // Check expiration
        let created_at = DateTime::<Utc>::from_timestamp(payload.created_at, 0)
            .ok_or(SocialAuthError::InvalidState)?;

        if Utc::now() > created_at + Duration::minutes(STATE_EXPIRATION_MINUTES) {
            return Err(SocialAuthError::StateExpired);
        }

        Ok(payload)
    }

    /// Sign data with HMAC-SHA256
    fn sign_data(&self, data: &[u8]) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.state_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> SocialAuthService {
        SocialAuthService {
            google_config: Some(OAuthProviderConfig {
                client_id: "test-client-id".to_string(),
                client_secret: "test-client-secret".to_string(),
                redirect_uri: "http://localhost:8080/callback".to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scopes: vec!["openid".to_string(), "email".to_string()],
                user_info_url: "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
            }),
            github_config: Some(OAuthProviderConfig {
                client_id: "test-github-client".to_string(),
                client_secret: "test-github-secret".to_string(),
                redirect_uri: "http://localhost:8080/github/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                scopes: vec!["user:email".to_string()],
                user_info_url: "https://api.github.com/user".to_string(),
            }),
            state_secret: "test-secret".to_string(),
            http_client: reqwest::Client::new(),
            frontend_url: "http://localhost:3000".to_string(),
        }
    }

    #[test]
    fn test_is_configured() {
        let service = create_test_service();
        assert!(service.is_google_configured());
        assert!(service.is_github_configured());
    }

    #[test]
    fn test_google_auth_url_generation() {
        let service = create_test_service();
        let result = service.google_auth_url(None, None);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.starts_with("https://accounts.google.com"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("scope="));
        assert!(url.contains("state="));
    }

    #[test]
    fn test_github_auth_url_generation() {
        let service = create_test_service();
        let result = service.github_auth_url(None, None);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.starts_with("https://github.com"));
        assert!(url.contains("client_id=test-github-client"));
    }

    #[test]
    fn test_auth_url_with_user_id_for_linking() {
        let service = create_test_service();
        let result = service.google_auth_url(Some("user-123".to_string()), None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_state_encode_decode_roundtrip() {
        let service = create_test_service();

        let payload = StatePayload {
            csrf_token: "test-csrf".to_string(),
            user_id: Some("user-123".to_string()),
            created_at: Utc::now().timestamp(),
            pkce_verifier: "test-verifier".to_string(),
            redirect_after: Some("/dashboard".to_string()),
        };

        let encoded = service.encode_state(&payload).unwrap();
        let decoded = service.decode_state(&encoded).unwrap();

        assert_eq!(decoded.csrf_token, payload.csrf_token);
        assert_eq!(decoded.user_id, payload.user_id);
        assert_eq!(decoded.pkce_verifier, payload.pkce_verifier);
        assert_eq!(decoded.redirect_after, payload.redirect_after);
    }

    #[test]
    fn test_state_invalid_signature() {
        let service = create_test_service();

        let payload = StatePayload {
            csrf_token: "test-csrf".to_string(),
            user_id: None,
            created_at: Utc::now().timestamp(),
            pkce_verifier: "test-verifier".to_string(),
            redirect_after: None,
        };

        let encoded = service.encode_state(&payload).unwrap();

        // Tamper with the state
        let tampered = format!("{}tampered", encoded);

        let result = service.decode_state(&tampered);
        assert!(matches!(result, Err(SocialAuthError::InvalidState)));
    }

    #[test]
    fn test_state_expired() {
        let service = create_test_service();

        let payload = StatePayload {
            csrf_token: "test-csrf".to_string(),
            user_id: None,
            created_at: (Utc::now() - Duration::minutes(STATE_EXPIRATION_MINUTES + 1)).timestamp(),
            pkce_verifier: "test-verifier".to_string(),
            redirect_after: None,
        };

        let encoded = service.encode_state(&payload).unwrap();
        let result = service.decode_state(&encoded);

        assert!(matches!(result, Err(SocialAuthError::StateExpired)));
    }

    #[test]
    fn test_provider_not_configured() {
        let service = SocialAuthService {
            google_config: None,
            github_config: None,
            state_secret: "test".to_string(),
            http_client: reqwest::Client::new(),
            frontend_url: "http://localhost:3000".to_string(),
        };

        let result = service.google_auth_url(None, None);
        assert!(matches!(
            result,
            Err(SocialAuthError::ProviderNotConfigured(_))
        ));
    }
}
