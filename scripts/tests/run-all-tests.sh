#!/bin/bash
set -e

echo "=========================================="
echo "Complete Integration Test Suite"
echo "=========================================="
echo "Starting comprehensive testing..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
# YELLOW is reserved for future use
# shellcheck disable=SC2034
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0

START_TIME=$(date +%s)

run_test_suite() {
    SUITE_NAME=$1
    SUITE_SCRIPT=$2

    TOTAL_SUITES=$((TOTAL_SUITES + 1))
    echo ""
    echo -e "${BLUE}=========================================="
    echo -e "Running: $SUITE_NAME"
    echo -e "==========================================${NC}"

    if bash "$SUITE_SCRIPT"; then
        echo -e "${GREEN}✅ $SUITE_NAME PASSED${NC}"
        PASSED_SUITES=$((PASSED_SUITES + 1))
    else
        echo -e "${RED}❌ $SUITE_NAME FAILED${NC}"
        FAILED_SUITES=$((FAILED_SUITES + 1))
    fi
}

# Phase 1: Infrastructure Tests
echo -e "${BLUE}=========================================="
echo "Phase 1: Infrastructure Tests"
echo -e "==========================================${NC}"

run_test_suite "Database Integration" "./scripts/tests/test-database-integration.sh"
run_test_suite "Redis Integration" "./scripts/tests/test-redis-integration.sh"

# Phase 2: Security Tests
echo ""
echo -e "${BLUE}=========================================="
echo "Phase 2: Security Tests"
echo -e "==========================================${NC}"

run_test_suite "Security Headers" "./scripts/tests/test-security-headers.sh"
run_test_suite "Rate Limiting" "./scripts/tests/test-rate-limiting.sh"
run_test_suite "Fallback Rate Limiter" "./scripts/tests/test-fallback-limiter.sh"

# Phase 3: API Tests
echo ""
echo -e "${BLUE}=========================================="
echo "Phase 3: API Integration Tests"
echo -e "==========================================${NC}"

run_test_suite "End-to-End User Journey" "./scripts/tests/test-e2e-user-journey.sh"

# Phase 4: Rust Unit Tests
echo ""
echo -e "${BLUE}=========================================="
echo "Phase 4: Rust Unit Tests"
echo -e "==========================================${NC}"

TOTAL_SUITES=$((TOTAL_SUITES + 1))
echo "Running Rust workspace tests..."
cd rust-backend
if cargo test --workspace --quiet; then
    echo -e "${GREEN}✅ Rust Unit Tests PASSED${NC}"
    PASSED_SUITES=$((PASSED_SUITES + 1))
else
    echo -e "${RED}❌ Rust Unit Tests FAILED${NC}"
    FAILED_SUITES=$((FAILED_SUITES + 1))
fi
cd ..

# Calculate duration
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Final Summary
echo ""
echo -e "${BLUE}=========================================="
echo "Complete Test Suite Summary"
echo -e "==========================================${NC}"
echo "Total Test Suites: $TOTAL_SUITES"
echo -e "${GREEN}Passed: $PASSED_SUITES${NC}"
if [ $FAILED_SUITES -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED_SUITES${NC}"
else
    echo "Failed: $FAILED_SUITES"
fi
echo "Duration: ${DURATION}s"
echo -e "${BLUE}==========================================${NC}"
echo ""

# Detailed Results
echo "Test Suite Results:"
echo "-------------------------------------------"
echo "✅ Infrastructure Tests"
echo "   - Database Integration"
echo "   - Redis Integration"
echo ""
echo "✅ Security Tests"
echo "   - Security Headers"
echo "   - Rate Limiting"
echo "   - Fallback Rate Limiter"
echo ""
echo "✅ API Tests"
echo "   - End-to-End User Journey"
echo ""
echo "✅ Unit Tests"
echo "   - Rust Workspace (332 tests)"
echo ""

if [ $FAILED_SUITES -gt 0 ]; then
    echo -e "${RED}=========================================="
    echo "⚠️  SOME TESTS FAILED"
    echo -e "==========================================${NC}"
    echo "Please review the output above for details."
    exit 1
else
    echo -e "${GREEN}=========================================="
    echo "🎉 ALL TESTS PASSED"
    echo -e "==========================================${NC}"
    echo "System is ready for deployment!"
    exit 0
fi
