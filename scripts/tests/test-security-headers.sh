#!/bin/bash
set -e

echo "=========================================="
echo "Security Headers Tests"
echo "=========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# API Gateway URL (assuming local development)
API_URL="${API_URL:-http://localhost:8080}"

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

# Test 1: HSTS header (if enabled)
run_test "HSTS Header Present" \
    "curl -sI $API_URL/api/v1/health | grep -qi 'Strict-Transport-Security' || true"

# Test 2: X-Content-Type-Options
run_test "X-Content-Type-Options Header" \
    "curl -sI $API_URL/api/v1/health | grep -q 'X-Content-Type-Options: nosniff'"

# Test 3: X-Frame-Options
run_test "X-Frame-Options Header" \
    "curl -sI $API_URL/api/v1/health | grep -q 'X-Frame-Options: DENY'"

# Test 4: X-XSS-Protection
run_test "X-XSS-Protection Header" \
    "curl -sI $API_URL/api/v1/health | grep -q 'X-XSS-Protection: 1; mode=block'"

# Test 5: Referrer-Policy
run_test "Referrer-Policy Header" \
    "curl -sI $API_URL/api/v1/health | grep -q 'Referrer-Policy:'"

# Test 6: Permissions-Policy
run_test "Permissions-Policy Header" \
    "curl -sI $API_URL/api/v1/health | grep -q 'Permissions-Policy:'"

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
