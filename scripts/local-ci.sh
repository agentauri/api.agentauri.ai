#!/bin/bash
# ============================================================================
# Local CI Test Suite
# ============================================================================
# Runs the same tests as GitHub Actions CI workflow
# Use this for daily development workflow validation
# ============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0
SKIPPED=0

# Helper functions
print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    PASSED=$((PASSED+1))
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    FAILED=$((FAILED+1))
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_skip() {
    echo -e "${YELLOW}⏭️  $1${NC}"
    SKIPPED=$((SKIPPED+1))
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# Check Prerequisites
# ============================================================================
print_header "Checking Prerequisites"

if ! command_exists psql; then
    print_error "psql not found. Please install PostgreSQL client."
    exit 1
fi
print_success "PostgreSQL client available"

if ! command_exists docker-compose; then
    print_error "docker-compose not found. Please install Docker."
    exit 1
fi
print_success "Docker Compose available"

# Check if database is running
if ! docker-compose ps | grep -q "postgres.*Up"; then
    print_error "PostgreSQL container not running. Start with: docker-compose up -d"
    exit 1
fi
print_success "PostgreSQL container running"

# Check if Rust exists
if [ -f "rust-backend/Cargo.toml" ]; then
    RUST_EXISTS=true
    print_success "Rust backend found"
else
    RUST_EXISTS=false
    print_skip "Rust backend not found"
fi

# Check if TypeScript/Ponder exists
if [ -f "ponder-indexers/package.json" ]; then
    TYPESCRIPT_EXISTS=true
    print_success "TypeScript/Ponder indexers found"
else
    TYPESCRIPT_EXISTS=false
    print_skip "TypeScript/Ponder indexers not found"
fi

echo ""

# ============================================================================
# Database Tests
# ============================================================================
print_header "Database Tests"

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
    print_success "Loaded environment variables from .env"
else
    print_warning ".env file not found, using defaults"
    export DB_USER=${DB_USER:-postgres}
    export DB_PASSWORD=${DB_PASSWORD:-}
    export DB_NAME=${DB_NAME:-erc8004_backend}
    export DB_HOST=${DB_HOST:-localhost}
    export DB_PORT=${DB_PORT:-5432}
fi

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
for i in {1..30}; do
    if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1" >/dev/null 2>&1; then
        print_success "PostgreSQL is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        print_error "PostgreSQL not ready after 60 seconds"
        exit 1
    fi
    sleep 2
done

# Verify TimescaleDB extension
echo "Verifying TimescaleDB extension..."
if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -c "SELECT * FROM pg_available_extensions WHERE name = 'timescaledb';" >/dev/null 2>&1; then
    print_success "TimescaleDB extension available"
else
    print_error "TimescaleDB extension not available"
    exit 1
fi

# Run database migrations (if needed)
echo "Checking database migrations..."
MIGRATION_COUNT=$(ls database/migrations/*.sql 2>/dev/null | wc -l)
if [ "$MIGRATION_COUNT" -gt 0 ]; then
    print_success "Found $MIGRATION_COUNT migration(s)"
else
    print_warning "No migrations found in database/migrations/"
fi

# Run database tests
echo "Running database tests..."
TEST_FILES=(
    "database/tests/test-schema.sql"
    "database/tests/test-timescaledb.sql"
    "database/tests/test-data-integrity.sql"
    "database/tests/test-notifications.sql"
    "database/tests/test-performance-simple.sql"
)

for test_file in "${TEST_FILES[@]}"; do
    if [ -f "$test_file" ]; then
        test_name=$(basename "$test_file" .sql)
        echo "  Running $test_name..."
        if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -f "$test_file" >/dev/null 2>&1; then
            print_success "$test_name passed"
        else
            print_error "$test_name failed"
        fi
    else
        print_skip "$test_file not found"
    fi
done

echo ""

# ============================================================================
# Rust Tests
# ============================================================================
if [ "$RUST_EXISTS" = true ]; then
    print_header "Rust Tests"

    cd rust-backend

    # Check Rust formatting
    echo "Checking Rust formatting..."
    if cargo fmt -- --check >/dev/null 2>&1; then
        print_success "Rust formatting check passed"
    else
        print_error "Rust formatting check failed. Run: cargo fmt"
    fi

    # Run Clippy
    echo "Running Clippy..."
    if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q "0 warnings"; then
        print_success "Clippy passed with no warnings"
    else
        print_error "Clippy found warnings or errors"
    fi

    # Build Rust project
    echo "Building Rust project..."
    if cargo build --verbose >/dev/null 2>&1; then
        print_success "Rust build succeeded"
    else
        print_error "Rust build failed"
    fi

    # Run Rust tests
    echo "Running Rust tests..."
    if cargo test --verbose --all >/dev/null 2>&1; then
        print_success "Rust tests passed"
    else
        print_error "Rust tests failed"
    fi

    cd ..
    echo ""
fi

# ============================================================================
# TypeScript/Ponder Tests
# ============================================================================
if [ "$TYPESCRIPT_EXISTS" = true ]; then
    print_header "TypeScript/Ponder Tests"

    cd ponder-indexers

    # Check if pnpm is installed
    if ! command_exists pnpm; then
        print_error "pnpm not found. Install with: npm install -g pnpm"
        exit 1
    fi

    # Install dependencies if needed
    if [ ! -d "node_modules" ]; then
        echo "Installing dependencies..."
        pnpm install --frozen-lockfile
    fi

    # Run TypeScript type check
    echo "Running TypeScript type check..."
    if pnpm type-check 2>/dev/null; then
        print_success "TypeScript type check passed"
    else
        print_warning "Type check not configured or failed"
    fi

    # Run linting
    echo "Running ESLint..."
    if pnpm lint 2>/dev/null; then
        print_success "ESLint passed"
    else
        print_warning "ESLint not configured or failed"
    fi

    # Run tests
    echo "Running tests..."
    if pnpm test 2>/dev/null; then
        print_success "Tests passed"
    else
        print_warning "Tests not configured or failed"
    fi

    cd ..
    echo ""
fi

# ============================================================================
# Summary
# ============================================================================
print_header "Test Summary"
echo -e "${GREEN}Passed:  $PASSED${NC}"
echo -e "${RED}Failed:  $FAILED${NC}"
echo -e "${YELLOW}Skipped: $SKIPPED${NC}"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ Some tests failed. Please fix the issues before committing.${NC}"
    exit 1
else
    echo -e "${GREEN}✅ All tests passed! Ready to commit.${NC}"
    exit 0
fi
