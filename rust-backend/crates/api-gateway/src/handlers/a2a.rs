//! A2A Protocol JSON-RPC 2.0 handlers
//!
//! Implements the Agent-to-Agent protocol for async task queries.
//! Reference: docs/protocols/A2A_INTEGRATION.md

use actix_web::web::Bytes;
use actix_web::{http::header, web, HttpRequest, HttpResponse, Responder};
use futures_util::stream;
use shared::DbPool;
use std::time::{Duration, Instant};
use tracing::instrument;
use uuid::Uuid;

use crate::handlers::helpers::extract_user_id_or_unauthorized;
use crate::middleware::get_verified_organization_id;
use crate::models::a2a::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, TaskCancelParams, TaskCancelResult,
    TaskGetParams, TaskGetResult, TaskSendParams, TaskSendResult, TaskStatus,
};
use crate::repositories::{A2aTaskRepository, CreditRepository};
use crate::services::{A2aAuditService, AuditActor, ToolRegistry};

// ============================================================================
// JSON-RPC Main Endpoint
// ============================================================================

/// A2A JSON-RPC 2.0 endpoint
///
/// Handles all A2A protocol methods through a single endpoint.
/// Methods: tasks/send, tasks/get, tasks/cancel
#[utoipa::path(
    post,
    path = "/api/v1/a2a/rpc",
    tag = "A2A Protocol",
    request_body = JsonRpcRequest,
    responses(
        (status = 200, description = "JSON-RPC response", body = JsonRpcResponse<serde_json::Value>),
        (status = 400, description = "Invalid JSON-RPC request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
#[instrument(skip(pool, req), fields(method = %payload.method))]
pub async fn a2a_rpc(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    payload: web::Json<JsonRpcRequest>,
) -> impl Responder {
    let request = payload.into_inner();
    let request_id = request.id.clone();

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::invalid_request(),
            request_id,
        ));
    }

    // Extract user ID from JWT
    let user_id = match extract_user_id_or_unauthorized(&req) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::new(-32000, "Unauthorized: missing or invalid authentication"),
                request_id,
            ));
        }
    };

    // Extract and verify organization ID (TEXT in DB)
    let org_id = match get_verified_organization_id(&req, &pool, &user_id).await {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::new(-32000, "Unauthorized: missing or invalid organization"),
                request_id,
            ));
        }
    };

    // Route to appropriate handler
    match request.method.as_str() {
        "tasks/send" => handle_tasks_send(&pool, &org_id, &request.params, request_id).await,
        "tasks/get" => handle_tasks_get(&pool, &org_id, &request.params, request_id).await,
        "tasks/cancel" => handle_tasks_cancel(&pool, &org_id, &request.params, request_id).await,
        _ => HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::method_not_found(),
            request_id,
        )),
    }
}

// ============================================================================
// Method Handlers
// ============================================================================

/// Maximum pending tasks per organization (rate limiting)
const MAX_PENDING_TASKS_PER_ORG: i64 = 100;

/// Maximum size for task arguments JSON (100KB)
const MAX_ARGUMENTS_SIZE_BYTES: usize = 100_000;

/// Maximum SSE stream duration (5 minutes)
const MAX_SSE_STREAM_DURATION_SECS: u64 = 300;

/// SSE poll interval
const SSE_POLL_INTERVAL_MS: u64 = 2000;

/// Handle tasks/send method
async fn handle_tasks_send(
    pool: &DbPool,
    org_id: &str,
    params: &serde_json::Value,
    request_id: serde_json::Value,
) -> HttpResponse {
    // Parse params
    let send_params: TaskSendParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params(&e.to_string()),
                request_id,
            ));
        }
    };

    // Validate tool name using centralized ToolRegistry
    if !ToolRegistry::is_valid(&send_params.task.tool) {
        let valid_tools = ToolRegistry::tool_names().join(", ");
        return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::invalid_params(&format!(
                "Unknown tool: '{}'. Valid tools: {}",
                send_params.task.tool, valid_tools
            )),
            request_id,
        ));
    }

    // SECURITY: Validate arguments size to prevent DoS via large payloads
    let args_json = match serde_json::to_string(&send_params.task.arguments) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!("Failed to serialize arguments: {:?}", e);
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params("Invalid arguments format"),
                request_id,
            ));
        }
    };

    if args_json.len() > MAX_ARGUMENTS_SIZE_BYTES {
        return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::invalid_params(&format!(
                "Arguments too large: {} bytes (max: {} bytes)",
                args_json.len(),
                MAX_ARGUMENTS_SIZE_BYTES
            )),
            request_id,
        ));
    }

    // SECURITY: Rate limiting - check pending task count per organization
    let pending_count = match A2aTaskRepository::count_pending_by_organization(pool, org_id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count pending tasks: {:?}", e);
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::internal_error(),
                request_id,
            ));
        }
    };

    if pending_count >= MAX_PENDING_TASKS_PER_ORG {
        tracing::warn!(
            org_id = %org_id,
            pending_count = pending_count,
            "Rate limit exceeded for task creation"
        );
        return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::new(
                -32003,
                format!(
                    "Rate limit exceeded: {} pending tasks (max: {})",
                    pending_count, MAX_PENDING_TASKS_PER_ORG
                ),
            ),
            request_id,
        ));
    }

    // SECURITY: Credit validation - check sufficient balance before creating task
    // Use centralized ToolRegistry for cost lookup
    let cost_micro_usdc = ToolRegistry::get_cost_micro_usdc(&send_params.task.tool);
    if cost_micro_usdc > 0 {
        match CreditRepository::get_balance(pool, org_id).await {
            Ok(Some(credit)) => {
                if credit.balance < cost_micro_usdc {
                    tracing::warn!(
                        org_id = %org_id,
                        balance = credit.balance,
                        required = cost_micro_usdc,
                        tool = %send_params.task.tool,
                        "Insufficient credits for task creation"
                    );
                    return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                        JsonRpcError::new(
                            -32004,
                            format!(
                                "Insufficient credits: balance {} micro-USDC, required {} micro-USDC",
                                credit.balance, cost_micro_usdc
                            ),
                        ),
                        request_id,
                    ));
                }
            }
            Ok(None) => {
                // No credits initialized for this organization
                tracing::warn!(
                    org_id = %org_id,
                    tool = %send_params.task.tool,
                    "Credits not initialized for organization"
                );
                return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                    JsonRpcError::new(
                        -32004,
                        "Credits not initialized. Please purchase credits first.",
                    ),
                    request_id,
                ));
            }
            Err(e) => {
                tracing::error!("Failed to check credit balance: {:?}", e);
                return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                    JsonRpcError::internal_error(),
                    request_id,
                ));
            }
        }
    }

    // Create task in database
    match A2aTaskRepository::create(
        pool,
        org_id,
        &send_params.task.tool,
        &send_params.task.arguments,
    )
    .await
    {
        Ok(task) => {
            // AUDIT: Log task creation
            if let Err(e) = A2aAuditService::log_created(
                pool,
                &task.id,
                org_id,
                &send_params.task.tool,
                AuditActor::ApiKey(org_id.to_string()), // In production, use actual API key prefix
            )
            .await
            {
                tracing::warn!("Failed to log task creation audit: {:?}", e);
                // Don't fail the request for audit failures
            }

            let result = TaskSendResult {
                task_id: task.id.to_string(),
                status: TaskStatus::Submitted,
                estimated_cost: ToolRegistry::get_cost_display(&send_params.task.tool)
                    .map(String::from),
            };
            HttpResponse::Ok().json(JsonRpcResponse::success(result, request_id))
        }
        Err(e) => {
            tracing::error!("Failed to create task: {:?}", e);
            HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::internal_error(),
                request_id,
            ))
        }
    }
}

/// Handle tasks/get method
async fn handle_tasks_get(
    pool: &DbPool,
    org_id: &str,
    params: &serde_json::Value,
    request_id: serde_json::Value,
) -> HttpResponse {
    // Parse params
    let get_params: TaskGetParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params(&e.to_string()),
                request_id,
            ));
        }
    };

    // Parse task ID
    let task_id = match Uuid::parse_str(&get_params.task_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params("Invalid task_id format"),
                request_id,
            ));
        }
    };

    // Find task
    match A2aTaskRepository::find_by_id_and_org(pool, &task_id, org_id).await {
        Ok(Some(task)) => {
            let result = task.to_get_result();
            HttpResponse::Ok().json(JsonRpcResponse::success(result, request_id))
        }
        Ok(None) => HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::task_not_found(),
            request_id,
        )),
        Err(e) => {
            tracing::error!("Failed to get task: {:?}", e);
            HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::internal_error(),
                request_id,
            ))
        }
    }
}

/// Handle tasks/cancel method
async fn handle_tasks_cancel(
    pool: &DbPool,
    org_id: &str,
    params: &serde_json::Value,
    request_id: serde_json::Value,
) -> HttpResponse {
    // Parse params
    let cancel_params: TaskCancelParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params(&e.to_string()),
                request_id,
            ));
        }
    };

    // Parse task ID
    let task_id = match Uuid::parse_str(&cancel_params.task_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::invalid_params("Invalid task_id format"),
                request_id,
            ));
        }
    };

    // Cancel task
    match A2aTaskRepository::cancel_task(pool, &task_id, org_id).await {
        Ok(Some(task)) => {
            // AUDIT: Log task cancellation
            if let Err(e) = A2aAuditService::log_cancelled(
                pool,
                &task.id,
                org_id,
                AuditActor::ApiKey(org_id.to_string()),
            )
            .await
            {
                tracing::warn!("Failed to log task cancellation audit: {:?}", e);
            }

            let result = TaskCancelResult {
                task_id: task.id.to_string(),
                status: TaskStatus::Cancelled,
            };
            HttpResponse::Ok().json(JsonRpcResponse::success(result, request_id))
        }
        Ok(None) => HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
            JsonRpcError::task_not_found(),
            request_id,
        )),
        Err(e) => {
            tracing::error!("Failed to cancel task: {:?}", e);
            HttpResponse::Ok().json(JsonRpcResponse::<()>::error(
                JsonRpcError::internal_error(),
                request_id,
            ))
        }
    }
}

// ============================================================================
// REST Endpoints (for convenience)
// ============================================================================

/// Get task status by ID (REST endpoint)
#[utoipa::path(
    get,
    path = "/api/v1/a2a/tasks/{id}",
    tag = "A2A Protocol",
    params(
        ("id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task status", body = TaskGetResult),
        (status = 404, description = "Task not found")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
#[instrument(skip(pool, req))]
pub async fn get_task_status(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let task_id_str = path.into_inner();

    // Extract user ID from JWT
    let user_id = match extract_user_id_or_unauthorized(&req) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "unauthorized",
                "message": "Missing or invalid authentication"
            }));
        }
    };

    // Extract and verify organization ID (TEXT in DB)
    let org_id = match get_verified_organization_id(&req, &pool, &user_id).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Parse task ID
    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid_request",
                "message": "Invalid task ID format"
            }));
        }
    };

    // Find task
    match A2aTaskRepository::find_by_id_and_org(&pool, &task_id, &org_id).await {
        Ok(Some(task)) => HttpResponse::Ok().json(task.to_get_result()),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "not_found",
            "message": "Task not found"
        })),
        Err(e) => {
            tracing::error!("Failed to get task: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "internal_error",
                "message": "Failed to retrieve task"
            }))
        }
    }
}

// ============================================================================
// SSE Streaming Endpoint
// ============================================================================

/// Stream task progress via Server-Sent Events
///
/// Returns SSE stream with task progress updates.
/// Events: progress, complete, error
#[utoipa::path(
    get,
    path = "/api/v1/a2a/tasks/{id}/stream",
    tag = "A2A Protocol",
    params(
        ("id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "SSE stream of task updates"),
        (status = 404, description = "Task not found")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
#[instrument(skip(pool, req))]
pub async fn stream_task_progress(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let task_id_str = path.into_inner();

    // Extract user ID from JWT
    let user_id = match extract_user_id_or_unauthorized(&req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Extract and verify organization ID (TEXT in DB)
    let org_id = match get_verified_organization_id(&req, &pool, &user_id).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Parse task ID
    let task_id = match Uuid::parse_str(&task_id_str) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid_request",
                "message": "Invalid task ID format"
            }));
        }
    };

    // Verify task exists and belongs to org
    match A2aTaskRepository::find_by_id_and_org(&pool, &task_id, &org_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "not_found",
                "message": "Task not found"
            }));
        }
        Err(e) => {
            tracing::error!("Failed to get task: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "internal_error",
                "message": "Failed to retrieve task"
            }));
        }
    }

    // Create SSE stream with timeout protection
    // SECURITY FIX: Added max stream duration to prevent memory leaks from long-running connections
    let pool_clone = pool.get_ref().clone();
    let stream_start = Instant::now();
    let max_duration = Duration::from_secs(MAX_SSE_STREAM_DURATION_SECS);
    let poll_interval = Duration::from_millis(SSE_POLL_INTERVAL_MS);

    let sse_stream = stream::unfold(
        (pool_clone, task_id, org_id, false, stream_start),
        move |(pool, task_id, org_id, done, start)| async move {
            if done {
                return None;
            }

            // SECURITY: Check if stream has exceeded max duration
            if start.elapsed() > max_duration {
                tracing::info!(
                    task_id = %task_id,
                    duration_secs = start.elapsed().as_secs(),
                    "SSE stream timeout - closing connection"
                );
                let sse_event = format!(
                    "event: timeout\ndata: {{\"error\":\"stream_timeout\",\"duration_secs\":{}}}\n\n",
                    start.elapsed().as_secs()
                );
                return Some((
                    Ok(Bytes::from(sse_event)),
                    (pool, task_id, org_id, true, start),
                ));
            }

            // Wait between polls (increased from 500ms to 2000ms to reduce load)
            tokio::time::sleep(poll_interval).await;

            // Fetch current task status
            match A2aTaskRepository::find_by_id_and_org(&pool, &task_id, &org_id).await {
                Ok(Some(task)) => {
                    let result = task.to_get_result();
                    let is_terminal = matches!(
                        result.status,
                        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
                    );

                    let event_type = match result.status {
                        TaskStatus::Completed => "complete",
                        TaskStatus::Failed | TaskStatus::Cancelled => "error",
                        _ => "progress",
                    };

                    let event_data = serde_json::to_string(&result).unwrap_or_default();
                    let sse_event = format!("event: {}\ndata: {}\n\n", event_type, event_data);

                    Some((
                        Ok::<_, actix_web::error::Error>(Bytes::from(sse_event)),
                        (pool, task_id, org_id, is_terminal, start),
                    ))
                }
                Ok(None) => {
                    let sse_event = "event: error\ndata: {\"error\":\"task_not_found\"}\n\n";
                    Some((
                        Ok(Bytes::from(sse_event)),
                        (pool, task_id, org_id, true, start),
                    ))
                }
                Err(e) => {
                    tracing::error!("Failed to fetch task for SSE: {:?}", e);
                    let sse_event = "event: error\ndata: {\"error\":\"internal_error\"}\n\n";
                    Some((
                        Ok(Bytes::from(sse_event)),
                        (pool, task_id, org_id, true, start),
                    ))
                }
            }
        },
    );

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .insert_header((header::CONNECTION, "keep-alive"))
        .streaming(sse_stream)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_is_valid() {
        // Valid tools
        assert!(ToolRegistry::is_valid("getMyFeedbacks"));
        assert!(ToolRegistry::is_valid("getAgentProfile"));
        assert!(ToolRegistry::is_valid("getReputationSummary"));
        assert!(ToolRegistry::is_valid("getTrend"));
        assert!(ToolRegistry::is_valid("getValidationHistory"));
        assert!(ToolRegistry::is_valid("getReputationReport"));

        // Invalid tools
        assert!(!ToolRegistry::is_valid("unknownTool"));
        assert!(!ToolRegistry::is_valid(""));
    }

    #[test]
    fn test_tool_registry_get_cost_display() {
        assert_eq!(
            ToolRegistry::get_cost_display("getMyFeedbacks"),
            Some("0.001 USDC")
        );
        assert_eq!(
            ToolRegistry::get_cost_display("getReputationSummary"),
            Some("0.01 USDC")
        );
        assert_eq!(
            ToolRegistry::get_cost_display("getReputationReport"),
            Some("0.20 USDC")
        );
        assert_eq!(ToolRegistry::get_cost_display("unknownTool"), None);
    }

    #[test]
    fn test_tool_registry_get_cost_micro_usdc() {
        // Tier 0: 0.001 USDC = 1,000 micro-USDC
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getMyFeedbacks"), 1_000);
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getAgentProfile"), 1_000);

        // Tier 1: 0.01 USDC = 10,000 micro-USDC
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getReputationSummary"),
            10_000
        );
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getTrend"), 10_000);
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getValidationHistory"),
            10_000
        );

        // Tier 3: 0.20 USDC = 200,000 micro-USDC
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getReputationReport"),
            200_000
        );

        // Unknown tool returns 0
        assert_eq!(ToolRegistry::get_cost_micro_usdc("unknownTool"), 0);
    }
}
