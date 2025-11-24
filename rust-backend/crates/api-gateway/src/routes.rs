//! Route configuration for the API

use actix_web::web;

use crate::handlers;

/// Configure all routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // Health check endpoint
            .route("/health", web::get().to(handlers::health::health_check)), // Future routes will be added here:
                                                                              // .service(auth_routes())
                                                                              // .service(trigger_routes())
                                                                              // .service(event_routes())
    );
}
