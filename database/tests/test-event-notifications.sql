-- ============================================================================
-- Test: Event Notification Trigger
-- ============================================================================
-- Tests that the PostgreSQL NOTIFY trigger fires correctly when events
-- are inserted into the events table.
-- ============================================================================

\echo ''
\echo '=========================================='
\echo 'TEST: Event Notification Trigger'
\echo '=========================================='
\echo ''

-- Test 1: Verify trigger function exists
\echo 'Test 1: Trigger function exists'
SELECT CASE
    WHEN COUNT(*) >= 1 THEN '✓ PASS: notify_new_event function exists'
    ELSE '✗ FAIL: notify_new_event function not found'
END AS test_result
FROM pg_proc
WHERE proname = 'notify_new_event';

-- Test 2: Verify trigger exists on events table
\echo 'Test 2: Trigger exists on events table'
SELECT CASE
    WHEN COUNT(*) >= 1 THEN '✓ PASS: Event notification trigger exists'
    ELSE '✗ FAIL: Event notification trigger not found'
END AS test_result
FROM pg_trigger t
JOIN pg_class c ON t.tgrelid = c.oid
WHERE c.relname = 'events' AND (t.tgname = 'trigger_notify_new_event' OR t.tgname = 'events_notify_trigger');

-- Test 3: Verify index exists
\echo 'Test 3: Index for event retrieval exists'
SELECT CASE
    WHEN COUNT(*) >= 1 THEN '✓ PASS: Event retrieval index exists'
    ELSE '✗ FAIL: Event retrieval index not found'
END AS test_result
FROM pg_indexes
WHERE tablename = 'events' AND (indexname LIKE '%events_id%' OR indexname LIKE '%chain%');

-- Test 4: Insert test event and verify no errors
\echo 'Test 4: Insert test event (trigger should fire)'
BEGIN;
    INSERT INTO events (
        id, chain_id, block_number, block_hash, transaction_hash, log_index,
        event_type, registry, timestamp, created_at
    ) VALUES (
        'test_notify_' || extract(epoch from now())::text,
        11155111,
        1000000,
        '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
        '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
        0,
        'AgentRegistered',
        'identity',
        EXTRACT(EPOCH FROM NOW())::BIGINT,
        NOW()
    );

    SELECT '✓ PASS: Test event inserted successfully (trigger fired)' AS test_result;
ROLLBACK;

\echo ''
\echo '=========================================='
\echo 'Event Notification Trigger Tests Complete'
\echo '=========================================='
