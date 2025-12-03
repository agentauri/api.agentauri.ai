//! Authentication handlers

use actix_web::{web, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use shared::{Config, DbPool};
use validator::Validate;

use crate::{
    models::{
        AuthResponse, Claims, ErrorResponse, LoginRequest, RegisterRequest, UserResponse,
        ROLE_OWNER,
    },
    repositories::{MemberRepository, OrganizationRepository, UserRepository},
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

    let response = AuthResponse {
        token,
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
            // Log error but don't block login (fail open for better availability)
            tracing::error!("Failed to check account lockout: {}", e);
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

    let response = AuthResponse {
        token,
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
