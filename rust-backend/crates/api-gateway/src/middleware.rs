//! Middleware for the API Gateway

use actix_cors::Cors;
use actix_web::http;

/// Configure CORS middleware
pub fn cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|origin, _req_head| {
            // In development, allow all origins
            // In production, restrict to specific domains
            origin.as_bytes().starts_with(b"http://localhost")
                || origin.as_bytes().starts_with(b"http://127.0.0.1")
                || origin.as_bytes().starts_with(b"https://")
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
