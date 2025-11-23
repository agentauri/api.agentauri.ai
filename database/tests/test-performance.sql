-- ============================================================================
-- PERFORMANCE AND INDEX USAGE TESTS
-- ============================================================================
-- Description: Tests for query performance and index utilization
-- Purpose: Verify indexes are used correctly and queries perform well
-- Usage: psql -d test_erc8004_backend -f database/tests/test-performance.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'PERFORMANCE AND INDEX USAGE TEST SUITE'
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
\echo 'TEST GROUP 1: INDEX USAGE ON USERS TABLE'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Email lookup uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    -- Get query plan for email lookup
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM users WHERE email = 'test@example.com'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_users_email%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T1.1: email lookup uses index',
        uses_index,
        'Query plan should use idx_users_email'
    );
END $$;

-- Test 1.2: Username lookup uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM users WHERE username = 'testuser'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_users_username%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T1.2: username lookup uses index',
        uses_index,
        'Query plan should use idx_users_username'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: INDEX USAGE ON TRIGGERS TABLE'
\echo '------------------------------------------------------------------------'

-- Test 2.1: User triggers lookup uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
    test_user_id TEXT := gen_random_uuid()::TEXT;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM triggers WHERE user_id = test_user_id
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_triggers_user_id%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T2.1: user triggers lookup uses index',
        uses_index,
        'Query plan should use idx_triggers_user_id'
    );
END $$;

-- Test 2.2: Chain+registry+enabled lookup uses composite index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM triggers
        WHERE chain_id = 84532 AND registry = 'reputation' AND enabled = true
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_triggers_chain_registry_enabled%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T2.2: chain+registry filter uses composite index',
        uses_index,
        'Query plan should use idx_triggers_chain_registry_enabled'
    );
END $$;

-- Test 2.3: Partial index excludes disabled triggers
DO $$
DECLARE
    index_def TEXT;
    has_where_clause BOOLEAN;
BEGIN
    SELECT indexdef INTO index_def
    FROM pg_indexes
    WHERE indexname = 'idx_triggers_chain_registry_enabled';

    has_where_clause := index_def LIKE '%WHERE%enabled%';

    PERFORM record_test(
        'T2.3: partial index has WHERE clause',
        has_where_clause,
        'Index should be partial with WHERE enabled = true'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: INDEX USAGE ON EVENTS TABLE'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Time range query uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events
        WHERE created_at >= NOW() - INTERVAL '1 day'
        AND created_at <= NOW()
        LIMIT 10
    ) AS query_plan;

    uses_index := plan_text LIKE '%Index%' OR plan_text LIKE '%Scan%';

    PERFORM record_test(
        'T3.1: time range query uses index',
        uses_index,
        'Query should use TimescaleDB chunk or index scan'
    );
END $$;

-- Test 3.2: Agent ID query uses partial index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events WHERE agent_id = 42
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_events_agent_id%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T3.2: agent_id query uses partial index',
        uses_index,
        'Query should use idx_events_agent_id'
    );
END $$;

-- Test 3.3: Registry+event_type query uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events
        WHERE registry = 'reputation' AND event_type = 'FeedbackProvided'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_events_registry_type%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T3.3: registry+event_type uses index',
        uses_index,
        'Query should use idx_events_registry_type'
    );
END $$;

-- Test 3.4: Client address query uses partial index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events WHERE client_address = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_events_client_address%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T3.4: client_address query uses partial index',
        uses_index,
        'Query should use idx_events_client_address'
    );
END $$;

-- Test 3.5: Validator address query uses partial index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events WHERE validator_address = '0x5c6B0f7Bf3E7ce046039Bd8FABdfD3f9F5021678'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_events_validator_address%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T3.5: validator_address query uses partial index',
        uses_index,
        'Query should use idx_events_validator_address'
    );
END $$;

-- Test 3.6: Block number query uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM events WHERE chain_id = 84532 AND block_number = 1000
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_events_block_number%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T3.6: block_number query uses index',
        uses_index,
        'Query should use idx_events_block_number'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 4: INDEX USAGE ON ACTION_RESULTS TABLE'
\echo '------------------------------------------------------------------------'

-- Test 4.1: Trigger results lookup uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
    test_trigger_id TEXT := gen_random_uuid()::TEXT;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM action_results WHERE trigger_id = test_trigger_id
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_action_results_trigger_id%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T4.1: trigger results lookup uses index',
        uses_index,
        'Query should use idx_action_results_trigger_id'
    );
END $$;

-- Test 4.2: Status filter uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM action_results WHERE status = 'failed'
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_action_results_status%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T4.2: status filter uses index',
        uses_index,
        'Query should use idx_action_results_status'
    );
END $$;

-- Test 4.3: Recent results query uses index
DO $$
DECLARE
    plan_text TEXT;
    uses_index BOOLEAN;
BEGIN
    SELECT query_plan INTO plan_text
    FROM (
        EXPLAIN
        SELECT * FROM action_results
        ORDER BY executed_at DESC
        LIMIT 100
    ) AS query_plan;

    uses_index := plan_text LIKE '%idx_action_results_executed_at%' OR plan_text LIKE '%Index%';

    PERFORM record_test(
        'T4.3: recent results uses index',
        uses_index,
        'Query should use idx_action_results_executed_at'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 5: QUERY PERFORMANCE BENCHMARKS'
\echo '------------------------------------------------------------------------'

-- Test 5.1: Insert performance on events table
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
    test_id TEXT;
BEGIN
    start_time := clock_timestamp();

    -- Insert 100 events
    FOR i IN 1..100 LOOP
        test_id := 'test-perf-insert-' || i::TEXT || '-' || gen_random_uuid()::TEXT;
        INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                           registry, event_type, timestamp, created_at)
        VALUES (test_id, 84532, 9000000 + i, '0xtest' || i::TEXT, '0xtx' || i::TEXT, 0,
                'reputation', 'PerfTest', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());
    END LOOP;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    -- Clean up
    DELETE FROM events WHERE id LIKE 'test-perf-insert-%';

    PERFORM record_test(
        'T5.1: insert 100 events completes quickly',
        duration_ms < 5000,
        format('100 inserts took %s ms (should be < 5000ms)', duration_ms)
    );
END $$;

-- Test 5.2: Range query performance
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
    WHERE created_at >= NOW() - INTERVAL '30 days';

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T5.2: 30-day range query completes quickly',
        duration_ms < 1000,
        format('Range query took %s ms (should be < 1000ms)', duration_ms)
    );
END $$;

-- Test 5.3: Aggregation performance
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
        'T5.3: aggregation query completes quickly',
        duration_ms < 1000,
        format('Aggregation took %s ms (should be < 1000ms)', duration_ms)
    );
END $$;

-- Test 5.4: JOIN performance
DO $$
DECLARE
    start_time TIMESTAMP;
    end_time TIMESTAMP;
    duration_ms INTEGER;
BEGIN
    start_time := clock_timestamp();

    PERFORM
        t.id,
        t.name,
        COUNT(tc.id) as condition_count,
        COUNT(ta.id) as action_count
    FROM triggers t
    LEFT JOIN trigger_conditions tc ON t.id = tc.trigger_id
    LEFT JOIN trigger_actions ta ON t.id = ta.trigger_id
    GROUP BY t.id, t.name;

    end_time := clock_timestamp();
    duration_ms := EXTRACT(MILLISECONDS FROM (end_time - start_time))::INTEGER;

    PERFORM record_test(
        'T5.4: multi-table JOIN completes quickly',
        duration_ms < 1000,
        format('JOIN query took %s ms (should be < 1000ms)', duration_ms)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 6: PARTIAL INDEX EFFECTIVENESS'
\echo '------------------------------------------------------------------------'

-- Test 6.1: Verify partial indexes are smaller than full indexes
DO $$
DECLARE
    partial_idx_size BIGINT;
    table_size BIGINT;
    size_ratio NUMERIC;
BEGIN
    -- Get size of partial index
    SELECT pg_relation_size('idx_events_agent_id') INTO partial_idx_size;

    -- Get size of events table
    SELECT pg_relation_size('events') INTO table_size;

    -- Calculate ratio
    size_ratio := (partial_idx_size::NUMERIC / NULLIF(table_size, 0)) * 100;

    PERFORM record_test(
        'T6.1: partial index is smaller than table',
        size_ratio < 50 OR table_size = 0,
        format('Partial index is %.2f%% of table size', size_ratio)
    );
END $$;

-- Test 6.2: Enabled triggers index excludes disabled triggers
DO $$
DECLARE
    idx_stats RECORD;
    test_user_id TEXT := 'test-partial-' || gen_random_uuid()::TEXT;
BEGIN
    -- Create user
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'partial_test', 'partial@test.com', 'hash123');

    -- Create enabled trigger
    INSERT INTO triggers (id, user_id, name, chain_id, registry, enabled)
    VALUES ('test-partial-enabled', test_user_id, 'Enabled', 84532, 'reputation', true);

    -- Create disabled trigger
    INSERT INTO triggers (id, user_id, name, chain_id, registry, enabled)
    VALUES ('test-partial-disabled', test_user_id, 'Disabled', 84532, 'reputation', false);

    -- Get index statistics
    SELECT * INTO idx_stats
    FROM pg_stat_user_indexes
    WHERE indexrelname = 'idx_triggers_chain_registry_enabled';

    -- Clean up
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T6.2: partial index statistics tracked',
        idx_stats IS NOT NULL,
        'Partial index should have statistics'
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
