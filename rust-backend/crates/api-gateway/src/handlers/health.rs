//! Health check and service endpoints

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use shared::DbPool;
use utoipa::{OpenApi, ToSchema};

use crate::openapi::ApiDoc;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub version: String,
}

/// Health check endpoint
///
/// Returns the health status of the API Gateway and its dependencies.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse)
    )
)]
pub async fn health_check(pool: web::Data<DbPool>) -> impl Responder {
    // Check database connection
    let db_status = match shared::db::check_health(&pool).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let response = HealthResponse {
        status: if db_status == "connected" {
            "healthy"
        } else {
            "unhealthy"
        }
        .to_string(),
        database: db_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    if db_status == "connected" {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

/// OpenAPI JSON endpoint
///
/// Returns the OpenAPI 3.0 specification for the API.
/// This endpoint is public and does not require authentication.
#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    tag = "Discovery",
    responses(
        (status = 200, description = "OpenAPI specification", content_type = "application/json")
    )
)]
pub async fn openapi_json() -> impl Responder {
    HttpResponse::Ok().content_type("application/json").body(
        ApiDoc::openapi()
            .to_json()
            .unwrap_or_else(|_| "{}".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            database: "connected".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"status":"error","message":"Failed to serialize response"}"#.to_string()
        });
        assert!(json.contains("healthy"));
        assert!(json.contains("connected"));
    }
}
