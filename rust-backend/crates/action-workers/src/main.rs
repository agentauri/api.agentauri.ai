//! Action Workers for api.8004.dev
//!
//! Consumes jobs from Redis queue and executes actions (Telegram, REST, MCP).
//! Implements a worker pool with configurable parallelism and graceful shutdown.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use shared::{db, ActionType, Config};
use tokio_util::sync::CancellationToken;

mod consumer;
mod dlq;
mod error;
mod metrics;
mod rate_limiter;
mod rest;
mod result_logger;
mod retry;
mod telegram;
mod template;
mod workers;

use consumer::{JobConsumer, RedisJobConsumer};
use dlq::RedisDlq;
use rate_limiter::TelegramRateLimiter;
use result_logger::PostgresResultLogger;
use retry::RetryPolicy;
use telegram::TeloxideTelegramClient;
use workers::TelegramWorker;

/// Number of concurrent workers
const NUM_WORKERS: usize = 5;

/// Timeout for BRPOP in seconds
const CONSUME_TIMEOUT_SECS: u64 = 5;

/// Interval for queue depth metric updates
const METRICS_UPDATE_INTERVAL_SECS: u64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    shared::init_tracing();

    tracing::info!("Starting Action Workers...");

    // Register Prometheus metrics
    metrics::register_metrics();

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

    let redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .context("Failed to connect to Redis")?;

    tracing::info!("Connected to Redis");

    // Create shared components
    let consumer = Arc::new(RedisJobConsumer::new(redis_conn.clone()));
    let dlq = Arc::new(RedisDlq::new(redis_conn.clone()));
    let logger = Arc::new(PostgresResultLogger::new(db_pool));
    let rate_limiter = Arc::new(TelegramRateLimiter::new());

    // Create Telegram client (from environment variable)
    let telegram_client = match TeloxideTelegramClient::from_env() {
        Ok(client) => Arc::new(client),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Telegram bot token not configured, Telegram worker will be disabled"
            );
            // For now, we'll still start but Telegram jobs will fail
            // In production, you might want to handle this differently
            return Err(e.into());
        }
    };

    // Cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

    // Spawn shutdown handler
    let shutdown_token = cancel_token.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Shutdown signal received, stopping workers...");
                shutdown_token.cancel();
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to listen for shutdown signal");
            }
        }
    });

    // Create Telegram worker template
    let telegram_worker = TelegramWorker::new(
        telegram_client,
        logger,
        dlq,
        rate_limiter,
        RetryPolicy::default(),
    );

    // Spawn worker pool
    let mut handles = Vec::new();
    metrics::set_active_workers(NUM_WORKERS);

    for worker_id in 0..NUM_WORKERS {
        let consumer = consumer.clone();
        let worker = telegram_worker.clone();
        let token = cancel_token.clone();

        let handle = tokio::spawn(async move {
            run_worker(worker_id, consumer, worker, token).await;
        });
        handles.push(handle);
    }

    tracing::info!(
        num_workers = NUM_WORKERS,
        "Worker pool started, ready to process jobs"
    );

    // Spawn metrics updater (queue depth)
    let metrics_consumer = consumer.clone();
    let metrics_token = cancel_token.clone();
    tokio::spawn(async move {
        update_metrics_loop(metrics_consumer, metrics_token).await;
    });

    // Wait for all workers to finish
    for handle in handles {
        let _ = handle.await;
    }

    metrics::set_active_workers(0);
    tracing::info!("All workers stopped, exiting");

    Ok(())
}

/// Run a single worker that consumes jobs from the queue
async fn run_worker<C, T, L, D, R>(
    worker_id: usize,
    consumer: Arc<C>,
    worker: TelegramWorker<T, L, D, R>,
    cancel_token: CancellationToken,
) where
    C: JobConsumer,
    T: telegram::TelegramClient + 'static,
    L: result_logger::ResultLogger + 'static,
    D: dlq::DeadLetterQueue + 'static,
    R: rate_limiter::RateLimiter + 'static,
{
    tracing::info!(worker_id = worker_id, "Worker started");

    loop {
        tokio::select! {
            // Check for cancellation
            _ = cancel_token.cancelled() => {
                tracing::info!(worker_id = worker_id, "Worker stopping due to shutdown");
                break;
            }

            // Try to consume a job
            result = consumer.consume(CONSUME_TIMEOUT_SECS) => {
                match result {
                    Ok(Some(job)) => {
                        // Check if this is a Telegram job
                        if job.action_type == ActionType::Telegram {
                            // TODO: In Phase 4, fetch actual event data from database
                            // For now, use the job config as event data
                            let event_data = job.config.clone();

                            if let Err(e) = worker.process(&job, &event_data).await {
                                tracing::error!(
                                    worker_id = worker_id,
                                    job_id = %job.id,
                                    error = %e,
                                    "Job processing failed (already moved to DLQ)"
                                );
                            }
                        } else {
                            tracing::debug!(
                                worker_id = worker_id,
                                action_type = %job.action_type,
                                job_id = %job.id,
                                "Skipping non-Telegram job (not implemented yet)"
                            );
                            // TODO: Route to appropriate worker based on action_type
                            // For now, these jobs will be ignored
                        }
                    }
                    Ok(None) => {
                        // Timeout - no job available, continue polling
                        tracing::trace!(worker_id = worker_id, "No job available, continuing...");
                    }
                    Err(e) => {
                        tracing::error!(
                            worker_id = worker_id,
                            error = %e,
                            "Error consuming job from queue"
                        );
                        // Brief pause before retrying
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    tracing::info!(worker_id = worker_id, "Worker stopped");
}

/// Periodically update queue depth metric
async fn update_metrics_loop<C: JobConsumer>(consumer: Arc<C>, cancel_token: CancellationToken) {
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                tracing::debug!("Metrics updater stopping");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(METRICS_UPDATE_INTERVAL_SECS)) => {
                match consumer.queue_len().await {
                    Ok(len) => {
                        metrics::set_queue_depth(len);
                        tracing::trace!(queue_depth = len, "Updated queue depth metric");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to get queue length for metrics");
                    }
                }
            }
        }
    }
}
