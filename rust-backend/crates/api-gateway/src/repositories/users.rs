//! User repository for database operations

use anyhow::{Context, Result};
use shared::models::User;
use shared::DbPool;
use uuid::Uuid;

pub struct UserRepository;

impl UserRepository {
    /// Create a new user
    pub async fn create(
        pool: &DbPool,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User> {
        let user_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, created_at, updated_at, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, true)
            RETURNING *
            "#,
        )
        .bind(&user_id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .context("Failed to create user")?;

        Ok(user)
    }

    /// Find user by username
    pub async fn find_by_username(pool: &DbPool, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to find user by username")?;

        Ok(user)
    }

    /// Find user by email
    pub async fn find_by_email(pool: &DbPool, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await
        .context("Failed to find user by email")?;

        Ok(user)
    }

    /// Find user by ID
    pub async fn find_by_id(pool: &DbPool, user_id: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("Failed to find user by ID")?;

        Ok(user)
    }

    /// Update last login timestamp
    pub async fn update_last_login(pool: &DbPool, user_id: &str) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE users
            SET last_login_at = $1
            WHERE id = $2
            "#,
        )
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to update last login")?;

        Ok(())
    }

    /// Check if username exists
    pub async fn username_exists(pool: &DbPool, username: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)
            "#,
        )
        .bind(username)
        .fetch_one(pool)
        .await
        .context("Failed to check if username exists")?;

        Ok(result)
    }

    /// Check if email exists
    pub async fn email_exists(pool: &DbPool, email: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .context("Failed to check if email exists")?;

        Ok(result)
    }
}
