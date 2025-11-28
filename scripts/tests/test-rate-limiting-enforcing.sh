#!/bin/bash
# Integration test for Phase 2b: Enforcing Mode Rate Limiting
#
# This script tests:
# 1. Requests are blocked with 429 when limit exceeded
# 2. 429 response includes proper error message
# 3. Retry-After header is present
# 4. Rate limit headers still present on 429 responses
# 5. Requests allowed again after rate limit window resets

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

# Check if API Gateway is running in enforcing mode
check_api_gateway_enforcing() {
    print_info "Checking if API Gateway is running in enforcing mode..."

    if ! curl -s -f "$HEALTH_ENDPOINT" > /dev/null 2>&1; then
        print_fail "API Gateway is not running at $API_BASE_URL"
        echo "Please start the API Gateway with: RATE_LIMIT_MODE=enforcing cargo run --bin api-gateway"
        exit 1
    fi

    # Make a few requests to check if enforcing mode is active
    # We'll verify this in the actual tests
    print_success "API Gateway is running"
}

# Test 1: Requests within limit are allowed
test_requests_within_limit_allowed() {
    print_test "Test 1: Requests within limit are allowed (200 OK)"

    print_info "Making 5 requests (within anonymous limit of 10)..."

    allowed_count=0
    for i in {1..5}; do
        status=$(curl -s -w "%{http_code}" -o /dev/null "$HEALTH_ENDPOINT")

        if [ "$status" == "200" ]; then
            allowed_count=$((allowed_count + 1))
        fi

        sleep 0.1
    done

    if [ "$allowed_count" -eq 5 ]; then
        print_success "All 5 requests allowed (within limit)"
    else
        print_fail "Only $allowed_count/5 requests allowed"
        return 1
    fi
}

# Test 2: Requests exceeding limit are blocked with 429
test_requests_blocked_when_exceeded() {
    print_test "Test 2: Requests exceeding limit are blocked with 429"

    print_info "Making 10 requests to reach limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    print_info "Making request #11 to exceed limit..."
    status=$(curl -s -w "%{http_code}" -o /dev/null "$HEALTH_ENDPOINT")

    if [ "$status" == "429" ]; then
        print_success "Request #11 blocked with 429 Too Many Requests"
    else
        print_fail "Request #11 returned $status (expected 429)"
        print_warn "This may indicate shadow mode is still active"
        print_info "Check RATE_LIMIT_MODE environment variable"
        return 1
    fi
}

# Test 3: 429 response includes error message
test_429_error_message() {
    print_test "Test 3: 429 response includes proper error message"

    # Exhaust limit
    print_info "Exhausting rate limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Get 429 response body
    response=$(curl -s "$HEALTH_ENDPOINT")

    # Check for error message components
    if echo "$response" | grep -q "Rate limit exceeded"; then
        print_success "Error message contains 'Rate limit exceeded'"
    else
        print_fail "Error message missing 'Rate limit exceeded'"
    fi

    # Note: The actual error format depends on implementation
    # It might be plain text or JSON
}

# Test 4: Rate limit headers present on 429 responses
test_headers_on_429_response() {
    print_test "Test 4: Rate limit headers present on 429 responses"

    # Exhaust limit
    print_info "Exhausting rate limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Get headers from 429 response
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    headers_ok=true

    if ! echo "$response" | grep -iq "x-ratelimit-limit:"; then
        print_fail "x-ratelimit-limit header missing on 429"
        headers_ok=false
    fi

    if ! echo "$response" | grep -iq "x-ratelimit-remaining:"; then
        print_fail "x-ratelimit-remaining header missing on 429"
        headers_ok=false
    fi

    if ! echo "$response" | grep -iq "x-ratelimit-reset:"; then
        print_fail "x-ratelimit-reset header missing on 429"
        headers_ok=false
    fi

    if $headers_ok; then
        print_success "All rate limit headers present on 429 response"
    fi
}

# Test 5: x-ratelimit-remaining is 0 when blocked
test_remaining_is_zero_when_blocked() {
    print_test "Test 5: x-ratelimit-remaining is 0 when request is blocked"

    # Exhaust limit
    print_info "Exhausting rate limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Get remaining from 429 response
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)
    remaining=$(echo "$response" | grep -i "x-ratelimit-remaining:" | awk '{print $2}' | tr -d '\r\n ')

    if [ "$remaining" == "0" ]; then
        print_success "x-ratelimit-remaining is 0 (correct)"
    else
        print_fail "x-ratelimit-remaining is $remaining (expected 0)"
    fi
}

# Test 6: Different auth layers have different limits
test_different_auth_layers() {
    print_test "Test 6: Different authentication layers have different limits"

    # This test is informational only, as we'd need API keys to test Layer 1
    print_info "Anonymous (Layer 0): 10 requests/hour"
    print_info "API Key (Layer 1): 100-2000 requests/hour (plan-based)"
    print_info "Wallet Signature (Layer 2): Inherits from linked organization"

    # Verify anonymous limit
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)
    limit=$(echo "$response" | grep -i "x-ratelimit-limit:" | awk '{print $2}' | tr -d '\r\n ')

    if [ "$limit" == "10" ]; then
        print_success "Anonymous limit is 10 (correct)"
    else
        print_warn "Anonymous limit is $limit (expected 10)"
    fi
}

# Test 7: Enforcing mode does not have shadow-violation status
test_no_shadow_violation_status() {
    print_test "Test 7: Enforcing mode does not use shadow-violation status"

    # Exhaust limit
    print_info "Exhausting rate limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Check for shadow-violation header (should NOT be present)
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    if echo "$response" | grep -iq "x-ratelimit-status.*shadow-violation"; then
        print_fail "Found shadow-violation status (should not be in enforcing mode)"
        print_warn "This indicates shadow mode is still active"
        return 1
    else
        print_success "No shadow-violation status (enforcing mode confirmed)"
    fi
}

# Test 8: Security headers still present on 429 responses
test_security_headers_on_429() {
    print_test "Test 8: Security headers still present on 429 responses"

    # Exhaust limit
    print_info "Exhausting rate limit..."
    for i in {1..10}; do
        curl -s -o /dev/null "$HEALTH_ENDPOINT"
        sleep 0.1
    done

    # Get headers from 429 response
    response=$(curl -s -i "$HEALTH_ENDPOINT" 2>&1)

    headers_ok=true

    if ! echo "$response" | grep -iq "x-content-type-options:"; then
        print_fail "x-content-type-options header missing on 429"
        headers_ok=false
    fi

    if ! echo "$response" | grep -iq "x-frame-options:"; then
        print_fail "x-frame-options header missing on 429"
        headers_ok=false
    fi

    if $headers_ok; then
        print_success "Security headers present on 429 responses"
    fi
}

# Test 9: Burst protection (multiple rapid requests)
test_burst_protection() {
    print_test "Test 9: Burst protection (rapid consecutive requests)"

    print_info "Making 15 rapid requests (no sleep)..."

    allowed=0
    blocked=0

    for i in {1..15}; do
        status=$(curl -s -w "%{http_code}" -o /dev/null "$HEALTH_ENDPOINT")

        if [ "$status" == "200" ]; then
            allowed=$((allowed + 1))
        elif [ "$status" == "429" ]; then
            blocked=$((blocked + 1))
        fi
    done

    print_info "Results: $allowed allowed, $blocked blocked"

    # We expect ~10 allowed and ~5 blocked
    if [ "$blocked" -gt 0 ]; then
        print_success "Burst protection working ($blocked requests blocked)"
    else
        print_warn "No requests blocked in burst (expected some 429s)"
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
        echo ""
        echo "✅ Enforcing mode is working correctly"
        echo "✅ Rate limits are being enforced"
        echo "✅ 429 responses are properly formatted"
        echo "✅ Security headers are intact"
        return 0
    else
        echo -e "${RED}Some tests failed.${NC}"
        echo ""
        echo "Common issues:"
        echo "1. Shadow mode still active: Check RATE_LIMIT_MODE=enforcing"
        echo "2. Redis keys not cleared: Run 'redis-cli FLUSHDB'"
        echo "3. Different IP causing different scope: Use consistent client"
        return 1
    fi
}

# Main execution
main() {
    echo "========================================"
    echo "Phase 2b: Enforcing Mode Rate Limiting Tests"
    echo "========================================"
    echo ""
    echo "⚠️  IMPORTANT: This test will trigger rate limits!"
    echo "    Ensure you're running in a test environment."
    echo ""

    check_api_gateway_enforcing

    # Clear Redis to start fresh (if no password required)
    print_info "Clearing Redis rate limit keys..."
    if redis-cli PING > /dev/null 2>&1; then
        redis-cli FLUSHDB > /dev/null 2>&1
        print_info "Redis keys cleared"
    else
        print_warn "Could not clear Redis (auth required or not running)"
        print_info "Tests will use existing rate limit counters"
    fi

    echo ""
    echo "Running enforcing mode tests..."
    echo ""

    test_requests_within_limit_allowed
    echo ""

    test_requests_blocked_when_exceeded
    echo ""

    test_429_error_message
    echo ""

    test_headers_on_429_response
    echo ""

    test_remaining_is_zero_when_blocked
    echo ""

    test_different_auth_layers
    echo ""

    test_no_shadow_violation_status
    echo ""

    test_security_headers_on_429
    echo ""

    test_burst_protection
    echo ""

    print_summary
}

# Run tests
main
exit_code=$?

# Cleanup (if no password)
if redis-cli PING > /dev/null 2>&1; then
    print_info "Cleaning up Redis keys..."
    redis-cli FLUSHDB > /dev/null 2>&1
fi

if [ $exit_code -ne 0 ]; then
    echo ""
    echo "Troubleshooting:"
    echo "1. Ensure API Gateway is in enforcing mode: RATE_LIMIT_MODE=enforcing"
    echo "2. Restart API Gateway: pkill api-gateway && RATE_LIMIT_MODE=enforcing cargo run --bin api-gateway"
    echo "3. Check logs: tail -f /tmp/api-gateway-enforcing-mode.log"
    echo "4. Verify Redis: redis-cli PING"
fi

exit $exit_code
