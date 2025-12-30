//! Event handlers for querying blockchain events
//!
//! Provides endpoints to query and filter events indexed by Ponder.

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use shared::DbPool;
use utoipa::{IntoParams, ToSchema};

use crate::handlers::helpers::extract_user_id_or_unauthorized;
use crate::middleware::get_verified_organization_id;
use crate::models::{ErrorResponse, PaginationMeta};

/// Query parameters for listing events
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventsQuery {
    /// Page number (1-indexed)
    #[param(minimum = 1)]
    pub page: Option<i64>,
    /// Items per page (default: 20, max: 100)
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    /// Filter by chain ID
    #[serde(rename = "chainId")]
    pub chain_id: Option<i64>,
    /// Filter by registry type (identity, reputation, validation)
    pub registry: Option<String>,
    /// Filter by event type (e.g., AgentCreated, ReputationUpdated)
    #[serde(rename = "eventType")]
    pub event_type: Option<String>,
    /// Filter by agent ID (hex address)
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
    /// Search in event data
    pub search: Option<String>,
}

/// Event response item
#[derive(Debug, Serialize, ToSchema)]
pub struct EventResponse {
    /// Event ID
    pub id: String,
    /// Chain ID where event occurred
    pub chain_id: i64,
    /// Block number
    pub block_number: i64,
    /// Transaction hash
    pub tx_hash: String,
    /// Log index within transaction
    pub log_index: i64,
    /// Event type (e.g., AgentCreated)
    pub event_type: String,
    /// Registry type (identity, reputation, validation)
    pub registry: String,
    /// Agent ID involved (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Event timestamp
    pub timestamp: i64,
    /// Event data as JSON
    pub data: serde_json::Value,
}

/// Paginated events response
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedEventsResponse {
    /// List of events
    pub data: Vec<EventResponse>,
    /// Pagination metadata
    pub pagination: PaginationMeta,
}

/// Get the active Ponder namespace from the database
async fn get_ponder_namespace(pool: &sqlx::PgPool) -> String {
    let namespace_result: Result<Option<String>, sqlx::Error> =
        sqlx::query_scalar(r#"SELECT value FROM public._ponder_meta WHERE key = 'app'"#)
            .fetch_optional(pool)
            .await;

    match namespace_result {
        Ok(Some(ns)) => ns,
        Ok(None) | Err(_) => {
            let table_result: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
                r#"
                SELECT tablename::text FROM pg_tables
                WHERE schemaname = 'public' AND tablename LIKE '%__Event'
                AND tablename NOT LIKE '%_reorg__%'
                ORDER BY tablename DESC LIMIT 1
                "#,
            )
            .fetch_optional(pool)
            .await;

            match table_result {
                Ok(Some(table_name)) => table_name.split("__").next().unwrap_or("").to_string(),
                _ => String::new(),
            }
        }
    }
}

/// List blockchain events
///
/// Returns paginated list of indexed blockchain events with optional filters.
#[utoipa::path(
    get,
    path = "/api/v1/events",
    tag = "Events",
    params(EventsQuery),
    security(("bearer_auth" = []), ("organization_id" = [])),
    responses(
        (status = 200, description = "List of events", body = PaginatedEventsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_events(
    pool: web::Data<DbPool>,
    req_http: HttpRequest,
    query: web::Query<EventsQuery>,
) -> impl Responder {
    // Get authenticated user_id
    let user_id = match extract_user_id_or_unauthorized(&req_http) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Verify organization membership
    let _organization_id = match get_verified_organization_id(&req_http, &pool, &user_id).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    // Parse pagination
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // Get Ponder namespace
    let namespace = get_ponder_namespace(pool.get_ref()).await;

    if namespace.is_empty() {
        return HttpResponse::Ok().json(PaginatedEventsResponse {
            data: vec![],
            pagination: PaginationMeta::new(0, limit, offset),
        });
    }

    // Build dynamic WHERE clause
    let mut conditions: Vec<String> = vec![];
    let mut param_index = 1;

    if query.chain_id.is_some() {
        conditions.push(format!(r#""chainId"::bigint = ${}"#, param_index));
        param_index += 1;
    }

    if query.registry.is_some() {
        conditions.push(format!(r#""registry" = ${}"#, param_index));
        param_index += 1;
    }

    if query.event_type.is_some() {
        conditions.push(format!(r#""eventType" = ${}"#, param_index));
        param_index += 1;
    }

    if query.agent_id.is_some() {
        conditions.push(format!(r#""agentId" = ${}"#, param_index));
        param_index += 1;
    }

    if query.search.is_some() {
        conditions.push(format!(
            r#"("eventType" ILIKE ${} OR "agentId" ILIKE ${} OR "registry" ILIKE ${})"#,
            param_index,
            param_index + 1,
            param_index + 2
        ));
        param_index += 3;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count query
    let count_query = format!(
        r#"SELECT COUNT(*)::bigint FROM public."{namespace}__Event" {where_clause}"#,
        namespace = namespace,
        where_clause = where_clause
    );

    // Data query
    let data_query = format!(
        r#"
        SELECT
            id,
            "chainId"::bigint as chain_id,
            block_number::bigint,
            COALESCE("transactionHash", '') as tx_hash,
            COALESCE(log_index, 0)::bigint as log_index,
            "eventType",
            COALESCE("registry", 'unknown') as registry,
            "agentId",
            "timestamp"::bigint,
            COALESCE("eventData", '{{}}'::jsonb) as event_data
        FROM public."{namespace}__Event"
        {where_clause}
        ORDER BY "timestamp" DESC, block_number DESC
        LIMIT ${limit_param} OFFSET ${offset_param}
        "#,
        namespace = namespace,
        where_clause = where_clause,
        limit_param = param_index,
        offset_param = param_index + 1
    );

    // Build queries with dynamic bindings
    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query);
    let mut data_builder = sqlx::query_as::<
        _,
        (
            String,
            i64,
            i64,
            String,
            i64,
            String,
            String,
            Option<String>,
            i64,
            serde_json::Value,
        ),
    >(&data_query);

    // Bind parameters in order
    if let Some(chain_id) = query.chain_id {
        count_builder = count_builder.bind(chain_id);
        data_builder = data_builder.bind(chain_id);
    }

    if let Some(ref registry) = query.registry {
        count_builder = count_builder.bind(registry);
        data_builder = data_builder.bind(registry);
    }

    if let Some(ref event_type) = query.event_type {
        count_builder = count_builder.bind(event_type);
        data_builder = data_builder.bind(event_type);
    }

    if let Some(ref agent_id) = query.agent_id {
        count_builder = count_builder.bind(agent_id);
        data_builder = data_builder.bind(agent_id);
    }

    // Create search pattern outside the if block so it lives long enough
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    if let Some(ref pattern) = search_pattern {
        count_builder = count_builder
            .bind(pattern.clone())
            .bind(pattern.clone())
            .bind(pattern.clone());
        data_builder = data_builder
            .bind(pattern.clone())
            .bind(pattern.clone())
            .bind(pattern.clone());
    }

    // Bind limit and offset
    data_builder = data_builder.bind(limit).bind(offset);

    // Execute count query
    let total = match count_builder.fetch_one(pool.get_ref()).await {
        Ok(count) => count,
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("does not exist") {
                return HttpResponse::Ok().json(PaginatedEventsResponse {
                    data: vec![],
                    pagination: PaginationMeta::new(0, limit, offset),
                });
            }
            tracing::error!("Failed to count events: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to query events",
            ));
        }
    };

    // Execute data query
    let events = match data_builder.fetch_all(pool.get_ref()).await {
        Ok(rows) => rows
            .into_iter()
            .map(
                |(
                    id,
                    chain_id,
                    block_number,
                    tx_hash,
                    log_index,
                    event_type,
                    registry,
                    agent_id,
                    timestamp,
                    data,
                )| {
                    EventResponse {
                        id,
                        chain_id,
                        block_number,
                        tx_hash,
                        log_index,
                        event_type,
                        registry,
                        agent_id,
                        timestamp,
                        data,
                    }
                },
            )
            .collect(),
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("does not exist") {
                return HttpResponse::Ok().json(PaginatedEventsResponse {
                    data: vec![],
                    pagination: PaginationMeta::new(0, limit, offset),
                });
            }
            tracing::error!("Failed to fetch events: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(
                "internal_error",
                "Failed to query events",
            ));
        }
    };

    HttpResponse::Ok().json(PaginatedEventsResponse {
        data: events,
        pagination: PaginationMeta::new(total, limit, offset),
    })
}
