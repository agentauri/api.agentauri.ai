//! Action Workers for api.8004.dev
//!
//! Consumes jobs from Redis queue and executes actions (Telegram, REST, MCP).

use anyhow::{Context, Result};
use shared::{db, Config};
use tokio::signal;

mod rest;
mod telegram;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    shared::init_tracing();

    tracing::info!("Starting Action Workers...");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Create database connection pool (for result logging)
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

    let _redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .context("Failed to connect to Redis")?;

    tracing::info!("Connected to Redis");

    // TODO: Start worker pools for different action types
    // TODO: Implement job consumption from Redis queues

    tracing::info!("Action Workers ready (placeholder mode)");

    // Wait for shutdown signal
    signal::ctrl_c()
        .await
        .context("Failed to listen for shutdown signal")?;

    tracing::info!("Shutdown signal received, stopping Action Workers...");

    Ok(())
}
