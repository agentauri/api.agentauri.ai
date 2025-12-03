-- ============================================================================
-- SCHEMA VALIDATION TESTS
-- ============================================================================
-- Description: Comprehensive tests for database schema integrity
-- Purpose: Verify all tables, columns, constraints, indexes, and triggers
-- Usage: psql -d test_erc8004_backend -f database/tests/test-schema.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'SCHEMA VALIDATION TEST SUITE'
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
\echo 'TEST GROUP 1: TABLE EXISTENCE'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Verify users table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.1: users table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users'),
        'users table should exist'
    );
END $$;

-- Test 1.2: Verify triggers table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.2: triggers table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'triggers'),
        'triggers table should exist'
    );
END $$;

-- Test 1.3: Verify trigger_conditions table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.3: trigger_conditions table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'trigger_conditions'),
        'trigger_conditions table should exist'
    );
END $$;

-- Test 1.4: Verify trigger_actions table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.4: trigger_actions table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'trigger_actions'),
        'trigger_actions table should exist'
    );
END $$;

-- Test 1.5: Verify trigger_state table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.5: trigger_state table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'trigger_state'),
        'trigger_state table should exist'
    );
END $$;

-- Test 1.6: Verify events table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.6: events table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'events'),
        'events table should exist'
    );
END $$;

-- Test 1.7: Verify checkpoints table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.7: checkpoints table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'checkpoints'),
        'checkpoints table should exist'
    );
END $$;

-- Test 1.8: Verify action_results table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.8: action_results table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'action_results'),
        'action_results table should exist'
    );
END $$;

-- Test 1.9: Verify agent_mcp_tokens table exists
DO $$
BEGIN
    PERFORM record_test(
        'T1.9: agent_mcp_tokens table exists',
        EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'agent_mcp_tokens'),
        'agent_mcp_tokens table should exist'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: COLUMN DATA TYPES'
\echo '------------------------------------------------------------------------'

-- Test 2.1: Verify users table columns
DO $$
DECLARE
    col_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO col_count
    FROM information_schema.columns
    WHERE table_name = 'users'
    AND column_name IN ('id', 'username', 'email', 'password_hash', 'created_at', 'updated_at', 'last_login_at', 'is_active')
    AND (
        (column_name = 'id' AND data_type = 'text') OR
        (column_name = 'username' AND data_type = 'text') OR
        (column_name = 'email' AND data_type = 'text') OR
        (column_name = 'password_hash' AND data_type = 'text') OR
        (column_name = 'created_at' AND data_type = 'timestamp with time zone') OR
        (column_name = 'updated_at' AND data_type = 'timestamp with time zone') OR
        (column_name = 'last_login_at' AND data_type = 'timestamp with time zone') OR
        (column_name = 'is_active' AND data_type = 'boolean')
    );

    PERFORM record_test(
        'T2.1: users table has correct columns',
        col_count = 8,
        format('Expected 8 columns with correct types, found %s', col_count)
    );
END $$;

-- Test 2.2: Verify triggers table columns
DO $$
DECLARE
    col_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO col_count
    FROM information_schema.columns
    WHERE table_name = 'triggers'
    AND column_name IN ('id', 'user_id', 'name', 'description', 'chain_id', 'registry', 'enabled', 'is_stateful', 'created_at', 'updated_at');

    PERFORM record_test(
        'T2.2: triggers table has correct columns',
        col_count = 10,
        format('Expected 10 columns, found %s', col_count)
    );
END $$;

-- Test 2.3: Verify events table has BIGINT for agent_id
DO $$
DECLARE
    correct_type BOOLEAN;
BEGIN
    SELECT data_type = 'bigint' INTO correct_type
    FROM information_schema.columns
    WHERE table_name = 'events' AND column_name = 'agent_id';

    PERFORM record_test(
        'T2.3: events.agent_id is BIGINT',
        correct_type,
        'agent_id should be BIGINT for large agent IDs'
    );
END $$;

-- Test 2.4: Verify JSONB columns exist
DO $$
DECLARE
    jsonb_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO jsonb_count
    FROM information_schema.columns
    WHERE data_type = 'jsonb'
    AND (
        (table_name = 'trigger_conditions' AND column_name = 'config') OR
        (table_name = 'trigger_actions' AND column_name = 'config') OR
        (table_name = 'trigger_state' AND column_name = 'state_data') OR
        (table_name = 'action_results' AND column_name = 'response_data')
    );

    PERFORM record_test(
        'T2.4: JSONB columns exist',
        jsonb_count = 4,
        format('Expected 4 JSONB columns, found %s', jsonb_count)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: PRIMARY KEYS'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Verify users primary key
DO $$
DECLARE
    pk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'users' AND constraint_type = 'PRIMARY KEY' AND constraint_name LIKE '%pkey'
    ) INTO pk_exists;

    PERFORM record_test(
        'T3.1: users has primary key',
        pk_exists,
        'users table should have PRIMARY KEY on id'
    );
END $$;

-- Test 3.2: Verify triggers primary key
DO $$
DECLARE
    pk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'triggers' AND constraint_type = 'PRIMARY KEY'
    ) INTO pk_exists;

    PERFORM record_test(
        'T3.2: triggers has primary key',
        pk_exists,
        'triggers table should have PRIMARY KEY on id'
    );
END $$;

-- Test 3.3: Verify events composite primary key
DO $$
DECLARE
    pk_cols INTEGER;
BEGIN
    SELECT COUNT(*) INTO pk_cols
    FROM information_schema.key_column_usage
    WHERE table_name = 'events'
    AND constraint_name LIKE '%pkey'
    AND column_name IN ('id', 'created_at');

    PERFORM record_test(
        'T3.3: events has composite primary key',
        pk_cols = 2,
        format('events should have composite PK (id, created_at), found %s columns', pk_cols)
    );
END $$;

-- Test 3.4: Verify trigger_state primary key
DO $$
DECLARE
    pk_column TEXT;
BEGIN
    SELECT column_name INTO pk_column
    FROM information_schema.key_column_usage
    WHERE table_name = 'trigger_state'
    AND constraint_name LIKE '%pkey';

    PERFORM record_test(
        'T3.4: trigger_state has primary key on trigger_id',
        pk_column = 'trigger_id',
        'trigger_state should have PRIMARY KEY on trigger_id'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 4: FOREIGN KEYS'
\echo '------------------------------------------------------------------------'

-- Test 4.1: Verify triggers.user_id foreign key
DO $$
DECLARE
    fk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
        WHERE tc.table_name = 'triggers'
        AND tc.constraint_type = 'FOREIGN KEY'
        AND kcu.column_name = 'user_id'
    ) INTO fk_exists;

    PERFORM record_test(
        'T4.1: triggers.user_id has foreign key',
        fk_exists,
        'triggers.user_id should reference users(id)'
    );
END $$;

-- Test 4.2: Verify trigger_conditions.trigger_id foreign key
DO $$
DECLARE
    fk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
        WHERE tc.table_name = 'trigger_conditions'
        AND tc.constraint_type = 'FOREIGN KEY'
        AND kcu.column_name = 'trigger_id'
    ) INTO fk_exists;

    PERFORM record_test(
        'T4.2: trigger_conditions.trigger_id has foreign key',
        fk_exists,
        'trigger_conditions.trigger_id should reference triggers(id)'
    );
END $$;

-- Test 4.3: Verify trigger_actions.trigger_id foreign key
DO $$
DECLARE
    fk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
        WHERE tc.table_name = 'trigger_actions'
        AND tc.constraint_type = 'FOREIGN KEY'
        AND kcu.column_name = 'trigger_id'
    ) INTO fk_exists;

    PERFORM record_test(
        'T4.3: trigger_actions.trigger_id has foreign key',
        fk_exists,
        'trigger_actions.trigger_id should reference triggers(id)'
    );
END $$;

-- Test 4.4: Verify trigger_state.trigger_id foreign key
DO $$
DECLARE
    fk_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
        WHERE tc.table_name = 'trigger_state'
        AND tc.constraint_type = 'FOREIGN KEY'
        AND kcu.column_name = 'trigger_id'
    ) INTO fk_exists;

    PERFORM record_test(
        'T4.4: trigger_state.trigger_id has foreign key',
        fk_exists,
        'trigger_state.trigger_id should reference triggers(id)'
    );
END $$;

-- Test 4.5: Verify action_results foreign key to triggers
DO $$
DECLARE
    fk_count INTEGER;
BEGIN
    SELECT COUNT(DISTINCT tc.constraint_name) INTO fk_count
    FROM information_schema.table_constraints tc
    WHERE tc.table_name = 'action_results'
    AND tc.constraint_type = 'FOREIGN KEY';

    PERFORM record_test(
        'T4.5: action_results has trigger foreign key',
        fk_count = 1,
        format('action_results should have 1 foreign key (trigger_id), found %s. Note: event_id cannot have FK due to composite key', fk_count)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 5: CHECK CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 5.1: Verify triggers.registry CHECK constraint
DO $$
DECLARE
    check_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.check_constraints
        WHERE constraint_name LIKE '%registry%'
        AND constraint_schema = 'public'
        AND check_clause LIKE '%identity%reputation%validation%'
    ) INTO check_exists;

    PERFORM record_test(
        'T5.1: triggers.registry has CHECK constraint',
        check_exists,
        'registry should be constrained to identity/reputation/validation'
    );
END $$;

-- Test 5.2: Verify trigger_actions.action_type CHECK constraint
DO $$
DECLARE
    check_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.check_constraints
        WHERE constraint_name LIKE '%action_type%'
        AND constraint_schema = 'public'
        AND check_clause LIKE '%telegram%rest%mcp%'
    ) INTO check_exists;

    PERFORM record_test(
        'T5.2: trigger_actions.action_type has CHECK constraint',
        check_exists,
        'action_type should be constrained to telegram/rest/mcp'
    );
END $$;

-- Test 5.3: Verify action_results.status CHECK constraint
DO $$
DECLARE
    check_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.check_constraints
        WHERE constraint_name LIKE '%status%'
        AND constraint_schema = 'public'
        AND check_clause LIKE '%success%failed%retrying%'
    ) INTO check_exists;

    PERFORM record_test(
        'T5.3: action_results.status has CHECK constraint',
        check_exists,
        'status should be constrained to success/failed/retrying'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 6: INDEXES'
\echo '------------------------------------------------------------------------'

-- Test 6.1: Verify users indexes
DO $$
DECLARE
    idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO idx_count
    FROM pg_indexes
    WHERE tablename = 'users'
    AND indexname IN ('idx_users_email', 'idx_users_username');

    PERFORM record_test(
        'T6.1: users has email and username indexes',
        idx_count = 2,
        format('Expected 2 indexes on users, found %s', idx_count)
    );
END $$;

-- Test 6.2: Verify triggers indexes
DO $$
DECLARE
    idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO idx_count
    FROM pg_indexes
    WHERE tablename = 'triggers'
    AND (indexname = 'idx_triggers_user_id' OR indexname = 'idx_triggers_org_chain_registry_enabled');

    PERFORM record_test(
        'T6.2: triggers has user_id and composite index',
        idx_count = 2,
        format('Expected 2 indexes on triggers, found %s', idx_count)
    );
END $$;

-- Test 6.3: Verify events indexes (at least 5)
DO $$
DECLARE
    idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO idx_count
    FROM pg_indexes
    WHERE tablename = 'events'
    AND indexname LIKE 'idx_events%';

    PERFORM record_test(
        'T6.3: events has multiple indexes',
        idx_count >= 5,
        format('Expected at least 5 indexes on events, found %s', idx_count)
    );
END $$;

-- Test 6.4: Verify partial indexes exist
DO $$
DECLARE
    partial_idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO partial_idx_count
    FROM pg_indexes
    WHERE (tablename = 'events' AND indexdef LIKE '%WHERE%')
    OR (tablename = 'triggers' AND indexdef LIKE '%WHERE%');

    PERFORM record_test(
        'T6.4: partial indexes exist',
        partial_idx_count >= 3,
        format('Expected at least 3 partial indexes, found %s', partial_idx_count)
    );
END $$;

-- Test 6.5: Verify action_results indexes
DO $$
DECLARE
    idx_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO idx_count
    FROM pg_indexes
    WHERE tablename = 'action_results'
    AND indexname LIKE 'idx_action_results%';

    PERFORM record_test(
        'T6.5: action_results has indexes',
        idx_count >= 4,
        format('Expected at least 4 indexes on action_results, found %s', idx_count)
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 7: TRIGGERS'
\echo '------------------------------------------------------------------------'

-- Test 7.1: Verify update_updated_at_column function exists
DO $$
DECLARE
    func_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_proc
        WHERE proname = 'update_updated_at_column'
    ) INTO func_exists;

    PERFORM record_test(
        'T7.1: update_updated_at_column function exists',
        func_exists,
        'Helper function for updated_at should exist'
    );
END $$;

-- Test 7.2: Verify notify_new_event function exists
DO $$
DECLARE
    func_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_proc
        WHERE proname = 'notify_new_event'
    ) INTO func_exists;

    PERFORM record_test(
        'T7.2: notify_new_event function exists',
        func_exists,
        'NOTIFY function for events should exist'
    );
END $$;

-- Test 7.3: Verify users has updated_at trigger
DO $$
DECLARE
    trigger_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'update_users_updated_at'
    ) INTO trigger_exists;

    PERFORM record_test(
        'T7.3: users has updated_at trigger',
        trigger_exists,
        'users table should have update_users_updated_at trigger'
    );
END $$;

-- Test 7.4: Verify triggers has updated_at trigger
DO $$
DECLARE
    trigger_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'update_triggers_updated_at'
    ) INTO trigger_exists;

    PERFORM record_test(
        'T7.4: triggers has updated_at trigger',
        trigger_exists,
        'triggers table should have update_triggers_updated_at trigger'
    );
END $$;

-- Test 7.5: Verify events has notify trigger
DO $$
DECLARE
    trigger_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname IN ('events_notify_trigger', 'trigger_notify_new_event')
    ) INTO trigger_exists;

    PERFORM record_test(
        'T7.5: events has notify trigger',
        trigger_exists,
        'events table should have events notification trigger'
    );
END $$;

-- Test 7.6: Verify agent_mcp_tokens has updated_at trigger
DO $$
DECLARE
    trigger_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'update_agent_mcp_tokens_updated_at'
    ) INTO trigger_exists;

    PERFORM record_test(
        'T7.6: agent_mcp_tokens has updated_at trigger',
        trigger_exists,
        'agent_mcp_tokens table should have update trigger'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 8: UNIQUE CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 8.1: Verify users.email is unique
DO $$
DECLARE
    unique_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'users'
        AND constraint_type = 'UNIQUE'
        AND constraint_name LIKE '%email%'
    ) INTO unique_exists;

    PERFORM record_test(
        'T8.1: users.email is UNIQUE',
        unique_exists,
        'email column should have UNIQUE constraint'
    );
END $$;

-- Test 8.2: Verify users.username is unique
DO $$
DECLARE
    unique_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'users'
        AND constraint_type = 'UNIQUE'
        AND constraint_name LIKE '%username%'
    ) INTO unique_exists;

    PERFORM record_test(
        'T8.2: users.username is UNIQUE',
        unique_exists,
        'username column should have UNIQUE constraint'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 9: NOT NULL CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 9.1: Verify critical NOT NULL constraints on users
-- Note: password_hash is nullable to support social-only authentication (Google, GitHub)
DO $$
DECLARE
    not_null_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO not_null_count
    FROM information_schema.columns
    WHERE table_name = 'users'
    AND is_nullable = 'NO'
    AND column_name IN ('id', 'username', 'email');

    PERFORM record_test(
        'T9.1: users has NOT NULL constraints',
        not_null_count = 3,
        format('Expected 3 NOT NULL columns on users (id, username, email), found %s', not_null_count)
    );
END $$;

-- Test 9.2: Verify critical NOT NULL constraints on triggers
DO $$
DECLARE
    not_null_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO not_null_count
    FROM information_schema.columns
    WHERE table_name = 'triggers'
    AND is_nullable = 'NO'
    AND column_name IN ('id', 'user_id', 'name', 'chain_id', 'registry');

    PERFORM record_test(
        'T9.2: triggers has NOT NULL constraints',
        not_null_count = 5,
        format('Expected 5 NOT NULL columns on triggers, found %s', not_null_count)
    );
END $$;

-- Test 9.3: Verify critical NOT NULL constraints on events
DO $$
DECLARE
    not_null_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO not_null_count
    FROM information_schema.columns
    WHERE table_name = 'events'
    AND is_nullable = 'NO'
    AND column_name IN ('id', 'chain_id', 'block_number', 'registry', 'event_type', 'timestamp');

    PERFORM record_test(
        'T9.3: events has NOT NULL constraints',
        not_null_count >= 5,
        format('Expected at least 5 NOT NULL columns on events, found %s', not_null_count)
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
        RAISE EXCEPTION E'\n❌ SCHEMA VALIDATION FAILED: % of % tests failed', failed_count, total_count;
    ELSE
        RAISE NOTICE E'\n✅ ALL SCHEMA VALIDATION TESTS PASSED: %/% tests', total_count, total_count;
    END IF;
END $$;
