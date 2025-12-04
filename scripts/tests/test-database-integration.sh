#!/bin/bash
set -e

echo "=========================================="
echo "Database Integration Tests"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Database connection
DB_HOST="localhost"
DB_PORT="5432"
DB_USER="postgres"
DB_NAME="agentauri_backend"

# Database password MUST be set via PGPASSWORD environment variable
# Do NOT hardcode passwords in this script for security reasons
if [ -z "$PGPASSWORD" ]; then
    echo -e "${RED}ERROR: PGPASSWORD environment variable is not set${NC}"
    echo "Please set your database password:"
    echo "  export PGPASSWORD='your_password'"
    echo ""
    echo "For local development, see database/README.md for setup instructions."
    exit 1
fi

# Note: PGPASSWORD is used directly by psql, no need for DB_PASSWORD variable

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

run_test() {
    TEST_NAME=$1
    TEST_COMMAND=$2

    TESTS_RUN=$((TESTS_RUN + 1))
    echo -e "\n${YELLOW}Test $TESTS_RUN: $TEST_NAME${NC}"

    if eval "$TEST_COMMAND"; then
        echo -e "${GREEN}✅ PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAILED${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

# Test 1: Database connection
run_test "Database Connection" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c 'SELECT 1;' > /dev/null 2>&1"

# Test 2: Check tables exist
run_test "Essential Tables Exist" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        SELECT COUNT(*) FROM information_schema.tables
        WHERE table_schema = 'public'
        AND table_name IN ('users', 'organizations', 'api_keys', 'triggers', 'oauth_clients', 'oauth_tokens', 'used_nonces');
    \" | grep -q '7'"

# Test 3: Check foreign keys
run_test "Foreign Key Constraints" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        SELECT COUNT(*) FROM pg_constraint
        WHERE contype = 'f'
        AND conrelid::regclass::text LIKE '%organizations%';
    \" | grep -q '[1-9]'"

# Test 4: Check indexes
run_test "Indexes on Critical Tables" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        SELECT COUNT(*) FROM pg_indexes
        WHERE tablename IN ('organizations', 'api_keys', 'triggers', 'used_nonces');
    \" | grep -q '[1-9]'"

# Test 5: Check OAuth tables structure
run_test "OAuth Tables Structure" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        SELECT column_name FROM information_schema.columns
        WHERE table_name = 'oauth_tokens'
        AND column_name IN ('access_token_hash', 'refresh_token_hash');
    \" | grep -q 'access_token_hash'"

# Test 6: Check used_nonces table for expired entries cleanup
run_test "Used Nonces Table Structure" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        SELECT column_name FROM information_schema.columns
        WHERE table_name = 'used_nonces'
        AND column_name = 'expires_at';
    \" | grep -q 'expires_at'"

# Test 7: Test data insertion (cleanup after)
run_test "Data Insertion and Cleanup" \
    "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \"
        INSERT INTO users (id, username, email, password_hash)
        VALUES ('test_user_id', 'testuser', 'test@test.com', 'hash')
        ON CONFLICT (id) DO NOTHING;
        DELETE FROM users WHERE id = 'test_user_id';
    \" > /dev/null 2>&1"

# Summary
echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "Tests Run: $TESTS_RUN"
echo -e "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Tests Failed: $TESTS_FAILED${NC}"
else
    echo "Tests Failed: $TESTS_FAILED"
fi
echo "=========================================="

# Exit with appropriate code
if [ $TESTS_FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi
