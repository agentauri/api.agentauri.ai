#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "======================================"
echo "Pre-Push Validation"
echo "======================================"
echo ""

# Change to rust-backend directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../rust-backend"

# Track failures
FAILED=0
CHECKS_RUN=0

# Function to print check result
print_result() {
    local check_name="$1"
    local exit_code="$2"

    if [ "$exit_code" -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $check_name"
    else
        echo -e "${RED}✗${NC} $check_name"
        FAILED=$((FAILED + 1))
    fi
    CHECKS_RUN=$((CHECKS_RUN + 1))
}

echo -e "${BLUE}Running pre-push checks...${NC}"
echo ""

# 1. Formatting check
echo -e "${YELLOW}[1/3]${NC} Checking code formatting..."
if cargo fmt -- --check > /dev/null 2>&1; then
    print_result "Code formatting" 0
else
    print_result "Code formatting" 1
    echo -e "${RED}      Run 'cargo fmt' to fix formatting issues${NC}"
fi
echo ""

# 2. Clippy check (using SQLx offline mode via .sqlx cache)
echo -e "${YELLOW}[2/3]${NC} Running Clippy linter..."
if env -u DATABASE_URL cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1; then
    print_result "Clippy linter" 0
else
    print_result "Clippy linter" 1
    echo -e "${RED}      Run 'cargo clippy --all-targets --all-features -- -D warnings' to see issues${NC}"
fi
echo ""

# 3. Unit tests (integration tests marked with #[ignore] are skipped)
echo -e "${YELLOW}[3/3]${NC} Running unit tests..."
# Integration tests marked with #[ignore] require DATABASE_URL
# These are run in CI with: cargo test -- --ignored
if env -u DATABASE_URL cargo test --workspace --lib --bins > /dev/null 2>&1; then
    print_result "Unit tests" 0
else
    print_result "Unit tests" 1
    echo -e "${RED}      Run 'cargo test --workspace' to see failing tests${NC}"
fi
echo ""

# Note about ignored tests
echo -e "${BLUE}Note:${NC} Integration tests marked with #[ignore] require DATABASE_URL"
echo -e "${BLUE}Note:${NC} CI runs integration tests with: cargo test -- --ignored"
echo -e "${BLUE}Note:${NC} SQLx compile-time verification uses cached metadata from .sqlx directory"
echo ""

# Summary
echo "======================================"
echo "Summary"
echo "======================================"
echo ""

PASSED=$((CHECKS_RUN - FAILED))
echo "Total checks: $CHECKS_RUN"
echo -e "Passed:       ${GREEN}$PASSED${NC}"

if [ "$FAILED" -gt 0 ]; then
    echo -e "Failed:       ${RED}$FAILED${NC}"
    echo ""
    echo -e "${RED}Pre-push validation failed!${NC}"
    echo -e "${YELLOW}Please fix the issues above before pushing.${NC}"
    exit 1
else
    echo -e "Failed:       ${GREEN}0${NC}"
    echo ""
    echo -e "${GREEN}All checks passed!${NC} Ready to push."
    exit 0
fi
