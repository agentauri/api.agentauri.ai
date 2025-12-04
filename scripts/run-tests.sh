#!/bin/bash

# ============================================================================
# MASTER TEST RUNNER
# ============================================================================
# Description: Runs all tests across the entire project
# Usage: ./scripts/run-tests.sh
# ============================================================================

set -e  # Exit on error

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0

print_header() {
    echo ""
    echo -e "${BLUE}========================================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_fail() {
    echo -e "${RED}[✗]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

run_test_suite() {
    local suite_name=$1
    local test_command=$2

    ((TOTAL_SUITES++))
    print_info "Running: $suite_name"

    if eval "$test_command" > /dev/null 2>&1; then
        print_success "$suite_name passed"
        ((PASSED_SUITES++))
        return 0
    else
        print_fail "$suite_name failed"
        ((FAILED_SUITES++))
        # Show error details
        echo -e "${YELLOW}Error details:${NC}"
        eval "$test_command" 2>&1 | tail -20
        return 1
    fi
}

# ============================================================================
# MAIN TEST EXECUTION
# ============================================================================

print_header "RUNNING ALL PROJECT TESTS"

# Check prerequisites
print_info "Checking prerequisites..."

# Check Docker
if ! docker ps | grep -q agentauri-postgres; then
    print_fail "Docker container agentauri-postgres is not running"
    print_info "Start it with: docker-compose up -d"
    exit 1
fi
print_success "Docker container is running"

# ============================================================================
# DATABASE TESTS
# ============================================================================

print_header "DATABASE TESTS"

DB_CONTAINER="agentauri-postgres"
DB_NAME="agentauri_backend"
DB_USER="postgres"

# Test 1: Schema validation
run_test_suite "Database Schema Validation" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-schema.sql"

# Test 2: TimescaleDB functionality
run_test_suite "TimescaleDB Functionality" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-timescaledb.sql"

# Test 3: Data integrity
run_test_suite "Data Integrity Constraints" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-data-integrity.sql"

# Test 4: Notifications
run_test_suite "PostgreSQL NOTIFY/LISTEN" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-notifications.sql"

# Test 5: Performance
run_test_suite "Query Performance" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-performance-simple.sql"

# Test 6: Event Notifications
run_test_suite "Event Notification System" \
    "docker exec -i $DB_CONTAINER psql -U $DB_USER -d $DB_NAME < database/tests/test-event-notifications.sql"

# ============================================================================
# RUST TESTS (when available)
# ============================================================================

if [ -d "rust-backend" ] && [ -f "rust-backend/Cargo.toml" ]; then
    print_header "RUST TESTS"
    run_test_suite "Rust Unit & Integration Tests" \
        "cd rust-backend && cargo test --all"
else
    print_info "Rust backend not yet implemented - skipping"
fi

# ============================================================================
# TYPESCRIPT TESTS (when available)
# ============================================================================

if [ -d "ponder-indexers" ] && [ -f "ponder-indexers/package.json" ]; then
    print_header "TYPESCRIPT TESTS"
    run_test_suite "Ponder Indexer Tests" \
        "cd ponder-indexers && pnpm test"
else
    print_info "Ponder indexers not yet implemented - skipping"
fi

# ============================================================================
# TEST SUMMARY
# ============================================================================

print_header "TEST SUMMARY"

echo -e "Total test suites: ${BLUE}$TOTAL_SUITES${NC}"
echo -e "Passed: ${GREEN}$PASSED_SUITES${NC}"
echo -e "Failed: ${RED}$FAILED_SUITES${NC}"

if [ $FAILED_SUITES -eq 0 ]; then
    echo ""
    print_success "ALL TESTS PASSED! ✨"
    echo ""
    exit 0
else
    echo ""
    print_fail "SOME TESTS FAILED"
    echo ""
    exit 1
fi
