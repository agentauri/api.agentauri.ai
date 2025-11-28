#!/bin/bash
set -e

echo "=========================================="
echo "Redis Integration Tests"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Redis password from environment or default
REDIS_PASSWORD="${REDIS_PASSWORD:-rXIuUZ6EiOZ34WAvEAnAlry9Qw01Nl9LPCJ8FVCTzCo=}"

# Redis CLI with auth
REDIS_CLI="redis-cli -a $REDIS_PASSWORD --no-auth-warning"

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

# Test 1: Redis connection
run_test "Redis PING" \
    "$REDIS_CLI ping | grep -q 'PONG'"

# Test 2: SET/GET operations
run_test "Redis SET/GET" \
    "$REDIS_CLI SET test_key 'test_value' > /dev/null && $REDIS_CLI GET test_key | grep -q 'test_value'"

# Test 3: TTL expiration
run_test "Redis TTL Expiration" \
    "$REDIS_CLI SET expiring_key 'value' EX 2 > /dev/null && sleep 3 && ! $REDIS_CLI GET expiring_key | grep -q 'value'"

# Test 4: Key deletion
run_test "Redis DEL" \
    "$REDIS_CLI SET delete_me 'value' > /dev/null && $REDIS_CLI DEL delete_me > /dev/null && ! $REDIS_CLI GET delete_me | grep -q 'value'"

# Test 5: Lua script execution
run_test "Lua Script Execution" \
    "$REDIS_CLI EVAL 'return {ARGV[1], ARGV[2]}' 0 'hello' 'world' | grep -q 'hello'"

# Test 6: Rate limiter Lua script (if file exists)
if [ -f "rust-backend/crates/shared/src/redis/rate_limit.lua" ]; then
    run_test "Rate Limiter Lua Script" \
        "$REDIS_CLI --eval rust-backend/crates/shared/src/redis/rate_limit.lua , rl:test:ip:127.0.0.1 100 1 \$(date +%s) | grep -q '1'"
fi

# Cleanup
$REDIS_CLI DEL test_key expiring_key delete_me > /dev/null 2>&1 || true
$REDIS_CLI KEYS "rl:test:*" | xargs -r $REDIS_CLI DEL > /dev/null 2>&1 || true

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

if [ $TESTS_FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi
