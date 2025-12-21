//! Query Executor Service
//!
//! Implements the query logic for A2A Protocol tools.
//! Queries are executed against the Ponder-indexed blockchain data.
//!
//! ## Tool Tiers
//!
//! - **Tier 0** (Raw data): getMyFeedbacks, getAgentProfile
//! - **Tier 1** (Aggregated): getReputationSummary, getTrend, getValidationHistory
//! - **Tier 2** (Analysis): Complex analysis - future
//! - **Tier 3** (AI-powered): getReputationReport - future

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shared::DbPool;

/// Cost per tool tier in USDC
pub mod costs {
    pub const TIER_0: f64 = 0.001; // Raw data
    pub const TIER_1: f64 = 0.01; // Aggregated
    pub const TIER_2: f64 = 0.10; // Analysis (future)
    pub const TIER_3: f64 = 0.20; // AI-powered (future)
}

/// Query executor service
pub struct QueryExecutor {
    pool: DbPool,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Execute a query tool
    ///
    /// Returns the result and the cost in USDC
    pub async fn execute(
        &self,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> Result<(serde_json::Value, f64)> {
        match tool {
            // Tier 0: Raw data queries
            "getMyFeedbacks" => {
                let result = self.get_my_feedbacks(arguments).await?;
                Ok((result, costs::TIER_0))
            }
            "getAgentProfile" => {
                let result = self.get_agent_profile(arguments).await?;
                Ok((result, costs::TIER_0))
            }
            // Tier 1: Aggregated queries
            "getReputationSummary" => {
                let result = self.get_reputation_summary(arguments).await?;
                Ok((result, costs::TIER_1))
            }
            "getTrend" => {
                let result = self.get_trend(arguments).await?;
                Ok((result, costs::TIER_1))
            }
            "getValidationHistory" => {
                let result = self.get_validation_history(arguments).await?;
                Ok((result, costs::TIER_1))
            }
            // Tier 3: AI-powered (stub for now)
            "getReputationReport" => {
                let result = self.get_reputation_report(arguments).await?;
                Ok((result, costs::TIER_3))
            }
            _ => anyhow::bail!("Unknown tool: {}", tool),
        }
    }

    // =========================================================================
    // Tier 0: Raw Data Queries
    // =========================================================================

    /// Get feedbacks for an agent
    ///
    /// Arguments:
    /// - agentId: The agent ID (required)
    /// - limit: Maximum number of feedbacks (default: 50)
    /// - offset: Pagination offset (default: 0)
    async fn get_my_feedbacks(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetFeedbacksParams =
            serde_json::from_value(args.clone()).context("Invalid arguments for getMyFeedbacks")?;

        let limit = params.limit.unwrap_or(50).min(100);
        let offset = params.offset.unwrap_or(0);

        let feedbacks = sqlx::query_as::<_, FeedbackRow>(
            r#"
            SELECT
                id,
                chain_id,
                block_number,
                transaction_hash,
                agent_id,
                client_address,
                feedback_index,
                score,
                tag1,
                tag2,
                file_uri,
                file_hash,
                timestamp,
                created_at
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(params.agent_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch feedbacks")?;

        // Count total feedbacks for this agent
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
            "#,
        )
        .bind(params.agent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count feedbacks")?;

        Ok(serde_json::json!({
            "feedbacks": feedbacks,
            "total": total.0,
            "limit": limit,
            "offset": offset
        }))
    }

    /// Get agent profile data
    ///
    /// Arguments:
    /// - agentId: The agent ID (required)
    async fn get_agent_profile(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetAgentParams = serde_json::from_value(args.clone())
            .context("Invalid arguments for getAgentProfile")?;

        // Get the AgentCreated event
        let created_event = sqlx::query_as::<_, AgentCreatedRow>(
            r#"
            SELECT
                id,
                chain_id,
                block_number,
                transaction_hash,
                agent_id,
                owner,
                token_uri,
                timestamp,
                created_at
            FROM ponder_events
            WHERE registry = 'identity'
              AND event_type = 'AgentCreated'
              AND agent_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(params.agent_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch agent created event")?;

        // Get latest metadata updates
        let metadata = sqlx::query_as::<_, MetadataRow>(
            r#"
            SELECT DISTINCT ON (metadata_key)
                metadata_key,
                metadata_value,
                timestamp
            FROM ponder_events
            WHERE registry = 'identity'
              AND event_type = 'MetadataUpdated'
              AND agent_id = $1
            ORDER BY metadata_key, timestamp DESC
            "#,
        )
        .bind(params.agent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch metadata")?;

        // Get current owner (latest OwnershipTransferred event)
        let current_owner = sqlx::query_scalar::<_, String>(
            r#"
            SELECT owner
            FROM ponder_events
            WHERE registry = 'identity'
              AND (event_type = 'AgentCreated' OR event_type = 'OwnershipTransferred')
              AND agent_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(params.agent_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch current owner")?;

        match created_event {
            Some(event) => Ok(serde_json::json!({
                "agentId": event.agent_id,
                "chainId": event.chain_id,
                "owner": current_owner.unwrap_or(event.owner),
                "tokenUri": event.token_uri,
                "createdAt": event.timestamp,
                "createdTx": event.transaction_hash,
                "metadata": metadata.into_iter().map(|m| serde_json::json!({
                    "key": m.metadata_key,
                    "value": m.metadata_value,
                    "updatedAt": m.timestamp
                })).collect::<Vec<_>>()
            })),
            None => Ok(serde_json::json!({
                "error": "Agent not found",
                "agentId": params.agent_id
            })),
        }
    }

    // =========================================================================
    // Tier 1: Aggregated Queries
    // =========================================================================

    /// Get reputation summary for an agent
    ///
    /// Arguments:
    /// - agentId: The agent ID (required)
    async fn get_reputation_summary(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetAgentParams = serde_json::from_value(args.clone())
            .context("Invalid arguments for getReputationSummary")?;

        // Aggregate reputation statistics
        let stats = sqlx::query_as::<_, ReputationStatsRow>(
            r#"
            SELECT
                COUNT(*) AS total_feedbacks,
                COALESCE(AVG(score), 0) AS avg_score,
                COALESCE(MIN(score), 0) AS min_score,
                COALESCE(MAX(score), 0) AS max_score,
                COALESCE(STDDEV(score), 0) AS stddev_score,
                COUNT(DISTINCT client_address) AS unique_clients,
                MIN(timestamp) AS first_feedback,
                MAX(timestamp) AS last_feedback
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
            "#,
        )
        .bind(params.agent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch reputation stats")?;

        // Get tag distribution
        let tags = sqlx::query_as::<_, TagCountRow>(
            r#"
            SELECT tag1 AS tag, COUNT(*) AS count
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
              AND tag1 IS NOT NULL
            GROUP BY tag1
            UNION ALL
            SELECT tag2 AS tag, COUNT(*) AS count
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
              AND tag2 IS NOT NULL
            GROUP BY tag2
            "#,
        )
        .bind(params.agent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch tag distribution")?;

        // Calculate score distribution (buckets: 1-20, 21-40, 41-60, 61-80, 81-100)
        let score_distribution = sqlx::query_as::<_, ScoreDistributionRow>(
            r#"
            SELECT
                CASE
                    WHEN score <= 20 THEN '1-20'
                    WHEN score <= 40 THEN '21-40'
                    WHEN score <= 60 THEN '41-60'
                    WHEN score <= 80 THEN '61-80'
                    ELSE '81-100'
                END AS bucket,
                COUNT(*) AS count
            FROM ponder_events
            WHERE registry = 'reputation'
              AND event_type = 'NewFeedback'
              AND agent_id = $1
            GROUP BY bucket
            ORDER BY bucket
            "#,
        )
        .bind(params.agent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch score distribution")?;

        Ok(serde_json::json!({
            "agentId": params.agent_id,
            "statistics": {
                "totalFeedbacks": stats.total_feedbacks,
                "averageScore": stats.avg_score,
                "minScore": stats.min_score,
                "maxScore": stats.max_score,
                "stdDevScore": stats.stddev_score,
                "uniqueClients": stats.unique_clients,
                "firstFeedback": stats.first_feedback,
                "lastFeedback": stats.last_feedback
            },
            "tagDistribution": tags.into_iter().map(|t| serde_json::json!({
                "tag": t.tag,
                "count": t.count
            })).collect::<Vec<_>>(),
            "scoreDistribution": score_distribution.into_iter().map(|s| serde_json::json!({
                "bucket": s.bucket,
                "count": s.count
            })).collect::<Vec<_>>()
        }))
    }

    /// Get reputation trend for an agent
    ///
    /// Arguments:
    /// - agentId: The agent ID (required)
    /// - period: Grouping period ("day", "week", "month") - default: "day"
    /// - limit: Number of periods (default: 30)
    async fn get_trend(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetTrendParams =
            serde_json::from_value(args.clone()).context("Invalid arguments for getTrend")?;

        let period = params.period.as_deref().unwrap_or("day");
        let limit = params.limit.unwrap_or(30).min(365);

        // Calculate lookback days based on period
        let lookback_days = match period {
            "week" => limit as i32 * 7,
            "month" => limit as i32 * 30,
            _ => limit as i32,
        };

        // Get trend data grouped by period
        let trend = sqlx::query_as::<_, TrendRow>(
            r#"
            WITH time_buckets AS (
                SELECT
                    date_trunc($2, to_timestamp(timestamp)) AS period_start,
                    COUNT(*) AS feedback_count,
                    COALESCE(AVG(score), 0) AS avg_score,
                    COUNT(DISTINCT client_address) AS unique_clients
                FROM ponder_events
                WHERE registry = 'reputation'
                  AND event_type = 'NewFeedback'
                  AND agent_id = $1
                  AND timestamp >= EXTRACT(EPOCH FROM NOW() - ($3::text || ' days')::INTERVAL)
                GROUP BY period_start
                ORDER BY period_start DESC
                LIMIT $4
            )
            SELECT
                period_start,
                feedback_count,
                avg_score,
                unique_clients
            FROM time_buckets
            ORDER BY period_start ASC
            "#,
        )
        .bind(params.agent_id)
        .bind(period)
        .bind(lookback_days.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch trend data")?;

        Ok(serde_json::json!({
            "agentId": params.agent_id,
            "period": period,
            "trend": trend.into_iter().map(|t| serde_json::json!({
                "periodStart": t.period_start,
                "feedbackCount": t.feedback_count,
                "avgScore": t.avg_score,
                "uniqueClients": t.unique_clients
            })).collect::<Vec<_>>()
        }))
    }

    /// Get validation history for an agent
    ///
    /// Arguments:
    /// - agentId: The agent ID (required)
    /// - limit: Maximum number of validations (default: 50)
    /// - offset: Pagination offset (default: 0)
    async fn get_validation_history(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetValidationParams = serde_json::from_value(args.clone())
            .context("Invalid arguments for getValidationHistory")?;

        let limit = params.limit.unwrap_or(50).min(100);
        let offset = params.offset.unwrap_or(0);

        let validations = sqlx::query_as::<_, ValidationRow>(
            r#"
            SELECT
                id,
                chain_id,
                block_number,
                transaction_hash,
                agent_id,
                validator_address,
                request_hash,
                response,
                response_uri,
                response_hash,
                tag,
                timestamp,
                created_at
            FROM ponder_events
            WHERE registry = 'validation'
              AND agent_id = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(params.agent_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch validations")?;

        // Count total validations
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM ponder_events
            WHERE registry = 'validation'
              AND agent_id = $1
            "#,
        )
        .bind(params.agent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count validations")?;

        Ok(serde_json::json!({
            "validations": validations,
            "total": total.0,
            "limit": limit,
            "offset": offset
        }))
    }

    // =========================================================================
    // Tier 3: AI-powered (Stub)
    // =========================================================================

    /// Get AI-powered reputation report (stub for now)
    async fn get_reputation_report(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let params: GetAgentParams = serde_json::from_value(args.clone())
            .context("Invalid arguments for getReputationReport")?;

        // For now, return a stub response
        // TODO: Integrate with LLM service for actual AI analysis
        Ok(serde_json::json!({
            "agentId": params.agent_id,
            "status": "not_implemented",
            "message": "AI-powered reputation reports will be available in a future release",
            "note": "Use getReputationSummary for aggregated statistics"
        }))
    }
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAgentParams {
    agent_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetFeedbacksParams {
    agent_id: i64,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTrendParams {
    agent_id: i64,
    #[serde(default)]
    period: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetValidationParams {
    agent_id: i64,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

// ============================================================================
// Database Row Types
// ============================================================================

#[derive(Debug, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeedbackRow {
    id: String,
    chain_id: i32,
    block_number: i64,
    transaction_hash: String,
    agent_id: Option<i64>,
    client_address: Option<String>,
    feedback_index: Option<i64>,
    score: Option<i32>,
    tag1: Option<String>,
    tag2: Option<String>,
    file_uri: Option<String>,
    file_hash: Option<String>,
    timestamp: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct AgentCreatedRow {
    #[allow(dead_code)]
    id: String,
    chain_id: i32,
    #[allow(dead_code)]
    block_number: i64,
    transaction_hash: String,
    agent_id: Option<i64>,
    owner: String,
    token_uri: Option<String>,
    timestamp: i64,
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct MetadataRow {
    metadata_key: Option<String>,
    metadata_value: Option<String>,
    timestamp: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ReputationStatsRow {
    total_feedbacks: i64,
    avg_score: f64,
    min_score: i32,
    max_score: i32,
    stddev_score: f64,
    unique_clients: i64,
    first_feedback: Option<i64>,
    last_feedback: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct TagCountRow {
    tag: Option<String>,
    count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ScoreDistributionRow {
    bucket: String,
    count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct TrendRow {
    period_start: chrono::DateTime<chrono::Utc>,
    feedback_count: i64,
    avg_score: f64,
    unique_clients: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidationRow {
    id: String,
    chain_id: i32,
    block_number: i64,
    transaction_hash: String,
    agent_id: Option<i64>,
    validator_address: Option<String>,
    request_hash: Option<String>,
    response: Option<i32>,
    response_uri: Option<String>,
    response_hash: Option<String>,
    tag: Option<String>,
    timestamp: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_costs() {
        assert_eq!(costs::TIER_0, 0.001);
        assert_eq!(costs::TIER_1, 0.01);
        assert_eq!(costs::TIER_2, 0.10);
        assert_eq!(costs::TIER_3, 0.20);
    }

    #[test]
    fn test_parse_get_agent_params() {
        let json = serde_json::json!({"agentId": 42});
        let params: GetAgentParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.agent_id, 42);
    }

    #[test]
    fn test_parse_get_feedbacks_params() {
        let json = serde_json::json!({"agentId": 42, "limit": 10, "offset": 5});
        let params: GetFeedbacksParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.agent_id, 42);
        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(5));
    }

    #[test]
    fn test_parse_get_trend_params() {
        let json = serde_json::json!({"agentId": 42, "period": "week", "limit": 12});
        let params: GetTrendParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.agent_id, 42);
        assert_eq!(params.period, Some("week".to_string()));
        assert_eq!(params.limit, Some(12));
    }
}
