//! Trigger condition handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;

use crate::{
    handlers::helpers::{
        extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
    middleware::{get_verified_organization_id, get_verified_organization_id_with_role},
    models::{
        can_write, ConditionResponse, CreateConditionRequest, ErrorResponse, SuccessResponse,
        UpdateConditionRequest,
    },
    repositories::{ConditionRepository, TriggerRepository},
};

/// Create a new condition for a trigger
///
/// Creates a new matching condition for the trigger. Requires write permission.
#[utoipa::path(
    post,
    path = "/api/v1/triggers/{trigger_id}/conditions",
    tag = "Conditions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID")
    ),
    request_body = CreateConditionRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 201, description = "Condition created", body = SuccessResponse<ConditionResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn create_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<CreateConditionRequest>,
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
        return forbidden("Insufficient permissions to create conditions");
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

    // Create condition
    let condition = match handle_db_error(
        ConditionRepository::create(
            &pool,
            &trigger_id,
            &req.condition_type,
            &req.field,
            &req.operator,
            &req.value,
            req.config.as_ref(),
        )
        .await,
        "create condition",
    ) {
        Ok(condition) => condition,
        Err(resp) => return resp,
    };

    let response = ConditionResponse::from(condition);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List conditions for a trigger
///
/// Returns all conditions for the specified trigger.
#[utoipa::path(
    get,
    path = "/api/v1/triggers/{trigger_id}/conditions",
    tag = "Conditions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "List of conditions", body = SuccessResponse<Vec<ConditionResponse>>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn list_conditions(
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

    // Get conditions
    let conditions = match handle_db_error(
        ConditionRepository::list_by_trigger(&pool, &trigger_id).await,
        "list conditions",
    ) {
        Ok(conditions) => conditions,
        Err(resp) => return resp,
    };

    let response: Vec<ConditionResponse> = conditions
        .into_iter()
        .map(ConditionResponse::from)
        .collect();

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update a condition
///
/// Updates condition configuration. Requires write permission.
#[utoipa::path(
    put,
    path = "/api/v1/triggers/{trigger_id}/conditions/{id}",
    tag = "Conditions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID"),
        ("id" = i32, Path, description = "Condition ID")
    ),
    request_body = UpdateConditionRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Condition updated", body = SuccessResponse<ConditionResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger or condition not found", body = ErrorResponse)
    )
)]
pub async fn update_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
    req: web::Json<UpdateConditionRequest>,
) -> impl Responder {
    let (trigger_id, condition_id) = path.into_inner();

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
        return forbidden("Insufficient permissions to update conditions");
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

    // Verify condition belongs to trigger
    let condition_trigger_id = match handle_db_error(
        ConditionRepository::get_trigger_id(&pool, condition_id).await,
        "get condition trigger_id",
    ) {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Condition not found"));
        }
        Err(resp) => return resp,
    };

    if condition_trigger_id != trigger_id {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    // Update condition
    let condition = match handle_db_error(
        ConditionRepository::update(
            &pool,
            condition_id,
            req.condition_type.as_deref(),
            req.field.as_deref(),
            req.operator.as_deref(),
            req.value.as_deref(),
            req.config.as_ref().map(Some),
        )
        .await,
        "update condition",
    ) {
        Ok(condition) => condition,
        Err(resp) => return resp,
    };

    let response = ConditionResponse::from(condition);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete a condition
///
/// Permanently removes the condition. Requires write permission.
#[utoipa::path(
    delete,
    path = "/api/v1/triggers/{trigger_id}/conditions/{id}",
    tag = "Conditions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID"),
        ("id" = i32, Path, description = "Condition ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 204, description = "Condition deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger or condition not found", body = ErrorResponse)
    )
)]
pub async fn delete_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (trigger_id, condition_id) = path.into_inner();

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
        return forbidden("Insufficient permissions to delete conditions");
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

    // Verify condition belongs to trigger
    let condition_trigger_id = match handle_db_error(
        ConditionRepository::get_trigger_id(&pool, condition_id).await,
        "get condition trigger_id",
    ) {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Condition not found"));
        }
        Err(resp) => return resp,
    };

    if condition_trigger_id != trigger_id {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    // Delete condition
    let deleted = match handle_db_error(
        ConditionRepository::delete(&pool, condition_id).await,
        "delete condition",
    ) {
        Ok(deleted) => deleted,
        Err(resp) => return resp,
    };

    if !deleted {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    HttpResponse::NoContent().finish()
}
