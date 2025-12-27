//! API Gateway for api.agentauri.ai
//!
//! REST API server providing trigger management and system queries.

use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Context;
use shared::{db, secrets, Config, RateLimiter};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use api_gateway::background_tasks::BackgroundTaskRunner;
use api_gateway::middleware::auth_extractor::AuthExtractor;
use api_gateway::middleware::metrics::{init_metrics, metrics_handler, PrometheusMetrics};
use api_gateway::middleware::query_tier::QueryTierExtractor;
use api_gateway::middleware::request_id::RequestId;
use api_gateway::middleware::security_headers::SecurityHeaders;
use api_gateway::middleware::unified_rate_limiter::UnifiedRateLimiter;
use api_gateway::openapi::ApiDoc;
use api_gateway::services::{
    start_a2a_task_processor, AuthRateLimiter, SocialAuthService, WalletService,
};
use api_gateway::{middleware, routes};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    shared::init_tracing();

    tracing::info!("Starting API Gateway...");

    // Log secrets backend configuration
    // In production, set SECRETS_BACKEND=aws or SECRETS_BACKEND=vault
    let secrets_backend = secrets::SecretsBackend::from_env();
    tracing::info!(
        backend = ?secrets_backend,
        "Secrets backend configured (set SECRETS_BACKEND=aws or vault for production)"
    );

    // Load secrets from configured backend (AWS/Vault in prod, .env in dev)
    // This validates that secrets loading works and logs any issues early
    match secrets::load_secrets().await {
        Ok(app_secrets) => {
            tracing::info!(
                redacted = ?app_secrets.redacted(),
                "Application secrets loaded successfully"
            );
        }
        Err(e) => {
            // In development, this is a warning (secrets may not all be configured)
            // In production with AWS/Vault backend, this should be fatal
            if matches!(secrets_backend, secrets::SecretsBackend::Env) {
                tracing::warn!(
                    error = %e,
                    "Failed to load all secrets from .env (some features may be disabled)"
                );
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to load secrets from {:?}: {}",
                    secrets_backend,
                    e
                ));
            }
        }
    }

    // Load configuration from environment
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

    // Initialize SocialAuthService for OAuth login (Google, GitHub)
    let social_auth_service = SocialAuthService::from_env();
    tracing::info!(
        "SocialAuthService initialized (Google: {}, GitHub: {}, frontend_url: {})",
        social_auth_service.is_google_configured(),
        social_auth_service.is_github_configured(),
        social_auth_service.frontend_url()
    );

    // Initialize AuthRateLimiter for code exchange (stricter: 10 per minute per IP)
    // This prevents brute-force attacks on OAuth authorization codes
    let code_exchange_rate_limiter = AuthRateLimiter::with_rates(500, 10);
    tracing::info!("Code exchange rate limiter initialized (10 req/min per IP)");

    // Initialize Redis client for rate limiting
    let redis_client = shared::redis::create_client(&config.redis.connection_url())
        .await
        .context("Failed to create Redis client")?;
    tracing::info!("Redis client connected for rate limiting");

    // Create RateLimiter instance (shared across all requests)
    let rate_limiter = RateLimiter::new(redis_client)
        .await
        .context("Failed to create rate limiter")?;
    tracing::info!(
        "Rate limiter initialized (mode: {})",
        std::env::var("RATE_LIMIT_MODE").unwrap_or_else(|_| "shadow".to_string())
    );

    // Initialize Prometheus metrics recorder (must be done before any metrics are recorded)
    let _prometheus_handle = init_metrics();
    tracing::info!("Prometheus metrics initialized (endpoint: /metrics)");

    // Start background tasks (nonce cleanup, payment nonce cleanup, auth failures cleanup)
    let bg_runner = BackgroundTaskRunner::new(db_pool.clone());
    let shutdown_token = bg_runner.start();
    tracing::info!(
        "Background tasks started (nonces, OAuth tokens, payment nonces, auth failures cleanup)"
    );

    // Start A2A task processor (processes submitted A2A Protocol tasks)
    let a2a_shutdown_token = start_a2a_task_processor(db_pool.clone());
    tracing::info!("A2A Task Processor started");

    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("API Gateway listening on {}", server_addr);

    // Start HTTP server
    let server = HttpServer::new(move || {
        App::new()
            // Add Prometheus metrics middleware (should be early to capture all requests)
            .wrap(PrometheusMetrics::new())
            // Add request ID middleware (must be first for tracing)
            .wrap(RequestId::new())
            // Add security headers middleware
            .wrap(SecurityHeaders::for_api())
            // Add logger middleware
            .wrap(Logger::default())
            // Add CORS middleware
            .wrap(middleware::cors())
            // Add rate limiting middleware chain (order matters!)
            // 1. UnifiedRateLimiter: Checks rate limits using AuthContext + QueryTier
            .wrap(UnifiedRateLimiter::new(rate_limiter.clone()))
            // 2. QueryTierExtractor: Extracts query tier from path/query params
            .wrap(QueryTierExtractor::new())
            // 3. AuthExtractor: Extracts auth context (IP, API key, or wallet signature)
            .wrap(AuthExtractor::new())
            // Configure JSON payload size limit (1MB)
            .app_data(web::JsonConfig::default().limit(1_048_576))
            // Store database pool in app state
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            // Store WalletService in app state (shared across all requests)
            .app_data(web::Data::new(wallet_service.clone()))
            // Store SocialAuthService in app state (shared across all requests)
            .app_data(web::Data::new(social_auth_service.clone()))
            // Store CodeExchangeRateLimiter in app state (for /auth/exchange endpoint)
            .app_data(web::Data::new(code_exchange_rate_limiter.clone()))
            // Prometheus metrics endpoint (for scraping)
            .route("/metrics", web::get().to(metrics_handler))
            // Configure routes
            .configure(routes::configure)
            // OpenAPI documentation endpoints
            .service(
                SwaggerUi::new("/api-docs/{_:.*}").url("/api/v1/openapi.json", ApiDoc::openapi()),
            )
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
                a2a_shutdown_token.cancel();
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
