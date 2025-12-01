-- ============================================================================
-- ANALYTICS QUERIES FOR ACTION EXECUTION MONITORING
-- ============================================================================
-- Purpose: Common queries for dashboards, monitoring, and performance analysis
-- Target: Query materialized views (fast reads, updated every 5 minutes)
-- Usage: These queries can be exposed via API endpoints for dashboard consumption
-- ============================================================================

-- ============================================================================
-- SECTION 1: SUCCESS RATE ANALYSIS
-- ============================================================================

-- Query 1.1: Success Rate by Action Type (Last 7 Days)
-- Purpose: Understand which action types are performing well
-- Use Case: Dashboard widget, SLA monitoring, capacity planning
-- API Endpoint: GET /api/v1/analytics/actions/success-rate?days=7
-- ============================================================================
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
WHERE hour > NOW() - INTERVAL '7 days'
GROUP BY action_type
ORDER BY total_executions DESC;

-- Query 1.2: Success Rate by Hour (Last 24 Hours)
-- Purpose: Time-series view of system health
-- Use Case: Dashboard graph, incident detection, trend analysis
-- API Endpoint: GET /api/v1/analytics/actions/hourly?hours=24
-- ============================================================================
SELECT
    hour,
    SUM(execution_count) AS total_executions,
    SUM(success_count) AS successes,
    SUM(failure_count) AS failures,
    ROUND(100.0 * SUM(success_count) / NULLIF(SUM(execution_count), 0), 2) AS success_rate,
    ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms,
    ROUND(AVG(p95_duration_ms), 2) AS p95_duration_ms
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour DESC;

-- Query 1.3: Success Rate Trend (Daily, Last 30 Days)
-- Purpose: Long-term success rate trends
-- Use Case: Weekly reports, SLA compliance tracking
-- API Endpoint: GET /api/v1/analytics/actions/daily?days=30
-- ============================================================================
SELECT
    DATE(hour) AS day,
    SUM(execution_count) AS total_executions,
    SUM(success_count) AS successes,
    SUM(failure_count) AS failures,
    ROUND(100.0 * SUM(success_count) / NULLIF(SUM(execution_count), 0), 2) AS success_rate,
    ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '30 days'
GROUP BY DATE(hour)
ORDER BY day DESC;

-- ============================================================================
-- SECTION 2: PERFORMANCE ANALYSIS
-- ============================================================================

-- Query 2.1: Slowest Triggers (P95 Latency)
-- Purpose: Identify triggers requiring optimization
-- Use Case: Performance optimization prioritization, SLA violation alerts
-- API Endpoint: GET /api/v1/analytics/triggers/slowest?limit=10
-- ============================================================================
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
LIMIT 10;

-- Query 2.2: Latency Distribution by Action Type
-- Purpose: Understand performance characteristics of each action type
-- Use Case: Capacity planning, timeout configuration
-- API Endpoint: GET /api/v1/analytics/actions/latency?action_type=telegram
-- ============================================================================
SELECT
    action_type,
    status,
    COUNT(*) AS sample_count,
    ROUND(AVG(avg_duration_ms), 2) AS mean_duration_ms,
    ROUND(AVG(p50_duration_ms), 2) AS median_duration_ms,
    ROUND(AVG(p95_duration_ms), 2) AS p95_duration_ms,
    ROUND(AVG(p99_duration_ms), 2) AS p99_duration_ms,
    MIN(min_duration_ms) AS fastest_ms,
    MAX(max_duration_ms) AS slowest_ms
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
GROUP BY action_type, status
ORDER BY action_type, status;

-- Query 2.3: Triggers Exceeding SLA (P95 > 5000ms)
-- Purpose: Alert on triggers violating performance SLAs
-- Use Case: Alerting, incident management
-- API Endpoint: GET /api/v1/analytics/triggers/sla-violations?threshold_ms=5000
-- ============================================================================
SELECT
    tps.trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    tps.total_executions,
    tps.p95_duration_ms,
    tps.success_rate,
    tps.last_execution_at,
    CASE
        WHEN tps.p95_duration_ms > 10000 THEN 'CRITICAL'
        WHEN tps.p95_duration_ms > 5000 THEN 'WARNING'
        ELSE 'OK'
    END AS sla_status
FROM trigger_performance_summary tps
LEFT JOIN triggers t ON t.id = tps.trigger_id
WHERE tps.p95_duration_ms > 5000
ORDER BY tps.p95_duration_ms DESC;

-- ============================================================================
-- SECTION 3: FAILURE ANALYSIS
-- ============================================================================

-- Query 3.1: Error Frequency by Trigger (Last 24 Hours)
-- Purpose: Identify triggers with recurring errors
-- Use Case: Incident response, trigger health monitoring
-- API Endpoint: GET /api/v1/analytics/failures/by-trigger?hours=24
-- ============================================================================
SELECT
    rf.trigger_id,
    t.name AS trigger_name,
    COUNT(*) AS error_count,
    COUNT(DISTINCT rf.error_message) AS unique_error_types,
    MAX(rf.executed_at) AS last_error_at,
    MIN(rf.executed_at) AS first_error_at,
    ROUND(AVG(rf.retry_count), 2) AS avg_retry_count,
    MAX(rf.retry_count) AS max_retry_count
FROM recent_failures rf
LEFT JOIN triggers t ON t.id = rf.trigger_id
GROUP BY rf.trigger_id, t.name
ORDER BY error_count DESC;

-- Query 3.2: Top Error Messages (Last 24 Hours)
-- Purpose: Identify common error patterns
-- Use Case: Bug prioritization, system health monitoring
-- API Endpoint: GET /api/v1/analytics/failures/top-errors?hours=24&limit=10
-- ============================================================================
SELECT
    error_message,
    COUNT(*) AS occurrence_count,
    COUNT(DISTINCT trigger_id) AS affected_triggers,
    MIN(executed_at) AS first_seen,
    MAX(executed_at) AS last_seen,
    ROUND(AVG(retry_count), 2) AS avg_retry_count,
    array_agg(DISTINCT action_type) AS action_types
FROM recent_failures
WHERE error_message IS NOT NULL
GROUP BY error_message
ORDER BY occurrence_count DESC
LIMIT 10;

-- Query 3.3: Failure Rate by Action Type (Last 24 Hours)
-- Purpose: Identify unreliable action types
-- Use Case: Infrastructure monitoring, reliability engineering
-- API Endpoint: GET /api/v1/analytics/failures/by-action-type?hours=24
-- ============================================================================
SELECT
    rf.action_type,
    COUNT(*) AS failure_count,
    COUNT(DISTINCT rf.trigger_id) AS affected_triggers,
    ROUND(AVG(rf.retry_count), 2) AS avg_retry_count,
    MAX(rf.executed_at) AS last_failure_at
FROM recent_failures rf
GROUP BY rf.action_type
ORDER BY failure_count DESC;

-- Query 3.4: Recent Failures with Context
-- Purpose: Detailed failure investigation
-- Use Case: Debugging, incident response
-- API Endpoint: GET /api/v1/analytics/failures/recent?limit=20
-- ============================================================================
SELECT
    rf.id,
    rf.trigger_id,
    t.name AS trigger_name,
    rf.action_type,
    rf.error_message,
    rf.retry_count,
    rf.executed_at,
    rf.duration_ms,
    rf.event_id,
    t.organization_id
FROM recent_failures rf
LEFT JOIN triggers t ON t.id = rf.trigger_id
ORDER BY rf.executed_at DESC
LIMIT 20;

-- ============================================================================
-- SECTION 4: TRIGGER HEALTH MONITORING
-- ============================================================================

-- Query 4.1: Triggers with Low Success Rates (<80%)
-- Purpose: Identify unreliable triggers
-- Use Case: Trigger health dashboard, proactive maintenance
-- API Endpoint: GET /api/v1/analytics/triggers/low-success-rate?threshold=80
-- ============================================================================
SELECT
    tps.trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    t.enabled,
    tps.total_executions,
    tps.success_count,
    tps.failure_count,
    tps.success_rate,
    tps.total_retries,
    tps.last_execution_at,
    CASE
        WHEN tps.success_rate < 50 THEN 'CRITICAL'
        WHEN tps.success_rate < 80 THEN 'WARNING'
        ELSE 'OK'
    END AS health_status
FROM trigger_performance_summary tps
LEFT JOIN triggers t ON t.id = tps.trigger_id
WHERE tps.success_rate < 80
ORDER BY tps.success_rate ASC;

-- Query 4.2: Most Active Triggers (Last 7 Days)
-- Purpose: Identify high-traffic triggers
-- Use Case: Capacity planning, cost allocation
-- API Endpoint: GET /api/v1/analytics/triggers/most-active?days=7&limit=10
-- ============================================================================
SELECT
    tps.trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    tps.total_executions,
    tps.success_rate,
    tps.avg_duration_ms,
    tps.total_retries,
    tps.last_execution_at,
    ROUND(tps.total_executions / NULLIF(tps.hours_active, 0), 2) AS executions_per_hour
FROM trigger_performance_summary tps
LEFT JOIN triggers t ON t.id = tps.trigger_id
WHERE tps.last_execution_at > NOW() - INTERVAL '7 days'
ORDER BY tps.total_executions DESC
LIMIT 10;

-- Query 4.3: Idle Triggers (No Executions in Last 7 Days)
-- Purpose: Identify unused triggers that can be disabled
-- Use Case: Resource optimization, cost reduction
-- API Endpoint: GET /api/v1/analytics/triggers/idle?days=7
-- ============================================================================
SELECT
    t.id AS trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    t.enabled,
    t.created_at,
    tps.last_execution_at,
    EXTRACT(EPOCH FROM (NOW() - COALESCE(tps.last_execution_at, t.created_at)))::INTEGER / 86400 AS days_idle
FROM triggers t
LEFT JOIN trigger_performance_summary tps ON tps.trigger_id = t.id
WHERE t.enabled = true
  AND (tps.last_execution_at IS NULL OR tps.last_execution_at < NOW() - INTERVAL '7 days')
ORDER BY days_idle DESC;

-- Query 4.4: Triggers with High Retry Rates
-- Purpose: Identify triggers experiencing transient failures
-- Use Case: Reliability engineering, retry policy optimization
-- API Endpoint: GET /api/v1/analytics/triggers/high-retries?threshold=0.5
-- ============================================================================
SELECT
    tps.trigger_id,
    t.name AS trigger_name,
    t.organization_id,
    tps.total_executions,
    tps.total_retries,
    tps.avg_retries,
    tps.max_retries,
    ROUND(tps.total_retries::NUMERIC / NULLIF(tps.total_executions, 0), 2) AS retry_ratio,
    tps.success_rate,
    tps.last_execution_at
FROM trigger_performance_summary tps
LEFT JOIN triggers t ON t.id = tps.trigger_id
WHERE tps.avg_retries > 0.5
ORDER BY tps.avg_retries DESC;

-- ============================================================================
-- SECTION 5: CAPACITY PLANNING & USAGE ANALYSIS
-- ============================================================================

-- Query 5.1: Hourly Throughput (Last 24 Hours)
-- Purpose: Understand system load patterns
-- Use Case: Capacity planning, scaling decisions
-- API Endpoint: GET /api/v1/analytics/throughput/hourly?hours=24
-- ============================================================================
SELECT
    hour,
    SUM(execution_count) AS total_executions,
    SUM(success_count) AS successes,
    SUM(failure_count) AS failures,
    ROUND(SUM(execution_count)::NUMERIC / 3600, 2) AS executions_per_second,
    ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms,
    ROUND(AVG(p95_duration_ms), 2) AS p95_duration_ms
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour DESC;

-- Query 5.2: Action Type Distribution (Last 30 Days)
-- Purpose: Understand resource allocation across action types
-- Use Case: Cost analysis, infrastructure planning
-- API Endpoint: GET /api/v1/analytics/actions/distribution?days=30
-- ============================================================================
SELECT
    action_type,
    total_executions,
    success_count,
    failure_count,
    percentage,
    ROUND(100.0 * success_count / NULLIF(total_executions, 0), 2) AS success_rate,
    avg_duration_ms,
    total_retries,
    last_execution_at
FROM action_type_distribution
ORDER BY total_executions DESC;

-- Query 5.3: Peak Hours Analysis (Last 7 Days)
-- Purpose: Identify peak traffic times
-- Use Case: Autoscaling configuration, maintenance scheduling
-- API Endpoint: GET /api/v1/analytics/throughput/peak-hours?days=7
-- ============================================================================
SELECT
    EXTRACT(HOUR FROM hour) AS hour_of_day,
    COUNT(*) AS data_points,
    SUM(execution_count) AS total_executions,
    ROUND(AVG(execution_count), 2) AS avg_executions_per_hour,
    MAX(execution_count) AS peak_executions,
    MIN(execution_count) AS min_executions
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '7 days'
GROUP BY EXTRACT(HOUR FROM hour)
ORDER BY hour_of_day;

-- Query 5.4: Growth Trend (Week-over-Week)
-- Purpose: Measure system growth
-- Use Case: Capacity forecasting, business reporting
-- API Endpoint: GET /api/v1/analytics/growth/weekly
-- ============================================================================
WITH weekly_stats AS (
    SELECT
        DATE_TRUNC('week', hour) AS week,
        SUM(execution_count) AS total_executions,
        SUM(success_count) AS successes,
        ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms
    FROM action_metrics_hourly
    WHERE hour > NOW() - INTERVAL '8 weeks'
    GROUP BY DATE_TRUNC('week', hour)
)
SELECT
    week,
    total_executions,
    successes,
    avg_duration_ms,
    LAG(total_executions) OVER (ORDER BY week) AS prev_week_executions,
    ROUND(100.0 * (total_executions - LAG(total_executions) OVER (ORDER BY week))::NUMERIC /
          NULLIF(LAG(total_executions) OVER (ORDER BY week), 0), 2) AS growth_percentage
FROM weekly_stats
ORDER BY week DESC;

-- ============================================================================
-- SECTION 6: ORGANIZATION-LEVEL ANALYTICS
-- ============================================================================

-- Query 6.1: Usage by Organization (Last 30 Days)
-- Purpose: Understand per-organization usage for billing/quota
-- Use Case: Usage-based billing, quota enforcement
-- API Endpoint: GET /api/v1/analytics/organizations/usage?days=30
-- ============================================================================
SELECT
    t.organization_id,
    o.name AS organization_name,
    COUNT(DISTINCT t.id) AS active_triggers,
    SUM(tps.total_executions) AS total_executions,
    SUM(tps.success_count) AS successes,
    ROUND(100.0 * SUM(tps.success_count) / NULLIF(SUM(tps.total_executions), 0), 2) AS success_rate,
    ROUND(AVG(tps.avg_duration_ms), 2) AS avg_duration_ms,
    SUM(tps.total_retries) AS total_retries
FROM triggers t
LEFT JOIN trigger_performance_summary tps ON tps.trigger_id = t.id
LEFT JOIN organizations o ON o.id = t.organization_id
WHERE tps.last_execution_at > NOW() - INTERVAL '30 days'
GROUP BY t.organization_id, o.name
ORDER BY total_executions DESC;

-- Query 6.2: Top Organizations by Activity
-- Purpose: Identify largest customers
-- Use Case: Customer success, resource allocation
-- API Endpoint: GET /api/v1/analytics/organizations/top?limit=10
-- ============================================================================
SELECT
    t.organization_id,
    o.name AS organization_name,
    o.plan,
    COUNT(DISTINCT t.id) AS trigger_count,
    SUM(tps.total_executions) AS total_executions,
    ROUND(100.0 * SUM(tps.success_count) / NULLIF(SUM(tps.total_executions), 0), 2) AS success_rate
FROM triggers t
LEFT JOIN trigger_performance_summary tps ON tps.trigger_id = t.id
LEFT JOIN organizations o ON o.id = t.organization_id
GROUP BY t.organization_id, o.name, o.plan
ORDER BY total_executions DESC
LIMIT 10;

-- ============================================================================
-- SECTION 7: SYSTEM HEALTH SUMMARY
-- ============================================================================

-- Query 7.1: Overall System Health (Last 24 Hours)
-- Purpose: Single query for system health dashboard
-- Use Case: Operations dashboard, status page
-- API Endpoint: GET /api/v1/analytics/health/summary
-- ============================================================================
SELECT
    'last_24_hours' AS time_window,
    SUM(execution_count) AS total_executions,
    SUM(success_count) AS total_successes,
    SUM(failure_count) AS total_failures,
    ROUND(100.0 * SUM(success_count) / NULLIF(SUM(execution_count), 0), 2) AS success_rate,
    ROUND(AVG(avg_duration_ms), 2) AS avg_duration_ms,
    ROUND(AVG(p95_duration_ms), 2) AS p95_duration_ms,
    ROUND(AVG(p99_duration_ms), 2) AS p99_duration_ms,
    SUM(total_retries) AS total_retries,
    COUNT(DISTINCT hour) AS hours_with_data
FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours';

-- Query 7.2: Analytics Refresh Status
-- Purpose: Monitor health of materialized views
-- Use Case: Operations dashboard, alerting
-- API Endpoint: GET /api/v1/analytics/meta/refresh-status
-- ============================================================================
SELECT * FROM get_analytics_refresh_status();

-- ============================================================================
-- END OF ANALYTICS QUERIES
-- ============================================================================
-- Usage Notes:
-- 1. All queries target materialized views (refreshed every 5 minutes)
-- 2. Add LIMIT and OFFSET for pagination in API endpoints
-- 3. Add WHERE clauses for filtering (organization_id, action_type, etc.)
-- 4. Wrap in CTEs for more complex transformations
-- 5. Add ORDER BY based on API requirements (sort by date, count, etc.)
-- ============================================================================
