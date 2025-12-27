//! Authentication handlers

use actix_web::{cookie::Cookie, web, HttpRequest, HttpResponse, Responder};
use alloy::primitives::{Address, PrimitiveSignature};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use shared::{Config, DbPool};
use uuid::Uuid;
use validator::Validate;

use crate::{
    models::{
        AuthResponse, Claims, ErrorResponse, LoginRequest, LogoutResponse, MeResponse,
        NonceResponse, OrganizationInfo, RefreshTokenRequest, RefreshTokenResponse,
        RegisterRequest, UserResponse, WalletInfo, WalletLoginRequest, ROLE_OWNER,
    },
    repositories::{
        MemberRepository, OrganizationRepository, UserIdentityRepository, UserRepository,
    },
};

/// Register a new user
///
/// Creates a new user account with username, email, and password.
/// Also creates a personal organization for the user.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = AuthResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 409, description = "Username or email already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn register(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<RegisterRequest>,
) -> impl Responder {
    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Check if username already exists
    match UserRepository::username_exists(&pool, &req.username).await {
        Ok(true) => {
            return HttpResponse::Conflict().json(ErrorResponse::new(
                "username_exists",
                "Username already taken",
            ));
        }
        Err(e) => {
            tracing::error!("Failed to check username existence: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process registration",
            ));
        }
        _ => {}
    }

    // Check if email already exists
    match UserRepository::email_exists(&pool, &req.email).await {
        Ok(true) => {
            return HttpResponse::Conflict().json(ErrorResponse::new(
                "email_exists",
                "Email already registered",
            ));
        }
        Err(e) => {
            tracing::error!("Failed to check email existence: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process registration",
            ));
        }
        _ => {}
    }

    // Hash password
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match argon2.hash_password(req.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process registration",
            ));
        }
    };

    // Start a transaction for atomic user+organization+member creation
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process registration",
            ));
        }
    };

    // Create user within transaction
    let user = match UserRepository::create_with_executor(
        &mut *tx,
        &req.username,
        &req.email,
        &password_hash,
    )
    .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            // Transaction will be rolled back automatically when dropped
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create user",
            ));
        }
    };

    // Create personal organization for the user within transaction
    // Try the base slug first, then add numeric suffix if collision occurs
    let base_slug = generate_personal_slug(&req.username);
    let mut slug_attempt = 0;
    let personal_org = loop {
        let slug = if slug_attempt == 0 {
            base_slug.clone()
        } else {
            format!("{}-{}", base_slug, slug_attempt)
        };

        match OrganizationRepository::create_with_executor(
            &mut *tx,
            &format!("{}'s Personal", req.username),
            &slug,
            None,
            &user.id,
            true, // is_personal
        )
        .await
        {
            Ok(org) => {
                // Log if we had to retry due to slug collision
                if slug_attempt > 0 {
                    tracing::warn!(
                        "Personal organization slug collision resolved for user {}, used slug: {}",
                        req.username,
                        org.slug
                    );
                }
                break org;
            }
            Err(e) => {
                let error_string = e.to_string();
                // Check if it's a slug collision (unique constraint violation)
                if (error_string.contains("duplicate key")
                    || error_string.contains("unique constraint")
                    || error_string.contains("organizations_slug_key"))
                    && slug_attempt < 3
                {
                    slug_attempt += 1;
                    continue;
                }
                // Not a slug collision or too many retries
                tracing::error!("Failed to create personal organization: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to complete registration",
                ));
            }
        }
    };

    // Add user as owner of personal organization within transaction
    if let Err(e) =
        MemberRepository::add_with_executor(&mut *tx, &personal_org.id, &user.id, ROLE_OWNER, None)
            .await
    {
        tracing::error!("Failed to add user as org owner: {}", e);
        // Transaction will be rolled back automatically when dropped
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to complete registration",
        ));
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to complete registration",
        ));
    }

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
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate authentication token",
            ));
        }
    };

    // Generate refresh token
    use crate::services::{UserRefreshTokenService, ACCESS_TOKEN_VALIDITY_SECS};
    let refresh_service = UserRefreshTokenService::new();
    let refresh_token = match refresh_service
        .create_refresh_token(&pool, &user.id, None, None)
        .await
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate authentication token",
            ));
        }
    };

    let response = AuthResponse {
        token,
        refresh_token,
        expires_in: ACCESS_TOKEN_VALIDITY_SECS,
        user: UserResponse::from(user),
    };

    HttpResponse::Created().json(response)
}

/// Login with username/email and password
///
/// Authenticates a user and returns a JWT token.
/// Implements account lockout for brute-force protection.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 400, description = "Validation error or no password set", body = ErrorResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 403, description = "Account disabled", body = ErrorResponse),
        (status = 429, description = "Account locked due to too many failed attempts", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn login(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<LoginRequest>,
) -> impl Responder {
    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Find user by username or email
    let user = if req.username_or_email.contains('@') {
        UserRepository::find_by_email(&pool, &req.username_or_email).await
    } else {
        UserRepository::find_by_username(&pool, &req.username_or_email).await
    };

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "invalid_credentials",
                "Invalid credentials",
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process login",
            ));
        }
    };

    // Check if user is active
    if !user.is_active {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "account_disabled",
            "Account is disabled",
        ));
    }

    // Check if account is locked (brute-force protection)
    match UserRepository::check_account_lockout(&pool, &user.id).await {
        Ok(Some(seconds_remaining)) => {
            tracing::warn!(
                user_id = %user.id,
                seconds_remaining = seconds_remaining,
                "Login attempt on locked account"
            );
            return HttpResponse::TooManyRequests().json(ErrorResponse::new(
                "account_locked",
                format!(
                    "Account is temporarily locked. Try again in {} seconds.",
                    seconds_remaining
                ),
            ));
        }
        Ok(None) => {
            // Account is not locked, proceed
        }
        Err(e) => {
            // SECURITY: Fail closed - deny login when lockout check fails
            // This prevents brute-force attacks from bypassing lockout protection
            // during database issues
            tracing::error!(
                user_id = %user.id,
                error = %e,
                "Failed to check account lockout - denying login for security"
            );
            return HttpResponse::ServiceUnavailable().json(ErrorResponse::new(
                "service_unavailable",
                "Unable to verify account status. Please try again later.",
            ));
        }
    }

    // Check if user has a password (social-only users don't)
    let password_hash = match &user.password_hash {
        Some(hash) => hash,
        None => {
            // User registered via social login, can't login with password
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "no_password",
                "This account uses social login. Please sign in with Google or GitHub.",
            ));
        }
    };

    // Verify password
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to parse password hash: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process login",
            ));
        }
    };

    let argon2 = Argon2::default();
    if argon2
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        // Record failed login attempt (may trigger lockout)
        match UserRepository::record_failed_login(&pool, &user.id).await {
            Ok(attempts) => {
                tracing::warn!(
                    user_id = %user.id,
                    failed_attempts = attempts,
                    "Failed login attempt"
                );
            }
            Err(e) => {
                tracing::error!("Failed to record failed login: {}", e);
            }
        }

        return HttpResponse::Unauthorized().json(ErrorResponse::new(
            "invalid_credentials",
            "Invalid credentials",
        ));
    }

    // Successful login - reset failed login attempts
    if let Err(e) = UserRepository::reset_failed_login(&pool, &user.id).await {
        tracing::warn!("Failed to reset failed login counter: {}", e);
    }

    // Update last login timestamp
    if let Err(e) = UserRepository::update_last_login(&pool, &user.id).await {
        tracing::warn!("Failed to update last login: {}", e);
    }

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
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate authentication token",
            ));
        }
    };

    // Generate refresh token
    use crate::services::{UserRefreshTokenService, ACCESS_TOKEN_VALIDITY_SECS};
    let refresh_service = UserRefreshTokenService::new();
    let refresh_token = match refresh_service
        .create_refresh_token(&pool, &user.id, None, None)
        .await
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate authentication token",
            ));
        }
    };

    let response = AuthResponse {
        token,
        refresh_token,
        expires_in: ACCESS_TOKEN_VALIDITY_SECS,
        user: UserResponse::from(user),
    };

    HttpResponse::Ok().json(response)
}

/// Generate a slug for a personal organization from a username
fn generate_personal_slug(username: &str) -> String {
    // Convert to lowercase and replace non-alphanumeric characters with hyphens
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

    // Remove consecutive hyphens and trim hyphens from ends
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

    // Trim trailing hyphen and add "-personal" suffix
    let trimmed = result.trim_end_matches('-');
    format!("{}-personal", trimmed)
}

// ============================================================================
// Session endpoints
// ============================================================================

/// Generate a nonce for SIWE wallet authentication
///
/// Returns a nonce and challenge message for wallet signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/auth/nonce",
    tag = "Authentication",
    responses(
        (status = 200, description = "Nonce generated", body = NonceResponse),
        (status = 500, description = "Failed to generate nonce", body = ErrorResponse)
    )
)]
pub async fn generate_nonce(pool: web::Data<DbPool>) -> impl Responder {
    let nonce = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    // Store nonce in database for verification
    let result = sqlx::query(
        r#"
        INSERT INTO used_nonces (nonce, expires_at)
        VALUES ($1, $2)
        ON CONFLICT (nonce) DO NOTHING
        "#,
    )
    .bind(&nonce)
    .bind(expires_at)
    .execute(pool.get_ref())
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to store nonce: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "internal_error",
            "Failed to generate nonce",
        ));
    }

    // Build SIWE message format
    let message = format!(
        "Sign this message to authenticate with AgentAuri.\n\nNonce: {}\nExpires: {}",
        nonce,
        expires_at.to_rfc3339()
    );

    HttpResponse::Ok().json(NonceResponse {
        nonce,
        expires_at,
        message,
    })
}

/// Get current authenticated user's session info
///
/// Returns the user profile with linked wallets, providers, and organizations.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(
        ("bearer_auth" = []),
        ("cookie_auth" = [])
    ),
    responses(
        (status = 200, description = "User session info", body = MeResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_me(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
) -> impl Responder {
    // Extract token from Authorization header or cookie
    let token = extract_token(&req);

    let token = match token {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ));
        }
    };

    // Validate JWT
    let claims = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(config.server.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            tracing::debug!("Invalid JWT token: {}", e);
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "invalid_token",
                "Invalid or expired token",
            ));
        }
    };

    // Fetch user
    let user = match UserRepository::find_by_id(&pool, &claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized()
                .json(ErrorResponse::new("user_not_found", "User not found"));
        }
        Err(e) => {
            tracing::error!("Failed to fetch user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch user info",
            ));
        }
    };

    // Fetch linked identities (providers and wallets)
    let identities = match UserIdentityRepository::find_by_user_id(&pool, &user.id).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!("Failed to fetch user identities: {}", e);
            Vec::new()
        }
    };

    let providers: Vec<String> = identities
        .iter()
        .filter(|i| i.provider != "wallet")
        .map(|i| i.provider.clone())
        .collect();

    let wallets: Vec<WalletInfo> = identities
        .iter()
        .filter(|i| i.provider == "wallet")
        .filter_map(|i| {
            i.wallet_address.as_ref().map(|addr| WalletInfo {
                address: addr.clone(),
                chain_id: i.chain_id,
            })
        })
        .collect();

    // Fetch user's organizations
    let memberships = match MemberRepository::find_by_user(&pool, &user.id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to fetch memberships: {}", e);
            Vec::new()
        }
    };

    let mut organizations = Vec::new();
    for membership in memberships {
        if let Ok(Some(org)) =
            OrganizationRepository::find_by_id(&pool, &membership.organization_id).await
        {
            organizations.push(OrganizationInfo {
                id: org.id,
                name: org.name,
                slug: org.slug,
                role: membership.role,
            });
        }
    }

    HttpResponse::Ok().json(MeResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        name: user.display_name,
        avatar: user.avatar_url,
        wallets,
        providers,
        organizations,
        created_at: user.created_at,
    })
}

/// Logout and clear authentication cookie
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Authentication",
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse)
    )
)]
pub async fn logout() -> impl Responder {
    // Build cookie deletion
    let cookie = Cookie::build("auth-token", "")
        .path("/")
        .http_only(true)
        .max_age(actix_web::cookie::time::Duration::ZERO) // Expire immediately
        .finish();

    HttpResponse::Ok().cookie(cookie).json(LogoutResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    })
}

/// Refresh access token using a refresh token
///
/// Exchanges a valid refresh token for a new access token and refresh token.
/// Implements token rotation: the old refresh token is invalidated.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Authentication",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshTokenResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn refresh_token(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: HttpRequest,
    body: web::Json<RefreshTokenRequest>,
) -> impl Responder {
    use crate::services::{UserRefreshTokenService, ACCESS_TOKEN_VALIDITY_SECS};

    // Validate request
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Extract client info for audit
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    // Validate and rotate the refresh token
    let refresh_service = UserRefreshTokenService::new();
    let (user_id, new_refresh_token) = match refresh_service
        .validate_and_rotate(
            &pool,
            &body.refresh_token,
            user_agent.as_deref(),
            ip_address.as_deref(),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::debug!("Refresh token validation failed: {}", e);
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "invalid_refresh_token",
                "Invalid or expired refresh token",
            ));
        }
    };

    // Look up the user to get username for JWT claims
    let user = match UserRepository::find_by_id(&pool, &user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::error!("User not found for valid refresh token: {}", user_id);
            return HttpResponse::InternalServerError()
                .json(ErrorResponse::new("internal_error", "User not found"));
        }
        Err(e) => {
            tracing::error!("Failed to lookup user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to process request",
            ));
        }
    };

    // Generate new JWT
    let claims = Claims::new(user.id.clone(), user.username.clone(), 1);
    let token = match encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.server.jwt_secret.as_bytes()),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate JWT: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate token",
            ));
        }
    };

    // Set auth-token cookie
    let is_production = std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "production")
        .unwrap_or(false);

    let cookie = Cookie::build("auth-token", &token)
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(actix_web::cookie::SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::hours(1))
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(RefreshTokenResponse {
            token,
            refresh_token: new_refresh_token,
            expires_in: ACCESS_TOKEN_VALIDITY_SECS,
        })
}

// ============================================================================
// Wallet authentication (SIWE)
// ============================================================================

/// Login with wallet using SIWE (Sign-In With Ethereum)
///
/// Verifies an EIP-191 signed message and returns a JWT token.
/// Creates a new user if this is the first login with this wallet.
#[utoipa::path(
    post,
    path = "/api/v1/auth/wallet",
    tag = "Authentication",
    request_body = WalletLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 400, description = "Invalid request or signature", body = ErrorResponse),
        (status = 401, description = "Invalid or expired nonce", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn wallet_login(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    req: web::Json<WalletLoginRequest>,
) -> impl Responder {
    use crate::services::{UserRefreshTokenService, ACCESS_TOKEN_VALIDITY_SECS};

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Parse wallet address
    let address = match req.address.parse::<Address>() {
        Ok(addr) => addr,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_address",
                "Invalid wallet address format",
            ));
        }
    };

    // Extract nonce from message
    let nonce = match extract_nonce_from_message(&req.message) {
        Some(n) => n,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_message",
                "Could not extract nonce from message",
            ));
        }
    };

    // Verify nonce exists and is not expired
    let nonce_valid: Option<chrono::DateTime<chrono::Utc>> = match sqlx::query_scalar(
        r#"
        SELECT expires_at FROM used_nonces
        WHERE nonce = $1 AND used_at IS NULL AND expires_at > NOW()
        "#,
    )
    .bind(&nonce)
    .fetch_optional(pool.get_ref())
    .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to verify nonce: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to verify nonce",
            ));
        }
    };

    if nonce_valid.is_none() {
        return HttpResponse::Unauthorized().json(ErrorResponse::new(
            "invalid_nonce",
            "Invalid or expired nonce",
        ));
    }

    // Mark nonce as used
    let _ = sqlx::query(r#"UPDATE used_nonces SET used_at = NOW() WHERE nonce = $1"#)
        .bind(&nonce)
        .execute(pool.get_ref())
        .await;

    // Parse and verify signature (EIP-191 personal sign)
    let signature = match req.signature.parse::<PrimitiveSignature>() {
        Ok(sig) => sig,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "invalid_signature",
                "Invalid signature format",
            ));
        }
    };

    // Recover signer from signature
    let message_hash = alloy::primitives::eip191_hash_message(req.message.as_bytes());
    let recovered = match signature.recover_address_from_prehash(&message_hash) {
        Ok(addr) => addr,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new(
                "signature_recovery_failed",
                "Failed to recover address from signature",
            ));
        }
    };

    // Verify recovered address matches claimed address
    if recovered != address {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "signature_mismatch",
            "Signature does not match claimed address",
        ));
    }

    // Checksummed address for storage
    let checksummed_address = format!("{:?}", address);

    // Find or create user for this wallet (using chain_id 1 for Ethereum mainnet)
    let chain_id = 1i32;

    // Check if wallet identity exists
    let identity =
        match UserIdentityRepository::find_by_wallet(&pool, &checksummed_address, chain_id).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to find wallet identity: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to process wallet login",
                ));
            }
        };

    let user = if let Some(identity) = identity {
        // Existing wallet - fetch user
        match UserRepository::find_by_id(&pool, &identity.user_id).await {
            Ok(Some(user)) => {
                // Update last used
                let _ = UserIdentityRepository::update_last_used(&pool, &identity.id).await;
                user
            }
            Ok(None) => {
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "user_not_found",
                    "Linked user not found",
                ));
            }
            Err(e) => {
                tracing::error!("Failed to fetch user: {}", e);
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse::new("internal_error", "Failed to fetch user"));
            }
        }
    } else {
        // New wallet - create user with transaction
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Failed to start transaction: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to process registration",
                ));
            }
        };

        // Generate username from address (first 6 + last 4 chars)
        let short_address = format!(
            "{}...{}",
            &checksummed_address[..6],
            &checksummed_address[checksummed_address.len() - 4..]
        );
        let username = format!("wallet_{}", &checksummed_address[2..10].to_lowercase());

        // Create user (no password, no email for wallet-only users)
        let placeholder_email = format!("{}@wallet.agentauri.local", &checksummed_address[2..10]);

        let user = match UserRepository::create_social_user(
            &mut *tx,
            &username,
            &placeholder_email,
            "wallet",
            None,
            Some(&short_address),
        )
        .await
        {
            Ok(user) => user,
            Err(e) => {
                tracing::error!("Failed to create wallet user: {}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "internal_error",
                    "Failed to create user",
                ));
            }
        };

        // Create wallet identity
        if let Err(e) = UserIdentityRepository::create_wallet(
            &mut *tx,
            &user.id,
            &checksummed_address,
            chain_id,
        )
        .await
        {
            tracing::error!("Failed to create wallet identity: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to link wallet",
            ));
        }

        // Create personal organization
        let base_slug = format!("{}-personal", &checksummed_address[2..10].to_lowercase());
        if let Ok(org) = OrganizationRepository::create_with_executor(
            &mut *tx,
            &format!("{}'s Personal", short_address),
            &base_slug,
            None,
            &user.id,
            true,
        )
        .await
        {
            let _ =
                MemberRepository::add_with_executor(&mut *tx, &org.id, &user.id, ROLE_OWNER, None)
                    .await;
        }

        // Commit transaction
        if let Err(e) = tx.commit().await {
            tracing::error!("Failed to commit transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to complete registration",
            ));
        }

        user
    };

    // Update last login
    let _ = UserRepository::update_last_login(&pool, &user.id).await;

    // Generate JWT
    let claims = Claims::new(user.id.clone(), user.username.clone(), 1);
    let token = match encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.server.jwt_secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to generate JWT: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate token",
            ));
        }
    };

    // Generate refresh token
    let refresh_service = UserRefreshTokenService::new();
    let user_agent = http_req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok());
    let ip_address = http_req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());
    let refresh_token = match refresh_service
        .create_refresh_token(&pool, &user.id, user_agent, ip_address.as_deref())
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to generate refresh token: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to generate refresh token",
            ));
        }
    };

    HttpResponse::Ok().json(AuthResponse {
        token,
        refresh_token,
        expires_in: ACCESS_TOKEN_VALIDITY_SECS,
        user: UserResponse::from(user),
    })
}

/// Extract nonce from SIWE message
fn extract_nonce_from_message(message: &str) -> Option<String> {
    for line in message.lines() {
        if let Some(nonce) = line.strip_prefix("Nonce: ") {
            return Some(nonce.trim().to_string());
        }
    }
    None
}

/// Extract JWT token from Authorization header or auth-token cookie
fn extract_token(req: &HttpRequest) -> Option<String> {
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
