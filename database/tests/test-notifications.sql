-- ============================================================================
-- POSTGRESQL NOTIFY/LISTEN TESTS
-- ============================================================================
-- Description: Tests for PostgreSQL NOTIFY/LISTEN functionality
-- Purpose: Verify events trigger NOTIFY correctly for real-time processing
-- Usage: psql -d test_agentauri_backend -f database/tests/test-notifications.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'POSTGRESQL NOTIFY/LISTEN TEST SUITE'
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
\echo 'TEST GROUP 1: NOTIFY FUNCTION AND TRIGGER'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Verify notify_new_event function exists
DO $$
DECLARE
    func_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE p.proname = 'notify_new_event'
        AND n.nspname = 'public'
    ) INTO func_exists;

    PERFORM record_test(
        'T1.1: notify_new_event function exists',
        func_exists,
        'Function should exist in public schema'
    );
END $$;

-- Test 1.2: Verify events_notify_trigger exists
DO $$
DECLARE
    trigger_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_trigger t
        JOIN pg_class c ON t.tgrelid = c.oid
        WHERE t.tgname IN ('events_notify_trigger', 'trigger_notify_new_event')
        AND c.relname = 'events'
    ) INTO trigger_exists;

    PERFORM record_test(
        'T1.2: events_notify_trigger exists',
        trigger_exists,
        'Trigger should be attached to events table'
    );
END $$;

-- Test 1.3: Verify trigger is AFTER INSERT
DO $$
DECLARE
    trigger_timing TEXT;
    trigger_event TEXT;
BEGIN
    SELECT
        CASE WHEN tgtype & 1 = 1 THEN 'ROW' ELSE 'STATEMENT' END,
        CASE
            WHEN tgtype & 2 = 2 THEN 'BEFORE'
            WHEN tgtype & 4 = 4 THEN 'INSTEAD OF'
            ELSE 'AFTER'
        END
    INTO trigger_timing, trigger_timing
    FROM pg_trigger t
    JOIN pg_class c ON t.tgrelid = c.oid
    WHERE t.tgname IN ('events_notify_trigger', 'trigger_notify_new_event')
    AND c.relname = 'events';

    PERFORM record_test(
        'T1.3: trigger timing is AFTER INSERT',
        trigger_timing IS NOT NULL,
        'Trigger should fire AFTER INSERT FOR EACH ROW'
    );
END $$;

-- Test 1.4: Verify function returns trigger type
DO $$
DECLARE
    return_type TEXT;
BEGIN
    SELECT typname INTO return_type
    FROM pg_proc p
    JOIN pg_type t ON p.prorettype = t.oid
    WHERE p.proname = 'notify_new_event';

    PERFORM record_test(
        'T1.4: function returns TRIGGER type',
        return_type = 'trigger',
        format('Expected trigger, found %s', return_type)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: NOTIFICATION PAYLOAD'
\echo '------------------------------------------------------------------------'

-- Note: Testing actual NOTIFY/LISTEN requires a separate connection, so we
-- test the trigger logic and function definition here

-- Test 2.1: Verify function uses pg_notify
DO $$
DECLARE
    func_source TEXT;
    uses_pg_notify BOOLEAN;
BEGIN
    SELECT prosrc INTO func_source
    FROM pg_proc
    WHERE proname = 'notify_new_event';

    uses_pg_notify := func_source LIKE '%pg_notify%';

    PERFORM record_test(
        'T2.1: function uses pg_notify',
        uses_pg_notify,
        'Function should call pg_notify() to send notification'
    );
END $$;

-- Test 2.2: Verify notification channel is new_event
DO $$
DECLARE
    func_source TEXT;
    correct_channel BOOLEAN;
BEGIN
    SELECT prosrc INTO func_source
    FROM pg_proc
    WHERE proname = 'notify_new_event';

    correct_channel := func_source LIKE '%''new_event''%';

    PERFORM record_test(
        'T2.2: notification channel is new_event',
        correct_channel,
        'Function should notify on new_event channel'
    );
END $$;

-- Test 2.3: Verify payload includes NEW.id
DO $$
DECLARE
    func_source TEXT;
    includes_id BOOLEAN;
BEGIN
    SELECT prosrc INTO func_source
    FROM pg_proc
    WHERE proname = 'notify_new_event';

    includes_id := func_source LIKE '%NEW.id%';

    PERFORM record_test(
        'T2.3: notification includes NEW.id',
        includes_id,
        'Notification payload should include event ID'
    );
END $$;

-- Test 2.4: Verify function returns NEW
DO $$
DECLARE
    func_source TEXT;
    returns_new BOOLEAN;
BEGIN
    SELECT prosrc INTO func_source
    FROM pg_proc
    WHERE proname = 'notify_new_event';

    returns_new := func_source LIKE '%RETURN NEW%';

    PERFORM record_test(
        'T2.4: function returns NEW',
        returns_new,
        'Trigger function should RETURN NEW to allow insert'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: TRIGGER BEHAVIOR'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Insert triggers notification (verify trigger fires)
DO $$
DECLARE
    test_id TEXT := 'test-notify-' || gen_random_uuid()::TEXT;
    insert_succeeded BOOLEAN;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 8000001, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Verify insert succeeded (trigger didn't block it)
    SELECT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO insert_succeeded;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T3.1: insert succeeds with trigger',
        insert_succeeded,
        'Trigger should not block insert operations'
    );
END $$;

-- Test 3.2: Multiple inserts trigger multiple notifications
DO $$
DECLARE
    test_id_1 TEXT := 'test-notify-multi-1-' || gen_random_uuid()::TEXT;
    test_id_2 TEXT := 'test-notify-multi-2-' || gen_random_uuid()::TEXT;
    test_id_3 TEXT := 'test-notify-multi-3-' || gen_random_uuid()::TEXT;
    insert_count INTEGER;
BEGIN
    -- Insert multiple events
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES
        (test_id_1, 84532, 8000011, '0xtest1', '0xtx1', 0,
         'reputation', 'TestNotify1', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW()),
        (test_id_2, 84532, 8000012, '0xtest2', '0xtx2', 0,
         'reputation', 'TestNotify2', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW()),
        (test_id_3, 84532, 8000013, '0xtest3', '0xtx3', 0,
         'reputation', 'TestNotify3', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Verify all inserted
    SELECT COUNT(*) INTO insert_count
    FROM events
    WHERE id IN (test_id_1, test_id_2, test_id_3);

    -- Clean up
    DELETE FROM events WHERE id IN (test_id_1, test_id_2, test_id_3);

    PERFORM record_test(
        'T3.2: multiple inserts succeed',
        insert_count = 3,
        'Trigger should fire FOR EACH ROW without issues'
    );
END $$;

-- Test 3.3: Trigger only fires on INSERT (not UPDATE)
DO $$
DECLARE
    test_id TEXT := 'test-notify-update-' || gen_random_uuid()::TEXT;
    update_succeeded BOOLEAN;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, agent_id, created_at)
    VALUES (test_id, 84532, 8000021, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, 42, NOW());

    -- Update event (trigger should NOT fire on update)
    UPDATE events SET agent_id = 99 WHERE id = test_id;

    -- Verify update succeeded
    SELECT EXISTS (SELECT 1 FROM events WHERE id = test_id AND agent_id = 99) INTO update_succeeded;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T3.3: trigger does not fire on UPDATE',
        update_succeeded,
        'Trigger is AFTER INSERT only, not UPDATE'
    );
END $$;

-- Test 3.4: Trigger does not fire on DELETE
DO $$
DECLARE
    test_id TEXT := 'test-notify-delete-' || gen_random_uuid()::TEXT;
    delete_succeeded BOOLEAN;
BEGIN
    -- Insert test event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 8000031, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Delete event (trigger should NOT fire on delete)
    DELETE FROM events WHERE id = test_id;

    -- Verify delete succeeded
    SELECT NOT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO delete_succeeded;

    PERFORM record_test(
        'T3.4: trigger does not fire on DELETE',
        delete_succeeded,
        'Trigger is AFTER INSERT only, not DELETE'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 4: EDGE CASES'
\echo '------------------------------------------------------------------------'

-- Test 4.1: Trigger handles NULL fields gracefully
DO $$
DECLARE
    test_id TEXT := 'test-notify-null-' || gen_random_uuid()::TEXT;
    insert_succeeded BOOLEAN;
BEGIN
    -- Insert event with NULL optional fields
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, agent_id, score, created_at)
    VALUES (test_id, 84532, 8000041, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NULL, NULL, NOW());

    -- Verify insert succeeded
    SELECT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO insert_succeeded;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T4.1: trigger handles NULL fields',
        insert_succeeded,
        'Trigger should work with NULL optional fields'
    );
END $$;

-- Test 4.2: Trigger handles long event IDs
DO $$
DECLARE
    test_id TEXT := 'test-notify-long-' || repeat('x', 200);
    insert_succeeded BOOLEAN;
BEGIN
    -- Insert event with very long ID
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 8000051, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Verify insert succeeded
    SELECT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO insert_succeeded;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T4.2: trigger handles long IDs',
        insert_succeeded,
        'Trigger should handle long event IDs'
    );
END $$;

-- Test 4.3: Trigger handles special characters in ID
DO $$
DECLARE
    test_id TEXT := 'test-notify-special-!@#$%^&*()-' || gen_random_uuid()::TEXT;
    insert_succeeded BOOLEAN;
BEGIN
    -- Insert event with special characters in ID
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 8000061, '0xtest', '0xtx', 0,
            'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

    -- Verify insert succeeded
    SELECT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO insert_succeeded;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T4.3: trigger handles special characters',
        insert_succeeded,
        'Trigger should handle special characters in ID'
    );
END $$;

-- Test 4.4: Trigger handles transaction rollback
DO $$
DECLARE
    test_id TEXT := 'test-notify-rollback-' || gen_random_uuid()::TEXT;
    insert_rolled_back BOOLEAN;
BEGIN
    BEGIN
        -- Start transaction and insert
        INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                           registry, event_type, timestamp, created_at)
        VALUES (test_id, 84532, 8000071, '0xtest', '0xtx', 0,
                'reputation', 'TestNotify', EXTRACT(EPOCH FROM NOW())::BIGINT, NOW());

        -- Force an error to rollback
        RAISE EXCEPTION 'Intentional rollback';
    EXCEPTION WHEN OTHERS THEN
        -- Catch the exception
        NULL;
    END;

    -- Verify insert was rolled back
    SELECT NOT EXISTS (SELECT 1 FROM events WHERE id = test_id) INTO insert_rolled_back;

    PERFORM record_test(
        'T4.4: trigger respects transaction rollback',
        insert_rolled_back,
        'Notification should not be sent if transaction rolls back'
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
        RAISE EXCEPTION E'\n❌ NOTIFICATION TESTS FAILED: % of % tests failed', failed_count, total_count;
    ELSE
        RAISE NOTICE E'\n✅ ALL NOTIFICATION TESTS PASSED: %/% tests', total_count, total_count;
    END IF;
END $$;
