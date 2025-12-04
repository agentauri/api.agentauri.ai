-- ============================================================================
-- DATA INTEGRITY TESTS
-- ============================================================================
-- Description: Tests for constraints, foreign keys, and data integrity rules
-- Purpose: Verify database enforces data integrity correctly
-- Usage: psql -d test_agentauri_backend -f database/tests/test-data-integrity.sql
-- ============================================================================

\set ON_ERROR_STOP on
\timing off
\pset pager off

\echo ''
\echo '========================================================================'
\echo 'DATA INTEGRITY TEST SUITE'
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
\echo 'TEST GROUP 1: FOREIGN KEY CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 1.1: Verify trigger without user fails
DO $$
DECLARE
    fk_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
        VALUES ('test-fk-1', 'nonexistent-user', 'Test', 84532, 'reputation', 'nonexistent-org');
    EXCEPTION WHEN foreign_key_violation THEN
        fk_violated := true;
    END;

    PERFORM record_test(
        'T1.1: trigger without valid user/org fails',
        fk_violated,
        'Foreign key constraint should prevent orphaned triggers'
    );
END $$;

-- Test 1.2: Verify trigger_conditions without trigger fails
DO $$
DECLARE
    fk_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value)
        VALUES ('nonexistent-trigger', 'test', 'test', '=', '1');
    EXCEPTION WHEN foreign_key_violation THEN
        fk_violated := true;
    END;

    PERFORM record_test(
        'T1.2: condition without valid trigger fails',
        fk_violated,
        'Foreign key constraint should prevent orphaned conditions'
    );
END $$;

-- Test 1.3: Verify trigger_actions without trigger fails
DO $$
DECLARE
    fk_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO trigger_actions (trigger_id, action_type, config)
        VALUES ('nonexistent-trigger', 'telegram', '{"test": true}'::jsonb);
    EXCEPTION WHEN foreign_key_violation THEN
        fk_violated := true;
    END;

    PERFORM record_test(
        'T1.3: action without valid trigger fails',
        fk_violated,
        'Foreign key constraint should prevent orphaned actions'
    );
END $$;

-- Test 1.4: Verify trigger_state without trigger fails
DO $$
DECLARE
    fk_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO trigger_state (trigger_id, state_data)
        VALUES ('nonexistent-trigger', '{}'::jsonb);
    EXCEPTION WHEN foreign_key_violation THEN
        fk_violated := true;
    END;

    PERFORM record_test(
        'T1.4: state without valid trigger fails',
        fk_violated,
        'Foreign key constraint should prevent orphaned state'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 2: CASCADE DELETE BEHAVIOR'
\echo '------------------------------------------------------------------------'

-- Test 2.1: Delete user cascades to triggers
DO $$
DECLARE
    test_user_id TEXT := 'test-cascade-user-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-cascade-org-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-cascade-trigger-' || gen_random_uuid()::TEXT;
    trigger_exists_after BOOLEAN;
BEGIN
    -- Create user
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'cascade_test_' || substring(test_user_id, 1, 8),
            'cascade_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    -- Create organization
    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org', 'test-org-' || substring(test_org_id, 1, 8), test_user_id, true);

    -- Create trigger
    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Cascade Test', 84532, 'reputation', test_org_id);

    -- Delete organization first (required due to ON DELETE RESTRICT on owner_id)
    -- This will cascade delete the trigger via organization_id FK
    DELETE FROM organizations WHERE id = test_org_id;

    -- Check if trigger was deleted (via organization cascade)
    SELECT EXISTS (SELECT 1 FROM triggers WHERE id = test_trigger_id) INTO trigger_exists_after;

    -- Cleanup: delete user
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T2.1: delete organization cascades to triggers',
        NOT trigger_exists_after,
        'Trigger should be deleted when organization is deleted'
    );
END $$;

-- Test 2.2: Delete trigger cascades to conditions
DO $$
DECLARE
    test_user_id TEXT := 'test-cascade-user2-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-cascade-org2-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-cascade-trigger2-' || gen_random_uuid()::TEXT;
    condition_id INTEGER;
    condition_exists_after BOOLEAN;
BEGIN
    -- Create user, organization and trigger
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'cascade_test2_' || substring(test_user_id, 1, 8),
            'cascade2_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 2', 'test-org2-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Cascade Test 2', 84532, 'reputation', test_org_id);

    -- Create condition
    INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value)
    VALUES (test_trigger_id, 'test', 'test', '=', '1')
    RETURNING id INTO condition_id;

    -- Delete trigger
    DELETE FROM triggers WHERE id = test_trigger_id;

    -- Check if condition was deleted
    SELECT EXISTS (SELECT 1 FROM trigger_conditions WHERE id = condition_id) INTO condition_exists_after;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T2.2: delete trigger cascades to conditions',
        NOT condition_exists_after,
        'Condition should be deleted when trigger is deleted'
    );
END $$;

-- Test 2.3: Delete trigger cascades to actions
DO $$
DECLARE
    test_user_id TEXT := 'test-cascade-user3-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-cascade-org3-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-cascade-trigger3-' || gen_random_uuid()::TEXT;
    action_id INTEGER;
    action_exists_after BOOLEAN;
BEGIN
    -- Create user, organization and trigger
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'cascade_test3_' || substring(test_user_id, 1, 8),
            'cascade3_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 3', 'test-org3-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Cascade Test 3', 84532, 'reputation', test_org_id);

    -- Create action
    INSERT INTO trigger_actions (trigger_id, action_type, config)
    VALUES (test_trigger_id, 'telegram', '{"chat_id": "123"}'::jsonb)
    RETURNING id INTO action_id;

    -- Delete trigger
    DELETE FROM triggers WHERE id = test_trigger_id;

    -- Check if action was deleted
    SELECT EXISTS (SELECT 1 FROM trigger_actions WHERE id = action_id) INTO action_exists_after;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T2.3: delete trigger cascades to actions',
        NOT action_exists_after,
        'Action should be deleted when trigger is deleted'
    );
END $$;

-- Test 2.4: Delete trigger cascades to state
DO $$
DECLARE
    test_user_id TEXT := 'test-cascade-user4-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-cascade-org4-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-cascade-trigger4-' || gen_random_uuid()::TEXT;
    state_exists_after BOOLEAN;
BEGIN
    -- Create user, organization and trigger
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'cascade_test4_' || substring(test_user_id, 1, 8),
            'cascade4_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 4', 'test-org4-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Cascade Test 4', 84532, 'reputation', test_org_id);

    -- Create state
    INSERT INTO trigger_state (trigger_id, state_data)
    VALUES (test_trigger_id, '{"count": 5}'::jsonb);

    -- Delete trigger
    DELETE FROM triggers WHERE id = test_trigger_id;

    -- Check if state was deleted
    SELECT EXISTS (SELECT 1 FROM trigger_state WHERE trigger_id = test_trigger_id) INTO state_exists_after;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T2.4: delete trigger cascades to state',
        NOT state_exists_after,
        'State should be deleted when trigger is deleted'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 3: CHECK CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 3.1: Invalid registry value fails
DO $$
DECLARE
    check_violated BOOLEAN := false;
    test_user_id TEXT := 'test-check-user1-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-check-org1-' || gen_random_uuid()::TEXT;
BEGIN
    -- Create user and organization
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'check_test1_' || substring(test_user_id, 1, 8),
            'check1_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org', 'test-org-' || substring(test_org_id, 1, 8), test_user_id, true);

    BEGIN
        INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
        VALUES ('test-check-1', test_user_id, 'Test', 84532, 'invalid_registry', test_org_id);
    EXCEPTION WHEN check_violation THEN
        check_violated := true;
    END;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T3.1: invalid registry value fails',
        check_violated,
        'Registry must be identity, reputation, or validation'
    );
END $$;

-- Test 3.2: Invalid action_type value fails
DO $$
DECLARE
    check_violated BOOLEAN := false;
    test_user_id TEXT := 'test-check-user2-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-check-org2-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-check-trigger2-' || gen_random_uuid()::TEXT;
BEGIN
    -- Create user, organization and trigger
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'check_test2_' || substring(test_user_id, 1, 8),
            'check2_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 2', 'test-org2-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Test', 84532, 'reputation', test_org_id);

    BEGIN
        INSERT INTO trigger_actions (trigger_id, action_type, config)
        VALUES (test_trigger_id, 'invalid_action', '{}'::jsonb);
    EXCEPTION WHEN check_violation THEN
        check_violated := true;
    END;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T3.2: invalid action_type fails',
        check_violated,
        'action_type must be telegram, rest, or mcp'
    );
END $$;

-- Test 3.3: Invalid action_results status fails
DO $$
DECLARE
    check_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO action_results (id, job_id, action_type, status)
        VALUES ('test-check-3', 'job-123', 'telegram', 'invalid_status');
    EXCEPTION WHEN check_violation THEN
        check_violated := true;
    END;

    PERFORM record_test(
        'T3.3: invalid status value fails',
        check_violated,
        'status must be success, failed, or retrying'
    );
END $$;

-- Test 3.4: Valid registry values succeed
DO $$
DECLARE
    test_user_id TEXT := 'test-check-user4-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-check-org4-' || gen_random_uuid()::TEXT;
    insert_count INTEGER := 0;
BEGIN
    -- Create user and organization
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'check_test4_' || substring(test_user_id, 1, 8),
            'check4_' || substring(test_user_id, 1, 8) || '@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 4', 'test-org4-' || substring(test_org_id, 1, 8), test_user_id, true);

    -- Try all valid registries
    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES
        ('test-check-4-1', test_user_id, 'Test Identity', 84532, 'identity', test_org_id),
        ('test-check-4-2', test_user_id, 'Test Reputation', 84532, 'reputation', test_org_id),
        ('test-check-4-3', test_user_id, 'Test Validation', 84532, 'validation', test_org_id);

    SELECT COUNT(*) INTO insert_count FROM triggers WHERE user_id = test_user_id;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T3.4: all valid registry values succeed',
        insert_count = 3,
        'Should accept identity, reputation, and validation'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 4: UNIQUE CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 4.1: Duplicate email fails
DO $$
DECLARE
    unique_violated BOOLEAN := false;
    test_email TEXT := 'duplicate_' || gen_random_uuid()::TEXT || '@test.com';
BEGIN
    -- Insert first user
    INSERT INTO users (id, username, email, password_hash)
    VALUES ('test-unique-1a', 'user1a', test_email, 'hash123');

    -- Try duplicate email
    BEGIN
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test-unique-1b', 'user1b', test_email, 'hash123');
    EXCEPTION WHEN unique_violation THEN
        unique_violated := true;
    END;

    -- Clean up
    DELETE FROM users WHERE email = test_email;

    PERFORM record_test(
        'T4.1: duplicate email fails',
        unique_violated,
        'Email must be unique across users'
    );
END $$;

-- Test 4.2: Duplicate username fails
DO $$
DECLARE
    unique_violated BOOLEAN := false;
    test_username TEXT := 'duplicate_user_' || substring(gen_random_uuid()::TEXT, 1, 8);
BEGIN
    -- Insert first user
    INSERT INTO users (id, username, email, password_hash)
    VALUES ('test-unique-2a', test_username, 'email2a_' || gen_random_uuid()::TEXT || '@test.com', 'hash123');

    -- Try duplicate username
    BEGIN
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test-unique-2b', test_username, 'email2b_' || gen_random_uuid()::TEXT || '@test.com', 'hash123');
    EXCEPTION WHEN unique_violation THEN
        unique_violated := true;
    END;

    -- Clean up
    DELETE FROM users WHERE username = test_username;

    PERFORM record_test(
        'T4.2: duplicate username fails',
        unique_violated,
        'Username must be unique across users'
    );
END $$;

-- Test 4.3: Duplicate event ID in same time fails
DO $$
DECLARE
    unique_violated BOOLEAN := false;
    test_id TEXT := 'test-unique-3-' || gen_random_uuid()::TEXT;
    test_time TIMESTAMPTZ := NOW();
BEGIN
    -- Insert first event
    INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                       registry, event_type, timestamp, created_at)
    VALUES (test_id, 84532, 1000, '0xabc', '0xdef', 0,
            'reputation', 'TestEvent', EXTRACT(EPOCH FROM test_time)::BIGINT, test_time);

    -- Try duplicate (same id and created_at)
    BEGIN
        INSERT INTO events (id, chain_id, block_number, block_hash, transaction_hash, log_index,
                           registry, event_type, timestamp, created_at)
        VALUES (test_id, 84532, 1001, '0xabc2', '0xdef2', 0,
                'reputation', 'TestEvent2', EXTRACT(EPOCH FROM test_time)::BIGINT, test_time);
    EXCEPTION WHEN unique_violation THEN
        unique_violated := true;
    END;

    -- Clean up
    DELETE FROM events WHERE id = test_id;

    PERFORM record_test(
        'T4.3: duplicate event (id, created_at) fails',
        unique_violated,
        'Composite primary key should prevent duplicates'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 5: NOT NULL CONSTRAINTS'
\echo '------------------------------------------------------------------------'

-- Test 5.1: NULL username fails
DO $$
DECLARE
    not_null_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test-null-1', NULL, 'test_null1@test.com', 'hash123');
    EXCEPTION WHEN not_null_violation THEN
        not_null_violated := true;
    END;

    PERFORM record_test(
        'T5.1: NULL username fails',
        not_null_violated,
        'Username cannot be NULL'
    );
END $$;

-- Test 5.2: NULL email fails
DO $$
DECLARE
    not_null_violated BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test-null-2', 'testnull2', NULL, 'hash123');
    EXCEPTION WHEN not_null_violation THEN
        not_null_violated := true;
    END;

    PERFORM record_test(
        'T5.2: NULL email fails',
        not_null_violated,
        'Email cannot be NULL'
    );
END $$;

-- Test 5.3: NULL registry fails
DO $$
DECLARE
    not_null_violated BOOLEAN := false;
    test_user_id TEXT := 'test-null-user3-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-null-org3-' || gen_random_uuid()::TEXT;
BEGIN
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'testnull3', 'testnull3@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org', 'test-org-' || substring(test_org_id, 1, 8), test_user_id, true);

    BEGIN
        INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
        VALUES ('test-null-3', test_user_id, 'Test', 84532, NULL, test_org_id);
    EXCEPTION WHEN not_null_violation THEN
        not_null_violated := true;
    END;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T5.3: NULL registry fails',
        not_null_violated,
        'Registry cannot be NULL'
    );
END $$;

-- Test 5.4: NULL config in trigger_actions fails
DO $$
DECLARE
    not_null_violated BOOLEAN := false;
    test_user_id TEXT := 'test-null-user4-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-null-org4-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-null-trigger4-' || gen_random_uuid()::TEXT;
BEGIN
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'testnull4', 'testnull4@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 4', 'test-org4-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Test', 84532, 'reputation', test_org_id);

    BEGIN
        INSERT INTO trigger_actions (trigger_id, action_type, config)
        VALUES (test_trigger_id, 'telegram', NULL);
    EXCEPTION WHEN not_null_violation THEN
        not_null_violated := true;
    END;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T5.4: NULL config in actions fails',
        not_null_violated,
        'Action config cannot be NULL'
    );
END $$;

\echo '------------------------------------------------------------------------'
\echo 'TEST GROUP 6: JSONB FIELD VALIDATION'
\echo '------------------------------------------------------------------------'

-- Test 6.1: Valid JSONB in trigger_conditions.config
DO $$
DECLARE
    test_user_id TEXT := 'test-jsonb-user1-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-jsonb-org1-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-jsonb-trigger1-' || gen_random_uuid()::TEXT;
    jsonb_valid BOOLEAN := false;
BEGIN
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'jsonb_test1', 'jsonb1@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org', 'test-org-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Test', 84532, 'reputation', test_org_id);

    INSERT INTO trigger_conditions (trigger_id, condition_type, field, operator, value, config)
    VALUES (test_trigger_id, 'ema_threshold', 'score', '>', '75',
            '{"window_size": 10, "alpha": 0.2}'::jsonb);

    SELECT EXISTS (
        SELECT 1 FROM trigger_conditions
        WHERE trigger_id = test_trigger_id
        AND config->>'window_size' = '10'
    ) INTO jsonb_valid;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T6.1: valid JSONB in conditions.config',
        jsonb_valid,
        'Should store and query JSONB config'
    );
END $$;

-- Test 6.2: Valid JSONB in trigger_actions.config
DO $$
DECLARE
    test_user_id TEXT := 'test-jsonb-user2-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-jsonb-org2-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-jsonb-trigger2-' || gen_random_uuid()::TEXT;
    jsonb_valid BOOLEAN := false;
BEGIN
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'jsonb_test2', 'jsonb2@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 2', 'test-org2-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Test', 84532, 'reputation', test_org_id);

    INSERT INTO trigger_actions (trigger_id, action_type, config)
    VALUES (test_trigger_id, 'rest',
            '{"method": "POST", "url": "https://example.com", "timeout_ms": 5000}'::jsonb);

    SELECT EXISTS (
        SELECT 1 FROM trigger_actions
        WHERE trigger_id = test_trigger_id
        AND config->>'method' = 'POST'
    ) INTO jsonb_valid;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T6.2: valid JSONB in actions.config',
        jsonb_valid,
        'Should store and query JSONB config'
    );
END $$;

-- Test 6.3: Invalid JSON fails
DO $$
DECLARE
    json_invalid BOOLEAN := false;
    test_user_id TEXT := 'test-jsonb-user3-' || gen_random_uuid()::TEXT;
    test_org_id TEXT := 'test-jsonb-org3-' || gen_random_uuid()::TEXT;
    test_trigger_id TEXT := 'test-jsonb-trigger3-' || gen_random_uuid()::TEXT;
BEGIN
    INSERT INTO users (id, username, email, password_hash)
    VALUES (test_user_id, 'jsonb_test3', 'jsonb3@test.com', 'hash123');

    INSERT INTO organizations (id, name, slug, owner_id, is_personal)
    VALUES (test_org_id, 'Test Org 3', 'test-org3-' || substring(test_org_id, 1, 8), test_user_id, true);

    INSERT INTO triggers (id, user_id, name, chain_id, registry, organization_id)
    VALUES (test_trigger_id, test_user_id, 'Test', 84532, 'reputation', test_org_id);

    BEGIN
        -- This should fail during string literal parsing
        EXECUTE 'INSERT INTO trigger_actions (trigger_id, action_type, config) VALUES ($1, $2, $3::jsonb)'
        USING test_trigger_id, 'telegram', '{invalid json}';
    EXCEPTION WHEN invalid_text_representation THEN
        json_invalid := true;
    END;

    -- Clean up (organization first due to ON DELETE RESTRICT)
    DELETE FROM organizations WHERE id = test_org_id;
    DELETE FROM users WHERE id = test_user_id;

    PERFORM record_test(
        'T6.3: invalid JSON fails',
        json_invalid,
        'Should reject malformed JSON'
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
        RAISE EXCEPTION E'\n❌ DATA INTEGRITY TESTS FAILED: % of % tests failed', failed_count, total_count;
    ELSE
        RAISE NOTICE E'\n✅ ALL DATA INTEGRITY TESTS PASSED: %/% tests', total_count, total_count;
    END IF;
END $$;
