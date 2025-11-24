//! Middleware for the API Gateway

use actix_cors::Cors;
use actix_web::http;
use std::env;

/// Configure CORS middleware
pub fn cors() -> Cors {
    // Get allowed origins from environment variable
    // Format: comma-separated list of origins
    // Example: ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com
    let allowed_origins = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| String::new());

    let origins: Vec<String> = allowed_origins
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    Cors::default()
        .allowed_origin_fn(move |origin, _req_head| {
            let origin_str = origin.to_str().unwrap_or("");

            if cfg!(debug_assertions) {
                // Development mode: Allow localhost
                origin_str.starts_with("http://localhost")
                    || origin_str.starts_with("http://127.0.0.1")
            } else {
                // Production mode: Whitelist only
                if origins.is_empty() {
                    tracing::warn!(
                        "ALLOWED_ORIGINS not set. Denying all CORS requests in production."
                    );
                    false
                } else {
                    origins.iter().any(|allowed| origin_str == allowed)
                }
            }
        })
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ])
        .max_age(3600)
}

// JWT authentication middleware will be added here in the future
