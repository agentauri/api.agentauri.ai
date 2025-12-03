//! Circuit Breaker Management Handlers
//!
//! Handlers for viewing and managing circuit breaker state and configuration.

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use shared::DbPool;

use crate::{
    handlers::helpers::{
        extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
    middleware::{get_verified_organization_id, get_verified_organization_id_with_role},
    models::{
        can_write, CircuitBreakerConfigResponse, CircuitBreakerStateResponse, ErrorResponse,
        SuccessResponse, UpdateCircuitBreakerConfigRequest,
    },
    repositories::TriggerRepository,
};

/// Get circuit breaker state for a trigger
///
/// GET /api/v1/triggers/{id}/circuit-breaker
/// Requires X-Organization-ID header
#[utoipa::path(
    get,
    path = "/api/v1/triggers/{id}/circuit-breaker",
    tag = "Circuit Breaker",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Circuit breaker state", body = SuccessResponse<CircuitBreakerStateResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn get_circuit_breaker_state(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get and verify organization_id from header (any role can view)
    let organization_id = match get_verified_organization_id(&req_http, &pool, &user_id).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    // Check if trigger belongs to the organization
    let belongs = match handle_db_error(
        TriggerRepository::belongs_to_organization(&pool, &trigger_id, &organization_id).await,
        "check trigger organization",
    ) {
        Ok(belongs) => belongs,
        Err(resp) => return resp,
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Get circuit breaker info
    let info = match handle_db_error(
        TriggerRepository::get_circuit_breaker_info(&pool, &trigger_id).await,
        "fetch circuit breaker info",
    ) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Trigger not found"));
        }
        Err(resp) => return resp,
    };

    // Parse config (use default if not set)
    let config = info
        .circuit_breaker_config
        .and_then(|v| serde_json::from_value::<CircuitBreakerConfigParsed>(v).ok())
        .unwrap_or_default();

    // Parse state (use default if not set)
    let state = info
        .circuit_breaker_state
        .and_then(|v| serde_json::from_value::<CircuitBreakerStateParsed>(v).ok())
        .unwrap_or_default();

    let response = CircuitBreakerStateResponse {
        trigger_id: info.id,
        trigger_name: info.name,
        state: state.state,
        failure_count: state.failure_count,
        last_failure_time: state.last_failure_time,
        opened_at: state.opened_at,
        half_open_calls: state.half_open_calls,
        config: CircuitBreakerConfigResponse {
            failure_threshold: config.failure_threshold,
            recovery_timeout_seconds: config.recovery_timeout_seconds,
            half_open_max_calls: config.half_open_max_calls,
        },
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update circuit breaker configuration
///
/// PATCH /api/v1/triggers/{id}/circuit-breaker
/// Requires X-Organization-ID header and write access
#[utoipa::path(
    patch,
    path = "/api/v1/triggers/{id}/circuit-breaker",
    tag = "Circuit Breaker",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    request_body = UpdateCircuitBreakerConfigRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Circuit breaker config updated", body = SuccessResponse<CircuitBreakerStateResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn update_circuit_breaker_config(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateCircuitBreakerConfigRequest>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get and verify organization_id from header (also gets role)
    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    // Check user has write access
    if !can_write(&role) {
        return forbidden("Insufficient permissions to update circuit breaker config");
    }

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Check if trigger belongs to the organization
    let belongs = match handle_db_error(
        TriggerRepository::belongs_to_organization(&pool, &trigger_id, &organization_id).await,
        "check trigger organization",
    ) {
        Ok(belongs) => belongs,
        Err(resp) => return resp,
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Update configuration
    if let Err(resp) = handle_db_error(
        TriggerRepository::update_circuit_breaker_config(
            &pool,
            &trigger_id,
            req.failure_threshold,
            req.recovery_timeout_seconds,
            req.half_open_max_calls,
        )
        .await,
        "update circuit breaker config",
    ) {
        return resp;
    }

    // Return updated state
    let info = match handle_db_error(
        TriggerRepository::get_circuit_breaker_info(&pool, &trigger_id).await,
        "fetch circuit breaker info",
    ) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Trigger not found"));
        }
        Err(resp) => return resp,
    };

    // Parse config (use default if not set)
    let config = info
        .circuit_breaker_config
        .and_then(|v| serde_json::from_value::<CircuitBreakerConfigParsed>(v).ok())
        .unwrap_or_default();

    // Parse state (use default if not set)
    let state = info
        .circuit_breaker_state
        .and_then(|v| serde_json::from_value::<CircuitBreakerStateParsed>(v).ok())
        .unwrap_or_default();

    let response = CircuitBreakerStateResponse {
        trigger_id: info.id,
        trigger_name: info.name,
        state: state.state,
        failure_count: state.failure_count,
        last_failure_time: state.last_failure_time,
        opened_at: state.opened_at,
        half_open_calls: state.half_open_calls,
        config: CircuitBreakerConfigResponse {
            failure_threshold: config.failure_threshold,
            recovery_timeout_seconds: config.recovery_timeout_seconds,
            half_open_max_calls: config.half_open_max_calls,
        },
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Reset circuit breaker to Closed state
///
/// POST /api/v1/triggers/{id}/circuit-breaker/reset
/// Requires X-Organization-ID header and write access
#[utoipa::path(
    post,
    path = "/api/v1/triggers/{id}/circuit-breaker/reset",
    tag = "Circuit Breaker",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Circuit breaker reset to Closed state", body = SuccessResponse<CircuitBreakerStateResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn reset_circuit_breaker(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get and verify organization_id from header (also gets role)
    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    // Check user has write access
    if !can_write(&role) {
        return forbidden("Insufficient permissions to reset circuit breaker");
    }

    // Check if trigger belongs to the organization
    let belongs = match handle_db_error(
        TriggerRepository::belongs_to_organization(&pool, &trigger_id, &organization_id).await,
        "check trigger organization",
    ) {
        Ok(belongs) => belongs,
        Err(resp) => return resp,
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Reset state
    if let Err(resp) = handle_db_error(
        TriggerRepository::reset_circuit_breaker_state(&pool, &trigger_id).await,
        "reset circuit breaker state",
    ) {
        return resp;
    }

    // Return updated state
    let info = match handle_db_error(
        TriggerRepository::get_circuit_breaker_info(&pool, &trigger_id).await,
        "fetch circuit breaker info",
    ) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Trigger not found"));
        }
        Err(resp) => return resp,
    };

    // Parse config (use default if not set)
    let config = info
        .circuit_breaker_config
        .and_then(|v| serde_json::from_value::<CircuitBreakerConfigParsed>(v).ok())
        .unwrap_or_default();

    // State should now be default (Closed)
    let state = CircuitBreakerStateParsed::default();

    let response = CircuitBreakerStateResponse {
        trigger_id: info.id,
        trigger_name: info.name,
        state: state.state,
        failure_count: state.failure_count,
        last_failure_time: state.last_failure_time,
        opened_at: state.opened_at,
        half_open_calls: state.half_open_calls,
        config: CircuitBreakerConfigResponse {
            failure_threshold: config.failure_threshold,
            recovery_timeout_seconds: config.recovery_timeout_seconds,
            half_open_max_calls: config.half_open_max_calls,
        },
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Internal struct for parsing circuit breaker config from JSON
#[derive(Debug, serde::Deserialize)]
struct CircuitBreakerConfigParsed {
    failure_threshold: u32,
    recovery_timeout_seconds: u64,
    half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfigParsed {
    fn default() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout_seconds: 3600,
            half_open_max_calls: 1,
        }
    }
}

/// Internal struct for parsing circuit breaker state from JSON
#[derive(Debug, serde::Deserialize)]
struct CircuitBreakerStateParsed {
    state: String,
    failure_count: u32,
    last_failure_time: Option<DateTime<Utc>>,
    opened_at: Option<DateTime<Utc>>,
    half_open_calls: u32,
}

impl Default for CircuitBreakerStateParsed {
    fn default() -> Self {
        Self {
            state: "Closed".to_string(),
            failure_count: 0,
            last_failure_time: None,
            opened_at: None,
            half_open_calls: 0,
        }
    }
}
