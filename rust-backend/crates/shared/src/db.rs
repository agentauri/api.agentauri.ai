//! Database connection pooling utilities

use crate::config::DatabaseConfig;
use crate::error::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// Type alias for the database pool
pub type DbPool = PgPool;

/// Create a new database connection pool
///
/// # Arguments
///
/// * `config` - Database configuration
///
/// # Returns
///
/// A configured PostgreSQL connection pool
///
/// # Errors
///
/// Returns an error if the pool cannot be created or if the connection fails
pub async fn create_pool(config: &DatabaseConfig) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.connection_url())
        .await?;

    tracing::info!(
        "Database connection pool created with {} max connections",
        config.max_connections
    );

    Ok(pool)
}

/// Run database migrations
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Returns
///
/// Ok if migrations succeed
///
/// # Errors
///
/// Returns an error if migrations fail
pub async fn run_migrations(_pool: &DbPool) -> Result<()> {
    // Note: Migrations are run manually using database/run-migrations.sh
    // This function is a placeholder for future automatic migration support
    // For now, it just logs a message
    tracing::info!("Database migrations should be run manually (see database/README.md)");
    Ok(())
}

/// Check database connection health
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Returns
///
/// Ok if the database is healthy
///
/// # Errors
///
/// Returns an error if the connection check fails
pub async fn check_health(pool: &DbPool) -> Result<()> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
