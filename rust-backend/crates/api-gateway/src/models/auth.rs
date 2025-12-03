//! Authentication DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Register request
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"username": "johndoe", "email": "john@example.com", "password": "securepassword123"}))]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 100))]
    pub password: String,
}

/// Login request
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"username_or_email": "johndoe", "password": "securepassword123"}))]
pub struct LoginRequest {
    #[validate(length(min = 1))]
    pub username_or_email: String,

    #[validate(length(min = 1))]
    pub password: String,
}

/// Authentication response with JWT token
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// User response (safe for API, without password)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ========================================================================
    // RegisterRequest validation tests
    // ========================================================================

    #[test]
    fn test_register_request_valid() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_register_request_username_too_short() {
        let req = RegisterRequest {
            username: "ab".to_string(), // min 3 chars
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_register_request_username_too_long() {
        let req = RegisterRequest {
            username: "a".repeat(51), // max 50 chars
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn test_register_request_invalid_email() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "not-an-email".to_string(),
            password: "securepassword123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn test_register_request_password_too_short() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "short".to_string(), // min 8 chars
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    #[test]
    fn test_register_request_password_too_long() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "a".repeat(101), // max 100 chars
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    // ========================================================================
    // LoginRequest validation tests
    // ========================================================================

    #[test]
    fn test_login_request_valid_username() {
        let req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "password123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_login_request_valid_email() {
        let req = LoginRequest {
            username_or_email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_login_request_empty_username() {
        let req = LoginRequest {
            username_or_email: "".to_string(),
            password: "password123".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("username_or_email"));
    }

    #[test]
    fn test_login_request_empty_password() {
        let req = LoginRequest {
            username_or_email: "testuser".to_string(),
            password: "".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    // ========================================================================
    // Claims tests
    // ========================================================================

    #[test]
    fn test_claims_new_creates_valid_claims() {
        let claims = Claims::new("user-123".to_string(), "testuser".to_string(), 1);

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "testuser");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 3600); // 1 hour in seconds
    }

    #[test]
    fn test_claims_new_different_expiration() {
        let claims = Claims::new("user-123".to_string(), "testuser".to_string(), 24);

        assert_eq!(claims.exp - claims.iat, 86400); // 24 hours in seconds
    }

    // ========================================================================
    // Serialization tests
    // ========================================================================

    #[test]
    fn test_auth_response_serialization() {
        let response = AuthResponse {
            token: "test-token".to_string(),
            user: UserResponse {
                id: "user-123".to_string(),
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                created_at: chrono::Utc::now(),
                last_login_at: None,
                is_active: true,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-token"));
        assert!(json.contains("user-123"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_user_response_serialization() {
        let user = UserResponse {
            id: "user-123".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            created_at: chrono::Utc::now(),
            last_login_at: Some(chrono::Utc::now()),
            is_active: true,
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("is_active"));
    }
}
