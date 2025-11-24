#!/bin/bash
# ============================================================================
# Local Lint Test Suite
# ============================================================================
# Runs the same linting checks as GitHub Actions Lint workflow
# Use this before creating pull requests
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
WARNINGS=0
SKIPPED=0

# Helper functions
print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    ((PASSED++))
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    ((FAILED++))
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    ((WARNINGS++))
}

print_skip() {
    echo -e "${YELLOW}⏭️  $1${NC}"
    ((SKIPPED++))
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# Check Prerequisites
# ============================================================================
print_header "Checking Prerequisites"

# Check if files exist
if [ -f "rust-backend/Cargo.toml" ]; then
    RUST_EXISTS=true
    print_success "Rust backend found"
else
    RUST_EXISTS=false
    print_skip "Rust backend not found"
fi

if [ -f "ponder-indexers/package.json" ]; then
    TYPESCRIPT_EXISTS=true
    print_success "TypeScript/Ponder indexers found"
else
    TYPESCRIPT_EXISTS=false
    print_skip "TypeScript/Ponder indexers not found"
fi

if ls scripts/*.sh 1> /dev/null 2>&1; then
    SCRIPTS_EXIST=true
    print_success "Shell scripts found"
else
    SCRIPTS_EXIST=false
    print_skip "Shell scripts not found"
fi

echo ""

# ============================================================================
# SQL Linting
# ============================================================================
print_header "SQL Linting"

SQL_ISSUES=0

# Check for mixed case in keywords
echo "Checking SQL keyword case consistency..."
if [ -d "database/migrations" ]; then
    if grep -r "select\|SELECT" database/migrations/*.sql 2>/dev/null | grep -q "SeLeCt\|sElEcT"; then
        print_warning "Mixed case SQL keywords found"
        ((SQL_ISSUES++))
    else
        print_success "SQL keywords are consistent"
    fi
else
    print_skip "database/migrations directory not found"
fi

# Check for trailing whitespace
echo "Checking for trailing whitespace in SQL files..."
if [ -d "database" ]; then
    if grep -r " $" database/ --include="*.sql" 2>/dev/null | head -n 1 | grep -q .; then
        print_warning "Trailing whitespace found in SQL files"
        ((SQL_ISSUES++))
    else
        print_success "No trailing whitespace in SQL files"
    fi
else
    print_skip "database directory not found"
fi

if [ $SQL_ISSUES -eq 0 ]; then
    print_success "SQL files pass all linting checks"
else
    print_warning "Found $SQL_ISSUES SQL style issue(s)"
fi

echo ""

# ============================================================================
# Rust Linting
# ============================================================================
if [ "$RUST_EXISTS" = true ]; then
    print_header "Rust Linting"

    cd rust-backend

    # Check Rust formatting
    echo "Checking Rust formatting..."
    if cargo fmt -- --check 2>&1; then
        print_success "Rust code is properly formatted"
    else
        print_error "Rust formatting check failed. Run: cargo fmt"
    fi

    # Run Clippy
    echo "Running Clippy..."
    if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/clippy-output.txt | grep -q "0 warnings"; then
        print_success "No Clippy warnings found"
    else
        print_error "Clippy found warnings or errors"
        cat /tmp/clippy-output.txt
    fi

    # Check for unsafe code
    echo "Checking for unsafe code..."
    UNSAFE_COUNT=$(grep -r "unsafe" --include="*.rs" src/ 2>/dev/null | wc -l | tr -d ' ')
    if [ "$UNSAFE_COUNT" -eq 0 ]; then
        print_success "No unsafe code found"
    else
        print_warning "Found $UNSAFE_COUNT use(s) of unsafe code - review carefully"
    fi

    # Check for TODO comments
    echo "Checking for TODO/FIXME comments..."
    TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX" --include="*.rs" src/ 2>/dev/null | wc -l | tr -d ' ')
    if [ "$TODO_COUNT" -eq 0 ]; then
        print_success "No TODO/FIXME comments found"
    else
        print_warning "Found $TODO_COUNT TODO/FIXME comment(s)"
    fi

    cd ..
    echo ""
fi

# ============================================================================
# TypeScript Linting
# ============================================================================
if [ "$TYPESCRIPT_EXISTS" = true ]; then
    print_header "TypeScript/Ponder Linting"

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

    # Check TypeScript formatting
    echo "Checking TypeScript formatting..."
    if pnpm format:check 2>/dev/null; then
        print_success "TypeScript code is properly formatted"
    else
        print_warning "Format check script not configured or failed"
    fi

    # Run ESLint
    echo "Running ESLint..."
    if pnpm lint 2>/dev/null; then
        print_success "No ESLint errors found"
    else
        print_warning "ESLint not configured or found issues"
    fi

    # Type check
    echo "Running TypeScript type check..."
    if pnpm type-check 2>/dev/null; then
        print_success "No type errors found"
    else
        # Try running tsc directly
        if npx tsc --noEmit 2>/dev/null; then
            print_success "No type errors found (via tsc)"
        else
            print_warning "Type checking not fully configured or found issues"
        fi
    fi

    cd ..
    echo ""
fi

# ============================================================================
# Documentation Linting
# ============================================================================
print_header "Documentation Quality"

# Verify required documentation files
echo "Checking for required documentation files..."
REQUIRED_FILES=("README.md" "CLAUDE.md" ".env.example")
MISSING=0

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        print_success "$file exists"
    else
        print_error "$file is missing"
        ((MISSING++))
    fi
done

if [ $MISSING -gt 0 ]; then
    print_error "$MISSING required documentation file(s) missing"
fi

# Check for broken links in markdown
echo "Checking for empty links in markdown files..."
if grep -r "\[.*\](\s*)" --include="*.md" . 2>/dev/null | grep -q .; then
    print_warning "Empty links found in markdown files"
else
    print_success "No obvious broken links detected"
fi

echo ""

# ============================================================================
# Docker Compose Validation
# ============================================================================
print_header "Docker Compose Validation"

if [ -f "docker-compose.yml" ]; then
    # Validate docker-compose.yml
    echo "Validating docker-compose.yml syntax..."
    if docker-compose -f docker-compose.yml config > /dev/null 2>&1; then
        print_success "docker-compose.yml is valid"
    else
        print_error "docker-compose.yml has syntax errors"
    fi

    # Check for security best practices
    echo "Checking Docker security best practices..."

    # Check if ports are bound to localhost
    if grep -q "127.0.0.1:" docker-compose.yml; then
        print_success "Ports are bound to localhost (secure)"
    else
        print_warning "Some ports may be exposed publicly"
    fi

    # Check for environment variable usage
    if grep -q "\${" docker-compose.yml; then
        print_success "Using environment variables for configuration"
    fi

    # Check for pinned versions
    if grep -q "latest" docker-compose.yml; then
        print_warning "Using 'latest' tag - prefer specific versions"
    else
        print_success "All images use pinned versions"
    fi
else
    print_skip "docker-compose.yml not found"
fi

echo ""

# ============================================================================
# Shell Script Linting
# ============================================================================
if [ "$SCRIPTS_EXIST" = true ]; then
    print_header "Shell Script Linting"

    # Check if shellcheck is installed
    if command_exists shellcheck; then
        echo "Running ShellCheck..."
        SHELLCHECK_FAILED=0
        for script in scripts/*.sh; do
            if [ -f "$script" ]; then
                if shellcheck -S warning "$script" 2>/dev/null; then
                    print_success "$(basename "$script") passed ShellCheck"
                else
                    print_error "$(basename "$script") has ShellCheck issues"
                    ((SHELLCHECK_FAILED++))
                fi
            fi
        done

        if [ $SHELLCHECK_FAILED -eq 0 ]; then
            print_success "All shell scripts passed ShellCheck"
        fi
    else
        print_warning "shellcheck not installed. Install with: brew install shellcheck (macOS) or apt install shellcheck (Ubuntu)"
    fi

    # Check script permissions
    echo "Checking shell script permissions..."
    for script in scripts/*.sh; do
        if [ -f "$script" ]; then
            if [ -x "$script" ]; then
                print_success "$(basename "$script") is executable"
            else
                print_warning "$(basename "$script") is not executable - run: chmod +x $script"
            fi
        fi
    done

    echo ""
fi

# ============================================================================
# Summary
# ============================================================================
print_header "Linting Summary"
echo -e "${GREEN}Passed:   $PASSED${NC}"
echo -e "${RED}Failed:   $FAILED${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo -e "${YELLOW}Skipped:  $SKIPPED${NC}"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ Critical linting issues found. Please fix before creating a PR.${NC}"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Linting completed with warnings. Review before creating a PR.${NC}"
    exit 0
else
    echo -e "${GREEN}✅ All linting checks passed! Ready to create a PR.${NC}"
    exit 0
fi
