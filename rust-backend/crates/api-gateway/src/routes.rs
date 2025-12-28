//! Route configuration for the API

use actix_web::web;

use crate::{handlers, middleware};

/// Configure all routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Get JWT secret from config (will be passed from app_data)
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "dev_secret_change_in_production".to_string());

    // Discovery endpoints (public, outside /api/v1 scope)
    cfg.service(
        web::resource("/.well-known/agent.json").route(web::get().to(handlers::get_agent_card)),
    );
    cfg.service(
        web::resource("/.well-known/security.txt").route(web::get().to(handlers::get_security_txt)),
    );

    cfg.service(
        web::scope("/api/v1")
            // Health check endpoint (no auth required)
            .route("/health", web::get().to(handlers::health_check))
            // OpenAPI JSON endpoint (no auth required - used by Swagger UI)
            .route("/openapi.json", web::get().to(handlers::openapi_json))
            // Ponder indexer status endpoints (no auth required - for monitoring)
            .service(
                web::scope("/ponder")
                    .route("/status", web::get().to(handlers::get_ponder_status))
                    .route("/events", web::get().to(handlers::get_ponder_events)),
            )
            // Authentication endpoints (no auth required)
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(handlers::register))
                    .route("/login", web::post().to(handlers::login))
                    // SIWE wallet login
                    .route("/wallet", web::post().to(handlers::wallet_login))
                    // Session management
                    .route("/nonce", web::post().to(handlers::generate_nonce))
                    .route("/me", web::get().to(handlers::get_me))
                    .route("/logout", web::post().to(handlers::logout))
                    .route("/refresh", web::post().to(handlers::refresh_token))
                    .route("/exchange", web::post().to(handlers::exchange_code))
                    // Social login endpoints (OAuth 2.0)
                    .route("/google", web::get().to(handlers::google_auth))
                    .route("/google/callback", web::get().to(handlers::google_callback))
                    .route("/github", web::get().to(handlers::github_auth))
                    .route("/github/callback", web::get().to(handlers::github_callback))
                    // Account linking endpoints (requires auth)
                    .route("/link/google", web::get().to(handlers::link_google))
                    .route("/link/github", web::get().to(handlers::link_github)),
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
                            )
                            // API Keys nested under organization
                            .route("/{id}/api-keys", web::get().to(handlers::list_org_api_keys))
                            .route(
                                "/{id}/api-keys",
                                web::post().to(handlers::create_org_api_key),
                            ),
                    )
                    // API Key endpoints (standalone - for backwards compat)
                    .service(
                        web::scope("/api-keys")
                            .route("", web::post().to(handlers::create_api_key))
                            .route("", web::get().to(handlers::list_api_keys))
                            .route("/{id}", web::get().to(handlers::get_api_key))
                            .route("/{id}", web::patch().to(handlers::update_api_key))
                            .route("/{id}", web::delete().to(handlers::revoke_api_key))
                            .route("/{id}/rotate", web::post().to(handlers::rotate_api_key))
                            .route("/{id}/regenerate", web::post().to(handlers::rotate_api_key)),
                    )
                    // OAuth client management endpoints (JWT auth required)
                    .service(
                        web::scope("/oauth/clients")
                            .route("", web::post().to(handlers::create_oauth_client))
                            .route("", web::get().to(handlers::list_oauth_clients))
                            .route("/{id}", web::delete().to(handlers::delete_oauth_client)),
                    )
                    // Agent endpoints
                    .service(
                        web::scope("/agents")
                            // Agent linking (Layer 2 - wallet signature auth)
                            .route("/link", web::post().to(handlers::link_agent))
                            .route("/linked", web::get().to(handlers::list_linked_agents))
                            .route("/{agent_id}/link", web::delete().to(handlers::unlink_agent))
                            // Agent following (simplified multi-registry monitoring)
                            .route("/following", web::get().to(handlers::list_following))
                            .route("/{agent_id}/follow", web::post().to(handlers::follow_agent))
                            .route("/{agent_id}/follow", web::put().to(handlers::update_follow))
                            .route(
                                "/{agent_id}/follow",
                                web::delete().to(handlers::unfollow_agent),
                            ),
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
                    // A2A Protocol endpoints (JSON-RPC 2.0)
                    .service(
                        web::scope("/a2a")
                            .route("/rpc", web::post().to(handlers::a2a_rpc))
                            .route("/tasks/{id}", web::get().to(handlers::get_task_status))
                            .route(
                                "/tasks/{id}/stream",
                                web::get().to(handlers::stream_task_progress),
                            ),
                    )
                    // Trigger endpoints
                    .service(
                        web::scope("/triggers")
                            .route("", web::post().to(handlers::create_trigger))
                            .route("", web::get().to(handlers::list_triggers))
                            .route("/{id}", web::get().to(handlers::get_trigger))
                            .route("/{id}", web::put().to(handlers::update_trigger))
                            .route("/{id}", web::delete().to(handlers::delete_trigger))
                            // Circuit breaker management endpoints
                            .route(
                                "/{id}/circuit-breaker",
                                web::get().to(handlers::get_circuit_breaker_state),
                            )
                            .route(
                                "/{id}/circuit-breaker",
                                web::patch().to(handlers::update_circuit_breaker_config),
                            )
                            .route(
                                "/{id}/circuit-breaker/reset",
                                web::post().to(handlers::reset_circuit_breaker),
                            )
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
