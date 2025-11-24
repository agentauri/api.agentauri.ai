#!/bin/bash
# ============================================================================
# Complete Local CI/CD Test Suite
# ============================================================================
# Runs ALL checks: CI, Lint, and Security
# Use this for comprehensive validation before pushing to main/develop
# or before creating major releases
# ============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Track results
CI_RESULT=0
LINT_RESULT=0
SECURITY_RESULT=0

# Helper functions
print_banner() {
    echo ""
    echo -e "${MAGENTA}╔════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${MAGENTA}║                                                                            ║${NC}"
    echo -e "${MAGENTA}║                 COMPLETE LOCAL CI/CD TEST SUITE                            ║${NC}"
    echo -e "${MAGENTA}║                                                                            ║${NC}"
    echo -e "${MAGENTA}╚════════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

print_section() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# Print Banner
# ============================================================================
print_banner

echo -e "${BLUE}This will run a complete test suite matching GitHub Actions:${NC}"
echo "  1. CI Tests (Database, Rust, TypeScript)"
echo "  2. Linting Checks (Code quality, formatting, style)"
echo "  3. Security Audit (Dependencies, secrets, Docker images)"
echo ""
echo -e "${YELLOW}Estimated time: 5-15 minutes depending on your system${NC}"
echo ""

# Ask for confirmation unless --yes flag is provided
if [ "$1" != "--yes" ] && [ "$1" != "-y" ]; then
    read -p "Continue? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 0
    fi
fi

# ============================================================================
# Check Prerequisites
# ============================================================================
print_section "Checking Prerequisites"

PREREQUISITES_OK=true

# Check if scripts exist and are executable
if [ ! -f "scripts/local-ci.sh" ]; then
    print_error "scripts/local-ci.sh not found"
    PREREQUISITES_OK=false
fi

if [ ! -f "scripts/local-lint.sh" ]; then
    print_error "scripts/local-lint.sh not found"
    PREREQUISITES_OK=false
fi

if [ ! -f "scripts/local-security.sh" ]; then
    print_error "scripts/local-security.sh not found"
    PREREQUISITES_OK=false
fi

if [ "$PREREQUISITES_OK" = false ]; then
    echo ""
    print_error "Required scripts are missing. Cannot continue."
    exit 1
fi

# Make scripts executable if they aren't already
chmod +x scripts/local-ci.sh scripts/local-lint.sh scripts/local-security.sh 2>/dev/null || true

print_success "All required scripts are available"

# Check if Docker is running (needed for CI tests)
if ! command_exists docker-compose; then
    print_error "docker-compose not found. Install Docker to run CI tests."
    exit 1
fi

if ! docker-compose ps >/dev/null 2>&1; then
    print_warning "Docker Compose not running. Start with: docker-compose up -d"
    read -p "Start Docker Compose now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        docker-compose up -d
        sleep 5
    else
        print_error "Docker is required for CI tests. Exiting."
        exit 1
    fi
fi

print_success "Docker Compose is running"

echo ""

# ============================================================================
# Run CI Tests
# ============================================================================
print_section "1/3: Running CI Tests"

START_TIME=$(date +%s)

if ./scripts/local-ci.sh; then
    CI_RESULT=0
    print_success "CI tests PASSED"
else
    CI_RESULT=$?
    print_error "CI tests FAILED (exit code: $CI_RESULT)"
fi

CI_DURATION=$(($(date +%s) - START_TIME))
echo ""
echo -e "${BLUE}CI tests completed in ${CI_DURATION}s${NC}"

# ============================================================================
# Run Lint Checks
# ============================================================================
print_section "2/3: Running Lint Checks"

START_TIME=$(date +%s)

if ./scripts/local-lint.sh; then
    LINT_RESULT=0
    print_success "Lint checks PASSED"
else
    LINT_RESULT=$?
    print_error "Lint checks FAILED (exit code: $LINT_RESULT)"
fi

LINT_DURATION=$(($(date +%s) - START_TIME))
echo ""
echo -e "${BLUE}Lint checks completed in ${LINT_DURATION}s${NC}"

# ============================================================================
# Run Security Audit
# ============================================================================
print_section "3/3: Running Security Audit"

START_TIME=$(date +%s)

if ./scripts/local-security.sh; then
    SECURITY_RESULT=0
    print_success "Security audit PASSED"
else
    SECURITY_RESULT=$?
    print_error "Security audit FAILED (exit code: $SECURITY_RESULT)"
fi

SECURITY_DURATION=$(($(date +%s) - START_TIME))
echo ""
echo -e "${BLUE}Security audit completed in ${SECURITY_DURATION}s${NC}"

# ============================================================================
# Final Summary
# ============================================================================
print_section "FINAL SUMMARY"

TOTAL_DURATION=$((CI_DURATION + LINT_DURATION + SECURITY_DURATION))

echo "Test Suite Results:"
echo ""

if [ $CI_RESULT -eq 0 ]; then
    echo -e "${GREEN}  ✅ CI Tests:        PASSED${NC} (${CI_DURATION}s)"
else
    echo -e "${RED}  ❌ CI Tests:        FAILED${NC} (${CI_DURATION}s)"
fi

if [ $LINT_RESULT -eq 0 ]; then
    echo -e "${GREEN}  ✅ Lint Checks:     PASSED${NC} (${LINT_DURATION}s)"
else
    echo -e "${RED}  ❌ Lint Checks:     FAILED${NC} (${LINT_DURATION}s)"
fi

if [ $SECURITY_RESULT -eq 0 ]; then
    echo -e "${GREEN}  ✅ Security Audit:  PASSED${NC} (${SECURITY_DURATION}s)"
else
    echo -e "${RED}  ❌ Security Audit:  FAILED${NC} (${SECURITY_DURATION}s)"
fi

echo ""
echo -e "${BLUE}Total execution time: ${TOTAL_DURATION}s${NC}"
echo ""

# Determine overall result
if [ $CI_RESULT -eq 0 ] && [ $LINT_RESULT -eq 0 ] && [ $SECURITY_RESULT -eq 0 ]; then
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                                                                            ║${NC}"
    echo -e "${GREEN}║                     ✅ ALL CHECKS PASSED! ✅                               ║${NC}"
    echo -e "${GREEN}║                                                                            ║${NC}"
    echo -e "${GREEN}║              Your code is ready to push and deploy!                        ║${NC}"
    echo -e "${GREEN}║                                                                            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    exit 0
else
    echo -e "${RED}╔════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║                                                                            ║${NC}"
    echo -e "${RED}║                     ❌ SOME CHECKS FAILED! ❌                              ║${NC}"
    echo -e "${RED}║                                                                            ║${NC}"
    echo -e "${RED}║              Please fix the issues before pushing.                         ║${NC}"
    echo -e "${RED}║                                                                            ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Provide helpful suggestions
    echo -e "${YELLOW}Suggestions:${NC}"
    if [ $CI_RESULT -ne 0 ]; then
        echo "  • Run './scripts/local-ci.sh' to see detailed CI test failures"
        echo "  • Fix test failures, formatting issues, or build errors"
    fi
    if [ $LINT_RESULT -ne 0 ]; then
        echo "  • Run './scripts/local-lint.sh' to see detailed linting issues"
        echo "  • Run 'cargo fmt' for Rust formatting"
        echo "  • Run 'cargo clippy --fix' for Rust lints"
    fi
    if [ $SECURITY_RESULT -ne 0 ]; then
        echo "  • Run './scripts/local-security.sh' to see detailed security issues"
        echo "  • Update vulnerable dependencies"
        echo "  • Remove any detected secrets from code"
    fi
    echo ""

    exit 1
fi
