//! Trigger condition handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;
use validator::Validate;

use crate::{
    middleware::{
        get_user_id, get_verified_organization_id, get_verified_organization_id_with_role,
    },
    models::{
        can_write, ConditionResponse, CreateConditionRequest, ErrorResponse, SuccessResponse,
        UpdateConditionRequest,
    },
    repositories::{ConditionRepository, TriggerRepository},
};

/// Create a new condition for a trigger
///
/// POST /api/v1/triggers/{trigger_id}/conditions
/// Requires X-Organization-ID header
pub async fn create_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<CreateConditionRequest>,
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
            "Insufficient permissions to create conditions",
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
                "Failed to create condition",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Create condition
    let condition = match ConditionRepository::create(
        &pool,
        &trigger_id,
        &req.condition_type,
        &req.field,
        &req.operator,
        &req.value,
        req.config.as_ref(),
    )
    .await
    {
        Ok(condition) => condition,
        Err(e) => {
            tracing::error!("Failed to create condition: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create condition",
            ));
        }
    };

    let response = ConditionResponse::from(condition);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List conditions for a trigger
///
/// GET /api/v1/triggers/{trigger_id}/conditions
/// Requires X-Organization-ID header
pub async fn list_conditions(
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
                "Failed to fetch conditions",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Get conditions
    let conditions = match ConditionRepository::list_by_trigger(&pool, &trigger_id).await {
        Ok(conditions) => conditions,
        Err(e) => {
            tracing::error!("Failed to list conditions: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch conditions",
            ));
        }
    };

    let response: Vec<ConditionResponse> = conditions
        .into_iter()
        .map(ConditionResponse::from)
        .collect();

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update a condition
///
/// PUT /api/v1/triggers/{trigger_id}/conditions/{id}
/// Requires X-Organization-ID header
pub async fn update_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
    req: web::Json<UpdateConditionRequest>,
) -> impl Responder {
    let (trigger_id, condition_id) = path.into_inner();

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
            "Insufficient permissions to update conditions",
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
                "Failed to update condition",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Verify condition belongs to trigger
    let condition_trigger_id = match ConditionRepository::get_trigger_id(&pool, condition_id).await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Condition not found"));
        }
        Err(e) => {
            tracing::error!("Failed to get condition trigger_id: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update condition",
            ));
        }
    };

    if condition_trigger_id != trigger_id {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    // Update condition
    let condition = match ConditionRepository::update(
        &pool,
        condition_id,
        req.condition_type.as_deref(),
        req.field.as_deref(),
        req.operator.as_deref(),
        req.value.as_deref(),
        req.config.as_ref().map(Some),
    )
    .await
    {
        Ok(condition) => condition,
        Err(e) => {
            tracing::error!("Failed to update condition: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update condition",
            ));
        }
    };

    let response = ConditionResponse::from(condition);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete a condition
///
/// DELETE /api/v1/triggers/{trigger_id}/conditions/{id}
/// Requires X-Organization-ID header
pub async fn delete_condition(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (trigger_id, condition_id) = path.into_inner();

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
            "Insufficient permissions to delete conditions",
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
                "Failed to delete condition",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Verify condition belongs to trigger
    let condition_trigger_id = match ConditionRepository::get_trigger_id(&pool, condition_id).await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Condition not found"));
        }
        Err(e) => {
            tracing::error!("Failed to get condition trigger_id: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete condition",
            ));
        }
    };

    if condition_trigger_id != trigger_id {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    // Delete condition
    let deleted = match ConditionRepository::delete(&pool, condition_id).await {
        Ok(deleted) => deleted,
        Err(e) => {
            tracing::error!("Failed to delete condition: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete condition",
            ));
        }
    };

    if !deleted {
        return HttpResponse::NotFound()
            .json(ErrorResponse::new("not_found", "Condition not found"));
    }

    HttpResponse::NoContent().finish()
}
