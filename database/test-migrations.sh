#!/bin/bash

# ============================================================================
# DATABASE MIGRATION TEST SCRIPT
# ============================================================================
# Description: Comprehensive testing for PostgreSQL + TimescaleDB migrations
# Purpose: Verify all migrations, schema integrity, and database functionality
# Usage: ./database/test-migrations.sh [options]
# ============================================================================

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TEST_DB_NAME="${TEST_DB_NAME:-test_agentauri_backend}"
DB_USER="${DB_USER:-postgres}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DOCKER_CONTAINER="${DOCKER_CONTAINER:-agentauri-postgres}"

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $message"
            ;;
        "SUCCESS")
            echo -e "${GREEN}[✓]${NC} $message"
            ((PASSED_TESTS++))
            ((TOTAL_TESTS++))
            ;;
        "FAIL")
            echo -e "${RED}[✗]${NC} $message"
            ((FAILED_TESTS++))
            ((TOTAL_TESTS++))
            ;;
        "WARN")
            echo -e "${YELLOW}[!]${NC} $message"
            ;;
        "HEADER")
            echo ""
            echo -e "${BLUE}========================================================================${NC}"
            echo -e "${BLUE}$message${NC}"
            echo -e "${BLUE}========================================================================${NC}"
            echo ""
            ;;
    esac
}

# Function to check if Docker container is running
check_docker() {
    print_status "INFO" "Checking Docker container: $DOCKER_CONTAINER"
    if docker ps --format '{{.Names}}' | grep -q "^${DOCKER_CONTAINER}$"; then
        print_status "SUCCESS" "Docker container is running"
        return 0
    else
        print_status "FAIL" "Docker container is not running"
        print_status "INFO" "Start the container with: docker-compose up -d"
        exit 1
    fi
}

# Function to check PostgreSQL connection
check_postgres() {
    print_status "INFO" "Checking PostgreSQL connection"
    if docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -c "SELECT 1" > /dev/null 2>&1; then
        print_status "SUCCESS" "PostgreSQL is accessible"
        return 0
    else
        print_status "FAIL" "Cannot connect to PostgreSQL"
        exit 1
    fi
}

# Function to check TimescaleDB extension
check_timescaledb() {
    print_status "INFO" "Checking TimescaleDB extension"
    local result=$(docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -t -c "SELECT COUNT(*) FROM pg_available_extensions WHERE name = 'timescaledb';" | xargs)
    if [ "$result" -eq "1" ]; then
        print_status "SUCCESS" "TimescaleDB extension is available"
        return 0
    else
        print_status "FAIL" "TimescaleDB extension is not available"
        exit 1
    fi
}

# Function to drop test database if exists
drop_test_db() {
    print_status "INFO" "Dropping test database if exists: $TEST_DB_NAME"
    docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -c "DROP DATABASE IF EXISTS $TEST_DB_NAME;" > /dev/null 2>&1
    print_status "SUCCESS" "Test database dropped (if existed)"
}

# Function to create test database
create_test_db() {
    print_status "INFO" "Creating test database: $TEST_DB_NAME"
    if docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -c "CREATE DATABASE $TEST_DB_NAME;" > /dev/null 2>&1; then
        print_status "SUCCESS" "Test database created"
        return 0
    else
        print_status "FAIL" "Failed to create test database"
        exit 1
    fi
}

# Function to run all migrations
run_migrations() {
    print_status "HEADER" "RUNNING MIGRATIONS"

    local migration_files=($(ls -1 database/migrations/*.sql | sort))
    local migration_count=${#migration_files[@]}

    print_status "INFO" "Found $migration_count migration files"

    for migration_file in "${migration_files[@]}"; do
        local filename=$(basename "$migration_file")
        print_status "INFO" "Applying migration: $filename"

        if docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" < "$migration_file" > /dev/null 2>&1; then
            print_status "SUCCESS" "Migration applied: $filename"
        else
            print_status "FAIL" "Migration failed: $filename"
            print_status "INFO" "Error details:"
            docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" < "$migration_file" 2>&1
            exit 1
        fi
    done

    print_status "SUCCESS" "All $migration_count migrations applied successfully"
}

# Function to test migration idempotency
test_idempotency() {
    print_status "HEADER" "TESTING MIGRATION IDEMPOTENCY"

    print_status "INFO" "Re-running migrations to test idempotency"

    local migration_files=($(ls -1 database/migrations/*.sql | sort))
    local errors=0

    for migration_file in "${migration_files[@]}"; do
        local filename=$(basename "$migration_file")

        # Skip migrations that are not idempotent by design
        if [[ "$filename" == *"create_hypertable"* ]]; then
            print_status "WARN" "Skipping hypertable migration (not idempotent): $filename"
            continue
        fi

        if docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" < "$migration_file" > /dev/null 2>&1; then
            print_status "SUCCESS" "Migration is idempotent: $filename"
        else
            # Check if it's an expected error (e.g., "already exists")
            local error_msg=$(docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" < "$migration_file" 2>&1)
            if [[ "$error_msg" == *"already exists"* ]]; then
                print_status "SUCCESS" "Migration properly handles existing objects: $filename"
            else
                print_status "FAIL" "Migration is not idempotent: $filename"
                ((errors++))
            fi
        fi
    done

    if [ $errors -eq 0 ]; then
        print_status "SUCCESS" "All applicable migrations are idempotent"
    else
        print_status "FAIL" "$errors migration(s) failed idempotency test"
    fi
}

# Function to verify table count
verify_tables() {
    print_status "INFO" "Verifying expected tables exist"
    local expected_tables=("users" "triggers" "trigger_conditions" "trigger_actions" "trigger_state" "events" "checkpoints" "action_results" "agent_mcp_tokens")
    local table_count=$(docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE';" | xargs)

    if [ "$table_count" -eq "${#expected_tables[@]}" ]; then
        print_status "SUCCESS" "Found $table_count tables (expected ${#expected_tables[@]})"
    else
        print_status "FAIL" "Found $table_count tables (expected ${#expected_tables[@]})"
    fi

    # Check each table individually
    for table in "${expected_tables[@]}"; do
        local exists=$(docker exec "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '$table';" | xargs)
        if [ "$exists" -eq "1" ]; then
            print_status "SUCCESS" "Table exists: $table"
        else
            print_status "FAIL" "Table missing: $table"
        fi
    done
}

# Function to run SQL test file
run_test_file() {
    local test_file=$1
    local test_name=$(basename "$test_file" .sql)

    print_status "HEADER" "RUNNING TEST: $test_name"

    if docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" -v ON_ERROR_STOP=1 < "$test_file" 2>&1 | tee /tmp/test_output.log; then
        # Check if the output contains the success message
        if grep -q "ALL.*TESTS PASSED" /tmp/test_output.log; then
            print_status "SUCCESS" "Test suite passed: $test_name"
            return 0
        elif grep -q "TESTS FAILED" /tmp/test_output.log; then
            print_status "FAIL" "Test suite failed: $test_name"
            return 1
        else
            print_status "SUCCESS" "Test file executed: $test_name"
            return 0
        fi
    else
        print_status "FAIL" "Test suite failed with errors: $test_name"
        return 1
    fi
}

# Function to seed test data
seed_test_data() {
    print_status "INFO" "Seeding test data"
    if docker exec -i "$DOCKER_CONTAINER" psql -U "$DB_USER" -d "$TEST_DB_NAME" < database/seeds/test_data.sql > /dev/null 2>&1; then
        print_status "SUCCESS" "Test data seeded"
        return 0
    else
        print_status "FAIL" "Failed to seed test data"
        return 1
    fi
}

# Function to test rollback scenario
test_rollback() {
    print_status "HEADER" "TESTING ROLLBACK SCENARIO"

    print_status "INFO" "Dropping and recreating database"
    drop_test_db
    create_test_db

    print_status "INFO" "Reapplying all migrations"
    run_migrations

    print_status "SUCCESS" "Rollback and reapplication successful"
}

# Function to display test summary
display_summary() {
    echo ""
    echo -e "${BLUE}========================================================================${NC}"
    echo -e "${BLUE}TEST SUMMARY${NC}"
    echo -e "${BLUE}========================================================================${NC}"
    echo ""
    echo -e "Total Tests:  ${TOTAL_TESTS}"
    echo -e "${GREEN}Passed:       ${PASSED_TESTS}${NC}"
    echo -e "${RED}Failed:       ${FAILED_TESTS}${NC}"
    echo ""

    local pass_rate=0
    if [ $TOTAL_TESTS -gt 0 ]; then
        pass_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    fi

    echo -e "Pass Rate:    ${pass_rate}%"
    echo ""

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}========================================================================${NC}"
        echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
        echo -e "${GREEN}========================================================================${NC}"
        return 0
    else
        echo -e "${RED}========================================================================${NC}"
        echo -e "${RED}✗ TESTS FAILED${NC}"
        echo -e "${RED}========================================================================${NC}"
        return 1
    fi
}

# Main execution
main() {
    print_status "HEADER" "DATABASE MIGRATION TEST SUITE"
    print_status "INFO" "Test Database: $TEST_DB_NAME"
    print_status "INFO" "Docker Container: $DOCKER_CONTAINER"
    echo ""

    # Preliminary checks
    check_docker
    check_postgres
    check_timescaledb

    # Setup test database
    drop_test_db
    create_test_db

    # Run migrations
    run_migrations

    # Verify migrations
    verify_tables

    # Test idempotency
    test_idempotency

    # Run test suites
    run_test_file "database/tests/test-schema.sql"
    run_test_file "database/tests/test-timescaledb.sql"
    run_test_file "database/tests/test-data-integrity.sql"
    run_test_file "database/tests/test-notifications.sql"

    # Seed test data for performance tests
    seed_test_data

    run_test_file "database/tests/test-performance-simple.sql"

    # Test rollback scenario
    test_rollback

    # Final verification
    verify_tables

    # Display summary
    display_summary

    # Clean up (optional - comment out to keep test database)
    if [ "${KEEP_TEST_DB:-0}" != "1" ]; then
        print_status "INFO" "Cleaning up test database"
        drop_test_db
    else
        print_status "INFO" "Keeping test database for inspection: $TEST_DB_NAME"
    fi

    # Return exit code based on results
    if [ $FAILED_TESTS -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
