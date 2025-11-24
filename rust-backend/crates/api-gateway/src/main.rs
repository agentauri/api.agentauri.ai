//! API Gateway for api.8004.dev
//!
//! REST API server providing trigger management and system queries.

use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Context;
use shared::{db, Config};

mod handlers;
mod middleware;
mod models;
mod repositories;
mod routes;

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

    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("API Gateway listening on {}", server_addr);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // Add logger middleware
            .wrap(Logger::default())
            // Add CORS middleware
            .wrap(middleware::cors())
            // Store database pool in app state
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Configure routes
            .configure(routes::configure)
    })
    .bind(&server_addr)
    .with_context(|| format!("Failed to bind to {}", server_addr))?
    .run()
    .await
    .context("Server error")?;

    Ok(())
}
