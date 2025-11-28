//! API Gateway for api.8004.dev
//!
//! REST API server providing trigger management and system queries.

use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Context;
use shared::{db, Config};

mod background_tasks;
mod handlers;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod validators;

use background_tasks::BackgroundTaskRunner;
use middleware::security_headers::SecurityHeaders;
use services::WalletService;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    shared::init_tracing();

    tracing::info!("Starting API Gateway...");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;

    // Create database connection pool
    let db_pool = db::create_pool(&config.database)
        .await
        .context("Failed to create database pool")?;

    // Run database migrations
    db::run_migrations(&db_pool)
        .await
        .context("Failed to run database migrations")?;

    // Check database health
    db::check_health(&db_pool)
        .await
        .context("Database health check failed")?;

    // Initialize WalletService with chain configs from environment (loaded once at startup)
    // This creates a shared HTTP client with connection pooling for RPC calls
    let chain_configs = WalletService::load_chain_configs_from_env();
    let wallet_service = WalletService::new(chain_configs.clone());
    tracing::info!(
        "WalletService initialized with {} chain configurations",
        chain_configs.len()
    );

    // Start background tasks (nonce cleanup)
    let bg_runner = BackgroundTaskRunner::new(db_pool.clone());
    let shutdown_token = bg_runner.start();
    tracing::info!("Background tasks started (nonce cleanup every hour)");

    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("API Gateway listening on {}", server_addr);

    // Start HTTP server
    let server = HttpServer::new(move || {
        App::new()
            // Add security headers middleware
            .wrap(SecurityHeaders::for_api())
            // Add logger middleware
            .wrap(Logger::default())
            // Add CORS middleware
            .wrap(middleware::cors())
            // Configure JSON payload size limit (1MB)
            .app_data(web::JsonConfig::default().limit(1_048_576))
            // Store database pool in app state
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Store WalletService in app state (shared across all requests)
            .app_data(web::Data::new(wallet_service.clone()))
            // Configure routes
            .configure(routes::configure)
    })
    .bind(&server_addr)
    .with_context(|| format!("Failed to bind to {}", server_addr))?;

    let server_handle = server.run();

    // Register graceful shutdown handler
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Shutdown signal received, stopping background tasks...");
                shutdown_token.cancel();
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to listen for shutdown signal");
            }
        }
    });

    // Run server and wait for completion
    server_handle.await.context("Server error")?;

    tracing::info!("API Gateway shutdown complete");

    Ok(())
}
