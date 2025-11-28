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
            // OAuth token endpoints (public - client credentials auth)
            .route("/oauth/token", web::post().to(handlers::token_endpoint))
            // Stripe webhook (no auth - uses signature verification)
            .route(
                "/billing/webhook",
                web::post().to(handlers::handle_stripe_webhook),
            )
            // Protected routes (JWT or API Key auth)
            .service(
                web::scope("")
                    .wrap(middleware::DualAuth::new(jwt_secret.clone()))
                    // Organization endpoints
                    .service(
                        web::scope("/organizations")
                            .route("", web::post().to(handlers::create_organization))
                            .route("", web::get().to(handlers::list_organizations))
                            .route("/{id}", web::get().to(handlers::get_organization))
                            .route("/{id}", web::put().to(handlers::update_organization))
                            .route("/{id}", web::delete().to(handlers::delete_organization))
                            // Ownership transfer endpoint
                            .route(
                                "/{id}/transfer",
                                web::post().to(handlers::transfer_ownership),
                            )
                            // Member endpoints
                            .route("/{id}/members", web::post().to(handlers::add_member))
                            .route("/{id}/members", web::get().to(handlers::list_members))
                            .route(
                                "/{id}/members/{user_id}",
                                web::put().to(handlers::update_member_role),
                            )
                            .route(
                                "/{id}/members/{user_id}",
                                web::delete().to(handlers::remove_member),
                            ),
                    )
                    // API Key endpoints
                    .service(
                        web::scope("/api-keys")
                            .route("", web::post().to(handlers::create_api_key))
                            .route("", web::get().to(handlers::list_api_keys))
                            .route("/{id}", web::get().to(handlers::get_api_key))
                            .route("/{id}", web::delete().to(handlers::revoke_api_key))
                            .route("/{id}/rotate", web::post().to(handlers::rotate_api_key)),
                    )
                    // OAuth client management endpoints (JWT auth required)
                    .service(
                        web::scope("/oauth/clients")
                            .route("", web::post().to(handlers::create_oauth_client))
                            .route("", web::get().to(handlers::list_oauth_clients))
                            .route("/{id}", web::delete().to(handlers::delete_oauth_client)),
                    )
                    // Agent linking endpoints (Layer 2 - wallet signature auth)
                    .service(
                        web::scope("/agents")
                            .route("/link", web::post().to(handlers::link_agent))
                            .route("/linked", web::get().to(handlers::list_linked_agents))
                            .route("/{agent_id}/link", web::delete().to(handlers::unlink_agent)),
                    )
                    // Billing endpoints
                    .service(
                        web::scope("/billing")
                            .route("/credits", web::get().to(handlers::get_credits))
                            .route(
                                "/credits/purchase",
                                web::post().to(handlers::purchase_credits),
                            )
                            .route("/transactions", web::get().to(handlers::list_transactions))
                            .route("/subscription", web::get().to(handlers::get_subscription)),
                    )
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
