//! Social authentication handlers for OAuth 2.0 providers (Google, GitHub)
//!
//! These handlers implement the OAuth 2.0 authorization code flow for social login.
//! They support both login/registration and account linking flows.

use actix_web::{http::header, web, HttpResponse, Responder};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use shared::{Config, DbPool};
use uuid::Uuid;

use crate::{
    models::{Claims, ErrorResponse},
    repositories::{
        MemberRepository, OrganizationRepository, UserIdentityRepository, UserRepository,
    },
    services::SocialAuthService,
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
/// GET /api/v1/auth/google
///
/// Redirects the user to Google's OAuth consent screen.
/// After authorization, Google redirects to /api/v1/auth/google/callback
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
/// GET /api/v1/auth/github
///
/// Redirects the user to GitHub's OAuth consent screen.
/// After authorization, GitHub redirects to /api/v1/auth/github/callback
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

/// Handle Google OAuth callback
///
/// GET /api/v1/auth/google/callback
///
/// Exchanges the authorization code for tokens, fetches the user profile,
/// and creates/logs in the user. Redirects to frontend with JWT token.
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
                tracing::error!("Google callback failed: {}", e);
                return redirect_with_error(&social_auth, "oauth_failed", &e.to_string());
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
    )
    .await
}

/// Handle GitHub OAuth callback
///
/// GET /api/v1/auth/github/callback
///
/// Exchanges the authorization code for tokens, fetches the user profile,
/// and creates/logs in the user. Redirects to frontend with JWT token.
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
                tracing::error!("GitHub callback failed: {}", e);
                return redirect_with_error(&social_auth, "oauth_failed", &e.to_string());
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
    )
    .await
}

/// Common handler for OAuth callbacks
///
/// This function:
/// 1. Checks if identity exists → login existing user
/// 2. Checks if email matches existing user → link identity to existing user
/// 3. Otherwise → create new user and identity
async fn handle_oauth_callback(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    provider: &str,
    profile: crate::services::OAuthUserProfile,
    redirect_after: Option<String>,
) -> HttpResponse {
    // 1. Check if identity already exists
    if let Ok(Some(identity)) =
        UserIdentityRepository::find_by_provider(&pool, provider, &profile.provider_user_id).await
    {
        // Identity exists - log in the user
        return login_existing_user(pool, config, social_auth, &identity.user_id, redirect_after)
            .await;
    }

    // 2. Get email from profile (required for new accounts)
    let email = match &profile.email {
        Some(email) if profile.email_verified => email.clone(),
        Some(email) => {
            // Email exists but not verified - for now, accept it with a warning
            tracing::warn!("User email {} not verified by {}", email, provider);
            email.clone()
        }
        None => {
            tracing::error!("No email provided by {}", provider);
            return redirect_with_error(
                &social_auth,
                "email_required",
                "Email is required for registration",
            );
        }
    };

    // 3. Check if email matches an existing user → link to existing account
    if let Ok(Some(existing_user)) = UserRepository::find_by_email(&pool, &email).await {
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
    generate_jwt_and_redirect(config, social_auth, user, redirect_after)
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
    generate_jwt_and_redirect(config, social_auth, user, redirect_after)
}

/// Generate JWT token and redirect to frontend
fn generate_jwt_and_redirect(
    config: web::Data<Config>,
    social_auth: web::Data<SocialAuthService>,
    user: shared::models::User,
    redirect_after: Option<String>,
) -> HttpResponse {
    // Generate JWT token
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

    // Build redirect URL
    let frontend_url = social_auth.frontend_url();
    let redirect_path = redirect_after.unwrap_or_else(|| "/".to_string());
    let redirect_url = format!(
        "{}{}?token={}",
        frontend_url,
        if redirect_path.starts_with('/') {
            redirect_path
        } else {
            format!("/{}", redirect_path)
        },
        token
    );

    HttpResponse::Found()
        .insert_header((header::LOCATION, redirect_url))
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
async fn find_unique_username(pool: &DbPool, base: &str) -> String {
    let mut username = base.to_string();
    let mut attempt = 0;

    loop {
        match UserRepository::username_exists(pool, &username).await {
            Ok(false) => return username,
            _ => {
                attempt += 1;
                username = format!("{}{}", base, attempt);
                if attempt > 100 {
                    // Fallback to UUID if too many collisions
                    return format!("user_{}", &Uuid::new_v4().to_string()[..8]);
                }
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
}
