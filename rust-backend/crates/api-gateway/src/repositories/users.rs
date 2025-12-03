//! User repository for database operations

use anyhow::{Context, Result};
use shared::models::User;
use shared::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub struct UserRepository;

impl UserRepository {
    /// Create a new user
    #[allow(dead_code)]
    pub async fn create(
        pool: &DbPool,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User> {
        Self::create_with_executor(pool, username, email, password_hash).await
    }

    /// Create a new user with a generic executor (supports transactions)
    pub async fn create_with_executor<'e, E>(
        executor: E,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<User>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
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
    #[allow(dead_code)]
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

    /// Create a new social login user (no password)
    ///
    /// This function creates a user without a password for social-only authentication.
    /// The user can later add a password or link additional providers.
    pub async fn create_social_user<'e, E>(
        executor: E,
        username: &str,
        email: &str,
        primary_auth_provider: &str,
        avatar_url: Option<&str>,
    ) -> Result<User>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let user_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                id, username, email, password_hash,
                created_at, updated_at, is_active,
                primary_auth_provider, avatar_url
            )
            VALUES ($1, $2, $3, NULL, $4, $5, true, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&user_id)
        .bind(username)
        .bind(email)
        .bind(now)
        .bind(now)
        .bind(primary_auth_provider)
        .bind(avatar_url)
        .fetch_one(executor)
        .await
        .context("Failed to create social user")?;

        Ok(user)
    }

    // ========================================================================
    // Account Lockout Methods
    // ========================================================================

    /// Record a failed login attempt and potentially lock the account
    ///
    /// Implements progressive lockout:
    /// - Threshold: 5 failed attempts
    /// - Lockout duration: 15min base, doubles each time (up to 4h max)
    ///
    /// Returns the number of failed attempts after this failure.
    pub async fn record_failed_login(pool: &DbPool, user_id: &str) -> Result<i32> {
        let now = chrono::Utc::now();

        // Lockout configuration constants
        const MAX_FAILED_ATTEMPTS: i32 = 5;
        const BASE_LOCKOUT_MINUTES: i64 = 15;
        const MAX_LOCKOUT_MULTIPLIER: i32 = 4; // Max 4h lockout (15 * 2^4 = 240min)

        // Get current failed attempts to calculate lockout duration
        let current = sqlx::query_scalar::<_, i32>(
            r#"SELECT failed_login_attempts FROM users WHERE id = $1"#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .context("Failed to get current failed attempts")?;

        let new_attempts = current + 1;

        // Calculate lockout time if threshold reached
        let locked_until = if new_attempts >= MAX_FAILED_ATTEMPTS {
            // Progressive lockout: 15min, 30min, 60min, 120min, 240min (max 4h)
            let lockout_cycles = ((new_attempts - MAX_FAILED_ATTEMPTS) / MAX_FAILED_ATTEMPTS)
                .min(MAX_LOCKOUT_MULTIPLIER);
            let multiplier = 2_i64.pow(lockout_cycles as u32);
            let lockout_duration = chrono::Duration::minutes(BASE_LOCKOUT_MINUTES * multiplier);
            Some(now + lockout_duration)
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE users SET
                failed_login_attempts = $1,
                last_failed_login = $2,
                locked_until = $3
            WHERE id = $4
            "#,
        )
        .bind(new_attempts)
        .bind(now)
        .bind(locked_until)
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to record failed login")?;

        Ok(new_attempts)
    }

    /// Reset failed login attempts (call on successful login)
    ///
    /// This clears the failed_login_attempts counter, locked_until, and last_failed_login
    /// fields, effectively unlocking the account.
    pub async fn reset_failed_login(pool: &DbPool, user_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET
                failed_login_attempts = 0,
                locked_until = NULL,
                last_failed_login = NULL
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to reset failed login")?;

        Ok(())
    }

    /// Check if an account is currently locked
    ///
    /// Returns:
    /// - Ok(None) if account is not locked
    /// - Ok(Some(seconds_remaining)) if account is locked
    pub async fn check_account_lockout(pool: &DbPool, user_id: &str) -> Result<Option<i64>> {
        let locked_until: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar(r#"SELECT locked_until FROM users WHERE id = $1"#)
                .bind(user_id)
                .fetch_one(pool)
                .await
                .context("Failed to check account lockout")?;

        match locked_until {
            Some(until) => {
                let now = chrono::Utc::now();
                if until > now {
                    let remaining = (until - now).num_seconds();
                    Ok(Some(remaining))
                } else {
                    // Lock has expired
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}
