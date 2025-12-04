//! Authentication DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

/// Validate password strength
///
/// Requirements:
/// - Minimum 12 characters (NIST SP 800-63B recommendation)
/// - At least 1 uppercase letter
/// - At least 1 lowercase letter
/// - At least 1 digit
/// - At least 1 special character
/// - Not a common password
fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    // Length check (min 12)
    if password.len() < 12 {
        let mut err = ValidationError::new("password_too_short");
        err.message = Some("Password must be at least 12 characters".into());
        return Err(err);
    }

    // Uppercase check
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        let mut err = ValidationError::new("password_no_uppercase");
        err.message = Some("Password must contain at least one uppercase letter".into());
        return Err(err);
    }

    // Lowercase check
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        let mut err = ValidationError::new("password_no_lowercase");
        err.message = Some("Password must contain at least one lowercase letter".into());
        return Err(err);
    }

    // Digit check
    if !password.chars().any(|c| c.is_ascii_digit()) {
        let mut err = ValidationError::new("password_no_digit");
        err.message = Some("Password must contain at least one digit".into());
        return Err(err);
    }

    // Special character check
    let special_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~";
    if !password.chars().any(|c| special_chars.contains(c)) {
        let mut err = ValidationError::new("password_no_special");
        err.message = Some("Password must contain at least one special character".into());
        return Err(err);
    }

    // Common password check (top 20 most common)
    const COMMON_PASSWORDS: &[&str] = &[
        "password123456",
        "123456password",
        "qwerty123456",
        "admin123456!",
        "letmein12345",
        "welcome12345",
        "monkey123456",
        "dragon123456",
        "master123456",
        "login1234567",
        "abc123456789",
        "password1234",
        "iloveyou1234",
        "sunshine1234",
        "princess1234",
        "football1234",
        "baseball1234",
        "trustno1234!",
        "shadow123456",
        "michael12345",
    ];
    let password_lower = password.to_lowercase();
    for common in COMMON_PASSWORDS {
        if password_lower.contains(common) {
            let mut err = ValidationError::new("password_too_common");
            err.message = Some("Password is too common".into());
            return Err(err);
        }
    }

    Ok(())
}

/// Register request
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({"username": "johndoe", "email": "john@example.com", "password": "SecurePass123!"}))]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(
        length(min = 12, max = 128),
        custom(function = "validate_password_strength")
    )]
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
            password: "SecurePass123!".to_string(), // Valid: 12+ chars, upper, lower, digit, special
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_register_request_username_too_short() {
        let req = RegisterRequest {
            username: "ab".to_string(), // min 3 chars
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
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
            password: "SecurePass123!".to_string(),
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
            password: "SecurePass123!".to_string(),
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
            password: "Short1!".to_string(), // min 12 chars
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
            password: format!("{}Aa1!", "a".repeat(125)), // max 128 chars
        };
        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.field_errors().contains_key("password"));
    }

    // ========================================================================
    // Password strength validation tests
    // ========================================================================

    #[test]
    fn test_password_strength_valid() {
        assert!(validate_password_strength("SecurePass123!").is_ok());
        assert!(validate_password_strength("MyP@ssw0rd!123").is_ok());
        assert!(validate_password_strength("C0mpl3x_P@ss!").is_ok());
    }

    #[test]
    fn test_password_strength_no_uppercase() {
        let result = validate_password_strength("securepass123!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_no_uppercase");
    }

    #[test]
    fn test_password_strength_no_lowercase() {
        let result = validate_password_strength("SECUREPASS123!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_no_lowercase");
    }

    #[test]
    fn test_password_strength_no_digit() {
        let result = validate_password_strength("SecurePassword!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_no_digit");
    }

    #[test]
    fn test_password_strength_no_special() {
        let result = validate_password_strength("SecurePass1234");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_no_special");
    }

    #[test]
    fn test_password_strength_too_short() {
        let result = validate_password_strength("Short1!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_too_short");
    }

    #[test]
    fn test_password_strength_common_password() {
        let result = validate_password_strength("Password123456!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "password_too_common");
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
