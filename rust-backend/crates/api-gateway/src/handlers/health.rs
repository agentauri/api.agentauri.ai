//! Health check endpoint

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use shared::DbPool;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub version: String,
}

/// Health check endpoint
///
/// Returns the health status of the API Gateway and its dependencies.
///
/// # Endpoint
///
/// `GET /api/v1/health`
///
/// # Response
///
/// ```json
/// {
///   "status": "healthy",
///   "database": "connected",
///   "version": "0.1.0"
/// }
/// ```
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

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("connected"));
    }
}
