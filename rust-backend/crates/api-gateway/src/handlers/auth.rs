//! Authentication handlers

use actix_web::{web, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use shared::{Config, DbPool};
use validator::Validate;

use crate::{
    models::{AuthResponse, Claims, ErrorResponse, LoginRequest, RegisterRequest, UserResponse},
    repositories::UserRepository,
};

/// Register a new user
///
/// POST /api/v1/auth/register
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

    // Create user
    let user = match UserRepository::create(&pool, &req.username, &req.email, &password_hash).await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create user",
            ));
        }
    };

    // Generate JWT token
    let claims = Claims::new(user.id.clone(), user.username.clone(), 24 * 7); // 7 days
    let token = match encode(
        &Header::default(),
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
/// POST /api/v1/auth/login
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

    // Verify password
    let parsed_hash = match PasswordHash::new(&user.password_hash) {
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
        return HttpResponse::Unauthorized().json(ErrorResponse::new(
            "invalid_credentials",
            "Invalid credentials",
        ));
    }

    // Update last login timestamp
    if let Err(e) = UserRepository::update_last_login(&pool, &user.id).await {
        tracing::warn!("Failed to update last login: {}", e);
    }

    // Generate JWT token
    let claims = Claims::new(user.id.clone(), user.username.clone(), 24 * 7); // 7 days
    let token = match encode(
        &Header::default(),
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
