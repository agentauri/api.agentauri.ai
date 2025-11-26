-- ============================================================================
-- SIMPLIFIED PERFORMANCE TESTS
-- ============================================================================
-- Description: Tests for query performance and index effectiveness
-- Purpose: Verify indexes exist and queries perform well
-- Usage: psql -d test_erc8004_backend -f database/tests/test-performance-simple.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'PERFORMANCE TEST SUITE (SIMPLIFIED)'
\echo '========================================================================'
\echo ''

-- Create test result tracking
CREATE TEMP TABLE IF NOT EXISTS test_results (
    test_name TEXT,
    status TEXT,
    message TEXT
);

-- Helper function to record test results
CREATE OR REPLACE FUNCTION record_test(test_name TEXT, passed BOOLEAN, message TEXT DEFAULT '')
RETURNS void AS $$
BEGIN
    INSERT INTO test_results VALUES (
        test_name,
        CASE WHEN passed THEN 'PASS' ELSE 'FAIL' END,
        message
    );
END;
$$ LANGUAGE plpgsql;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 1: INDEX EXISTENCE (ALL TABLES)'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Verify all expected indexes exist
DO $$
DECLARE
    idx_count INTEGER;
    expected_indexes TEXT[] := ARRAY[
        'idx_users_email',
        'idx_users_username',
        'idx_triggers_user_id',
        'idx_triggers_org_chain_registry_enabled',
        'idx_trigger_conditions_trigger_id',
        'idx_trigger_actions_trigger_id',
        'idx_events_chain_id_created_at',
        'idx_events_agent_id',
        'idx_events_registry_type',
        'idx_events_client_address',
        'idx_events_validator_address',
        'idx_events_block_number',
        'idx_action_results_trigger_id',
        'idx_action_results_event_id',
        'idx_action_results_status',
        'idx_action_results_executed_at',
        'idx_action_results_action_type'
    ];
    missing_count INTEGER := 0;
    idx TEXT;
BEGIN
    FOR idx IN SELECT unnest(expected_indexes) LOOP
        IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = idx) THEN
            RAISE NOTICE 'Missing index: %', idx;
            missing_count := missing_count + 1;
        END IF;
    END LOOP;

    PERFORM record_test(
        'T1.1: all expected indexes exist',
        missing_count = 0,
        format('%s of %s indexes exist', array_length(expected_indexes, 1) - missing_count, array_length(expected_indexes, 1))
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: QUERY PERFORMANCE BENCHMARKS'
\echo '------------------------------------------------------------------------'

-- Test 2.1: Email lookup performs quickly
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    result_count INTEGER;
BEGIN
    start_time := clock_timestamp();

    SELECT COUNT(*) INTO result_count
    FROM users
    WHERE email = 'alice@example.com';

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.1: email lookup is fast',
        duration_ms < 100,
        format('Email lookup took %s ms (should be < 100ms)', duration_ms)
    );
END $$;

-- Test 2.2: Trigger lookup by user performs quickly
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    result_count INTEGER;
BEGIN
    start_time := clock_timestamp();

    SELECT COUNT(*) INTO result_count
    FROM triggers
    WHERE user_id = 'test-user-1';

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.2: trigger lookup by user is fast',
        duration_ms < 100,
        format('Trigger lookup took %s ms (should be < 100ms)', duration_ms)
    );
END $$;

-- Test 2.3: Event range query performs quickly
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    result_count INTEGER;
BEGIN
    start_time := clock_timestamp();

    SELECT COUNT(*) INTO result_count
    FROM events
    WHERE created_at >= NOW() - INTERVAL '7 days';

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.3: 7-day range query is fast',
        duration_ms < 1000,
        format('Range query took %s ms (should be < 1000ms)', duration_ms)
    );
END $$;

-- Test 2.4: Insert performance on events table
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    test_ids TEXT[] := ARRAY[]::TEXT[];
    test_id TEXT;
    i INTEGER;
BEGIN
    start_time := clock_timestamp();

    -- Insert 50 events
    FOR i IN 1..50 LOOP
        test_id := 'test-perf-' || i::TEXT || '-' || gen_random_uuid()::TEXT;
        test_ids := array_append(test_ids, test_id);

        INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                           registry, event_type, timestamp, created_at)
        VALUES (test_id, 84532, 9000000 + i, '0xtest' || i::TEXT, '0xtx' || i::TEXT, 0,
                'reputation', 'PerfTest', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());
    END LOOP;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    -- Clean up
    DELETE FROM events WHERE id = ANY(test_ids);

    PERFORM record_test(
        'T2.4: batch insert (50 events) is fast',
        duration_ms < 3000,
        format('50 inserts took %s ms (should be < 3000ms)', duration_ms)
    );
END $$;

-- Test 2.5: JOIN performance
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    result_count INTEGER;
BEGIN
    start_time := clock_timestamp();

    SELECT COUNT(*) INTO result_count
    FROM triggers t
    LEFT JOIN trigger_conditions tc ON t.id = tc.trigger_id
    LEFT JOIN trigger_actions ta ON t.id = ta.trigger_id;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.5: multi-table JOIN is fast',
        duration_ms < 500,
        format('JOIN query took %s ms (should be < 500ms)', duration_ms)
    );
END $$;

-- Test 2.6: Aggregation query performance
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
BEGIN
    start_time := clock_timestamp();

    PERFORM
        chain_id,
        registry,
        COUNT(*) as event_count
    FROM events
    WHERE created_at >= NOW() - INTERVAL '7 days'
    GROUP BY chain_id, registry;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.6: aggregation query is fast',
        duration_ms < 1000,
        format('Aggregation took %s ms (should be < 1000ms)', duration_ms)
    );
END $$;

-- Test 2.7: Agent-specific event query
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    result_count INTEGER;
BEGIN
    start_time := clock_timestamp();

    SELECT COUNT(*) INTO result_count
    FROM events
    WHERE agent_id = 42;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T2.7: agent-specific query is fast',
        duration_ms < 100,
        format('Agent query took %s ms (should be < 100ms)', duration_ms)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: INDEX SELECTIVITY AND EFFECTIVENESS'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Partial indexes save space
DO $$
DECLARE
    partial_idx_size BIGINT;
    table_size BIGINT;
    size_ratio NUMERIC;
BEGIN
    SELECT pg_relation_size('idx_events_agent_id') INTO partial_idx_size;
    SELECT pg_relation_size('events') INTO table_size;

    size_ratio := (partial_idx_size::NUMERIC / NULLIF(table_size, 0)) * 100;

    PERFORM record_test(
        'T3.1: partial index is smaller than table',
        size_ratio < 100 OR table_size = 0,
        format('Partial index is %s%% of table size', ROUND(COALESCE(size_ratio, 0), 2))
    );
END $$;

-- Test 3.2: Index statistics are being collected
DO $$
DECLARE
    stats_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO stats_count
    FROM pg_stat_user_indexes
    WHERE schemaname = 'public';

    PERFORM record_test(
        'T3.2: index statistics collected',
        stats_count > 0,
        format('Found statistics for %s indexes', stats_count)
    );
END $$;

\echo ''
\echo '========================================================================'
\echo 'TEST RESULTS SUMMARY'
\echo '========================================================================'
\echo ''

-- Display results
SELECT
    status,
    COUNT(*) as count,
    ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER (), 2) as percentage
FROM test_results
GROUP BY status
ORDER BY status DESC;

\echo ''
\echo 'Detailed Results:'
\echo '------------------------------------------------------------------------'

SELECT
    test_name,
    status,
    CASE
        WHEN status = 'PASS' THEN '✓ ' || test_name
        ELSE '✗ ' || test_name || ' - ' || message
    END as result
FROM test_results
ORDER BY test_name;

-- Final result
DO $$
DECLARE
    failed_count INTEGER;
    total_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO failed_count FROM test_results WHERE status = 'FAIL';
    SELECT COUNT(*) INTO total_count FROM test_results;

    IF failed_count > 0 THEN
        RAISE EXCEPTION E'\n❌ PERFORMANCE TESTS FAILED: % of % tests failed', failed_count, total_count;
    ELSE
        RAISE NOTICE E'\n✅ ALL PERFORMANCE TESTS PASSED: %/% tests', total_count, total_count;
    END IF;
END $$;
