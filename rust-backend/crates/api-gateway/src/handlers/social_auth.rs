//! Social authentication handlers for OAuth 2.0 providers (Google, GitHub)
//!
//! These handlers implement the OAuth 2.0 authorization code flow for social login.
//! They support both login/registration and account linking flows.

use actix_web::{http::header, web, HttpRequest, HttpResponse, Responder};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use shared::{Config, DbPool};
use uuid::Uuid;

use crate::{
    models::{Claims, ErrorResponse},
    repositories::{
        MemberRepository, OrganizationRepository, UserIdentityRepository, UserRepository,
    },
    services::{SocialAuthError, SocialAuthService},
};

/// Query parameters for OAuth initiation
#[derive(Debug, serde::Deserialize)]
pub struct OAuthInitQuery {
    /// Optional redirect URL after authentication
    redirect_after: Option<String>,
}

/// Query parameters for OAuth callback
#[derive(Debug, serde::Deserialize)]
pub struct OAuthCallbackQuery {
    /// Authorization code from provider
    code: String,
    /// State parameter for CSRF protection
    state: String,
}

/// Initiate Google OAuth login
///
/// Redirects the user to Google's OAuth consent screen.
/// After authorization, Google redirects to /api/v1/auth/google/callback
#[utoipa::path(
    get,
    path = "/api/v1/auth/google",
    tag = "Authentication",
    params(
        ("redirect_after" = Option<String>, Query, description = "URL to redirect after authentication")
    ),
    responses(
        (status = 302, description = "Redirect to Google OAuth"),
        (status = 500, description = "Failed to initialize OAuth", body = ErrorResponse)
    )
)]
pub async fn google_auth(
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthInitQuery>,
) -> impl Responder {
    match social_auth.google_auth_url(None, query.redirect_after.clone()) {
        Ok(url) => HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            .finish(),
        Err(e) => {
            tracing::error!("Failed to generate Google auth URL: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::new(
                "oauth_error",
                "Failed to initialize Google authentication",
            ))
        }
    }
}

/// Initiate GitHub OAuth login
///
/// Redirects the user to GitHub's OAuth consent screen.
/// After authorization, GitHub redirects to /api/v1/auth/github/callback
#[utoipa::path(
    get,
    path = "/api/v1/auth/github",
    tag = "Authentication",
    params(
        ("redirect_after" = Option<String>, Query, description = "URL to redirect after authentication")
    ),
    responses(
        (status = 302, description = "Redirect to GitHub OAuth"),
        (status = 500, description = "Failed to initialize OAuth", body = ErrorResponse)
    )
)]
pub async fn github_auth(
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthInitQuery>,
) -> impl Responder {
    match social_auth.github_auth_url(None, query.redirect_after.clone()) {
        Ok(url) => HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            .finish(),
        Err(e) => {
            tracing::error!("Failed to generate GitHub auth URL: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::new(
                "oauth_error",
                "Failed to initialize GitHub authentication",
            ))
        }
    }
}

// ============================================================================
// Account Linking Endpoints
// ============================================================================

/// Link Google account to existing user
///
/// Requires authentication. Redirects to Google OAuth to link the provider
/// to the currently authenticated user's account.
#[utoipa::path(
    get,
    path = "/api/v1/auth/link/google",
    tag = "Authentication",
    security(
        ("bearer_auth" = []),
        ("cookie_auth" = [])
    ),
    params(
        ("redirect_after" = Option<String>, Query, description = "URL to redirect after linking")
    ),
    responses(
        (status = 302, description = "Redirect to Google OAuth"),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 500, description = "Failed to initialize OAuth", body = ErrorResponse)
    )
)]
pub async fn link_google(
    req: HttpRequest,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthInitQuery>,
) -> impl Responder {
    // Extract and validate JWT to get user_id
    let user_id = match extract_user_id_from_request(&req, &config) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match social_auth.google_auth_url(Some(user_id), query.redirect_after.clone()) {
        Ok(url) => HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            .finish(),
        Err(e) => {
            tracing::error!("Failed to generate Google auth URL for linking: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::new(
                "oauth_error",
                "Failed to initialize account linking",
            ))
        }
    }
}

/// Link GitHub account to existing user
///
/// Requires authentication. Redirects to GitHub OAuth to link the provider
/// to the currently authenticated user's account.
#[utoipa::path(
    get,
    path = "/api/v1/auth/link/github",
    tag = "Authentication",
    security(
        ("bearer_auth" = []),
        ("cookie_auth" = [])
    ),
    params(
        ("redirect_after" = Option<String>, Query, description = "URL to redirect after linking")
    ),
    responses(
        (status = 302, description = "Redirect to GitHub OAuth"),
        (status = 401, description = "Authentication required", body = ErrorResponse),
        (status = 500, description = "Failed to initialize OAuth", body = ErrorResponse)
    )
)]
pub async fn link_github(
    req: HttpRequest,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthInitQuery>,
) -> impl Responder {
    // Extract and validate JWT to get user_id
    let user_id = match extract_user_id_from_request(&req, &config) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match social_auth.github_auth_url(Some(user_id), query.redirect_after.clone()) {
        Ok(url) => HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            .finish(),
        Err(e) => {
            tracing::error!("Failed to generate GitHub auth URL for linking: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse::new(
                "oauth_error",
                "Failed to initialize account linking",
            ))
        }
    }
}

/// Extract user_id from JWT in request (header or cookie)
fn extract_user_id_from_request(
    req: &HttpRequest,
    config: &Config,
) -> Result<String, HttpResponse> {
    // Extract token from Authorization header or cookie
    let token = extract_token_from_request(req);

    let token = match token {
        Some(t) => t,
        None => {
            return Err(HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required to link accounts",
            )));
        }
    };

    // Validate JWT and extract user_id
    match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(config.server.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => Ok(data.claims.sub),
        Err(e) => {
            tracing::debug!("Invalid JWT token for account linking: {}", e);
            Err(HttpResponse::Unauthorized().json(ErrorResponse::new(
                "invalid_token",
                "Invalid or expired token",
            )))
        }
    }
}

/// Extract JWT token from Authorization header or auth-token cookie
fn extract_token_from_request(req: &HttpRequest) -> Option<String> {
    // First try Authorization header
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Then try cookie
    if let Some(cookie) = req.cookie("auth-token") {
        return Some(cookie.value().to_string());
    }

    None
}

/// Handle Google OAuth callback
///
/// Exchanges the authorization code for tokens, fetches the user profile,
/// and creates/logs in the user. Redirects to frontend with JWT token.
#[utoipa::path(
    get,
    path = "/api/v1/auth/google/callback",
    tag = "Authentication",
    params(
        ("code" = String, Query, description = "Authorization code from Google"),
        ("state" = String, Query, description = "State parameter for CSRF protection")
    ),
    responses(
        (status = 302, description = "Redirect to frontend with JWT token"),
        (status = 500, description = "OAuth callback failed", body = ErrorResponse)
    )
)]
pub async fn google_callback(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthCallbackQuery>,
) -> impl Responder {
    // Exchange code for profile
    let (profile, state_payload) =
        match social_auth.google_callback(&query.code, &query.state).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(
                    provider = "google",
                    error = %e,
                    error_type = ?e,
                    "OAuth callback failed"
                );
                let (error_code, user_message) = map_oauth_error_to_user_message(&e);
                return redirect_with_error(&social_auth, error_code, user_message);
            }
        };

    // Handle the OAuth result
    handle_oauth_callback(
        pool,
        config,
        social_auth,
        "google",
        profile,
        state_payload.redirect_after,
        state_payload.user_id, // Account linking mode
    )
    .await
}

/// Handle GitHub OAuth callback
///
/// Exchanges the authorization code for tokens, fetches the user profile,
/// and creates/logs in the user. Redirects to frontend with JWT token.
#[utoipa::path(
    get,
    path = "/api/v1/auth/github/callback",
    tag = "Authentication",
    params(
        ("code" = String, Query, description = "Authorization code from GitHub"),
        ("state" = String, Query, description = "State parameter for CSRF protection")
    ),
    responses(
        (status = 302, description = "Redirect to frontend with JWT token"),
        (status = 500, description = "OAuth callback failed", body = ErrorResponse)
    )
)]
pub async fn github_callback(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    query: web::Query<OAuthCallbackQuery>,
) -> impl Responder {
    // Exchange code for profile
    let (profile, state_payload) =
        match social_auth.github_callback(&query.code, &query.state).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(
                    provider = "github",
                    error = %e,
                    error_type = ?e,
                    "OAuth callback failed"
                );
                let (error_code, user_message) = map_oauth_error_to_user_message(&e);
                return redirect_with_error(&social_auth, error_code, user_message);
            }
        };

    // Handle the OAuth result
    handle_oauth_callback(
        pool,
        config,
        social_auth,
        "github",
        profile,
        state_payload.redirect_after,
        state_payload.user_id, // Account linking mode
    )
    .await
}

/// Map SocialAuthError to user-friendly (code, message) tuple
/// This prevents leaking internal error details to the frontend
fn map_oauth_error_to_user_message(e: &SocialAuthError) -> (&'static str, &'static str) {
    match e {
        SocialAuthError::InvalidState | SocialAuthError::StateExpired => (
            "session_expired",
            "Your session has expired. Please try again.",
        ),
        SocialAuthError::TokenExchangeFailed(_) => (
            "auth_failed",
            "Authentication with the provider failed. Please try again.",
        ),
        SocialAuthError::ProfileFetchFailed(_) => (
            "profile_error",
            "Could not retrieve your profile from the provider. Please try again.",
        ),
        SocialAuthError::EmailNotProvided => (
            "email_required",
            "A verified email address is required for registration.",
        ),
        SocialAuthError::EmailNotVerified => (
            "email_not_verified",
            "Please verify your email with the provider before signing in.",
        ),
        SocialAuthError::ProviderNotConfigured(_) => (
            "provider_unavailable",
            "This login method is currently unavailable.",
        ),
        _ => (
            "auth_error",
            "An error occurred during authentication. Please try again.",
        ),
    }
}

/// Common handler for OAuth callbacks
///
/// This function handles both login and account linking flows:
///
/// ## Login Flow (link_to_user_id = None):
/// 1. Checks if identity exists → login existing user
/// 2. Checks if email matches existing user → link identity to existing user
/// 3. Otherwise → create new user and identity
///
/// ## Account Linking Flow (link_to_user_id = Some):
/// 1. Checks if identity exists → error (already linked to another account)
/// 2. Links identity to the specified user
async fn handle_oauth_callback(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    provider: &str,
    profile: crate::services::OAuthUserProfile,
    redirect_after: Option<String>,
    link_to_user_id: Option<String>,
) -> HttpResponse {
    // Account linking mode - user is already authenticated
    if let Some(user_id) = link_to_user_id {
        return handle_account_linking(
            pool,
            config,
            social_auth,
            &user_id,
            provider,
            &profile,
            redirect_after,
        )
        .await;
    }

    // Normal login flow
    // 1. Check if identity already exists - handle errors explicitly to prevent duplicates
    match UserIdentityRepository::find_by_provider(&pool, provider, &profile.provider_user_id).await
    {
        Ok(Some(identity)) => {
            // Identity exists - log in the user
            return login_existing_user(
                pool,
                config,
                social_auth,
                &identity.user_id,
                redirect_after,
            )
            .await;
        }
        Ok(None) => {
            // Identity not found, continue to check email
        }
        Err(e) => {
            // Database error - abort to prevent creating duplicate accounts
            tracing::error!(
                provider = provider,
                provider_user_id = %profile.provider_user_id,
                error = %e,
                "Database error while looking up identity - aborting to prevent duplicates"
            );
            return redirect_with_error(
                &social_auth,
                "service_unavailable",
                "Authentication service temporarily unavailable. Please try again.",
            );
        }
    }

    // 2. Get email from profile (required for new accounts)
    let email = match &profile.email {
        Some(email) if profile.email_verified => email.clone(),
        Some(email) => {
            // Email exists but not verified - log warning but continue
            tracing::warn!(
                email = %email,
                provider = provider,
                "User email not verified by provider - proceeding with caution"
            );
            email.clone()
        }
        None => {
            tracing::error!(provider = provider, "No email provided by provider");
            return redirect_with_error(
                &social_auth,
                "email_required",
                "A verified email address is required for registration.",
            );
        }
    };

    // 3. Check if email matches an existing user → link to existing account
    // Handle errors explicitly to prevent creating duplicate accounts
    match UserRepository::find_by_email(&pool, &email).await {
        Ok(Some(existing_user)) => {
            // Email matches existing user - link this identity to that user
            return link_and_login(
                pool,
                config,
                social_auth,
                &existing_user.id,
                provider,
                &profile,
                redirect_after,
            )
            .await;
        }
        Ok(None) => {
            // No existing user with this email, proceed to create new user
        }
        Err(e) => {
            // Database error - abort to prevent creating duplicate accounts
            tracing::error!(
                email = %email,
                error = %e,
                "Database error while looking up user by email - aborting to prevent duplicates"
            );
            return redirect_with_error(
                &social_auth,
                "service_unavailable",
                "Authentication service temporarily unavailable. Please try again.",
            );
        }
    }

    // 4. Create new user and identity
    create_new_user(
        pool,
        config,
        social_auth,
        provider,
        &profile,
        &email,
        redirect_after,
    )
    .await
}

/// Log in an existing user
async fn login_existing_user(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    user_id: &str,
    redirect_after: Option<String>,
) -> HttpResponse {
    // Fetch user
    let user = match UserRepository::find_by_id(&pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::error!("User {} not found for identity", user_id);
            return redirect_with_error(&social_auth, "user_not_found", "User account not found");
        }
        Err(e) => {
            tracing::error!("Failed to fetch user: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Failed to fetch user");
        }
    };

    // Check if user is active
    if !user.is_active {
        return redirect_with_error(&social_auth, "account_disabled", "Account is disabled");
    }

    // Update last login
    if let Err(e) = UserRepository::update_last_login(&pool, user_id).await {
        tracing::warn!("Failed to update last login: {}", e);
    }

    // Generate JWT and redirect
    generate_jwt_and_redirect(&pool, config, social_auth, user, redirect_after).await
}

/// Link identity to existing user and log in
async fn link_and_login(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    user_id: &str,
    provider: &str,
    profile: &crate::services::OAuthUserProfile,
    redirect_after: Option<String>,
) -> HttpResponse {
    // Start transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Database error");
        }
    };

    // Create identity
    if let Err(e) = UserIdentityRepository::create(
        &mut *tx,
        user_id,
        provider,
        &profile.provider_user_id,
        profile.email.as_deref(),
        profile.display_name.as_deref(),
        profile.avatar_url.as_deref(),
    )
    .await
    {
        tracing::error!("Failed to create identity: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to link account");
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to link account");
    }

    tracing::info!(
        "Linked {} identity to existing user {} via email match",
        provider,
        user_id
    );

    // Log in the user
    login_existing_user(pool, config, social_auth, user_id, redirect_after).await
}

/// Handle explicit account linking (user is already authenticated)
///
/// This is called when a user explicitly links a new provider to their account.
/// Unlike auto-linking by email, this requires the user to be authenticated first.
async fn handle_account_linking(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    user_id: &str,
    provider: &str,
    profile: &crate::services::OAuthUserProfile,
    redirect_after: Option<String>,
) -> HttpResponse {
    // Check if this provider identity is already linked to ANY user
    match UserIdentityRepository::find_by_provider(&pool, provider, &profile.provider_user_id).await
    {
        Ok(Some(existing_identity)) => {
            if existing_identity.user_id == user_id {
                // Already linked to this user - just log in
                tracing::info!(
                    "Provider {} already linked to user {}, proceeding with login",
                    provider,
                    user_id
                );
                return login_existing_user(pool, config, social_auth, user_id, redirect_after)
                    .await;
            }
            // Linked to a different user - error
            tracing::warn!(
                "Attempted to link {} identity to user {} but already linked to {}",
                provider,
                user_id,
                existing_identity.user_id
            );
            return redirect_with_error(
                &social_auth,
                "already_linked",
                "This account is already linked to a different user.",
            );
        }
        Ok(None) => {
            // Not linked to anyone, proceed with linking
        }
        Err(e) => {
            tracing::error!("Failed to check identity: {}", e);
            return redirect_with_error(
                &social_auth,
                "internal_error",
                "Failed to check account linking.",
            );
        }
    }

    // Verify the target user exists
    let user = match UserRepository::find_by_id(&pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::error!("Link target user {} not found", user_id);
            return redirect_with_error(&social_auth, "user_not_found", "User account not found.");
        }
        Err(e) => {
            tracing::error!("Failed to fetch user: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Failed to link account.");
        }
    };

    // Start transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Database error.");
        }
    };

    // Create the identity link
    if let Err(e) = UserIdentityRepository::create(
        &mut *tx,
        user_id,
        provider,
        &profile.provider_user_id,
        profile.email.as_deref(),
        profile.display_name.as_deref(),
        profile.avatar_url.as_deref(),
    )
    .await
    {
        tracing::error!("Failed to create identity: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to link account.");
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to link account.");
    }

    tracing::info!(
        "Successfully linked {} identity to user {} via explicit linking",
        provider,
        user_id
    );

    // Generate JWT and redirect
    generate_jwt_and_redirect(&pool, config, social_auth, user, redirect_after).await
}

/// Create a new user and identity
async fn create_new_user(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    provider: &str,
    profile: &crate::services::OAuthUserProfile,
    email: &str,
    redirect_after: Option<String>,
) -> HttpResponse {
    // Generate username from display name or email
    let base_username = profile
        .display_name
        .as_ref()
        .map(|n| slugify_username(n))
        .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());

    // Start transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Database error");
        }
    };

    // Find unique username
    let username = find_unique_username(&pool, &base_username).await;

    // Create user (no password for social-only users)
    let user = match UserRepository::create_social_user(
        &mut *tx,
        &username,
        email,
        provider,
        profile.avatar_url.as_deref(),
        profile.display_name.as_deref(),
    )
    .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Failed to create account");
        }
    };

    // Create identity
    if let Err(e) = UserIdentityRepository::create(
        &mut *tx,
        &user.id,
        provider,
        &profile.provider_user_id,
        profile.email.as_deref(),
        profile.display_name.as_deref(),
        profile.avatar_url.as_deref(),
    )
    .await
    {
        tracing::error!("Failed to create identity: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to create account");
    }

    // Create personal organization
    let org_slug = generate_personal_slug(&username);
    let personal_org = match OrganizationRepository::create_with_executor(
        &mut *tx,
        &format!("{}'s Personal", username),
        &org_slug,
        None,
        &user.id,
        true,
    )
    .await
    {
        Ok(org) => org,
        Err(e) => {
            tracing::error!("Failed to create personal organization: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Failed to create account");
        }
    };

    // Add user as owner of personal organization
    if let Err(e) =
        MemberRepository::add_with_executor(&mut *tx, &personal_org.id, &user.id, "owner", None)
            .await
    {
        tracing::error!("Failed to add user as org owner: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to create account");
    }

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return redirect_with_error(&social_auth, "internal_error", "Failed to create account");
    }

    tracing::info!("Created new user {} via {} social login", user.id, provider);

    // Generate JWT and redirect
    generate_jwt_and_redirect(&pool, config, social_auth, user, redirect_after).await
}

/// Validate redirect path to prevent open redirect attacks
///
/// This function blocks various open redirect attack vectors:
/// - Protocol-relative URLs (`//evil.com`)
/// - Backslash escape attacks (`/\evil.com`)
/// - Absolute URLs in path (`https://evil.com`)
/// - URL-encoded bypass attempts (`%2f%2fevil.com`)
///
/// # Arguments
/// * `path` - The redirect path to validate
///
/// # Returns
/// * `Ok(String)` - The validated path (or "/" for empty paths)
/// * `Err(&'static str)` - Error message describing the validation failure
fn validate_redirect_path(path: &str) -> Result<String, &'static str> {
    // Empty or whitespace → use default "/"
    let path = path.trim();
    if path.is_empty() {
        return Ok("/".to_string());
    }

    // Must start with single /
    if !path.starts_with('/') {
        return Err("Redirect path must start with /");
    }

    // Block protocol-relative URLs (//evil.com)
    if path.starts_with("//") {
        return Err("Protocol-relative URLs not allowed");
    }

    // Block backslash escape (/\evil.com)
    if path.contains("/\\") || path.contains("\\") {
        return Err("Backslash in path not allowed");
    }

    // Block absolute URLs in path
    if path.contains("://") {
        return Err("Absolute URLs not allowed in path");
    }

    // URL decode and revalidate (catch %2f%2f, %3a%2f%2f, %5c bypass attempts)
    if let Ok(decoded) = urlencoding::decode(path) {
        let decoded_str = decoded.as_ref();
        if decoded_str.starts_with("//")
            || decoded_str.contains("://")
            || decoded_str.contains("\\")
        {
            return Err("Encoded redirect bypass detected");
        }
    }

    Ok(path.to_string())
}

/// Generate JWT token and redirect to frontend with auth-token and refresh-token cookies
///
/// In production, tokens are set as HttpOnly cookies for security.
/// In development, we use Authorization Code Flow - generating a temporary code
/// that the frontend exchanges for tokens via POST /api/v1/auth/exchange.
/// This prevents tokens from being exposed in URLs/browser history.
async fn generate_jwt_and_redirect(
    pool: &DbPool,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    user: shared::models::User,
    redirect_after: Option<String>,
) -> HttpResponse {
    use crate::services::{OAuthCodeService, UserRefreshTokenService, REFRESH_TOKEN_VALIDITY_DAYS};

    // Check environment
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production")
        .unwrap_or(false);

    // Validate and sanitize redirect path to prevent open redirect attacks
    let redirect_path = match redirect_after {
        Some(ref path) => validate_redirect_path(path).unwrap_or_else(|e| {
            tracing::warn!(
                path = %path,
                error = %e,
                "Invalid redirect path detected, using default"
            );
            "/".to_string()
        }),
        None => "/".to_string(),
    };

    let frontend_url = social_auth.frontend_url();

    // In development, use Authorization Code Flow to avoid exposing tokens in URLs
    if !is_production {
        let code_service = OAuthCodeService::new();
        let code = match code_service.create_code(pool, &user.id).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to generate OAuth code: {}", e);
                return redirect_with_error(
                    &social_auth,
                    "internal_error",
                    "Failed to generate authentication code",
                );
            }
        };

        // Redirect with code parameter - frontend will exchange it via POST /api/v1/auth/exchange
        let separator = if redirect_path.contains('?') {
            "&"
        } else {
            "?"
        };
        let redirect_url = format!(
            "{}{}{}code={}",
            frontend_url, redirect_path, separator, code
        );

        return HttpResponse::Found()
            .insert_header((header::LOCATION, redirect_url))
            .finish();
    }

    // Production flow: Generate tokens and set as cookies
    let claims = Claims::new(user.id.clone(), user.username.clone(), 1); // 1 hour
    let token = match encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.server.jwt_secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to generate JWT: {}", e);
            return redirect_with_error(&social_auth, "internal_error", "Failed to generate token");
        }
    };

    // Generate refresh token
    let refresh_service = UserRefreshTokenService::new();
    let refresh_token = match refresh_service
        .create_refresh_token(pool, &user.id, None, None)
        .await
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return redirect_with_error(
                &social_auth,
                "internal_error",
                "Failed to generate refresh token",
            );
        }
    };

    // Build redirect URL with validated path
    let redirect_url = format!("{}{}", frontend_url, redirect_path);

    // Build auth-token cookie with security flags
    let auth_cookie = actix_web::cookie::Cookie::build("auth-token", &token)
        .path("/")
        .http_only(true)
        .secure(true) // Production always uses HTTPS
        .same_site(actix_web::cookie::SameSite::Lax) // Lax for OAuth redirects
        .max_age(actix_web::cookie::time::Duration::hours(1))
        .finish();

    // Build refresh-token cookie with longer expiry
    let refresh_cookie = actix_web::cookie::Cookie::build("refresh-token", &refresh_token)
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(actix_web::cookie::SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(
            REFRESH_TOKEN_VALIDITY_DAYS,
        ))
        .finish();

    HttpResponse::Found()
        .insert_header((header::LOCATION, redirect_url))
        .cookie(auth_cookie)
        .cookie(refresh_cookie)
        .finish()
}

/// Redirect to frontend with error
fn redirect_with_error(
    social_auth: &SocialAuthService,
    error_code: &str,
    error_message: &str,
) -> HttpResponse {
    let frontend_url = social_auth.frontend_url();
    let redirect_url = format!(
        "{}/auth/error?code={}&message={}",
        frontend_url,
        urlencoding::encode(error_code),
        urlencoding::encode(error_message)
    );

    HttpResponse::Found()
        .insert_header((header::LOCATION, redirect_url))
        .finish()
}

/// Find a unique username by appending numbers if necessary
///
/// Handles database errors gracefully by falling back to UUID-based username
/// to prevent infinite loops or excessive database load during failures.
async fn find_unique_username(pool: &DbPool, base: &str) -> String {
    let mut username = base.to_string();
    let mut attempt = 0;

    loop {
        match UserRepository::username_exists(pool, &username).await {
            Ok(false) => return username,
            Ok(true) => {
                // Username exists, try next number
                attempt += 1;
                username = format!("{}{}", base, attempt);
                if attempt > 100 {
                    tracing::warn!(
                        base_username = %base,
                        "Exceeded 100 username collision attempts, falling back to UUID"
                    );
                    return format!("user_{}", &Uuid::new_v4().to_string()[..8]);
                }
            }
            Err(e) => {
                // Database error - log and use UUID fallback to avoid infinite loop
                tracing::error!(
                    username = %username,
                    error = %e,
                    "Database error checking username availability, using UUID fallback"
                );
                return format!("user_{}", &Uuid::new_v4().to_string()[..8]);
            }
        }
    }
}

/// Convert a display name to a valid username slug
fn slugify_username(name: &str) -> String {
    name.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Generate a slug for a personal organization from a username
fn generate_personal_slug(username: &str) -> String {
    let slug: String = username
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    let mut result = String::new();
    let mut prev_was_hyphen = false;

    for c in slug.chars() {
        if c == '-' {
            if !prev_was_hyphen && !result.is_empty() {
                result.push(c);
                prev_was_hyphen = true;
            }
        } else {
            result.push(c);
            prev_was_hyphen = false;
        }
    }

    let trimmed = result.trim_end_matches('-');
    format!("{}-personal", trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_username() {
        assert_eq!(slugify_username("John Doe"), "john_doe");
        assert_eq!(slugify_username("jane-doe"), "jane_doe");
        assert_eq!(slugify_username("  User  123  "), "user__123");
        assert_eq!(slugify_username("María García"), "mara_garca");
    }

    #[test]
    fn test_generate_personal_slug() {
        assert_eq!(generate_personal_slug("johndoe"), "johndoe-personal");
        assert_eq!(generate_personal_slug("john_doe"), "john-doe-personal");
        assert_eq!(generate_personal_slug("John"), "john-personal");
    }

    #[test]
    fn test_map_oauth_error_to_user_message_invalid_state() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::InvalidState);
        assert_eq!(code, "session_expired");
        assert!(msg.contains("expired"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_state_expired() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::StateExpired);
        assert_eq!(code, "session_expired");
        assert!(msg.contains("expired"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_token_exchange_failed() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::TokenExchangeFailed(
            "secret error details".to_string(),
        ));
        assert_eq!(code, "auth_failed");
        // Ensure secret details are NOT leaked
        assert!(!msg.contains("secret"));
        assert!(msg.contains("failed"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_profile_fetch_failed() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::ProfileFetchFailed(
            "internal API response".to_string(),
        ));
        assert_eq!(code, "profile_error");
        // Ensure internal details are NOT leaked
        assert!(!msg.contains("internal"));
        assert!(!msg.contains("API"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_email_not_provided() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::EmailNotProvided);
        assert_eq!(code, "email_required");
        assert!(msg.contains("email"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_email_not_verified() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::EmailNotVerified);
        assert_eq!(code, "email_not_verified");
        assert!(msg.contains("verify"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_provider_not_configured() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::ProviderNotConfigured(
            "google".to_string(),
        ));
        assert_eq!(code, "provider_unavailable");
        // Ensure provider name is NOT leaked
        assert!(!msg.contains("google"));
    }

    #[test]
    fn test_map_oauth_error_to_user_message_internal_error() {
        let (code, msg) = map_oauth_error_to_user_message(&SocialAuthError::Internal(
            "database connection string".to_string(),
        ));
        assert_eq!(code, "auth_error");
        // Ensure internal details are NOT leaked
        assert!(!msg.contains("database"));
        assert!(!msg.contains("connection"));
    }

    // ==================== Open Redirect Validation Tests ====================

    #[test]
    fn test_validate_redirect_path_valid_paths() {
        // Valid paths should pass
        assert_eq!(validate_redirect_path("/").unwrap(), "/");
        assert_eq!(validate_redirect_path("/dashboard").unwrap(), "/dashboard");
        assert_eq!(
            validate_redirect_path("/api/v1/triggers").unwrap(),
            "/api/v1/triggers"
        );
        assert_eq!(
            validate_redirect_path("/path?query=value").unwrap(),
            "/path?query=value"
        );
        assert_eq!(
            validate_redirect_path("/path#fragment").unwrap(),
            "/path#fragment"
        );
    }

    #[test]
    fn test_validate_redirect_path_empty_returns_default() {
        assert_eq!(validate_redirect_path("").unwrap(), "/");
        assert_eq!(validate_redirect_path("  ").unwrap(), "/");
        assert_eq!(validate_redirect_path("\t\n").unwrap(), "/");
    }

    #[test]
    fn test_validate_redirect_path_blocks_protocol_relative() {
        // Protocol-relative URLs (//evil.com) should be blocked
        assert!(validate_redirect_path("//evil.com").is_err());
        assert!(validate_redirect_path("//evil.com/path").is_err());
        assert!(validate_redirect_path("//localhost").is_err());
    }

    #[test]
    fn test_validate_redirect_path_blocks_no_leading_slash() {
        // Paths without leading / should be blocked
        assert!(validate_redirect_path("evil.com").is_err());
        assert!(validate_redirect_path("dashboard").is_err());
        assert!(validate_redirect_path("https://evil.com").is_err());
    }

    #[test]
    fn test_validate_redirect_path_blocks_backslash_escape() {
        // Backslash attacks should be blocked
        assert!(validate_redirect_path("/\\evil.com").is_err());
        assert!(validate_redirect_path("/path\\evil.com").is_err());
        assert!(validate_redirect_path("\\evil.com").is_err());
    }

    #[test]
    fn test_validate_redirect_path_blocks_absolute_url_in_path() {
        // Absolute URLs in path should be blocked
        assert!(validate_redirect_path("/https://evil.com").is_err());
        assert!(validate_redirect_path("/path?url=https://evil.com").is_err());
        assert!(validate_redirect_path("/javascript://alert(1)").is_err());
    }

    #[test]
    fn test_validate_redirect_path_blocks_encoded_bypasses() {
        // URL-encoded bypass attempts should be blocked
        // %2f = /, %5c = \, %3a = :
        assert!(validate_redirect_path("/%2f%2fevil.com").is_err()); // //evil.com
        assert!(validate_redirect_path("/%2Fevil.com").is_err()); // /evil.com (starts ok, but decoded would be //evil.com after first /)
        assert!(validate_redirect_path("/%5cevil.com").is_err()); // \evil.com
        assert!(validate_redirect_path("/path%3a%2f%2fevil.com").is_err()); // path://evil.com
    }

    #[test]
    fn test_validate_redirect_path_allows_safe_encoded_chars() {
        // Safe encoded characters should be allowed
        // %20 = space, %3F = ?, %26 = &
        assert!(validate_redirect_path("/path%20with%20spaces").is_ok());
        assert!(validate_redirect_path("/search%3Fquery%3Dtest").is_ok());
    }
}
