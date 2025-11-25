//! Event Processor for api.8004.dev
//!
//! Listens to PostgreSQL NOTIFY events and evaluates triggers against incoming events.

use anyhow::{Context, Result};
use shared::{db, Config};
use tokio::signal;

mod listener;
mod queue;
mod trigger_engine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    shared::init_tracing();

    tracing::info!("Starting Event Processor...");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Create database connection pool
    let db_pool = db::create_pool(&config.database)
        .await
        .context("Failed to create database pool")?;

    // Check database health
    db::check_health(&db_pool)
        .await
        .context("Database health check failed")?;

    // Create Redis connection
    let redis_client = redis::Client::open(config.redis.connection_url())
        .context("Failed to create Redis client")?;

    let redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .context("Failed to connect to Redis")?;

    tracing::info!("Connected to Redis");

    // Start listening to PostgreSQL NOTIFY
    let listener_handle = tokio::spawn({
        let db_pool = db_pool.clone();
        let redis_conn = redis_conn.clone();
        async move { listener::start_listening(db_pool, redis_conn).await }
    });

    // Wait for either shutdown signal OR listener failure
    tokio::select! {
        result = signal::ctrl_c() => {
            result.context("Failed to listen for shutdown signal")?;
            tracing::info!("Shutdown signal received, stopping Event Processor...");
        }
        result = listener_handle => {
            match result {
                Ok(Ok(())) => {
                    tracing::warn!("Listener exited cleanly (unexpected)");
                }
                Ok(Err(e)) => {
                    tracing::error!("Listener failed: {:#}", e);
                    return Err(e.context("Event listener failed"));
                }
                Err(e) => {
                    tracing::error!("Listener task panicked: {}", e);
                    anyhow::bail!("Event listener task panicked: {}", e);
                }
            }
        }
    }

    Ok(())
}
