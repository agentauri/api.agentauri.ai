//! Trigger handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;

use crate::{
    handlers::helpers::{
        extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
    middleware::{get_verified_organization_id, get_verified_organization_id_with_role},
    models::{
        can_write, ActionResponse, ConditionResponse, CreateTriggerRequest, ErrorResponse,
        PaginatedResponse, PaginationMeta, PaginationParams, SuccessResponse,
        TriggerDetailResponse, TriggerResponse, UpdateTriggerRequest,
    },
    repositories::{ActionRepository, ConditionRepository, MemberRepository, TriggerRepository},
};

/// Create a new trigger
///
/// Creates a new trigger for event-driven actions. Requires write permission.
#[utoipa::path(
    post,
    path = "/api/v1/triggers",
    tag = "Triggers",
    request_body = CreateTriggerRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 201, description = "Trigger created", body = SuccessResponse<TriggerResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse)
    )
)]
pub async fn create_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateTriggerRequest>,
) -> impl Responder {
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
        return forbidden("Insufficient permissions to create triggers");
    }

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Create trigger
    let trigger = match handle_db_error(
        TriggerRepository::create(
            &pool,
            &user_id,
            &organization_id,
            &req.name,
            req.description.as_deref(),
            req.chain_id,
            &req.registry,
            req.enabled.unwrap_or(true),
            req.is_stateful.unwrap_or(false),
        )
        .await,
        "create trigger",
    ) {
        Ok(trigger) => trigger,
        Err(resp) => return resp,
    };

    let response = TriggerResponse::from(trigger);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List triggers for organization
///
/// Returns paginated list of triggers for the organization.
#[utoipa::path(
    get,
    path = "/api/v1/triggers",
    tag = "Triggers",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "List of triggers", body = PaginatedResponse<TriggerResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn list_triggers(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Get and verify organization_id from header (any role can list)
    let organization_id = match get_verified_organization_id(&req_http, &pool, &user_id).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Execute count and list in parallel for better performance
    let (total_result, triggers_result) = tokio::join!(
        TriggerRepository::count_by_organization(&pool, &organization_id),
        TriggerRepository::list_by_organization(&pool, &organization_id, query.limit, query.offset)
    );

    // Handle count result
    let total = match handle_db_error(total_result, "count triggers") {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Handle list result
    let triggers = match handle_db_error(triggers_result, "list triggers") {
        Ok(triggers) => triggers,
        Err(resp) => return resp,
    };

    let response = PaginatedResponse {
        data: triggers.into_iter().map(TriggerResponse::from).collect(),
        pagination: PaginationMeta::new(total, query.limit, query.offset),
    };

    HttpResponse::Ok().json(response)
}

/// Get a single trigger with conditions and actions
///
/// Returns trigger details including its conditions and actions.
#[utoipa::path(
    get,
    path = "/api/v1/triggers/{id}",
    tag = "Triggers",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Trigger details", body = SuccessResponse<TriggerDetailResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn get_trigger(
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

    // Get trigger
    let trigger = match handle_db_error(
        TriggerRepository::find_by_id(&pool, &trigger_id).await,
        "fetch trigger",
    ) {
        Ok(Some(trigger)) => trigger,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Trigger not found"));
        }
        Err(resp) => return resp,
    };

    // Fetch conditions and actions in parallel for better performance
    let (conditions_result, actions_result) = tokio::join!(
        ConditionRepository::list_by_trigger(&pool, &trigger_id),
        ActionRepository::list_by_trigger(&pool, &trigger_id)
    );

    // Handle conditions result
    let conditions = match handle_db_error(conditions_result, "fetch conditions") {
        Ok(conditions) => conditions,
        Err(resp) => return resp,
    };

    // Handle actions result
    let actions = match handle_db_error(actions_result, "fetch actions") {
        Ok(actions) => actions,
        Err(resp) => return resp,
    };

    let response = TriggerDetailResponse {
        trigger: TriggerResponse::from(trigger),
        conditions: conditions
            .into_iter()
            .map(ConditionResponse::from)
            .collect(),
        actions: actions.into_iter().map(ActionResponse::from).collect(),
    };

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update a trigger
///
/// Updates trigger configuration. Requires write permission.
#[utoipa::path(
    put,
    path = "/api/v1/triggers/{id}",
    tag = "Triggers",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    request_body = UpdateTriggerRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Trigger updated", body = SuccessResponse<TriggerResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn update_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateTriggerRequest>,
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
        return forbidden("Insufficient permissions to update triggers");
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

    // Update trigger
    let trigger = match handle_db_error(
        TriggerRepository::update(
            &pool,
            &trigger_id,
            req.name.as_deref(),
            req.description.as_ref().map(|d| Some(d.as_str())),
            req.chain_id,
            req.registry.as_deref(),
            req.enabled,
            req.is_stateful,
        )
        .await,
        "update trigger",
    ) {
        Ok(trigger) => trigger,
        Err(resp) => return resp,
    };

    let response = TriggerResponse::from(trigger);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete a trigger
///
/// Permanently deletes a trigger and its conditions/actions. Requires write permission.
#[utoipa::path(
    delete,
    path = "/api/v1/triggers/{id}",
    tag = "Triggers",
    params(
        ("id" = String, Path, description = "Trigger ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 204, description = "Trigger deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Trigger not found", body = ErrorResponse)
    )
)]
pub async fn delete_trigger(
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
        return forbidden("Insufficient permissions to delete triggers");
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

    // Delete trigger
    let deleted = match handle_db_error(
        TriggerRepository::delete(&pool, &trigger_id).await,
        "delete trigger",
    ) {
        Ok(deleted) => deleted,
        Err(resp) => return resp,
    };

    if !deleted {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    HttpResponse::NoContent().finish()
}

// =============================================================================
// Organization-scoped endpoints (path parameter for org_id)
// =============================================================================

/// List triggers for organization (path-based)
///
/// Returns paginated list of triggers for the specified organization.
/// Organization ID is taken from the URL path.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/triggers",
    tag = "Triggers",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of triggers", body = PaginatedResponse<TriggerResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_org_triggers(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let org_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Validate pagination
    if let Err(e) = query.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Invalid pagination: {}", e),
        ));
    }

    // Check membership (any role can list)
    match handle_db_error(
        MemberRepository::get_role(&pool, &org_id, &user_id).await,
        "check membership",
    ) {
        Ok(Some(_)) => {} // User is a member
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Organization not found"))
        }
        Err(resp) => return resp,
    };

    // Execute count and list in parallel for better performance
    let (total_result, triggers_result) = tokio::join!(
        TriggerRepository::count_by_organization(&pool, &org_id),
        TriggerRepository::list_by_organization(&pool, &org_id, query.limit, query.offset)
    );

    // Handle count result
    let total = match handle_db_error(total_result, "count triggers") {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Handle list result
    let triggers = match handle_db_error(triggers_result, "list triggers") {
        Ok(triggers) => triggers,
        Err(resp) => return resp,
    };

    let response = PaginatedResponse {
        data: triggers.into_iter().map(TriggerResponse::from).collect(),
        pagination: PaginationMeta::new(total, query.limit, query.offset),
    };

    HttpResponse::Ok().json(response)
}
