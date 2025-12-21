//! Agent Follow Handlers
//!
//! Simplified interface for following all activities of an ERC-8004 agent
//! across identity, reputation, and validation registries.
//!
//! # Endpoints
//!
//! - `POST /api/v1/agents/{agent_id}/follow` - Start following an agent
//! - `GET /api/v1/agents/following` - List followed agents
//! - `PUT /api/v1/agents/{agent_id}/follow` - Update follow settings
//! - `DELETE /api/v1/agents/{agent_id}/follow` - Stop following an agent

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use shared::DbPool;
use tracing::info;

use crate::{
    handlers::helpers::{
        extract_user_id_or_unauthorized, forbidden, handle_db_error, validate_request,
    },
    middleware::{get_verified_organization_id, get_verified_organization_id_with_role},
    models::{
        agent_follows::{
            redact_secrets, AgentFollowDetailResponse, AgentFollowPath, AgentFollowResponse,
            ChainIdQuery, FollowActionRequest, FollowActionSummary, FollowAgentRequest,
            ListFollowsQuery, TriggerIds, UpdateFollowRequest,
        },
        can_write, ErrorResponse, PaginatedResponse, PaginationMeta, PaginationParams,
        SuccessResponse,
    },
    repositories::{
        ActionRepository, AgentFollowRepository, ConditionRepository, TriggerRepository,
    },
};

const MAX_FOLLOWS_PER_ORG: i64 = 100;
const REGISTRIES: [&str; 3] = ["identity", "reputation", "validation"];

// ============================================================================
// Handlers
// ============================================================================

/// Follow an agent
///
/// POST /api/v1/agents/{agent_id}/follow
///
/// Creates 3 triggers (one per registry) to monitor all activities
/// of the specified agent.
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/follow",
    tag = "Agent Follows",
    params(
        ("agent_id" = i64, Path, description = "Agent token ID to follow")
    ),
    request_body = FollowAgentRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 201, description = "Agent followed", body = SuccessResponse<AgentFollowDetailResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 409, description = "Already following this agent", body = ErrorResponse),
        (status = 429, description = "Too many follows", body = ErrorResponse)
    )
)]
pub async fn follow_agent(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<AgentFollowPath>,
    req: web::Json<FollowAgentRequest>,
) -> impl Responder {
    let agent_id = path.agent_id;

    // Auth checks
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    if !can_write(&role) {
        return forbidden("Insufficient permissions to follow agents");
    }

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Check rate limit
    let current_count = match handle_db_error(
        AgentFollowRepository::count_by_organization(&pool, &organization_id, None, None).await,
        "count follows",
    ) {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    if current_count >= MAX_FOLLOWS_PER_ORG {
        return HttpResponse::TooManyRequests().json(ErrorResponse::new(
            "rate_limit",
            format!("Maximum {} follows per organization", MAX_FOLLOWS_PER_ORG),
        ));
    }

    // Check if already following
    let existing = match handle_db_error(
        AgentFollowRepository::find_by_agent_and_org(
            &pool,
            agent_id,
            req.chain_id,
            &organization_id,
        )
        .await,
        "check existing follow",
    ) {
        Ok(existing) => existing,
        Err(resp) => return resp,
    };

    if existing.is_some() {
        return HttpResponse::Conflict().json(ErrorResponse::new(
            "already_following",
            "Already following this agent. Use PUT to update.",
        ));
    }

    // Create 3 triggers (one per registry) in a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "database_error",
                "Failed to start transaction",
            ));
        }
    };

    let mut trigger_ids = Vec::with_capacity(3);

    for registry in REGISTRIES {
        let trigger_name = format!("Follow Agent {} - {}", agent_id, registry);
        let trigger_description = format!(
            "Auto-generated trigger for following agent {} on {} registry",
            agent_id, registry
        );

        // Create trigger
        let trigger = match TriggerRepository::create_in_tx(
            &mut *tx,
            &user_id,
            &organization_id,
            &trigger_name,
            Some(&trigger_description),
            req.chain_id,
            registry,
            true,  // enabled
            false, // not stateful
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to create {} trigger: {}", registry, e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "trigger_creation_failed",
                    format!("Failed to create {} trigger", registry),
                ));
            }
        };

        // Create condition: agent_id = {agent_id}
        if let Err(e) = ConditionRepository::create_in_tx(
            &mut *tx,
            &trigger.id,
            "field_match",
            "agent_id",
            "equals",
            &agent_id.to_string(),
            None,
        )
        .await
        {
            tracing::error!("Failed to create condition: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "condition_creation_failed",
                "Failed to create condition",
            ));
        }

        // Create actions
        for (idx, action) in req.actions.iter().enumerate() {
            if let Err(e) = ActionRepository::create_in_tx(
                &mut *tx,
                &trigger.id,
                &action.action_type,
                idx as i32,
                &action.config,
            )
            .await
            {
                tracing::error!("Failed to create action: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "action_creation_failed",
                    "Failed to create action",
                ));
            }
        }

        trigger_ids.push(trigger.id);
    }

    // Create the follow record
    let follow = match AgentFollowRepository::create(
        &mut *tx,
        agent_id,
        req.chain_id,
        &organization_id,
        &user_id,
        &trigger_ids[0], // identity
        &trigger_ids[1], // reputation
        &trigger_ids[2], // validation
    )
    .await
    {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to create follow record: {}", e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "follow_creation_failed",
                "Failed to create follow record",
            ));
        }
    };

    // Commit transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "transaction_failed",
            "Failed to commit transaction",
        ));
    }

    info!(
        agent_id = agent_id,
        chain_id = req.chain_id,
        organization_id = %organization_id,
        follow_id = %follow.id,
        "Agent follow created"
    );

    let response = AgentFollowDetailResponse {
        follow: build_follow_response(&follow, &req.actions),
        trigger_ids: TriggerIds {
            identity: trigger_ids[0].clone(),
            reputation: trigger_ids[1].clone(),
            validation: trigger_ids[2].clone(),
        },
    };

    HttpResponse::Created().json(SuccessResponse::new(response))
}

/// List followed agents
///
/// GET /api/v1/agents/following
#[utoipa::path(
    get,
    path = "/api/v1/agents/following",
    tag = "Agent Follows",
    params(
        ("chain_id" = Option<i32>, Query, description = "Filter by chain ID"),
        ("enabled" = Option<bool>, Query, description = "Filter by enabled status"),
        ("limit" = Option<i64>, Query, description = "Maximum items per page"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "List of followed agents", body = PaginatedResponse<AgentFollowResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn list_following(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<ListFollowsQuery>,
    pagination: web::Query<PaginationParams>,
) -> impl Responder {
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let organization_id = match get_verified_organization_id(&req_http, &pool, &user_id).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    let limit = pagination.limit.min(100);
    let offset = pagination.offset;

    // Parallel count and list
    let (total_result, follows_result) = tokio::join!(
        AgentFollowRepository::count_by_organization(
            &pool,
            &organization_id,
            query.chain_id,
            query.enabled
        ),
        AgentFollowRepository::list_by_organization(
            &pool,
            &organization_id,
            query.chain_id,
            query.enabled,
            limit,
            offset
        )
    );

    let total = match handle_db_error(total_result, "count follows") {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    let follows = match handle_db_error(follows_result, "list follows") {
        Ok(follows) => follows,
        Err(resp) => return resp,
    };

    // Build responses
    let responses: Vec<AgentFollowResponse> = follows.into_iter().map(|f| f.into()).collect();

    HttpResponse::Ok().json(PaginatedResponse {
        data: responses,
        pagination: PaginationMeta::new(total, limit, offset),
    })
}

/// Unfollow an agent
///
/// DELETE /api/v1/agents/{agent_id}/follow?chain_id=xxx
#[utoipa::path(
    delete,
    path = "/api/v1/agents/{agent_id}/follow",
    tag = "Agent Follows",
    params(
        ("agent_id" = i64, Path, description = "Agent token ID"),
        ("chain_id" = i32, Query, description = "Chain ID")
    ),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 204, description = "Unfollowed successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not following this agent", body = ErrorResponse)
    )
)]
pub async fn unfollow_agent(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<AgentFollowPath>,
    query: web::Query<ChainIdQuery>,
) -> impl Responder {
    let agent_id = path.agent_id;

    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    if !can_write(&role) {
        return forbidden("Insufficient permissions to unfollow agents");
    }

    // Find the follow
    let follow = match handle_db_error(
        AgentFollowRepository::find_by_agent_and_org(
            &pool,
            agent_id,
            query.chain_id,
            &organization_id,
        )
        .await,
        "find follow",
    ) {
        Ok(Some(f)) => f,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Not following this agent"))
        }
        Err(resp) => return resp,
    };

    // Start transaction to delete triggers and follow
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "database_error",
                "Failed to start transaction",
            ));
        }
    };

    // Delete all 3 triggers (cascade will handle conditions/actions)
    for trigger_id in [
        &follow.trigger_identity_id,
        &follow.trigger_reputation_id,
        &follow.trigger_validation_id,
    ] {
        if let Err(e) = TriggerRepository::delete_in_tx(&mut *tx, trigger_id).await {
            tracing::error!("Failed to delete trigger {}: {}", trigger_id, e);
            let _ = tx.rollback().await;
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "trigger_deletion_failed",
                "Failed to delete trigger",
            ));
        }
    }

    // Delete the follow record
    if let Err(e) = AgentFollowRepository::delete(&mut *tx, &follow.id).await {
        tracing::error!("Failed to delete follow: {}", e);
        let _ = tx.rollback().await;
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "follow_deletion_failed",
            "Failed to delete follow",
        ));
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "transaction_failed",
            "Failed to commit transaction",
        ));
    }

    info!(
        agent_id = agent_id,
        chain_id = query.chain_id,
        organization_id = %organization_id,
        "Agent unfollowed"
    );

    HttpResponse::NoContent().finish()
}

/// Update follow settings
///
/// PUT /api/v1/agents/{agent_id}/follow?chain_id=xxx
#[utoipa::path(
    put,
    path = "/api/v1/agents/{agent_id}/follow",
    tag = "Agent Follows",
    params(
        ("agent_id" = i64, Path, description = "Agent token ID"),
        ("chain_id" = i32, Query, description = "Chain ID")
    ),
    request_body = UpdateFollowRequest,
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "Follow updated", body = SuccessResponse<AgentFollowResponse>),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not following this agent", body = ErrorResponse)
    )
)]
pub async fn update_follow(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    path: web::Path<AgentFollowPath>,
    query: web::Query<ChainIdQuery>,
    req: web::Json<UpdateFollowRequest>,
) -> impl Responder {
    let agent_id = path.agent_id;

    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (organization_id, role) =
        match get_verified_organization_id_with_role(&req_http, &pool, &user_id).await {
            Ok(result) => result,
            Err(response) => return response,
        };

    if !can_write(&role) {
        return forbidden("Insufficient permissions to update follow");
    }

    // Validate request
    if let Err(resp) = validate_request(&*req) {
        return resp;
    }

    // Find the follow
    let follow = match handle_db_error(
        AgentFollowRepository::find_by_agent_and_org(
            &pool,
            agent_id,
            query.chain_id,
            &organization_id,
        )
        .await,
        "find follow",
    ) {
        Ok(Some(f)) => f,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ErrorResponse::new("not_found", "Not following this agent"))
        }
        Err(resp) => return resp,
    };

    // Start transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "database_error",
                "Failed to start transaction",
            ));
        }
    };

    // Update enabled status on all 3 triggers if provided
    if let Some(enabled) = req.enabled {
        for trigger_id in [
            &follow.trigger_identity_id,
            &follow.trigger_reputation_id,
            &follow.trigger_validation_id,
        ] {
            if let Err(e) =
                TriggerRepository::update_enabled_in_tx(&mut *tx, trigger_id, enabled).await
            {
                tracing::error!("Failed to update trigger enabled status: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "update_failed",
                    "Failed to update trigger",
                ));
            }
        }
    }

    // Update actions on all 3 triggers if provided
    if let Some(ref actions) = req.actions {
        for trigger_id in [
            &follow.trigger_identity_id,
            &follow.trigger_reputation_id,
            &follow.trigger_validation_id,
        ] {
            // Delete existing actions
            if let Err(e) = ActionRepository::delete_by_trigger_in_tx(&mut *tx, trigger_id).await {
                tracing::error!("Failed to delete actions: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "update_failed",
                    "Failed to update actions",
                ));
            }

            // Create new actions
            for (idx, action) in actions.iter().enumerate() {
                if let Err(e) = ActionRepository::create_in_tx(
                    &mut *tx,
                    trigger_id,
                    &action.action_type,
                    idx as i32,
                    &action.config,
                )
                .await
                {
                    tracing::error!("Failed to create action: {}", e);
                    let _ = tx.rollback().await;
                    return HttpResponse::InternalServerError().json(ErrorResponse::new(
                        "update_failed",
                        "Failed to create action",
                    ));
                }
            }
        }
    }

    // Update follow record enabled status
    let updated_follow = if req.enabled.is_some() {
        match AgentFollowRepository::update_enabled(&pool, &follow.id, req.enabled.unwrap()).await {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to update follow: {}", e);
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "update_failed",
                    "Failed to update follow",
                ));
            }
        }
    } else {
        follow
    };

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse::new(
            "transaction_failed",
            "Failed to commit transaction",
        ));
    }

    info!(
        agent_id = agent_id,
        chain_id = query.chain_id,
        organization_id = %organization_id,
        "Agent follow updated"
    );

    // Build response with updated actions if provided
    let action_summaries = req.actions.as_ref().map(|actions| {
        actions
            .iter()
            .map(|a| FollowActionSummary {
                action_type: a.action_type.clone(),
                config_preview: redact_secrets(&a.config),
            })
            .collect()
    });

    let mut response: AgentFollowResponse = updated_follow.into();
    if let Some(summaries) = action_summaries {
        response.actions = summaries;
    }

    HttpResponse::Ok().json(SuccessResponse::new(response))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn build_follow_response(
    follow: &shared::models::AgentFollow,
    actions: &[FollowActionRequest],
) -> AgentFollowResponse {
    AgentFollowResponse {
        id: follow.id.clone(),
        agent_id: follow.agent_id,
        chain_id: follow.chain_id,
        organization_id: follow.organization_id.clone(),
        enabled: follow.enabled,
        registries_monitored: 3,
        actions: actions
            .iter()
            .map(|a| FollowActionSummary {
                action_type: a.action_type.clone(),
                config_preview: redact_secrets(&a.config),
            })
            .collect(),
        created_at: follow.created_at,
        updated_at: follow.updated_at,
    }
}
