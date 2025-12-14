//! Action Workers for api.agentauri.ai
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
use rest::ReqwestHttpClient;
use result_logger::PostgresResultLogger;
use retry::RetryPolicy;
use telegram::TeloxideTelegramClient;
use workers::{RestWorker, TelegramWorker};

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

    // Initialize Prometheus metrics exporter with default address (0.0.0.0:9090)
    metrics::init_metrics_default();

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

    // Create HTTP client for REST worker
    let http_client = Arc::new(ReqwestHttpClient::new().context("Failed to create HTTP client")?);
    tracing::info!("HTTP client initialized for REST worker");

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

    // Create Telegram worker
    let telegram_worker = TelegramWorker::new(
        telegram_client,
        logger.clone(),
        dlq.clone(),
        rate_limiter,
        RetryPolicy::default(),
    );

    // Create REST worker
    let rest_worker = RestWorker::new(http_client, logger, dlq, RetryPolicy::default());

    // Spawn worker pool
    let mut handles = Vec::new();
    metrics::set_active_workers(NUM_WORKERS);

    for worker_id in 0..NUM_WORKERS {
        let consumer = consumer.clone();
        let telegram_worker = telegram_worker.clone();
        let rest_worker = rest_worker.clone();
        let token = cancel_token.clone();

        let handle = tokio::spawn(async move {
            run_worker(worker_id, consumer, telegram_worker, rest_worker, token).await;
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
async fn run_worker<C, T, L1, D1, R, H, L2, D2>(
    worker_id: usize,
    consumer: Arc<C>,
    telegram_worker: TelegramWorker<T, L1, D1, R>,
    rest_worker: RestWorker<H, L2, D2>,
    cancel_token: CancellationToken,
) where
    C: JobConsumer,
    T: telegram::TelegramClient + 'static,
    L1: result_logger::ResultLogger + 'static,
    D1: dlq::DeadLetterQueue + 'static,
    R: rate_limiter::RateLimiter + 'static,
    H: rest::HttpClient + 'static,
    L2: result_logger::ResultLogger + 'static,
    D2: dlq::DeadLetterQueue + 'static,
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
                        // Use event_data from the job (populated by event-processor)
                        let event_data = job.event_data.clone();

                        match job.action_type {
                            ActionType::Telegram => {
                                if let Err(e) = telegram_worker.process(&job, &event_data).await {
                                    tracing::error!(
                                        worker_id = worker_id,
                                        job_id = %job.id,
                                        error = %e,
                                        "Telegram job processing failed (already moved to DLQ)"
                                    );
                                }
                            }
                            ActionType::Rest => {
                                if let Err(e) = rest_worker.process(&job, &event_data).await {
                                    tracing::error!(
                                        worker_id = worker_id,
                                        job_id = %job.id,
                                        error = %e,
                                        "REST job processing failed (already moved to DLQ)"
                                    );
                                }
                            }
                            ActionType::Mcp => {
                                tracing::debug!(
                                    worker_id = worker_id,
                                    job_id = %job.id,
                                    "Skipping MCP job (not implemented yet)"
                                );
                                // TODO: Implement MCP worker in Phase 5
                            }
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
