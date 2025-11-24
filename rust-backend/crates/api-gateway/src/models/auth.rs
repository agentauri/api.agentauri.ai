//! Authentication DTOs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Register request
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 100))]
    pub password: String,
}

/// Login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1))]
    pub username_or_email: String,

    #[validate(length(min = 1))]
    pub password: String,
}

/// Authentication response with JWT token
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// User response (safe for API, without password)
#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl From<shared::models::User> for UserResponse {
    fn from(user: shared::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
            is_active: user.is_active,
        }
    }
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // Subject (user_id)
    pub username: String, // Username for convenience
    pub exp: i64,         // Expiration time (as UTC timestamp)
    pub iat: i64,         // Issued at (as UTC timestamp)
}

impl Claims {
    pub fn new(user_id: String, username: String, expiration_hours: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        let exp = now + (expiration_hours * 3600);

        Self {
            sub: user_id,
            username,
            exp,
            iat: now,
        }
    }
}
