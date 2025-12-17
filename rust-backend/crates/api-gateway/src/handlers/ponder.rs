//! Ponder indexer status endpoints
//!
//! Provides status information about the blockchain indexer including
//! sync progress per chain and overall health status.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use shared::DbPool;
use utoipa::ToSchema;

/// Chain sync status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChainSyncStatus {
    /// Chain name (e.g., "ethereumSepolia")
    pub chain: String,
    /// Chain ID
    pub chain_id: i64,
    /// Current synced block number
    pub current_block: i64,
    /// Whether the chain is fully synced
    pub is_synced: bool,
    /// Last sync timestamp (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
}

/// Ponder indexer status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PonderStatusResponse {
    /// Overall status of the indexer
    pub status: String,
    /// Ponder schema name
    pub schema: String,
    /// List of chain sync statuses
    pub chains: Vec<ChainSyncStatus>,
    /// Total number of indexed events
    pub total_events: i64,
    /// Last activity timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<String>,
}

/// Error response for Ponder status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PonderStatusError {
    pub error: String,
    pub details: Option<String>,
}

/// Get the active Ponder namespace from the database
async fn get_ponder_namespace(pool: &sqlx::PgPool) -> String {
    // Try to find namespace from _ponder_meta table
    let namespace_result: Result<Option<String>, sqlx::Error> =
        sqlx::query_scalar(r#"SELECT value FROM public._ponder_meta WHERE key = 'app'"#)
            .fetch_optional(pool)
            .await;

    match namespace_result {
        Ok(Some(ns)) => ns,
        Ok(None) | Err(_) => {
            // Fall back to finding namespace from table names
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
                Ok(Some(table_name)) => {
                    // Extract namespace from table name like "0684__Event"
                    table_name.split("__").next().unwrap_or("").to_string()
                }
                _ => String::new(),
            }
        }
    }
}

/// Get Ponder indexer status
///
/// Returns the current sync status of the blockchain indexer,
/// including progress for each configured chain.
#[utoipa::path(
    get,
    path = "/api/v1/ponder/status",
    tag = "Ponder",
    responses(
        (status = 200, description = "Ponder status retrieved successfully", body = PonderStatusResponse),
        (status = 500, description = "Failed to retrieve Ponder status", body = PonderStatusError)
    )
)]
pub async fn get_ponder_status(pool: web::Data<DbPool>) -> impl Responder {
    // Chain configurations matching ponder.config.ts
    let chain_configs: Vec<(&str, i64)> = vec![
        ("ethereumSepolia", 11155111),
        ("baseSepolia", 84532),
        ("lineaSepolia", 59141),
        ("polygonAmoy", 80002),
        ("ethereumMainnet", 1),
        ("baseMainnet", 8453),
        ("lineaMainnet", 59144),
    ];

    let mut chains: Vec<ChainSyncStatus> = Vec::new();
    let mut total_events: i64 = 0;
    let mut last_activity_at: Option<String> = None;
    let schema = "public";

    // Get the active Ponder namespace
    let namespace = get_ponder_namespace(pool.get_ref()).await;

    if namespace.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "status": "initializing",
            "schema": schema,
            "chains": [],
            "total_events": 0,
            "debug_error": "No Ponder namespace found"
        }));
    }

    // Query the namespaced Event table
    let query = format!(
        r#"
        SELECT
            COALESCE(COUNT(*), 0)::bigint as event_count,
            COALESCE(MAX("timestamp")::text, '') as last_timestamp
        FROM public."{namespace}__Event"
        "#,
        namespace = namespace
    );

    let status_result: Result<Option<(i64, String)>, sqlx::Error> =
        sqlx::query_as(&query).fetch_optional(pool.get_ref()).await;

    match status_result {
        Ok(Some((count, timestamp))) => {
            total_events = count;
            if !timestamp.is_empty() {
                last_activity_at = Some(timestamp);
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!("Ponder query error: {}", e);
            return HttpResponse::InternalServerError().json(PonderStatusError {
                error: "Failed to query Ponder status".to_string(),
                details: Some(e.to_string()),
            });
        }
    }

    // Get per-chain checkpoint info from Ponder's Event table
    // Use the same namespaced table
    for (chain_name, chain_id) in &chain_configs {
        let chain_query = format!(
            r#"
            SELECT COALESCE(MAX(block_number), 0)::bigint
            FROM public."{namespace}__Event"
            WHERE chain_id::bigint = $1
            "#,
            namespace = namespace
        );
        let block_result: Result<Option<i64>, sqlx::Error> = sqlx::query_scalar(&chain_query)
            .bind(*chain_id)
            .fetch_optional(pool.get_ref())
            .await;

        match block_result {
            Ok(Some(current_block)) if current_block > 0 => {
                chains.push(ChainSyncStatus {
                    chain: chain_name.to_string(),
                    chain_id: *chain_id,
                    current_block,
                    is_synced: true, // We'll assume synced if we have recent blocks
                    last_sync_at: None,
                });
            }
            Ok(_) => {
                // Chain not yet indexed or no events
                chains.push(ChainSyncStatus {
                    chain: chain_name.to_string(),
                    chain_id: *chain_id,
                    current_block: 0,
                    is_synced: false,
                    last_sync_at: None,
                });
            }
            Err(e) => {
                // Log error but continue with other chains
                tracing::warn!("Failed to get block for chain {}: {}", chain_name, e);
                chains.push(ChainSyncStatus {
                    chain: chain_name.to_string(),
                    chain_id: *chain_id,
                    current_block: 0,
                    is_synced: false,
                    last_sync_at: None,
                });
            }
        }
    }

    // Determine overall status
    let active_chains = chains.iter().filter(|c| c.current_block > 0).count();
    let status = if active_chains == 0 {
        "initializing"
    } else if active_chains == chains.len() {
        "healthy"
    } else {
        "partial"
    };

    HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "schema": schema,
        "namespace": namespace,
        "chains": chains,
        "total_events": total_events,
        "last_activity_at": last_activity_at,
    }))
}

/// Get event counts per chain
///
/// Returns detailed event statistics grouped by chain and event type.
#[utoipa::path(
    get,
    path = "/api/v1/ponder/events",
    tag = "Ponder",
    responses(
        (status = 200, description = "Event statistics retrieved successfully"),
        (status = 500, description = "Failed to retrieve event statistics", body = PonderStatusError)
    )
)]
pub async fn get_ponder_events(pool: web::Data<DbPool>) -> impl Responder {
    // Find the active Ponder namespace first
    let namespace = get_ponder_namespace(pool.get_ref()).await;

    if namespace.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "events": [],
            "total_types": 0
        }));
    }

    // Get event counts grouped by chain and type using namespaced table
    let query = format!(
        r#"
        SELECT
            "chainId"::bigint,
            "eventType",
            COUNT(*)::bigint as count
        FROM public."{namespace}__Event"
        GROUP BY "chainId", "eventType"
        ORDER BY "chainId", count DESC
        "#,
        namespace = namespace
    );

    let events_result: Result<Vec<(i64, String, i64)>, sqlx::Error> =
        sqlx::query_as(&query).fetch_all(pool.get_ref()).await;

    match events_result {
        Ok(rows) => {
            let events: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(chain_id, event_name, count)| {
                    serde_json::json!({
                        "chain_id": chain_id,
                        "event_name": event_name,
                        "count": count
                    })
                })
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "events": events,
                "total_types": events.len()
            }))
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("does not exist") {
                return HttpResponse::Ok().json(serde_json::json!({
                    "events": [],
                    "total_types": 0
                }));
            }
            HttpResponse::InternalServerError().json(PonderStatusError {
                error: "Failed to query event statistics".to_string(),
                details: Some(e.to_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ponder_status_response_serialization() {
        let response = PonderStatusResponse {
            status: "healthy".to_string(),
            schema: "ponder".to_string(),
            chains: vec![ChainSyncStatus {
                chain: "ethereumSepolia".to_string(),
                chain_id: 11155111,
                current_block: 12345678,
                is_synced: true,
                last_sync_at: None,
            }],
            total_events: 1000,
            last_activity_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(json.contains("healthy"));
        assert!(json.contains("ethereumSepolia"));
        assert!(json.contains("11155111"));
    }
}
