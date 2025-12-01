-- Migration: Add Analytics Materialized Views
-- Description: Create materialized views for efficient action execution analytics
-- Created: 2025-11-30
-- Purpose: Enable fast dashboard queries, performance monitoring, and failure analysis

-- ============================================================================
-- MATERIALIZED VIEW 1: Hourly Action Metrics
-- ============================================================================
-- Purpose: Aggregate action execution metrics by hour, action type, and status
-- Use Case: Time-series dashboards, hourly throughput analysis, SLA monitoring
-- Refresh: Every 5 minutes via refresh_action_analytics() function
-- ============================================================================

CREATE MATERIALIZED VIEW action_metrics_hourly AS
SELECT
    date_trunc('hour', executed_at) AS hour,
    action_type,
    status,
    COUNT(*) AS execution_count,
    AVG(duration_ms)::NUMERIC(10,2) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p99_duration_ms,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failure_count,
    SUM(CASE WHEN status = 'retrying' THEN 1 ELSE 0 END) AS retrying_count,
    SUM(retry_count) AS total_retries,
    SUM(CASE WHEN retry_count > 0 THEN 1 ELSE 0 END) AS actions_with_retries
FROM action_results
GROUP BY date_trunc('hour', executed_at), action_type, status
ORDER BY hour DESC, action_type, status;

-- Indexes for action_metrics_hourly
-- Unique index required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_action_metrics_hourly_unique
    ON action_metrics_hourly(hour, action_type, status);

-- Index for time-range queries (last 24 hours, last 7 days, etc.)
CREATE INDEX idx_action_metrics_hourly_hour
    ON action_metrics_hourly(hour DESC);

-- Index for filtering by action type
CREATE INDEX idx_action_metrics_hourly_action_type
    ON action_metrics_hourly(action_type);

-- Index for filtering by status
CREATE INDEX idx_action_metrics_hourly_status
    ON action_metrics_hourly(status);

-- Note: Partial indexes with NOW() are not supported (NOW() is not IMMUTABLE)
-- Instead, we rely on the hour index for time-range queries
-- The PostgreSQL query planner will efficiently use idx_action_metrics_hourly_hour

COMMENT ON MATERIALIZED VIEW action_metrics_hourly IS
    'Hourly aggregated metrics for action execution performance, success rates, and retry statistics. Refreshed every 5 minutes.';

-- ============================================================================
-- MATERIALIZED VIEW 2: Trigger Performance Summary
-- ============================================================================
-- Purpose: Per-trigger aggregate statistics for performance analysis
-- Use Case: Identify slow triggers, calculate success rates, find triggers needing optimization
-- Refresh: Every 5 minutes via refresh_action_analytics() function
-- ============================================================================

CREATE MATERIALIZED VIEW trigger_performance_summary AS
SELECT
    trigger_id,
    COUNT(*) AS total_executions,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failure_count,
    SUM(CASE WHEN status = 'retrying' THEN 1 ELSE 0 END) AS retrying_count,
    ROUND(100.0 * SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) / NULLIF(COUNT(*), 0), 2) AS success_rate,
    AVG(duration_ms)::NUMERIC(10,2) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms)::NUMERIC(10,2) AS p99_duration_ms,
    SUM(retry_count) AS total_retries,
    AVG(retry_count)::NUMERIC(10,2) AS avg_retries,
    MAX(retry_count) AS max_retries,
    MAX(executed_at) AS last_execution_at,
    MIN(executed_at) AS first_execution_at,
    EXTRACT(EPOCH FROM (MAX(executed_at) - MIN(executed_at))) / 3600 AS hours_active
FROM action_results
WHERE trigger_id IS NOT NULL
GROUP BY trigger_id
ORDER BY total_executions DESC;

-- Indexes for trigger_performance_summary
-- Unique index required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_trigger_performance_summary_unique
    ON trigger_performance_summary(trigger_id);

-- Index for sorting by total executions (most active triggers)
CREATE INDEX idx_trigger_performance_summary_executions
    ON trigger_performance_summary(total_executions DESC);

-- Index for finding triggers with low success rates (health monitoring)
CREATE INDEX idx_trigger_performance_summary_success_rate
    ON trigger_performance_summary(success_rate ASC NULLS FIRST);

-- Index for finding slowest triggers (performance optimization)
CREATE INDEX idx_trigger_performance_summary_avg_duration
    ON trigger_performance_summary(avg_duration_ms DESC NULLS LAST);

-- Index for finding triggers with high retry counts (reliability issues)
CREATE INDEX idx_trigger_performance_summary_retries
    ON trigger_performance_summary(total_retries DESC);

COMMENT ON MATERIALIZED VIEW trigger_performance_summary IS
    'Per-trigger aggregate statistics including success rates, performance metrics, and retry counts. Used for trigger health monitoring and optimization.';

-- ============================================================================
-- MATERIALIZED VIEW 3: Recent Failures (Last 24 Hours)
-- ============================================================================
-- Purpose: Track recent failures for debugging and alerting
-- Use Case: Error dashboards, incident response, failure pattern analysis
-- Refresh: Every 5 minutes via refresh_action_analytics() function
-- ============================================================================

CREATE MATERIALIZED VIEW recent_failures AS
SELECT
    id,
    job_id,
    trigger_id,
    event_id,
    action_type,
    error_message,
    retry_count,
    executed_at,
    duration_ms,
    response_data
FROM action_results
WHERE status = 'failed'
  AND executed_at > NOW() - INTERVAL '24 hours'
ORDER BY executed_at DESC;

-- Indexes for recent_failures
-- Unique index required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_recent_failures_unique
    ON recent_failures(id);

-- Index for filtering by trigger_id (find all failures for a trigger)
CREATE INDEX idx_recent_failures_trigger_id
    ON recent_failures(trigger_id)
    WHERE trigger_id IS NOT NULL;

-- Index for time-based queries
CREATE INDEX idx_recent_failures_executed_at
    ON recent_failures(executed_at DESC);

-- Index for filtering by action type
CREATE INDEX idx_recent_failures_action_type
    ON recent_failures(action_type);

-- Index for grouping errors by message (find common errors)
CREATE INDEX idx_recent_failures_error_message
    ON recent_failures(error_message)
    WHERE error_message IS NOT NULL;

COMMENT ON MATERIALIZED VIEW recent_failures IS
    'Failed action executions in the last 24 hours. Used for debugging, alerting, and failure pattern analysis.';

-- ============================================================================
-- MATERIALIZED VIEW 4: Action Type Distribution (Last 30 Days)
-- ============================================================================
-- Purpose: Understand action type usage patterns and resource allocation
-- Use Case: Capacity planning, cost analysis, usage reporting
-- Refresh: Every 5 minutes via refresh_action_analytics() function
-- ============================================================================

CREATE MATERIALIZED VIEW action_type_distribution AS
SELECT
    action_type,
    COUNT(*) AS total_executions,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) AS success_count,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failure_count,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2) AS percentage,
    AVG(duration_ms)::NUMERIC(10,2) AS avg_duration_ms,
    SUM(retry_count) AS total_retries,
    MAX(executed_at) AS last_execution_at
FROM action_results
WHERE executed_at > NOW() - INTERVAL '30 days'
GROUP BY action_type
ORDER BY total_executions DESC;

-- Indexes for action_type_distribution
-- Unique index required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_action_type_distribution_unique
    ON action_type_distribution(action_type);

-- Index for sorting by usage
CREATE INDEX idx_action_type_distribution_executions
    ON action_type_distribution(total_executions DESC);

COMMENT ON MATERIALIZED VIEW action_type_distribution IS
    'Distribution of action types over the last 30 days. Used for capacity planning and usage analysis.';

-- ============================================================================
-- REFRESH FUNCTION: Refresh All Analytics Views
-- ============================================================================
-- Purpose: Single function to refresh all materialized views
-- Usage: Call from scheduled job (cron, pg_cron) every 5 minutes
-- Performance: Uses CONCURRENTLY to avoid locking, requires unique indexes
-- ============================================================================

CREATE OR REPLACE FUNCTION refresh_action_analytics()
RETURNS TABLE(
    view_name TEXT,
    refresh_duration_ms INTEGER,
    rows_refreshed BIGINT,
    success BOOLEAN,
    error_message TEXT
) AS $$
DECLARE
    start_time TIMESTAMPTZ;
    end_time TIMESTAMPTZ;
    row_count BIGINT;
BEGIN
    -- Refresh action_metrics_hourly
    view_name := 'action_metrics_hourly';
    start_time := clock_timestamp();
    BEGIN
        REFRESH MATERIALIZED VIEW CONCURRENTLY action_metrics_hourly;
        end_time := clock_timestamp();
        SELECT COUNT(*) INTO row_count FROM action_metrics_hourly;
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := row_count;
        success := TRUE;
        error_message := NULL;
        RETURN NEXT;
    EXCEPTION WHEN OTHERS THEN
        end_time := clock_timestamp();
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := 0;
        success := FALSE;
        error_message := SQLERRM;
        RETURN NEXT;
    END;

    -- Refresh trigger_performance_summary
    view_name := 'trigger_performance_summary';
    start_time := clock_timestamp();
    BEGIN
        REFRESH MATERIALIZED VIEW CONCURRENTLY trigger_performance_summary;
        end_time := clock_timestamp();
        SELECT COUNT(*) INTO row_count FROM trigger_performance_summary;
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := row_count;
        success := TRUE;
        error_message := NULL;
        RETURN NEXT;
    EXCEPTION WHEN OTHERS THEN
        end_time := clock_timestamp();
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := 0;
        success := FALSE;
        error_message := SQLERRM;
        RETURN NEXT;
    END;

    -- Refresh recent_failures
    view_name := 'recent_failures';
    start_time := clock_timestamp();
    BEGIN
        REFRESH MATERIALIZED VIEW CONCURRENTLY recent_failures;
        end_time := clock_timestamp();
        SELECT COUNT(*) INTO row_count FROM recent_failures;
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := row_count;
        success := TRUE;
        error_message := NULL;
        RETURN NEXT;
    EXCEPTION WHEN OTHERS THEN
        end_time := clock_timestamp();
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := 0;
        success := FALSE;
        error_message := SQLERRM;
        RETURN NEXT;
    END;

    -- Refresh action_type_distribution
    view_name := 'action_type_distribution';
    start_time := clock_timestamp();
    BEGIN
        REFRESH MATERIALIZED VIEW CONCURRENTLY action_type_distribution;
        end_time := clock_timestamp();
        SELECT COUNT(*) INTO row_count FROM action_type_distribution;
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := row_count;
        success := TRUE;
        error_message := NULL;
        RETURN NEXT;
    EXCEPTION WHEN OTHERS THEN
        end_time := clock_timestamp();
        refresh_duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;
        rows_refreshed := 0;
        success := FALSE;
        error_message := SQLERRM;
        RETURN NEXT;
    END;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION refresh_action_analytics() IS
    'Refreshes all analytics materialized views. Returns timing and success status for each view. Schedule to run every 5 minutes.';

-- ============================================================================
-- HELPER FUNCTION: Get Analytics Refresh Status
-- ============================================================================
-- Purpose: Check when views were last refreshed and their current state
-- Usage: SELECT * FROM get_analytics_refresh_status();
-- ============================================================================

CREATE OR REPLACE FUNCTION get_analytics_refresh_status()
RETURNS TABLE(
    view_name TEXT,
    row_count BIGINT,
    size_bytes BIGINT,
    last_refresh TIMESTAMPTZ,
    age_minutes INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        v.view_name::TEXT,
        v.row_count,
        v.size_bytes,
        v.last_refresh,
        EXTRACT(EPOCH FROM (NOW() - v.last_refresh))::INTEGER / 60 AS age_minutes
    FROM (
        SELECT
            'action_metrics_hourly'::TEXT AS view_name,
            (SELECT COUNT(*) FROM action_metrics_hourly) AS row_count,
            pg_total_relation_size('action_metrics_hourly') AS size_bytes,
            (SELECT pg_stat_get_last_vacuum_time('action_metrics_hourly'::regclass)) AS last_refresh
        UNION ALL
        SELECT
            'trigger_performance_summary'::TEXT,
            (SELECT COUNT(*) FROM trigger_performance_summary),
            pg_total_relation_size('trigger_performance_summary'),
            (SELECT pg_stat_get_last_vacuum_time('trigger_performance_summary'::regclass))
        UNION ALL
        SELECT
            'recent_failures'::TEXT,
            (SELECT COUNT(*) FROM recent_failures),
            pg_total_relation_size('recent_failures'),
            (SELECT pg_stat_get_last_vacuum_time('recent_failures'::regclass))
        UNION ALL
        SELECT
            'action_type_distribution'::TEXT,
            (SELECT COUNT(*) FROM action_type_distribution),
            pg_total_relation_size('action_type_distribution'),
            (SELECT pg_stat_get_last_vacuum_time('action_type_distribution'::regclass))
    ) v;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_analytics_refresh_status() IS
    'Returns status of all analytics materialized views including row counts, size, and last refresh time.';

-- ============================================================================
-- INITIAL DATA POPULATION
-- ============================================================================
-- Populate materialized views with current data
-- This is done non-concurrently on first run (no existing data to query)
-- ============================================================================

REFRESH MATERIALIZED VIEW action_metrics_hourly;
REFRESH MATERIALIZED VIEW trigger_performance_summary;
REFRESH MATERIALIZED VIEW recent_failures;
REFRESH MATERIALIZED VIEW action_type_distribution;

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================
-- Next Steps:
-- 1. Schedule refresh_action_analytics() to run every 5 minutes (cron/pg_cron)
-- 2. Create API endpoints to query these views (see analytics.sql for examples)
-- 3. Build Grafana dashboards using these views
-- 4. Set up alerts based on success_rate thresholds
-- ============================================================================
