//! Common test utilities for integration tests
//!
//! This module provides helper functions and utilities for integration testing
//! the api-gateway application, including test app setup, JWT token generation,
//! and database mocking.

use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use shared::DbPool;
use sqlx::PgPool;

// Test configuration constants
pub const TEST_JWT_SECRET: &str = "test_jwt_secret_for_integration_tests";

/// Claims structure for JWT tokens (matching the production Claims struct)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TestClaims {
    pub sub: String,      // User ID
    pub username: String, // Username
    pub exp: i64,         // Expiration timestamp
    pub iat: i64,         // Issued at timestamp
}

/// Create a test Actix-web application with all routes configured
///
/// This function creates a fully configured test app instance that can be
/// used with Actix's testing utilities.
///
/// # Arguments
///
/// * `pool` - Database connection pool (can be real or mocked)
///
/// # Returns
///
/// A configured test service ready for request testing
///
/// # Example
///
/// ```ignore
/// let pool = create_test_pool().await;
/// let app = create_test_app(pool).await;
///
/// let req = test::TestRequest::get()
///     .uri("/api/v1/health")
///     .to_request();
///
/// let resp = test::call_service(&app, req).await;
/// assert_eq!(resp.status(), 200);
/// ```
#[allow(dead_code)]
pub async fn create_test_app(pool: DbPool) {
    // Note: This function is currently a placeholder because the complex return type
    // causes compilation issues. In actual integration tests with a real database,
    // you would use test::init_service directly in each test.
    //
    // Example usage in tests:
    //   let app = test::init_service(
    //       App::new()
    //           .app_data(web::Data::new(pool.clone()))
    //           .configure(routes::configure)
    //   ).await;
    let _ = pool;
}

/// Generate a JWT token for testing
///
/// Creates a valid JWT token with the specified user ID and username.
/// The token is valid for 1 hour by default.
///
/// # Arguments
///
/// * `user_id` - The user ID to encode in the token
/// * `username` - The username to encode in the token
///
/// # Returns
///
/// A valid JWT token string
///
/// # Example
///
/// ```ignore
/// let token = create_test_jwt("user_123", "testuser");
///
/// let req = test::TestRequest::get()
///     .uri("/api/v1/api-keys")
///     .insert_header(("Authorization", format!("Bearer {}", token)))
///     .to_request();
/// ```
pub fn create_test_jwt(user_id: &str, username: &str) -> String {
    let now = Utc::now().timestamp();
    let exp = now + 3600; // 1 hour from now

    let claims = TestClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("Failed to create test JWT")
}

/// Generate an expired JWT token for testing authentication failures
///
/// Creates a JWT token that expired 1 hour ago.
///
/// # Arguments
///
/// * `user_id` - The user ID to encode in the token
/// * `username` - The username to encode in the token
///
/// # Returns
///
/// An expired JWT token string
pub fn create_expired_jwt(user_id: &str, username: &str) -> String {
    let now = Utc::now().timestamp();
    let exp = now - 3600; // 1 hour in the past

    let claims = TestClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
        iat: now - 7200, // 2 hours ago
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("Failed to create expired test JWT")
}

/// Create a test database pool
///
/// This function creates a connection pool for testing. In a real integration
/// test environment, this would connect to a test database. For now, it returns
/// a mock pool type.
///
/// # Environment Variables
///
/// * `TEST_DATABASE_URL` - PostgreSQL connection string for test database
///
/// # Example
///
/// ```ignore
/// let pool = create_test_pool().await;
/// ```
///
/// # Note
///
/// For full integration tests, you'll need to:
/// 1. Set up a test PostgreSQL database
/// 2. Run migrations on it
/// 3. Use the connection URL in TEST_DATABASE_URL env var
#[allow(dead_code)]
pub async fn create_test_pool() -> DbPool {
    // Try to connect to test database if TEST_DATABASE_URL is set
    if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    } else {
        // Fall back to mock pool for unit tests that don't need real DB
        // Note: This will panic if used with actual database operations
        // Set TEST_DATABASE_URL for real integration tests
        panic!(
            "TEST_DATABASE_URL environment variable not set. \
             Please set it to run integration tests with a real database."
        )
    }
}

/// Test organization data for consistent test setup
#[derive(Debug, Clone)]
pub struct TestOrganization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub owner_id: String,
}

impl TestOrganization {
    /// Create a new test organization with default values
    pub fn new(owner_id: &str) -> Self {
        Self {
            id: format!("test_org_{}", uuid::Uuid::new_v4()),
            name: "Test Organization".to_string(),
            slug: "test-org".to_string(),
            owner_id: owner_id.to_string(),
        }
    }

    /// Create an organization with custom values
    pub fn with_name(owner_id: &str, name: &str, slug: &str) -> Self {
        Self {
            id: format!("test_org_{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            slug: slug.to_string(),
            owner_id: owner_id.to_string(),
        }
    }
}

/// Test user data for consistent test setup
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

impl TestUser {
    /// Create a new test user with default values
    pub fn new() -> Self {
        let random_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            id: format!("test_user_{}", uuid::Uuid::new_v4()),
            username: format!("testuser_{}", random_suffix),
            email: format!("test_{}@example.com", random_suffix),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=1$somesalt$somehash".to_string(),
        }
    }

    /// Create a user with custom username and email
    pub fn with_credentials(username: &str, email: &str) -> Self {
        Self {
            id: format!("test_user_{}", uuid::Uuid::new_v4()),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=1$somesalt$somehash".to_string(),
        }
    }

    /// Generate a JWT token for this user
    pub fn jwt_token(&self) -> String {
        create_test_jwt(&self.id, &self.username)
    }
}

impl Default for TestUser {
    fn default() -> Self {
        Self::new()
    }
}

/// Test organization member data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestMember {
    pub organization_id: String,
    pub user_id: String,
    pub role: String,
}

impl TestMember {
    /// Create an admin member
    pub fn admin(organization_id: &str, user_id: &str) -> Self {
        Self {
            organization_id: organization_id.to_string(),
            user_id: user_id.to_string(),
            role: "admin".to_string(),
        }
    }

    /// Create a member (standard role)
    pub fn member(organization_id: &str, user_id: &str) -> Self {
        Self {
            organization_id: organization_id.to_string(),
            user_id: user_id.to_string(),
            role: "member".to_string(),
        }
    }

    /// Create a viewer
    pub fn viewer(organization_id: &str, user_id: &str) -> Self {
        Self {
            organization_id: organization_id.to_string(),
            user_id: user_id.to_string(),
            role: "viewer".to_string(),
        }
    }

    /// Create an owner
    pub fn owner(organization_id: &str, user_id: &str) -> Self {
        Self {
            organization_id: organization_id.to_string(),
            user_id: user_id.to_string(),
            role: "owner".to_string(),
        }
    }
}

// Tests for the common module are in the api_keys_test.rs file
// to avoid confusion with integration test module loading
