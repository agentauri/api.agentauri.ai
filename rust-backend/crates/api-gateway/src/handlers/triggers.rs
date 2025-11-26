//! Trigger handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;
use validator::Validate;

use crate::{
    middleware::{
        get_user_id, get_verified_organization_id, get_verified_organization_id_with_role,
    },
    models::{
        can_write, ActionResponse, ConditionResponse, CreateTriggerRequest, ErrorResponse,
        PaginatedResponse, PaginationMeta, PaginationParams, SuccessResponse,
        TriggerDetailResponse, TriggerResponse, UpdateTriggerRequest,
    },
    repositories::{ActionRepository, ConditionRepository, TriggerRepository},
};

/// Create a new trigger
///
/// POST /api/v1/triggers
/// Requires X-Organization-ID header
pub async fn create_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    req: web::Json<CreateTriggerRequest>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Get and verify organization_id from header (also gets role)
    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    // Check user has write access
    if !can_write(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to create triggers",
        ));
    }

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Create trigger
    let trigger = match TriggerRepository::create(
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
    .await
    {
        Ok(trigger) => trigger,
        Err(e) => {
            tracing::error!("Failed to create trigger: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create trigger",
            ));
        }
    };

    let response = TriggerResponse::from(trigger);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List triggers for organization
///
/// GET /api/v1/triggers?limit=20&offset=0
/// Requires X-Organization-ID header
pub async fn list_triggers(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
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

    // Get total count
    let total = match TriggerRepository::count_by_organization(&pool, &organization_id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count triggers: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch triggers",
            ));
        }
    };

    // Get triggers
    let triggers = match TriggerRepository::list_by_organization(
        &pool,
        &organization_id,
        query.limit,
        query.offset,
    )
    .await
    {
        Ok(triggers) => triggers,
        Err(e) => {
            tracing::error!("Failed to list triggers: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch triggers",
            ));
        }
    };

    let response = PaginatedResponse {
        data: triggers.into_iter().map(TriggerResponse::from).collect(),
        pagination: PaginationMeta::new(total, query.limit, query.offset),
    };

    HttpResponse::Ok().json(response)
}

/// Get a single trigger with conditions and actions
///
/// GET /api/v1/triggers/{id}
/// Requires X-Organization-ID header
pub async fn get_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Get and verify organization_id from header (any role can view)
    let organization_id = match get_verified_organization_id(&req_http, &pool, &user_id).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    // Check if trigger belongs to the organization
    let belongs = match TriggerRepository::belongs_to_organization(
        &pool,
        &trigger_id,
        &organization_id,
    )
    .await
    {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch trigger",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Get trigger
    let trigger = match TriggerRepository::find_by_id(&pool, &trigger_id).await {
        Ok(Some(trigger)) => trigger,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Trigger not found"));
        }
        Err(e) => {
            tracing::error!("Failed to fetch trigger: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch trigger",
            ));
        }
    };

    // Get conditions
    let conditions = match ConditionRepository::list_by_trigger(&pool, &trigger_id).await {
        Ok(conditions) => conditions,
        Err(e) => {
            tracing::error!("Failed to fetch conditions: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch trigger details",
            ));
        }
    };

    // Get actions
    let actions = match ActionRepository::list_by_trigger(&pool, &trigger_id).await {
        Ok(actions) => actions,
        Err(e) => {
            tracing::error!("Failed to fetch actions: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch trigger details",
            ));
        }
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
/// PUT /api/v1/triggers/{id}
/// Requires X-Organization-ID header
pub async fn update_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<UpdateTriggerRequest>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Get and verify organization_id from header (also gets role)
    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    // Check user has write access
    if !can_write(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to update triggers",
        ));
    }

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Check if trigger belongs to the organization
    let belongs = match TriggerRepository::belongs_to_organization(
        &pool,
        &trigger_id,
        &organization_id,
    )
    .await
    {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update trigger",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Update trigger
    let trigger = match TriggerRepository::update(
        &pool,
        &trigger_id,
        req.name.as_deref(),
        req.description.as_ref().map(|d| Some(d.as_str())),
        req.chain_id,
        req.registry.as_deref(),
        req.enabled,
        req.is_stateful,
    )
    .await
    {
        Ok(trigger) => trigger,
        Err(e) => {
            tracing::error!("Failed to update trigger: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update trigger",
            ));
        }
    };

    let response = TriggerResponse::from(trigger);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete a trigger
///
/// DELETE /api/v1/triggers/{id}
/// Requires X-Organization-ID header
pub async fn delete_trigger(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let trigger_id = path.into_inner();

    // Get authenticated user_id
    let user_id = match get_user_id(&req_http) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse::new(
                "unauthorized",
                "Authentication required",
            ))
        }
    };

    // Get and verify organization_id from header (also gets role)
    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    // Check user has write access
    if !can_write(&role) {
        return HttpResponse::Forbidden().json(ErrorResponse::new(
            "forbidden",
            "Insufficient permissions to delete triggers",
        ));
    }

    // Check if trigger belongs to the organization
    let belongs = match TriggerRepository::belongs_to_organization(
        &pool,
        &trigger_id,
        &organization_id,
    )
    .await
    {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger organization: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete trigger",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Delete trigger
    let deleted = match TriggerRepository::delete(&pool, &trigger_id).await {
        Ok(deleted) => deleted,
        Err(e) => {
            tracing::error!("Failed to delete trigger: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete trigger",
            ));
        }
    };

    if !deleted {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    HttpResponse::NoContent().finish()
}
