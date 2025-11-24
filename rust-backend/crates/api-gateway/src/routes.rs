//! Route configuration for the API

use actix_web::web;

use crate::{handlers, middleware};

/// Configure all routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Get JWT secret from config (will be passed from app_data)
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "dev_secret_change_in_production".to_string());

    cfg.service(
        web::scope("/api/v1")
            // Health check endpoint (no auth required)
            .route("/health", web::get().to(handlers::health_check))
            // Authentication endpoints (no auth required)
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(handlers::register))
                    .route("/login", web::post().to(handlers::login)),
            )
            // Protected routes (JWT auth required)
            .service(
                web::scope("")
                    .wrap(middleware::JwtAuth::new(jwt_secret))
                    // Trigger endpoints
                    .service(
                        web::scope("/triggers")
                            .route("", web::post().to(handlers::create_trigger))
                            .route("", web::get().to(handlers::list_triggers))
                            .route("/{id}", web::get().to(handlers::get_trigger))
                            .route("/{id}", web::put().to(handlers::update_trigger))
                            .route("/{id}", web::delete().to(handlers::delete_trigger))
                            // Nested condition endpoints
                            .route(
                                "/{trigger_id}/conditions",
                                web::post().to(handlers::create_condition),
                            )
                            .route(
                                "/{trigger_id}/conditions",
                                web::get().to(handlers::list_conditions),
                            )
                            .route(
                                "/{trigger_id}/conditions/{id}",
                                web::put().to(handlers::update_condition),
                            )
                            .route(
                                "/{trigger_id}/conditions/{id}",
                                web::delete().to(handlers::delete_condition),
                            )
                            // Nested action endpoints
                            .route(
                                "/{trigger_id}/actions",
                                web::post().to(handlers::create_action),
                            )
                            .route(
                                "/{trigger_id}/actions",
                                web::get().to(handlers::list_actions),
                            )
                            .route(
                                "/{trigger_id}/actions/{id}",
                                web::put().to(handlers::update_action),
                            )
                            .route(
                                "/{trigger_id}/actions/{id}",
                                web::delete().to(handlers::delete_action),
                            ),
                    ),
            ),
    );
}
