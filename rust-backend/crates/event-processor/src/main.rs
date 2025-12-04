//! Event Processor for api.agentauri.ai
//!
//! Listens to PostgreSQL NOTIFY events and evaluates triggers against incoming events.
//! Uses dual-path architecture for zero event loss:
//! 1. PRIMARY: PostgreSQL NOTIFY → PgListener → process_event (99% of events)
//! 2. FALLBACK: Polling → discover unprocessed → process_event (1% of events)

use anyhow::{Context, Result};
use event_processor::{PollingFallback, TriggerStateManager};
use shared::{db, Config};
use std::sync::Arc;
use tokio::signal;

// These modules are only used by listener which is specific to the binary
mod listener;

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

    // Create shared state manager for stateful triggers
    let state_manager = Arc::new(TriggerStateManager::new(db_pool.clone()));

    // Create job queue for action enqueueing (use lib version to match PollingFallback)
    let job_queue = event_processor::queue::RedisJobQueue::new(redis_conn.clone());

    // Start polling fallback (runs every 60 seconds)
    let polling_fallback = Arc::new(PollingFallback::new(
        db_pool.clone(),
        job_queue.clone(),
        state_manager.clone(),
    ));

    let polling_handle = tokio::spawn({
        let fallback = polling_fallback.clone();
        async move { fallback.start().await }
    });

    tracing::info!("Started polling fallback (60s interval)");

    // FIX 4.2: Start automatic state cleanup (Production Readiness)
    // Cleans up trigger state older than 30 days every 24 hours
    let cleanup_handle = tokio::spawn({
        let db_pool = db_pool.clone();
        async move {
            const CLEANUP_INTERVAL_SECS: u64 = 86400; // 24 hours
            const RETENTION_DAYS: i32 = 30;

            tracing::info!(
                "Starting automatic state cleanup (interval: {}h, retention: {}d)",
                CLEANUP_INTERVAL_SECS / 3600,
                RETENTION_DAYS
            );

            loop {
                // Wait 24 hours before first cleanup
                tokio::time::sleep(tokio::time::Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;

                // Run cleanup
                let state_manager = TriggerStateManager::new(db_pool.clone());
                match state_manager.cleanup_expired(RETENTION_DAYS).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::info!(
                                deleted = deleted,
                                retention_days = RETENTION_DAYS,
                                "State cleanup completed successfully"
                            );
                        } else {
                            tracing::debug!("State cleanup: no expired records");
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            error_id = "STATE_CLEANUP_FAILED",
                            "State cleanup failed, will retry in 24 hours"
                        );
                        // Don't exit - continue with next iteration
                    }
                }
            }
        }
    });

    tracing::info!("Started automatic state cleanup (24h interval, 30d retention)");

    // Start listening to PostgreSQL NOTIFY (primary path)
    let listener_handle = tokio::spawn({
        let db_pool = db_pool.clone();
        let redis_conn = redis_conn.clone();
        async move { listener::start_listening(db_pool, redis_conn).await }
    });

    tracing::info!("Started PostgreSQL NOTIFY listener (primary path)");

    // Wait for shutdown signal, listener failure, polling failure, OR cleanup failure
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
        result = polling_handle => {
            match result {
                Ok(Ok(())) => {
                    tracing::warn!("Polling fallback exited cleanly (unexpected)");
                }
                Ok(Err(e)) => {
                    tracing::error!("Polling fallback failed: {:#}", e);
                    return Err(e.context("Polling fallback failed"));
                }
                Err(e) => {
                    tracing::error!("Polling fallback task panicked: {}", e);
                    anyhow::bail!("Polling fallback task panicked: {}", e);
                }
            }
        }
        result = cleanup_handle => {
            match result {
                Ok(()) => {
                    tracing::warn!("State cleanup task exited (unexpected)");
                }
                Err(e) => {
                    tracing::error!("State cleanup task panicked: {}", e);
                    anyhow::bail!("State cleanup task panicked: {}", e);
                }
            }
        }
    }

    Ok(())
}
