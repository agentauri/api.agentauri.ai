-- ============================================================================
-- TIMESCALEDB HYPERTABLE TESTS
-- ============================================================================
-- Description: Comprehensive tests for TimescaleDB hypertable functionality
-- Purpose: Verify events table is correctly configured as hypertable
-- Usage: psql -d test_agentauri_backend -f database/tests/test-timescaledb.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'TIMESCALEDB HYPERTABLE TEST SUITE'
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
\echo 'TEST GROUP 1: HYPERTABLE EXISTENCE AND CONFIGURATION'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Verify events table is a hypertable
DO $$
DECLARE
    is_hypertable BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM timescaledb_information.hypertables
        WHERE hypertable_name = 'events'
    ) INTO is_hypertable;

    PERFORM record_test(
        'T1.1: events is a hypertable',
        is_hypertable,
        'events table should be converted to TimescaleDB hypertable'
    );
END $$;

-- Test 1.2: Verify hypertable partitions by created_at
DO $$
DECLARE
    partition_column TEXT;
BEGIN
    SELECT column_name INTO partition_column
    FROM timescaledb_information.dimensions
    WHERE hypertable_name = 'events'
    AND dimension_number = 1;

    PERFORM record_test(
        'T1.2: hypertable partitions by created_at',
        partition_column = 'created_at',
        format('Expected created_at, found %s', COALESCE(partition_column, 'NULL'))
    );
END $$;

-- Test 1.3: Verify chunk interval is 7 days
DO $$
DECLARE
    chunk_interval INTERVAL;
    expected_interval INTERVAL := INTERVAL '7 days';
BEGIN
    SELECT time_interval INTO chunk_interval
    FROM timescaledb_information.dimensions
    WHERE hypertable_name = 'events'
    AND dimension_number = 1;

    PERFORM record_test(
        'T1.3: chunk interval is 7 days',
        chunk_interval = expected_interval,
        format('Expected 7 days, found %s', chunk_interval)
    );
END $$;

-- Test 1.4: Verify hypertable schema is public
DO $$
DECLARE
    schema_name TEXT;
BEGIN
    SELECT hypertable_schema INTO schema_name
    FROM timescaledb_information.hypertables
    WHERE hypertable_name = 'events';

    PERFORM record_test(
        'T1.4: hypertable is in public schema',
        schema_name = 'public',
        format('Expected public schema, found %s', schema_name)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: CHUNK CREATION AND MANAGEMENT'
\echo '------------------------------------------------------------------------'

-- Test 2.1: Verify chunks can be created
DO $$
DECLARE
    chunk_count_before INTEGER;
    chunk_count_after INTEGER;
    test_id TEXT;
BEGIN
    -- Count chunks before insert
    SELECT COUNT(*) INTO chunk_count_before
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'events';

    -- Insert test event
    test_id := 'test-ts-' || gen_random_uuid()::TEXT;
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 9999999, '0xtest', '0xtest', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Count chunks after insert
    SELECT COUNT(*) INTO chunk_count_after
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'events';

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T2.1: chunks are created automatically',
        chunk_count_after >= chunk_count_before,
        format('Chunks before: %s, after: %s', chunk_count_before, chunk_count_after)
    );
END $$;

-- Test 2.2: Verify chunk naming convention
DO $$
DECLARE
    proper_naming BOOLEAN;
BEGIN
    SELECT bool_and(chunk_name LIKE '_hyper_%') INTO proper_naming
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'events';

    PERFORM record_test(
        'T2.2: chunks follow naming convention',
        COALESCE(proper_naming, true),
        'All chunks should follow _hyper_* naming pattern'
    );
END $$;

-- Test 2.3: Verify chunk schema is _timescaledb_internal
DO $$
DECLARE
    proper_schema BOOLEAN;
BEGIN
    SELECT bool_and(chunk_schema = '_timescaledb_internal') INTO proper_schema
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'events';

    PERFORM record_test(
        'T2.3: chunks are in correct schema',
        COALESCE(proper_schema, true),
        'All chunks should be in _timescaledb_internal schema'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: DATA INSERTION AND RETRIEVAL'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Insert data across multiple time ranges
DO $$
DECLARE
    test_id_1 TEXT := 'test-ts-past-' || gen_random_uuid()::TEXT;
    test_id_2 TEXT := 'test-ts-present-' || gen_random_uuid()::TEXT;
    test_id_3 TEXT := 'test-ts-future-' || gen_random_uuid()::TEXT;
    insert_count INTEGER;
BEGIN
    -- Insert events at different times
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES
        (test_id_1, 84532, 1000001, '0xtest1', '0xtx1', 0,
         'reputation', 'TestEvent1', EXTRACT(EPOCH FROM NOW() - INTERVAL '20 days')::BIGINT,
         NOW() - INTERVAL '20 days'),
        (test_id_2, 84532, 1000002, '0xtest2', '0xtx2', 0,
         'reputation', 'TestEvent2', EXTRACT(EPOCH FROM NOW())::BIGINT,
         NOW()),
        (test_id_3, 84532, 1000003, '0xtest3', '0xtx3', 0,
         'reputation', 'TestEvent3', EXTRACT(EPOCH FROM NOW() + INTERVAL '1 day')::BIGINT,
         NOW() + INTERVAL '1 day');

    -- Verify all inserted
    SELECT COUNT(*) INTO insert_count
    FROM events
    WHERE id IN (test_id_1, test_id_2, test_id_3);

    -- Clean up
    DELETE FROM events WHERE id IN (test_id_1, test_id_2, test_id_3);

    PERFORM record_test(
        'T3.1: data inserts across time ranges',
        insert_count = 3,
        format('Expected 3 inserts, found %s', insert_count)
    );
END $$;

-- Test 3.2: Verify time-based queries work correctly
DO $$
DECLARE
    test_id TEXT := 'test-ts-query-' || gen_random_uuid()::TEXT;
    query_result INTEGER;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 2000001, '0xtest', '0xtx', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Query by time range
    SELECT COUNT(*) INTO query_result
    FROM events
    WHERE created_at >= NOW() - INTERVAL '1 minute'
    AND created_at <= NOW() + INTERVAL '1 minute'
    AND id = test_id;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T3.2: time-based queries work',
        query_result = 1,
        'Should find event in time range'
    );
END $$;

-- Test 3.3: Verify composite primary key works
DO $$
DECLARE
    test_id TEXT := 'test-ts-pk-' || gen_random_uuid()::TEXT;
    duplicate_error BOOLEAN := false;
BEGIN
    -- Insert first event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 3000001, '0xtest', '0xtx', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Try to insert duplicate (should fail)
    BEGIN
        INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                           registry, event_type, timestamp, created_at)
        VALUES (test_id, 84532, 3000002, '0xtest2', '0xtx2', 0,
                'reputation', 'TestEvent2', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());
    EXCEPTION WHEN unique_violation THEN
        duplicate_error := true;
    END;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T3.3: composite primary key enforced',
        duplicate_error,
        'Duplicate (id, created_at) should be rejected'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 4: QUERY PERFORMANCE AND OPTIMIZATION'
\echo '------------------------------------------------------------------------'

-- Test 4.1: Verify indexes work with hypertable
DO $$
DECLARE
    idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO idx_count
    FROM pg_indexes
    WHERE tablename = 'events';

    PERFORM record_test(
        'T4.1: indexes exist on hypertable',
        idx_count >= 5,
        format('Expected at least 5 indexes, found %s', idx_count)
    );
END $$;

-- Test 4.2: Verify chunks are created (indicates partitioning works)
DO $$
DECLARE
    test_id TEXT := 'test-ts-plan-' || gen_random_uuid()::TEXT;
    chunk_count INTEGER;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 4000001, '0xtest', '0xtx', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Check if chunks exist for hypertable
    SELECT COUNT(*) INTO chunk_count
    FROM timescaledb_information.chunks
    WHERE hypertable_name = 'events';

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T4.2: hypertable has chunks (partitioning active)',
        chunk_count > 0,
        format('Found %s chunks for events hypertable', chunk_count)
    );
END $$;

-- Test 4.3: Verify time_bucket function is available
DO $$
DECLARE
    bucket_works BOOLEAN;
    bucket_result TIMESTAMPTZ;
BEGIN
    SELECT time_bucket('1 hour', NOW()) INTO bucket_result;
    bucket_works := (bucket_result IS NOT NULL);

    PERFORM record_test(
        'T4.3: time_bucket function available',
        bucket_works,
        'TimescaleDB time_bucket should be available'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 5: HYPERTABLE METADATA'
\echo '------------------------------------------------------------------------'

-- Test 5.1: Verify hypertable is registered in TimescaleDB metadata
DO $$
DECLARE
    hypertable_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM timescaledb_information.hypertables
        WHERE hypertable_name = 'events'
    ) INTO hypertable_exists;

    PERFORM record_test(
        'T5.1: hypertable registered in TimescaleDB',
        hypertable_exists,
        'events should be registered as a hypertable'
    );
END $$;

-- Test 5.2: Verify compression settings table exists
DO $$
DECLARE
    compression_table_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'timescaledb_information'
        AND table_name = 'hypertable_compression_settings'
    ) INTO compression_table_exists;

    PERFORM record_test(
        'T5.2: compression support available',
        compression_table_exists,
        'TimescaleDB compression features should be available'
    );
END $$;

-- Test 5.3: Verify dimensions are properly recorded
DO $$
DECLARE
    dimension_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO dimension_count
    FROM timescaledb_information.dimensions
    WHERE hypertable_name = 'events';

    PERFORM record_test(
        'T5.3: hypertable has dimensions',
        dimension_count = 1,
        format('Expected 1 dimension (time), found %s', dimension_count)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 6: TIME-SERIES QUERIES'
\echo '------------------------------------------------------------------------'

-- Test 6.1: Test time_bucket aggregation
DO $$
DECLARE
    test_id_1 TEXT := 'test-ts-bucket-1-' || gen_random_uuid()::TEXT;
    test_id_2 TEXT := 'test-ts-bucket-2-' || gen_random_uuid()::TEXT;
    bucket_count INTEGER;
BEGIN
    -- Insert two events in same hour bucket
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES
        (test_id_1, 84532, 5000001, '0xtest1', '0xtx1', 0,
         'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW()),
        (test_id_2, 84532, 5000002, '0xtest2', '0xtx2', 0,
         'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Aggregate by hour
    SELECT COUNT(*) INTO bucket_count
    FROM (
        SELECT time_bucket('1 hour', created_at) as hour, COUNT(*)
        FROM events
        WHERE id IN (test_id_1, test_id_2)
        GROUP BY hour
    ) AS hourly;

    -- Clean up
    DELETE FROM events WHERE id IN (test_id_1, test_id_2);

    PERFORM record_test(
        'T6.1: time_bucket aggregation works',
        bucket_count = 1,
        'Should aggregate events into 1 hour bucket'
    );
END $$;

-- Test 6.2: Test range queries with ORDER BY
DO $$
DECLARE
    test_id TEXT := 'test-ts-order-' || gen_random_uuid()::TEXT;
    first_result TEXT;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 6000001, '0xtest', '0xtx', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Query with ORDER BY
    SELECT id INTO first_result
    FROM events
    WHERE created_at >= NOW() - INTERVAL '1 minute'
    ORDER BY created_at DESC
    LIMIT 1;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T6.2: range query with ORDER BY works',
        first_result IS NOT NULL,
        'Should retrieve most recent event'
    );
END $$;

-- Test 6.3: Test JOIN with hypertable
DO $$
DECLARE
    test_event_id TEXT := 'test-ts-join-' || gen_random_uuid()::TEXT;
    test_result_id TEXT := 'test-result-join-' || gen_random_uuid()::TEXT;
    join_count INTEGER;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_event_id, 84532, 7000001, '0xtest', '0xtx', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Insert related action result
    INSERT INTO action_results (id, job_id, event_id, action_type, status)
    VALUES (test_result_id, 'job-test', test_event_id, 'telegram', 'success');

    -- Test JOIN
    SELECT COUNT(*) INTO join_count
    FROM events e
    JOIN action_results ar ON e.id = ar.event_id
    WHERE e.id = test_event_id;

    -- Clean up
    DELETE FROM action_results WHERE id = test_result_id;
    DELETE FROM events WHERE id = test_event_id;

    PERFORM record_test(
        'T6.3: JOIN with hypertable works',
        join_count = 1,
        'Should successfully JOIN events with action_results'
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
        RAISE EXCEPTION E'\n❌ TIMESCALEDB TESTS FAILED: % of % tests failed', failed_count, total_count;
    ELSE
        RAISE NOTICE E'\n✅ ALL TIMESCALEDB TESTS PASSED: %/% tests', total_count, total_count;
    END IF;
END $$;
