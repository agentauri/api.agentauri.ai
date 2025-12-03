//! Trigger action handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;

use crate::{
    handlers::helpers::{
        extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
    middleware::{get_verified_organization_id, get_verified_organization_id_with_role},
    models::{
        can_write, ActionResponse, CreateActionRequest, ErrorResponse, SuccessResponse,
        UpdateActionRequest,
    },
    repositories::{ActionRepository, TriggerRepository},
};

/// Create a new action for a trigger
///
/// Creates a new action to execute when the trigger matches. Requires write permission.
#[utoipa::path(
    post,
    path = "/api/v1/triggers/{trigger_id}/actions",
    tag = "Actions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID")
    ),
    request_body = CreateActionRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 201, description = "Action created", body = SuccessResponse<ActionResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn create_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<CreateActionRequest>,
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
        return forbidden("Insufficient permissions to create actions");
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

    // Create action
    let action = match handle_db_error(
        ActionRepository::create(
            &pool,
            &trigger_id,
            &req.action_type,
            req.priority.unwrap_or(0),
            &req.config,
        )
        .await,
        "create action",
    ) {
        Ok(action) => action,
        Err(resp) => return resp,
    };

    let response = ActionResponse::from(action);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List actions for a trigger
///
/// Returns all actions for the specified trigger.
#[utoipa::path(
    get,
    path = "/api/v1/triggers/{trigger_id}/actions",
    tag = "Actions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "List of actions", body = SuccessResponse<Vec<ActionResponse>>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn list_actions(
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

    // Get actions
    let actions = match handle_db_error(
        ActionRepository::list_by_trigger(&pool, &trigger_id).await,
        "list actions",
    ) {
        Ok(actions) => actions,
        Err(resp) => return resp,
    };

    let response: Vec<ActionResponse> = actions.into_iter().map(ActionResponse::from).collect();

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update an action
///
/// Updates action configuration. Requires write permission.
#[utoipa::path(
    put,
    path = "/api/v1/triggers/{trigger_id}/actions/{id}",
    tag = "Actions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID"),
        ("id" = i32, Path, description = "Action ID")
    ),
    request_body = UpdateActionRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Action updated", body = SuccessResponse<ActionResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger or action not found", body = ErrorResponse)
    )
)]
pub async fn update_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
    req: web::Json<UpdateActionRequest>,
) -> impl Responder {
    let (trigger_id, action_id) = path.into_inner();

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
        return forbidden("Insufficient permissions to update actions");
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

    // Verify action belongs to trigger
    let action_trigger_id = match handle_db_error(
        ActionRepository::get_trigger_id(&pool, action_id).await,
        "get action trigger_id",
    ) {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Action not found"));
        }
        Err(resp) => return resp,
    };

    if action_trigger_id != trigger_id {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    // Update action
    let action = match handle_db_error(
        ActionRepository::update(
            &pool,
            action_id,
            req.action_type.as_deref(),
            req.priority,
            req.config.as_ref(),
        )
        .await,
        "update action",
    ) {
        Ok(action) => action,
        Err(resp) => return resp,
    };

    let response = ActionResponse::from(action);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete an action
///
/// Permanently removes the action. Requires write permission.
#[utoipa::path(
    delete,
    path = "/api/v1/triggers/{trigger_id}/actions/{id}",
    tag = "Actions",
    params(
        ("trigger_id" = String, Path, description = "Trigger ID"),
        ("id" = i32, Path, description = "Action ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 204, description = "Action deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger or action not found", body = ErrorResponse)
    )
)]
pub async fn delete_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (trigger_id, action_id) = path.into_inner();

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
        return forbidden("Insufficient permissions to delete actions");
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

    // Verify action belongs to trigger
    let action_trigger_id = match handle_db_error(
        ActionRepository::get_trigger_id(&pool, action_id).await,
        "get action trigger_id",
    ) {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Action not found"));
        }
        Err(resp) => return resp,
    };

    if action_trigger_id != trigger_id {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    // Delete action
    let deleted = match handle_db_error(
        ActionRepository::delete(&pool, action_id).await,
        "delete action",
    ) {
        Ok(deleted) => deleted,
        Err(resp) => return resp,
    };

    if !deleted {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    HttpResponse::NoContent().finish()
}
