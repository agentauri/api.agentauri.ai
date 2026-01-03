//! Database connection pooling utilities
//!
//! Supports optional read replica configuration for read scaling.
//! When a read replica is configured via `DB_READ_HOST`, SELECT queries
//! can be routed to the read pool while writes go to the primary pool.

use crate::config::DatabaseConfig;
use crate::error::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use std::time::Duration;

/// Type alias for the database pool (backward compatibility)
pub type DbPool = PgPool;

/// Database pools wrapper supporting read/write splitting
///
/// When a read replica is configured, provides separate pools for
/// read (SELECT) and write (INSERT/UPDATE/DELETE) operations.
/// Falls back to the write pool for reads if no replica is configured.
#[derive(Clone)]
pub struct DbPools {
    /// Primary pool for write operations
    write: Arc<PgPool>,
    /// Read replica pool (same as write if no replica configured)
    read: Arc<PgPool>,
    /// Whether a dedicated read replica is configured
    has_read_replica: bool,
}

impl DbPools {
    /// Create database pools from configuration
    ///
    /// If `DB_READ_HOST` is configured, creates a separate read pool.
    /// Otherwise, the read pool uses the same connection as write.
    pub async fn from_config(config: &DatabaseConfig) -> Result<Self> {
        // Create primary (write) pool
        let write_pool = create_pool(config).await?;
        let write = Arc::new(write_pool);

        // Create read pool if replica is configured (with retry)
        let (read, has_read_replica) = if let Some(replica_url) = config.read_replica_url() {
            let replica_config = config.read_replica.as_ref().unwrap();
            let read_pool =
                create_read_replica_pool_with_retry(&replica_url, replica_config, config).await?;

            tracing::info!(
                "Read replica pool created: max={}, min={}",
                replica_config.max_connections,
                replica_config.min_connections
            );

            (Arc::new(read_pool), true)
        } else {
            tracing::info!("No read replica configured, using primary pool for reads");
            (Arc::clone(&write), false)
        };

        Ok(Self {
            write,
            read,
            has_read_replica,
        })
    }

    /// Get the write (primary) pool for INSERT/UPDATE/DELETE operations
    #[inline]
    pub fn write(&self) -> &PgPool {
        &self.write
    }

    /// Get the read pool for SELECT operations
    /// Returns the replica pool if configured, otherwise the primary pool
    #[inline]
    pub fn read(&self) -> &PgPool {
        &self.read
    }

    /// Get a reference to the primary pool (backward compatibility)
    /// Prefer using `write()` or `read()` for clarity
    #[inline]
    pub fn primary(&self) -> &PgPool {
        &self.write
    }

    /// Check if a dedicated read replica is configured
    #[inline]
    pub fn has_read_replica(&self) -> bool {
        self.has_read_replica
    }

    /// Get pool statistics for monitoring
    pub fn stats(&self) -> DbPoolStats {
        DbPoolStats {
            write_size: self.write.size(),
            write_idle: self.write.num_idle(),
            read_size: self.read.size(),
            read_idle: self.read.num_idle(),
            has_read_replica: self.has_read_replica,
        }
    }
}

/// Pool statistics for monitoring
#[derive(Debug, Clone)]
pub struct DbPoolStats {
    pub write_size: u32,
    pub write_idle: usize,
    pub read_size: u32,
    pub read_idle: usize,
    pub has_read_replica: bool,
}

impl std::fmt::Display for DbPoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.has_read_replica {
            write!(
                f,
                "write(size={}, idle={}), read(size={}, idle={})",
                self.write_size, self.write_idle, self.read_size, self.read_idle
            )
        } else {
            write!(
                f,
                "primary(size={}, idle={}), no read replica",
                self.write_size, self.write_idle
            )
        }
    }
}

use crate::config::DatabaseReadReplicaConfig;

/// Default retry configuration
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_BASE_DELAY_MS: u64 = 1000;

/// Create read replica pool with retry logic
async fn create_read_replica_pool_with_retry(
    replica_url: &str,
    replica_config: &DatabaseReadReplicaConfig,
    config: &DatabaseConfig,
) -> Result<PgPool> {
    let base_delay = Duration::from_millis(DEFAULT_BASE_DELAY_MS);
    let mut last_error = None;

    for attempt in 1..=DEFAULT_MAX_RETRIES {
        match PgPoolOptions::new()
            .max_connections(replica_config.max_connections)
            .min_connections(replica_config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .connect(replica_url)
            .await
        {
            Ok(pool) => {
                if attempt > 1 {
                    tracing::info!(
                        attempt = attempt,
                        "Read replica connection succeeded after {} attempts",
                        attempt
                    );
                }
                return Ok(pool);
            }
            Err(e) => {
                let delay = base_delay * 2_u32.pow(attempt - 1);
                tracing::warn!(
                    attempt = attempt,
                    max_retries = DEFAULT_MAX_RETRIES,
                    next_retry_ms = delay.as_millis() as u64,
                    error = %e,
                    "Read replica connection failed, retrying..."
                );
                last_error = Some(e);

                if attempt < DEFAULT_MAX_RETRIES {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    tracing::error!(
        max_retries = DEFAULT_MAX_RETRIES,
        "Read replica connection failed after all retries"
    );

    Err(last_error.unwrap().into())
}

/// Create a new database connection pool with retry logic
///
/// Attempts to connect with exponential backoff: 1s, 2s, 4s, 8s, 16s
///
/// # Arguments
///
/// * `config` - Database configuration
/// * `max_retries` - Maximum number of connection attempts (default: 5)
/// * `base_delay_ms` - Base delay between retries in ms (default: 1000)
///
/// # Returns
///
/// A configured PostgreSQL connection pool
///
/// # Errors
///
/// Returns an error if all retry attempts fail
pub async fn create_pool_with_retry(
    config: &DatabaseConfig,
    max_retries: Option<u32>,
    base_delay_ms: Option<u64>,
) -> Result<DbPool> {
    let max_retries = max_retries.unwrap_or(DEFAULT_MAX_RETRIES);
    let base_delay = Duration::from_millis(base_delay_ms.unwrap_or(DEFAULT_BASE_DELAY_MS));
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match create_pool_internal(config).await {
            Ok(pool) => {
                if attempt > 1 {
                    tracing::info!(
                        attempt = attempt,
                        "Database connection succeeded after {} attempts",
                        attempt
                    );
                }
                return Ok(pool);
            }
            Err(e) => {
                let delay = base_delay * 2_u32.pow(attempt - 1);
                tracing::warn!(
                    attempt = attempt,
                    max_retries = max_retries,
                    next_retry_ms = delay.as_millis() as u64,
                    error = %e,
                    "Database connection failed, retrying..."
                );
                last_error = Some(e);

                if attempt < max_retries {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    tracing::error!(
        max_retries = max_retries,
        "Database connection failed after all retries"
    );

    Err(last_error.unwrap())
}

/// Create a new database connection pool (internal, no retry)
async fn create_pool_internal(config: &DatabaseConfig) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
        .connect(&config.connection_url())
        .await?;

    tracing::info!(
        "Database pool created: max={}, min={}, acquire_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        config.max_connections,
        config.min_connections,
        config.acquire_timeout_secs,
        config.idle_timeout_secs,
        config.max_lifetime_secs
    );

    Ok(pool)
}

/// Create a new database connection pool
///
/// This is a convenience wrapper that uses default retry settings.
/// For explicit retry control, use `create_pool_with_retry`.
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
/// Returns an error if the pool cannot be created or if all retries fail
pub async fn create_pool(config: &DatabaseConfig) -> Result<DbPool> {
    create_pool_with_retry(config, None, None).await
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
