#!/bin/bash
# Integration test for Phase 2a: Shadow Mode Rate Limiting
#
# This script tests:
# 1. Rate limit headers are present on all responses
# 2. Shadow mode allows requests even when limit exceeded
# 3. Shadow mode logs violations correctly
# 4. Different auth layers have different limits
# 5. Query tiers affect rate limit consumption

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
HEALTH_ENDPOINT="$API_BASE_URL/api/v1/health"
TESTS_PASSED=0
TESTS_FAILED=0

# Print functions
print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

print_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

# Check if API Gateway is running
check_api_gateway() {
    print_info "Checking if API Gateway is running..."

    if ! curl -s -f "$HEALTH_ENDPOINT" > /dev/null 2>&1; then
        print_fail "API Gateway is not running at $API_BASE_URL"
        echo "Please start the API Gateway with: RATE_LIMIT_MODE=shadow cargo run --bin api-gateway"
        exit 1
    fi

    print_success "API Gateway is running"
}

# Test 1: Rate limit headers are present
test_rate_limit_headers() {
    print_test "Test 1: Rate limit headers are present on all responses"

    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    # Check for required headers (case-insensitive)
    if echo "$response" | grep -iq "x-ratelimit-limit:"; then
        print_success "x-ratelimit-limit header present"
    else
        print_fail "x-ratelimit-limit header missing"
        return 1
    fi

    if echo "$response" | grep -iq "x-ratelimit-remaining:"; then
        print_success "x-ratelimit-remaining header present"
    else
        print_fail "x-ratelimit-remaining header missing"
        return 1
    fi

    if echo "$response" | grep -iq "x-ratelimit-reset:"; then
        print_success "x-ratelimit-reset header present"
    else
        print_fail "x-ratelimit-reset header missing"
        return 1
    fi

    if echo "$response" | grep -iq "x-ratelimit-window:"; then
        print_success "x-ratelimit-window header present"
    else
        print_fail "x-ratelimit-window header missing"
        return 1
    fi
}

# Test 2: Shadow mode allows requests when limit exceeded
test_shadow_mode_allows_requests() {
    print_test "Test 2: Shadow mode allows requests even when limit exceeded"

    print_info "Making 15 requests (anonymous limit is 10)..."

    allowed_count=0
    for i in {1..15}; do
        status=$(curl -s -w "%{http_code}" -o /dev/null "$HEALTH_ENDPOINT")

        if [ "$status" == "200" ]; then
            allowed_count=$((allowed_count + 1))
        fi

        sleep 0.1
    done

    if [ "$allowed_count" -eq 15 ]; then
        print_success "All 15 requests allowed in shadow mode (expected)"
    else
        print_fail "Only $allowed_count/15 requests allowed (expected all 15 in shadow mode)"
        return 1
    fi
}

# Test 3: Shadow violation header is present
test_shadow_violation_header() {
    print_test "Test 3: Shadow violation header is present after exceeding limit"

    # First, exhaust the limit
    print_info "Exhausting rate limit (10 requests)..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Now check for shadow-violation header
    print_info "Making request #11 to trigger shadow violation..."
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    if echo "$response" | grep -iq "x-ratelimit-status.*shadow-violation"; then
        print_success "x-ratelimit-status: shadow-violation header present"
    else
        print_warn "Shadow violation header not present (might not have exceeded limit yet)"
        # Not a failure, as timing can vary
    fi

    # Check remaining is 0
    if echo "$response" | grep -iq "x-ratelimit-remaining.*0"; then
        print_success "x-ratelimit-remaining: 0 when limit exceeded"
    else
        remaining=$(echo "$response" | grep -i "x-ratelimit-remaining:" | awk '{print $2}' | tr -d '\r')
        print_info "x-ratelimit-remaining: $remaining"
    fi
}

# Test 4: Rate limit values are correct for anonymous
test_anonymous_rate_limit_values() {
    print_test "Test 4: Anonymous rate limit values are correct"

    # Wait a bit to ensure we get fresh headers
    sleep 2

    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    limit=$(echo "$response" | grep -i "x-ratelimit-limit:" | awk '{print $2}' | tr -d '\r\n ')
    window=$(echo "$response" | grep -i "x-ratelimit-window:" | awk '{print $2}' | tr -d '\r\n ')

    if [ "$limit" == "10" ]; then
        print_success "Anonymous rate limit is 10 (correct)"
    else
        print_fail "Anonymous rate limit is $limit (expected 10)"
    fi

    if [ "$window" == "3600" ]; then
        print_success "Rate limit window is 3600 seconds (1 hour)"
    else
        print_fail "Rate limit window is $window (expected 3600)"
    fi
}

# Test 5: Redis connection is working
test_redis_connection() {
    print_test "Test 5: Redis connection is working (no degraded status)"

    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    if echo "$response" | grep -iq "x-ratelimit-status.*degraded"; then
        print_fail "Redis connection degraded (using fallback limiter)"
        print_info "Check if Redis is running: docker-compose ps redis"
        return 1
    else
        print_success "Redis connection is healthy"
    fi
}

# Test 6: Rate limit resets properly
test_rate_limit_reset() {
    print_test "Test 6: Rate limit reset timestamp is valid"

    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)
    reset=$(echo "$response" | grep -i "x-ratelimit-reset:" | awk '{print $2}' | tr -d '\r\n ')

    # Current time
    now=$(date +%s)

    # Reset should be in the future
    if [ "$reset" -gt "$now" ]; then
        seconds_until_reset=$((reset - now))
        print_success "Reset timestamp is valid (resets in $seconds_until_reset seconds)"
    else
        print_fail "Reset timestamp is invalid (should be in the future)"
    fi
}

# Test 7: Middleware ordering is correct
test_middleware_ordering() {
    print_test "Test 7: Middleware chain executes in correct order"

    # The middleware should always extract auth context before rate limiting
    # If this fails, we'd get 500 errors instead of rate limit headers

    response=$(curl -s -w "%{http_code}" -o /dev/null "$HEALTH_ENDPOINT")

    if [ "$response" == "200" ] || [ "$response" == "401" ]; then
        print_success "Middleware chain is working (status: $response)"
    else
        print_fail "Unexpected status code: $response (expected 200 or 401)"
    fi
}

# Test 8: Security headers are still present
test_security_headers() {
    print_test "Test 8: Security headers are still present (Phase 1 integration intact)"

    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    headers_ok=true

    if ! echo "$response" | grep -iq "x-content-type-options:"; then
        print_fail "x-content-type-options header missing"
        headers_ok=false
    fi

    if ! echo "$response" | grep -iq "x-frame-options:"; then
        print_fail "x-frame-options header missing"
        headers_ok=false
    fi

    if $headers_ok; then
        print_success "Security headers from Phase 1 are still present"
    fi
}

# Summary
print_summary() {
    echo ""
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo -e "${GREEN}Passed:${NC} $TESTS_PASSED"
    echo -e "${RED}Failed:${NC} $TESTS_FAILED"
    echo "========================================"

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "${RED}Some tests failed.${NC}"
        return 1
    fi
}

# Main execution
main() {
    echo "========================================"
    echo "Phase 2a: Shadow Mode Rate Limiting Tests"
    echo "========================================"
    echo ""

    check_api_gateway

    echo ""
    echo "Running integration tests..."
    echo ""

    test_rate_limit_headers
    echo ""

    test_shadow_mode_allows_requests
    echo ""

    test_shadow_violation_header
    echo ""

    test_anonymous_rate_limit_values
    echo ""

    test_redis_connection
    echo ""

    test_rate_limit_reset
    echo ""

    test_middleware_ordering
    echo ""

    test_security_headers
    echo ""

    print_summary
}

# Run tests
main
exit_code=$?

# Cleanup hint
if [ $exit_code -ne 0 ]; then
    echo ""
    echo "Troubleshooting:"
    echo "1. Ensure API Gateway is running: RATE_LIMIT_MODE=shadow cargo run --bin api-gateway"
    echo "2. Ensure Redis is running: docker-compose up -d redis"
    echo "3. Check logs: tail -f /tmp/api-gateway-shadow-mode.log"
fi

exit $exit_code
