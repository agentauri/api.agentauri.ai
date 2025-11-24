//! Trigger action handlers

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;
use validator::Validate;

use crate::{
    middleware::get_user_id,
    models::{
        ActionResponse, CreateActionRequest, ErrorResponse, SuccessResponse, UpdateActionRequest,
    },
    repositories::{ActionRepository, TriggerRepository},
};

/// Create a new action for a trigger
///
/// POST /api/v1/triggers/{trigger_id}/actions
pub async fn create_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<String>,
    req: web::Json<CreateActionRequest>,
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

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Check if trigger belongs to user
    let belongs = match TriggerRepository::belongs_to_user(&pool, &trigger_id, &user_id).await {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger ownership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create action",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Create action
    let action = match ActionRepository::create(
        &pool,
        &trigger_id,
        &req.action_type,
        req.priority.unwrap_or(0),
        &req.config,
    )
    .await
    {
        Ok(action) => action,
        Err(e) => {
            tracing::error!("Failed to create action: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to create action",
            ));
        }
    };

    let response = ActionResponse::from(action);
    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List actions for a trigger
///
/// GET /api/v1/triggers/{trigger_id}/actions
pub async fn list_actions(
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

    // Check if trigger belongs to user
    let belongs = match TriggerRepository::belongs_to_user(&pool, &trigger_id, &user_id).await {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger ownership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch actions",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Get actions
    let actions = match ActionRepository::list_by_trigger(&pool, &trigger_id).await {
        Ok(actions) => actions,
        Err(e) => {
            tracing::error!("Failed to list actions: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to fetch actions",
            ));
        }
    };

    let response: Vec<ActionResponse> = actions.into_iter().map(ActionResponse::from).collect();

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Update an action
///
/// PUT /api/v1/triggers/{trigger_id}/actions/{id}
pub async fn update_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
    req: web::Json<UpdateActionRequest>,
) -> impl Responder {
    let (trigger_id, action_id) = path.into_inner();

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

    // Validate request
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(
            "validation_error",
            format!("Validation failed: {}", e),
        ));
    }

    // Check if trigger belongs to user
    let belongs = match TriggerRepository::belongs_to_user(&pool, &trigger_id, &user_id).await {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger ownership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update action",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Verify action belongs to trigger
    let action_trigger_id = match ActionRepository::get_trigger_id(&pool, action_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Action not found"));
        }
        Err(e) => {
            tracing::error!("Failed to get action trigger_id: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update action",
            ));
        }
    };

    if action_trigger_id != trigger_id {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    // Update action
    let action = match ActionRepository::update(
        &pool,
        action_id,
        req.action_type.as_deref(),
        req.priority,
        req.config.as_ref(),
    )
    .await
    {
        Ok(action) => action,
        Err(e) => {
            tracing::error!("Failed to update action: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to update action",
            ));
        }
    };

    let response = ActionResponse::from(action);
    HttpResponse::Ok().json(SuccessResponse::new(response))
}

/// Delete an action
///
/// DELETE /api/v1/triggers/{trigger_id}/actions/{id}
pub async fn delete_action(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (trigger_id, action_id) = path.into_inner();

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

    // Check if trigger belongs to user
    let belongs = match TriggerRepository::belongs_to_user(&pool, &trigger_id, &user_id).await {
        Ok(belongs) => belongs,
        Err(e) => {
            tracing::error!("Failed to check trigger ownership: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete action",
            ));
        }
    };

    if !belongs {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Trigger not found"));
    }

    // Verify action belongs to trigger
    let action_trigger_id = match ActionRepository::get_trigger_id(&pool, action_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Action not found"));
        }
        Err(e) => {
            tracing::error!("Failed to get action trigger_id: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete action",
            ));
        }
    };

    if action_trigger_id != trigger_id {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    // Delete action
    let deleted = match ActionRepository::delete(&pool, action_id).await {
        Ok(deleted) => deleted,
        Err(e) => {
            tracing::error!("Failed to delete action: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to delete action",
            ));
        }
    };

    if !deleted {
        return HttpResponse::NotFound().json(ErrorResponse::new("not_found", "Action not found"));
    }

    HttpResponse::NoContent().finish()
}
