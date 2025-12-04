# Analytics API Integration Guide

## Overview

This guide provides implementation recommendations for exposing analytics materialized views via REST API endpoints. Designed for backend developers integrating with the Result Logger Analytics system.

## Recommended API Endpoints

### Base URL
```
/api/v1/analytics
```

### Authentication
- **Layer 1**: API Key (recommended for programmatic access)
- **Layer 2**: JWT Token (recommended for dashboard users)
- **Rate Limiting**: 100 requests/hour (Starter), 500/hour (Pro), 2000/hour (Enterprise)

## Endpoint Specifications

### 1. Success Rate Endpoints

#### 1.1 GET /api/v1/analytics/actions/success-rate

**Purpose**: Get success rates by action type over a time period

**Query Parameters**:
- `days` (integer, optional): Number of days to analyze (default: 7, max: 90)
- `action_type` (string, optional): Filter by action type (telegram, rest, mcp)

**Request Example**:
```bash
curl -H "Authorization: Bearer $JWT_TOKEN" \
  "https://api.agentauri.ai/api/v1/analytics/actions/success-rate?days=7"
```

**Response Schema** (200 OK):
```json
{
  "data": [
    {
      "action_type": "telegram",
      "total_executions": 1250,
      "successes": 1180,
      "failures": 65,
      "retrying": 5,
      "success_rate": 94.40,
      "failure_rate": 5.20,
      "total_retries": 23,
      "avg_duration_ms": 245.50
    },
    {
      "action_type": "rest",
      "total_executions": 850,
      "successes": 820,
      "failures": 30,
      "retrying": 0,
      "success_rate": 96.47,
      "failure_rate": 3.53,
      "total_retries": 12,
      "avg_duration_ms": 380.25
    }
  ],
  "metadata": {
    "days": 7,
    "generated_at": "2025-11-30T12:00:00Z"
  }
}
```

**SQL Query** (from `analytics.sql`):
```sql
SELECT
    action_type,
    SUM(execution_count) AS total_executions,
    SUM(success_count) AS successes,
    SUM(failure_count) AS failures,
    SUM(retrying_count) AS retrying,
    ROUND(100.0 * SUM(success_count) / NULLIF(SUM(execution_count), 0), 2) AS success_rate,
    ROUND(100.0 * SUM(failure_count) / NULLIF(SUM(execution_count), 0), 2) AS failure_rate,
    SUM(total_retries) AS total_retries,
    ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '$1 days'
  AND ($2::TEXT IS NULL OR action_type = $2)
GROUP BY action_type
ORDER BY total_executions DESC;
```

**Rust Implementation**:
```rust
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct SuccessRateQuery {
    days: Option<i32>,
    action_type: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ActionSuccessRate {
    action_type: String,
    total_executions: i64,
    successes: i64,
    failures: i64,
    retrying: i64,
    success_rate: f64,
    failure_rate: f64,
    total_retries: i64,
    avg_duration_ms: f64,
}

pub async fn get_success_rate(
    pool: web::Data<PgPool>,
    query: web::Query<SuccessRateQuery>,
) -> Result<HttpResponse, Error> {
    let days = query.days.unwrap_or(7).min(90);
    let action_type = query.action_type.as_deref();

    let results = sqlx::query_as!(
        ActionSuccessRate,
        r#"
        SELECT
            action_type,
            SUM(execution_count) AS total_executions,
            SUM(success_count) AS successes,
            SUM(failure_count) AS failures,
            SUM(retrying_count) AS retrying,
            ROUND(100.0 * SUM(success_count) / NULLIF(SUM(execution_count), 0), 2) AS "success_rate!",
            ROUND(100.0 * SUM(failure_count) / NULLIF(SUM(execution_count), 0), 2) AS "failure_rate!",
            SUM(total_retries) AS total_retries,
            ROUND(AVG(avg_duration_ms), 2) AS "avg_duration_ms!"
        FROM action_metrics_hourly
        WHERE hour > NOW() - INTERVAL '1 day' * $1
          AND ($2::TEXT IS NULL OR action_type = $2)
        GROUP BY action_type
        ORDER BY total_executions DESC
        "#,
        days,
        action_type
    )
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(json!({
        "data": results,
        "metadata": {
            "days": days,
            "generated_at": chrono::Utc::now()
        }
    })))
}
```

**Error Responses**:
- `400 Bad Request`: Invalid query parameters
- `401 Unauthorized`: Missing or invalid authentication
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Database error

---

#### 1.2 GET /api/v1/analytics/actions/hourly

**Purpose**: Get hourly time-series data for dashboard graphs

**Query Parameters**:
- `hours` (integer, optional): Number of hours to retrieve (default: 24, max: 168)

**Response Schema**:
```json
{
  "data": [
    {
      "hour": "2025-11-30T11:00:00Z",
      "total_executions": 125,
      "successes": 118,
      "failures": 7,
      "success_rate": 94.40,
      "avg_duration_ms": 250.30,
      "p95_duration_ms": 480.00
    },
    {
      "hour": "2025-11-30T10:00:00Z",
      "total_executions": 130,
      "successes": 125,
      "failures": 5,
      "success_rate": 96.15,
      "avg_duration_ms": 240.50,
      "p95_duration_ms": 450.00
    }
  ],
  "metadata": {
    "hours": 24,
    "data_points": 24
  }
}
```

**Use Case**: Line graphs showing success rate over time

---

### 2. Performance Endpoints

#### 2.1 GET /api/v1/analytics/triggers/slowest

**Purpose**: Identify slowest triggers for optimization

**Query Parameters**:
- `limit` (integer, optional): Number of results (default: 10, max: 100)
- `metric` (string, optional): Sort by `avg_duration_ms`, `p95_duration_ms`, or `max_duration_ms` (default: p95_duration_ms)

**Response Schema**:
```json
{
  "data": [
    {
      "trigger_id": "trigger_abc123",
      "trigger_name": "High Score Alert",
      "organization_id": "org_xyz789",
      "total_executions": 450,
      "success_rate": 92.50,
      "avg_duration_ms": 1250.00,
      "p95_duration_ms": 3500.00,
      "max_duration_ms": 8000,
      "total_retries": 15,
      "last_execution_at": "2025-11-30T11:45:00Z"
    }
  ],
  "metadata": {
    "limit": 10,
    "sort_by": "p95_duration_ms"
  }
}
```

**SQL Query**:
```sql
SELECT
    tps.trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    tps.total_executions,
    tps.success_rate,
    tps.avg_duration_ms,
    tps.p95_duration_ms,
    tps.max_duration_ms,
    tps.total_retries,
    tps.last_execution_at
FROM trigger_performance_summary tps
LEFT JOIN triggers t ON t.id = tps.trigger_id
ORDER BY tps.p95_duration_ms DESC NULLS LAST
LIMIT $1;
```

**Authorization**: Only return triggers belonging to user's organization(s)

```rust
// Add organization filter
WHERE t.organization_id = ANY($2::TEXT[])
```

---

#### 2.2 GET /api/v1/analytics/triggers/{trigger_id}/stats

**Purpose**: Get detailed statistics for a specific trigger

**Path Parameters**:
- `trigger_id` (string, required): Trigger ID

**Response Schema**:
```json
{
  "data": {
    "trigger_id": "trigger_abc123",
    "trigger_name": "High Score Alert",
    "organization_id": "org_xyz789",
    "enabled": true,
    "total_executions": 450,
    "success_count": 416,
    "failure_count": 34,
    "retrying_count": 0,
    "success_rate": 92.44,
    "avg_duration_ms": 1250.00,
    "p50_duration_ms": 980.00,
    "p95_duration_ms": 3500.00,
    "p99_duration_ms": 5200.00,
    "max_duration_ms": 8000,
    "total_retries": 15,
    "avg_retries": 0.03,
    "max_retries": 3,
    "first_execution_at": "2025-11-01T10:00:00Z",
    "last_execution_at": "2025-11-30T11:45:00Z",
    "hours_active": 701.75
  }
}
```

---

### 3. Failure Analysis Endpoints

#### 3.1 GET /api/v1/analytics/failures/by-trigger

**Purpose**: Identify triggers with recurring errors

**Query Parameters**:
- `hours` (integer, optional): Time window in hours (default: 24, max: 168)
- `limit` (integer, optional): Number of results (default: 10, max: 100)

**Response Schema**:
```json
{
  "data": [
    {
      "trigger_id": "trigger_abc123",
      "trigger_name": "High Score Alert",
      "error_count": 15,
      "unique_error_types": 3,
      "last_error_at": "2025-11-30T11:45:00Z",
      "first_error_at": "2025-11-30T02:00:00Z",
      "avg_retry_count": 1.8,
      "max_retry_count": 3
    }
  ],
  "metadata": {
    "hours": 24,
    "total_failures": 42
  }
}
```

---

#### 3.2 GET /api/v1/analytics/failures/top-errors

**Purpose**: Identify most common error messages for bug prioritization

**Query Parameters**:
- `hours` (integer, optional): Time window (default: 24, max: 168)
- `limit` (integer, optional): Number of errors (default: 10, max: 50)

**Response Schema**:
```json
{
  "data": [
    {
      "error_message": "Connection timeout to Telegram API",
      "occurrence_count": 25,
      "affected_triggers": 8,
      "first_seen": "2025-11-30T08:00:00Z",
      "last_seen": "2025-11-30T11:45:00Z",
      "avg_retry_count": 2.1,
      "action_types": ["telegram"]
    },
    {
      "error_message": "Webhook endpoint returned 404",
      "occurrence_count": 12,
      "affected_triggers": 3,
      "first_seen": "2025-11-30T09:30:00Z",
      "last_seen": "2025-11-30T11:20:00Z",
      "avg_retry_count": 1.5,
      "action_types": ["rest"]
    }
  ]
}
```

---

#### 3.3 GET /api/v1/analytics/failures/recent

**Purpose**: Get detailed recent failures for debugging

**Query Parameters**:
- `limit` (integer, optional): Number of failures (default: 20, max: 100)
- `action_type` (string, optional): Filter by action type
- `trigger_id` (string, optional): Filter by trigger

**Response Schema**:
```json
{
  "data": [
    {
      "id": "result_xyz789",
      "trigger_id": "trigger_abc123",
      "trigger_name": "High Score Alert",
      "action_type": "telegram",
      "error_message": "Connection timeout",
      "retry_count": 2,
      "executed_at": "2025-11-30T11:45:00Z",
      "duration_ms": 5000,
      "event_id": "event_123",
      "organization_id": "org_xyz789"
    }
  ],
  "pagination": {
    "limit": 20,
    "total": 42
  }
}
```

---

### 4. Trigger Health Endpoints

#### 4.1 GET /api/v1/analytics/triggers/health

**Purpose**: Dashboard view of trigger health status

**Query Parameters**:
- `threshold` (integer, optional): Success rate threshold % (default: 80, range: 0-100)
- `organization_id` (string, optional): Filter by organization (admin only)

**Response Schema**:
```json
{
  "data": [
    {
      "trigger_id": "trigger_abc123",
      "trigger_name": "High Score Alert",
      "organization_id": "org_xyz789",
      "enabled": true,
      "total_executions": 450,
      "success_count": 350,
      "failure_count": 100,
      "success_rate": 77.78,
      "total_retries": 45,
      "last_execution_at": "2025-11-30T11:45:00Z",
      "health_status": "WARNING"
    }
  ],
  "summary": {
    "critical_triggers": 2,
    "warning_triggers": 5,
    "ok_triggers": 143
  }
}
```

**Health Status Logic**:
- `CRITICAL`: success_rate < 50%
- `WARNING`: success_rate < 80%
- `OK`: success_rate >= 80%

---

#### 4.2 GET /api/v1/analytics/triggers/idle

**Purpose**: Find unused triggers that can be disabled

**Query Parameters**:
- `days` (integer, optional): Idle threshold in days (default: 7, max: 90)

**Response Schema**:
```json
{
  "data": [
    {
      "trigger_id": "trigger_old123",
      "trigger_name": "Unused Alert",
      "organization_id": "org_xyz789",
      "enabled": true,
      "created_at": "2025-01-15T10:00:00Z",
      "last_execution_at": "2025-10-20T14:30:00Z",
      "days_idle": 41
    }
  ],
  "metadata": {
    "idle_threshold_days": 7,
    "total_idle_triggers": 12
  }
}
```

---

### 5. System Health & Monitoring

#### 5.1 GET /api/v1/analytics/health/summary

**Purpose**: Single endpoint for system health dashboard

**Response Schema**:
```json
{
  "data": {
    "time_window": "last_24_hours",
    "total_executions": 2850,
    "total_successes": 2685,
    "total_failures": 165,
    "success_rate": 94.21,
    "avg_duration_ms": 320.50,
    "p95_duration_ms": 850.00,
    "p99_duration_ms": 1500.00,
    "total_retries": 58,
    "hours_with_data": 24
  },
  "status": "healthy",
  "timestamp": "2025-11-30T12:00:00Z"
}
```

**Status Logic**:
- `healthy`: success_rate >= 95%
- `degraded`: success_rate >= 90%
- `unhealthy`: success_rate < 90%

---

#### 5.2 GET /api/v1/analytics/meta/refresh-status

**Purpose**: Monitor materialized view freshness

**Response Schema**:
```json
{
  "data": [
    {
      "view_name": "action_metrics_hourly",
      "row_count": 6480,
      "size_bytes": 1300000,
      "last_refresh": "2025-11-30T11:55:00Z",
      "age_minutes": 5
    },
    {
      "view_name": "trigger_performance_summary",
      "row_count": 150,
      "size_bytes": 45000,
      "last_refresh": "2025-11-30T11:55:00Z",
      "age_minutes": 5
    }
  ],
  "all_views_fresh": true
}
```

---

### 6. Organization Analytics

#### 6.1 GET /api/v1/analytics/organizations/usage

**Purpose**: Usage-based billing and quota tracking

**Query Parameters**:
- `days` (integer, optional): Time window (default: 30, max: 90)
- `organization_id` (string, optional): Specific organization (admin only)

**Response Schema**:
```json
{
  "data": [
    {
      "organization_id": "org_xyz789",
      "organization_name": "Acme Corp",
      "active_triggers": 15,
      "total_executions": 12500,
      "successes": 11875,
      "success_rate": 95.00,
      "avg_duration_ms": 340.20,
      "total_retries": 125
    }
  ],
  "metadata": {
    "days": 30
  }
}
```

**Authorization**: Users can only see their own organization's data. Admins can see all.

---

## Common Patterns

### Pagination

All list endpoints support pagination:

```rust
#[derive(Deserialize)]
pub struct PaginationQuery {
    limit: Option<i32>,
    offset: Option<i32>,
}

// Apply in SQL
LIMIT $1 OFFSET $2
```

**Response includes pagination metadata**:
```json
{
  "data": [...],
  "pagination": {
    "limit": 20,
    "offset": 0,
    "total": 150,
    "has_more": true
  }
}
```

### Filtering by Organization

**Multi-tenant isolation**:
```rust
// Extract user's organization IDs from JWT
let org_ids: Vec<String> = extract_user_organizations(&jwt_claims)?;

// Add WHERE clause
WHERE t.organization_id = ANY($1::TEXT[])
```

### Error Handling

**Standard error response**:
```json
{
  "error": {
    "code": "INVALID_QUERY_PARAMETER",
    "message": "Parameter 'days' must be between 1 and 90",
    "details": {
      "parameter": "days",
      "value": 365,
      "max_value": 90
    }
  }
}
```

### Caching Strategy

**Recommended caching**:
1. **Client-side**: Cache for 5 minutes (aligns with refresh schedule)
2. **Server-side**: No caching needed (views are pre-aggregated)
3. **CDN**: Cache public health endpoints for 1 minute

**HTTP Headers**:
```
Cache-Control: public, max-age=300
ETag: "view-refresh-timestamp-hash"
```

### Rate Limiting

**Per-endpoint limits**:
- `/health/summary`: 100/hour (higher, public-facing)
- Analytics queries: Inherit account tier limits
- Admin endpoints: 500/hour (higher for monitoring)

## Testing Recommendations

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn test_get_success_rate_default_params() {
        let pool = setup_test_db().await;
        let query = web::Query(SuccessRateQuery {
            days: None,
            action_type: None,
        });

        let response = get_success_rate(
            web::Data::new(pool),
            query
        ).await.unwrap();

        assert_eq!(response.status(), 200);
    }

    #[actix_web::test]
    async fn test_get_success_rate_with_filter() {
        let pool = setup_test_db().await;
        let query = web::Query(SuccessRateQuery {
            days: Some(7),
            action_type: Some("telegram".to_string()),
        });

        let response = get_success_rate(
            web::Data::new(pool),
            query
        ).await.unwrap();

        let body: serde_json::Value = test::read_body_json(response).await;
        assert!(body["data"].is_array());
    }
}
```

### Integration Tests

```bash
# Test success rate endpoint
curl -X GET \
  "http://localhost:8080/api/v1/analytics/actions/success-rate?days=7" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  | jq '.data[0].success_rate'

# Expected: 90-100 (percentage)
```

## Performance Considerations

1. **Always use indexes**: All queries in `analytics.sql` are optimized for indexed access
2. **Limit result sets**: Default LIMIT 10-20, max 100
3. **Avoid N+1 queries**: Use LEFT JOIN to fetch trigger names
4. **Cache organization lookups**: Store user's org_ids in JWT claims
5. **Monitor slow queries**: Log queries taking > 100ms

## Monitoring & Alerting

**Recommended alerts**:
1. **Stale views**: If age_minutes > 10 (views not refreshing)
2. **Slow queries**: If query time > 100ms (p95)
3. **High error rate**: If success_rate < 90% (system health)
4. **Trigger health**: If any trigger has success_rate < 80%

**Prometheus metrics**:
```rust
// Example metrics
analytics_query_duration_seconds{endpoint="success_rate"}
analytics_view_age_minutes{view="action_metrics_hourly"}
analytics_query_errors_total{endpoint="triggers_slowest"}
```

## Next Steps

1. Implement endpoints in `/rust-backend/crates/api-gateway/src/handlers/analytics.rs`
2. Add routes in `/rust-backend/crates/api-gateway/src/routes.rs`
3. Create integration tests in `/rust-backend/crates/api-gateway/tests/analytics_tests.rs`
4. Update API documentation in `/rust-backend/crates/api-gateway/API_DOCUMENTATION.md`
5. Create Grafana dashboard using these endpoints
