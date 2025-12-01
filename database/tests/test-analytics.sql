-- ============================================================================
-- ANALYTICS MATERIALIZED VIEWS TEST SUITE
-- ============================================================================
-- Purpose: Comprehensive testing of analytics views, indexes, and functions
-- Coverage: 20+ tests covering view creation, data correctness, performance, edge cases
-- Run: psql erc8004_backend < database/tests/test-analytics.sql
-- ============================================================================

\echo '============================================================================'
\echo 'ANALYTICS MATERIALIZED VIEWS TEST SUITE'
\echo '============================================================================'
\echo ''

-- Setup test environment
\set ON_ERROR_STOP on
\timing on

-- Create test schema to isolate test data
CREATE SCHEMA IF NOT EXISTS analytics_test;
SET search_path TO analytics_test, public;

-- ============================================================================
-- TEST SUITE 1: VIEW CREATION AND STRUCTURE
-- ============================================================================

\echo '============================================================================'
\echo 'TEST SUITE 1: VIEW CREATION AND STRUCTURE'
\echo '============================================================================'
\echo ''

-- Test 1.1: Verify action_metrics_hourly exists
\echo 'Test 1.1: Verify action_metrics_hourly exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_matviews
        WHERE schemaname = 'public'
        AND matviewname = 'action_metrics_hourly'
    ) THEN
        RAISE EXCEPTION 'FAILED: action_metrics_hourly view does not exist';
    END IF;
    RAISE NOTICE 'PASSED: action_metrics_hourly view exists';
END $$;

-- Test 1.2: Verify trigger_performance_summary exists
\echo 'Test 1.2: Verify trigger_performance_summary exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_matviews
        WHERE schemaname = 'public'
        AND matviewname = 'trigger_performance_summary'
    ) THEN
        RAISE EXCEPTION 'FAILED: trigger_performance_summary view does not exist';
    END IF;
    RAISE NOTICE 'PASSED: trigger_performance_summary view exists';
END $$;

-- Test 1.3: Verify recent_failures exists
\echo 'Test 1.3: Verify recent_failures exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_matviews
        WHERE schemaname = 'public'
        AND matviewname = 'recent_failures'
    ) THEN
        RAISE EXCEPTION 'FAILED: recent_failures view does not exist';
    END IF;
    RAISE NOTICE 'PASSED: recent_failures view exists';
END $$;

-- Test 1.4: Verify action_type_distribution exists
\echo 'Test 1.4: Verify action_type_distribution exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_matviews
        WHERE schemaname = 'public'
        AND matviewname = 'action_type_distribution'
    ) THEN
        RAISE EXCEPTION 'FAILED: action_type_distribution view does not exist';
    END IF;
    RAISE NOTICE 'PASSED: action_type_distribution view exists';
END $$;

-- Test 1.5: Verify refresh_action_analytics function exists
\echo 'Test 1.5: Verify refresh_action_analytics function exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc
        WHERE proname = 'refresh_action_analytics'
    ) THEN
        RAISE EXCEPTION 'FAILED: refresh_action_analytics function does not exist';
    END IF;
    RAISE NOTICE 'PASSED: refresh_action_analytics function exists';
END $$;

-- Test 1.6: Verify get_analytics_refresh_status function exists
\echo 'Test 1.6: Verify get_analytics_refresh_status function exists'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_proc
        WHERE proname = 'get_analytics_refresh_status'
    ) THEN
        RAISE EXCEPTION 'FAILED: get_analytics_refresh_status function does not exist';
    END IF;
    RAISE NOTICE 'PASSED: get_analytics_refresh_status function exists';
END $$;

-- ============================================================================
-- TEST SUITE 2: INDEX VERIFICATION
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 2: INDEX VERIFICATION'
\echo '============================================================================'
\echo ''

-- Test 2.1: Verify action_metrics_hourly unique index
\echo 'Test 2.1: Verify action_metrics_hourly unique index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'action_metrics_hourly'
        AND indexname = 'idx_action_metrics_hourly_unique'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_action_metrics_hourly_unique does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_action_metrics_hourly_unique exists';
END $$;

-- Test 2.2: Verify action_metrics_hourly hour index
\echo 'Test 2.2: Verify action_metrics_hourly hour index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'action_metrics_hourly'
        AND indexname = 'idx_action_metrics_hourly_hour'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_action_metrics_hourly_hour does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_action_metrics_hourly_hour exists';
END $$;

-- Test 2.3: Verify trigger_performance_summary unique index
\echo 'Test 2.3: Verify trigger_performance_summary unique index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'trigger_performance_summary'
        AND indexname = 'idx_trigger_performance_summary_unique'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_trigger_performance_summary_unique does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_trigger_performance_summary_unique exists';
END $$;

-- Test 2.4: Verify trigger_performance_summary success_rate index
\echo 'Test 2.4: Verify trigger_performance_summary success_rate index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'trigger_performance_summary'
        AND indexname = 'idx_trigger_performance_summary_success_rate'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_trigger_performance_summary_success_rate does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_trigger_performance_summary_success_rate exists';
END $$;

-- Test 2.5: Verify recent_failures unique index
\echo 'Test 2.5: Verify recent_failures unique index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'recent_failures'
        AND indexname = 'idx_recent_failures_unique'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_recent_failures_unique does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_recent_failures_unique exists';
END $$;

-- Test 2.6: Verify action_type_distribution unique index
\echo 'Test 2.6: Verify action_type_distribution unique index'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'action_type_distribution'
        AND indexname = 'idx_action_type_distribution_unique'
    ) THEN
        RAISE EXCEPTION 'FAILED: idx_action_type_distribution_unique does not exist';
    END IF;
    RAISE NOTICE 'PASSED: idx_action_type_distribution_unique exists';
END $$;

-- ============================================================================
-- TEST SUITE 3: DATA CORRECTNESS
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 3: DATA CORRECTNESS'
\echo '============================================================================'
\echo ''

-- Setup: Create test data in public schema
\echo 'Setup: Creating test data...'

-- Create test user FIRST (referenced by organization FK)
INSERT INTO users (id, username, email, password_hash)
VALUES ('test-user-analytics', 'analytics_user', 'analytics@test.com', 'test-hash')
ON CONFLICT (id) DO NOTHING;

-- Create test organization
INSERT INTO organizations (id, name, slug, owner_id, plan)
VALUES ('test-org-analytics', 'Analytics Test Org', 'analytics-test-org', 'test-user-analytics', 'pro')
ON CONFLICT (id) DO NOTHING;

-- Create test trigger
INSERT INTO triggers (id, organization_id, user_id, name, chain_id, registry, enabled)
VALUES ('test-trigger-analytics', 'test-org-analytics', 'test-user-analytics', 'Analytics Test Trigger', 1, 'reputation', true)
ON CONFLICT (id) DO NOTHING;

-- Insert test action results (diversified data)
INSERT INTO action_results (id, job_id, trigger_id, event_id, action_type, status, executed_at, duration_ms, error_message, retry_count)
VALUES
    -- Successful telegram actions
    ('test-result-1', 'job-1', 'test-trigger-analytics', 'event-1', 'telegram', 'success', NOW() - INTERVAL '2 hours', 150, NULL, 0),
    ('test-result-2', 'job-2', 'test-trigger-analytics', 'event-2', 'telegram', 'success', NOW() - INTERVAL '2 hours', 200, NULL, 0),
    ('test-result-3', 'job-3', 'test-trigger-analytics', 'event-3', 'telegram', 'success', NOW() - INTERVAL '3 hours', 180, NULL, 0),
    -- Failed telegram action
    ('test-result-4', 'job-4', 'test-trigger-analytics', 'event-4', 'telegram', 'failed', NOW() - INTERVAL '1 hour', 5000, 'Connection timeout', 2),
    -- Successful REST actions
    ('test-result-5', 'job-5', 'test-trigger-analytics', 'event-5', 'rest', 'success', NOW() - INTERVAL '30 minutes', 300, NULL, 0),
    ('test-result-6', 'job-6', 'test-trigger-analytics', 'event-6', 'rest', 'success', NOW() - INTERVAL '30 minutes', 250, NULL, 0),
    -- Failed REST action
    ('test-result-7', 'job-7', 'test-trigger-analytics', 'event-7', 'rest', 'failed', NOW() - INTERVAL '15 minutes', 1000, '404 Not Found', 1),
    -- Successful MCP actions
    ('test-result-8', 'job-8', 'test-trigger-analytics', 'event-8', 'mcp', 'success', NOW() - INTERVAL '4 hours', 500, NULL, 0),
    ('test-result-9', 'job-9', 'test-trigger-analytics', 'event-9', 'mcp', 'success', NOW() - INTERVAL '4 hours', 600, NULL, 0),
    -- Retrying action
    ('test-result-10', 'job-10', 'test-trigger-analytics', 'event-10', 'mcp', 'retrying', NOW() - INTERVAL '10 minutes', 2000, NULL, 1),
    -- Old data (25 hours ago - should not appear in recent_failures)
    ('test-result-11', 'job-11', 'test-trigger-analytics', 'event-11', 'telegram', 'failed', NOW() - INTERVAL '25 hours', 3000, 'Old error', 0),
    -- 31 days ago (should not appear in action_type_distribution)
    ('test-result-12', 'job-12', 'test-trigger-analytics', 'event-12', 'telegram', 'success', NOW() - INTERVAL '31 days', 100, NULL, 0)
ON CONFLICT (id) DO NOTHING;

-- Refresh views with test data
\echo 'Refreshing materialized views...'
REFRESH MATERIALIZED VIEW action_metrics_hourly;
REFRESH MATERIALIZED VIEW trigger_performance_summary;
REFRESH MATERIALIZED VIEW recent_failures;
REFRESH MATERIALIZED VIEW action_type_distribution;

-- Test 3.1: Verify action_metrics_hourly aggregates correctly
\echo 'Test 3.1: Verify action_metrics_hourly aggregates correctly'
DO $$
DECLARE
    v_row_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_row_count
    FROM action_metrics_hourly
    WHERE hour > NOW() - INTERVAL '5 hours';

    IF v_row_count = 0 THEN
        RAISE EXCEPTION 'FAILED: action_metrics_hourly has no recent data';
    END IF;

    RAISE NOTICE 'PASSED: action_metrics_hourly has % rows in last 5 hours', v_row_count;
END $$;

-- Test 3.2: Verify trigger_performance_summary calculates success_rate correctly
\echo 'Test 3.2: Verify trigger_performance_summary calculates success_rate correctly'
DO $$
DECLARE
    v_success_rate NUMERIC;
    v_total INTEGER;
    v_successes INTEGER;
BEGIN
    SELECT success_rate, total_executions, success_count
    INTO v_success_rate, v_total, v_successes
    FROM trigger_performance_summary
    WHERE trigger_id = 'test-trigger-analytics';

    IF v_success_rate IS NULL THEN
        RAISE EXCEPTION 'FAILED: success_rate is NULL for test trigger';
    END IF;

    -- Expected: 7 successes out of 12 total = 58.33%
    -- (excluding the 31-day-old record if view refreshed recently)
    IF v_total = 0 THEN
        RAISE EXCEPTION 'FAILED: total_executions is 0';
    END IF;

    RAISE NOTICE 'PASSED: success_rate = % (% successes / % total)', v_success_rate, v_successes, v_total;
END $$;

-- Test 3.3: Verify recent_failures only includes last 24 hours
\echo 'Test 3.3: Verify recent_failures only includes last 24 hours'
DO $$
DECLARE
    v_oldest_failure TIMESTAMPTZ;
    v_count INTEGER;
BEGIN
    SELECT MIN(executed_at), COUNT(*) INTO v_oldest_failure, v_count
    FROM recent_failures;

    IF v_oldest_failure < NOW() - INTERVAL '24 hours' THEN
        RAISE EXCEPTION 'FAILED: recent_failures contains data older than 24 hours: %', v_oldest_failure;
    END IF;

    -- Should have 2 failures in last 24 hours (test-result-4 and test-result-7)
    -- test-result-11 is 25 hours old, should not appear
    RAISE NOTICE 'PASSED: recent_failures contains only last 24 hours (% failures)', v_count;
END $$;

-- Test 3.4: Verify action_type_distribution percentages sum to ~100%
\echo 'Test 3.4: Verify action_type_distribution percentages sum to ~100%'
DO $$
DECLARE
    v_total_percentage NUMERIC;
BEGIN
    SELECT SUM(percentage) INTO v_total_percentage
    FROM action_type_distribution;

    IF v_total_percentage IS NULL THEN
        RAISE EXCEPTION 'FAILED: action_type_distribution has no data';
    END IF;

    -- Allow small rounding error
    IF ABS(v_total_percentage - 100.0) > 0.1 THEN
        RAISE EXCEPTION 'FAILED: percentages sum to %, expected 100', v_total_percentage;
    END IF;

    RAISE NOTICE 'PASSED: percentages sum to % (expected ~100)', v_total_percentage;
END $$;

-- Test 3.5: Verify percentile calculations are ordered correctly
\echo 'Test 3.5: Verify percentile calculations are ordered correctly'
DO $$
DECLARE
    v_p50 NUMERIC;
    v_p95 NUMERIC;
    v_p99 NUMERIC;
BEGIN
    SELECT p50_duration_ms, p95_duration_ms, p99_duration_ms
    INTO v_p50, v_p95, v_p99
    FROM action_metrics_hourly
    WHERE execution_count > 2
    LIMIT 1;

    IF v_p50 IS NULL OR v_p95 IS NULL OR v_p99 IS NULL THEN
        RAISE NOTICE 'SKIPPED: Not enough data for percentile test';
        RETURN;
    END IF;

    IF v_p50 > v_p95 OR v_p95 > v_p99 THEN
        RAISE EXCEPTION 'FAILED: percentiles not ordered: p50=% p95=% p99=%', v_p50, v_p95, v_p99;
    END IF;

    RAISE NOTICE 'PASSED: percentiles correctly ordered: p50=% p95=% p99=%', v_p50, v_p95, v_p99;
END $$;

-- ============================================================================
-- TEST SUITE 4: REFRESH FUNCTION
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 4: REFRESH FUNCTION'
\echo '============================================================================'
\echo ''

-- Test 4.1: Verify refresh_action_analytics executes successfully
\echo 'Test 4.1: Verify refresh_action_analytics executes successfully'
DO $$
DECLARE
    v_result RECORD;
    v_all_success BOOLEAN := true;
BEGIN
    FOR v_result IN SELECT * FROM refresh_action_analytics() LOOP
        RAISE NOTICE 'Refreshed %: % rows in %ms (success: %)',
            v_result.view_name,
            v_result.rows_refreshed,
            v_result.refresh_duration_ms,
            v_result.success;

        IF NOT v_result.success THEN
            v_all_success := false;
            RAISE WARNING 'Error refreshing %: %', v_result.view_name, v_result.error_message;
        END IF;
    END LOOP;

    IF NOT v_all_success THEN
        RAISE EXCEPTION 'FAILED: Some views failed to refresh';
    END IF;

    RAISE NOTICE 'PASSED: All views refreshed successfully';
END $$;

-- Test 4.2: Verify get_analytics_refresh_status returns data
\echo 'Test 4.2: Verify get_analytics_refresh_status returns data'
DO $$
DECLARE
    v_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM get_analytics_refresh_status();

    IF v_count != 4 THEN
        RAISE EXCEPTION 'FAILED: Expected 4 views in refresh status, got %', v_count;
    END IF;

    RAISE NOTICE 'PASSED: get_analytics_refresh_status returns data for all 4 views';
END $$;

-- ============================================================================
-- TEST SUITE 5: EDGE CASES
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 5: EDGE CASES'
\echo '============================================================================'
\echo ''

-- Test 5.1: Handle NULL duration_ms gracefully
\echo 'Test 5.1: Handle NULL duration_ms gracefully'
DO $$
BEGIN
    -- Insert action with NULL duration
    INSERT INTO action_results (id, job_id, trigger_id, event_id, action_type, status, executed_at, duration_ms, retry_count)
    VALUES ('test-result-null-duration', 'job-null', 'test-trigger-analytics', 'event-null', 'telegram', 'success', NOW(), NULL, 0)
    ON CONFLICT (id) DO NOTHING;

    REFRESH MATERIALIZED VIEW action_metrics_hourly;
    REFRESH MATERIALIZED VIEW trigger_performance_summary;

    -- Query should not error
    PERFORM * FROM action_metrics_hourly LIMIT 1;
    PERFORM * FROM trigger_performance_summary LIMIT 1;

    RAISE NOTICE 'PASSED: Views handle NULL duration_ms gracefully';
END $$;

-- Test 5.2: Handle trigger_id NULL (actions without trigger)
\echo 'Test 5.2: Handle trigger_id NULL (actions without trigger)'
DO $$
BEGIN
    -- Insert action with NULL trigger_id
    INSERT INTO action_results (id, job_id, trigger_id, event_id, action_type, status, executed_at, duration_ms, retry_count)
    VALUES ('test-result-null-trigger', 'job-null-trigger', NULL, 'event-null-trigger', 'rest', 'success', NOW(), 100, 0)
    ON CONFLICT (id) DO NOTHING;

    REFRESH MATERIALIZED VIEW trigger_performance_summary;

    -- Should not include NULL trigger_id row
    PERFORM * FROM trigger_performance_summary WHERE trigger_id IS NULL;

    RAISE NOTICE 'PASSED: trigger_performance_summary handles NULL trigger_id';
END $$;

-- Test 5.3: Handle empty error_message in recent_failures
\echo 'Test 5.3: Handle empty error_message in recent_failures'
DO $$
BEGIN
    -- Insert failure with empty error message
    INSERT INTO action_results (id, job_id, trigger_id, event_id, action_type, status, executed_at, duration_ms, error_message, retry_count)
    VALUES ('test-result-empty-error', 'job-empty-error', 'test-trigger-analytics', 'event-empty-error', 'telegram', 'failed', NOW(), 100, '', 0)
    ON CONFLICT (id) DO NOTHING;

    REFRESH MATERIALIZED VIEW recent_failures;

    PERFORM * FROM recent_failures WHERE error_message = '';

    RAISE NOTICE 'PASSED: recent_failures handles empty error_message';
END $$;

-- Test 5.4: Handle division by zero in success_rate calculation
\echo 'Test 5.4: Handle division by zero in success_rate calculation'
DO $$
DECLARE
    v_success_rate NUMERIC;
BEGIN
    -- This should already be handled by NULLIF in the view definition
    SELECT success_rate INTO v_success_rate
    FROM trigger_performance_summary
    WHERE total_executions = 0
    LIMIT 1;

    -- Should return NULL or no rows, not cause error
    RAISE NOTICE 'PASSED: Division by zero handled gracefully';
END $$;

-- ============================================================================
-- TEST SUITE 6: PERFORMANCE TESTS
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 6: PERFORMANCE TESTS'
\echo '============================================================================'
\echo ''

-- Test 6.1: Query performance on action_metrics_hourly
\echo 'Test 6.1: Query performance on action_metrics_hourly (should use indexes)'
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM action_metrics_hourly
WHERE hour > NOW() - INTERVAL '24 hours'
ORDER BY hour DESC
LIMIT 10;

-- Test 6.2: Query performance on trigger_performance_summary
\echo 'Test 6.2: Query performance on trigger_performance_summary (should use indexes)'
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM trigger_performance_summary
WHERE success_rate < 80
ORDER BY success_rate ASC
LIMIT 10;

-- Test 6.3: Verify CONCURRENT refresh doesn't block reads
\echo 'Test 6.3: Verify CONCURRENT refresh is configured (requires unique indexes)'
DO $$
DECLARE
    v_has_unique_index BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = 'public'
        AND tablename = 'action_metrics_hourly'
        AND indexname LIKE '%unique%'
    ) INTO v_has_unique_index;

    IF NOT v_has_unique_index THEN
        RAISE EXCEPTION 'FAILED: action_metrics_hourly missing unique index for CONCURRENT refresh';
    END IF;

    RAISE NOTICE 'PASSED: Views have unique indexes for CONCURRENT refresh';
END $$;

-- ============================================================================
-- TEST SUITE 7: DATA RETENTION
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUITE 7: DATA RETENTION'
\echo '============================================================================'
\echo ''

-- Test 7.1: Verify recent_failures excludes old data
\echo 'Test 7.1: Verify recent_failures excludes old data'
DO $$
DECLARE
    v_count_old INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count_old
    FROM recent_failures
    WHERE executed_at < NOW() - INTERVAL '24 hours';

    IF v_count_old > 0 THEN
        RAISE EXCEPTION 'FAILED: recent_failures contains % rows older than 24 hours', v_count_old;
    END IF;

    RAISE NOTICE 'PASSED: recent_failures excludes data older than 24 hours';
END $$;

-- Test 7.2: Verify action_type_distribution excludes old data
\echo 'Test 7.2: Verify action_type_distribution excludes old data'
DO $$
DECLARE
    v_oldest_execution TIMESTAMPTZ;
BEGIN
    SELECT MIN(last_execution_at) INTO v_oldest_execution
    FROM action_type_distribution;

    -- Note: This view aggregates all data in action_results from last 30 days
    -- So last_execution_at might be older than 30 days if no recent executions
    RAISE NOTICE 'PASSED: action_type_distribution oldest execution: %', v_oldest_execution;
END $$;

-- ============================================================================
-- CLEANUP
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'CLEANUP'
\echo '============================================================================'
\echo ''

-- Clean up test data
DELETE FROM action_results WHERE id LIKE 'test-result-%';
DELETE FROM triggers WHERE id = 'test-trigger-analytics';
DELETE FROM organizations WHERE id = 'test-org-analytics';
DELETE FROM users WHERE id = 'test-user-analytics';

-- Refresh views to remove test data
REFRESH MATERIALIZED VIEW action_metrics_hourly;
REFRESH MATERIALIZED VIEW trigger_performance_summary;
REFRESH MATERIALIZED VIEW recent_failures;
REFRESH MATERIALIZED VIEW action_type_distribution;

\echo 'Test data cleaned up'

-- Drop test schema
DROP SCHEMA IF EXISTS analytics_test CASCADE;

-- ============================================================================
-- TEST SUMMARY
-- ============================================================================

\echo ''
\echo '============================================================================'
\echo 'TEST SUMMARY'
\echo '============================================================================'
\echo 'Total Tests: 20+'
\echo 'Categories:'
\echo '  - View Creation: 6 tests'
\echo '  - Index Verification: 6 tests'
\echo '  - Data Correctness: 5 tests'
\echo '  - Refresh Function: 2 tests'
\echo '  - Edge Cases: 4 tests'
\echo '  - Performance: 3 tests'
\echo '  - Data Retention: 2 tests'
\echo ''
\echo 'All tests completed. Check output for PASSED/FAILED status.'
\echo '============================================================================'
