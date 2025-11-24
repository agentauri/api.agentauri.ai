#!/bin/bash
# ============================================================================
# Local Security Audit Suite
# ============================================================================
# Runs the same security checks as GitHub Actions Security workflow
# Use this weekly/monthly or before major releases
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
print_header "Checking Security Tools"

# Check for optional security tools
TOOLS_MISSING=0

if command_exists cargo-audit; then
    print_success "cargo-audit available"
else
    print_warning "cargo-audit not installed. Install with: cargo install cargo-audit"
    ((TOOLS_MISSING++))
fi

if command_exists trivy; then
    print_success "trivy available"
else
    print_warning "trivy not installed. Install with: brew install trivy (macOS)"
    ((TOOLS_MISSING++))
fi

if command_exists gitleaks; then
    print_success "gitleaks available"
else
    print_warning "gitleaks not installed. Install with: brew install gitleaks (macOS)"
    ((TOOLS_MISSING++))
fi

if command_exists hadolint; then
    print_success "hadolint available"
else
    print_warning "hadolint not installed. Install with: brew install hadolint (macOS)"
    ((TOOLS_MISSING++))
fi

if [ $TOOLS_MISSING -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}$TOOLS_MISSING security tool(s) missing. Some checks will be skipped.${NC}"
    echo -e "${YELLOW}To install all tools (macOS):${NC}"
    echo "  cargo install cargo-audit"
    echo "  brew install trivy gitleaks hadolint"
fi

echo ""

# Check if files exist
if [ -f "rust-backend/Cargo.toml" ]; then
    RUST_EXISTS=true
else
    RUST_EXISTS=false
fi

if [ -f "ponder-indexers/package.json" ]; then
    TYPESCRIPT_EXISTS=true
else
    TYPESCRIPT_EXISTS=false
fi

# ============================================================================
# Dependency Vulnerability Scanning
# ============================================================================
print_header "Dependency Vulnerability Scanning"

# Rust dependencies
if [ "$RUST_EXISTS" = true ]; then
    if command_exists cargo-audit; then
        echo "Running cargo-audit for Rust dependencies..."
        cd rust-backend
        if cargo audit 2>&1 | tee /tmp/cargo-audit.txt; then
            VULNS=$(grep -c "warning" /tmp/cargo-audit.txt || echo "0")
            if [ "$VULNS" -eq 0 ]; then
                print_success "No vulnerabilities found in Rust dependencies"
            else
                print_warning "Found $VULNS potential vulnerability/vulnerabilities in Rust dependencies"
                cat /tmp/cargo-audit.txt
            fi
        else
            print_error "cargo-audit failed"
        fi
        cd ..
    else
        print_skip "cargo-audit not installed - skipping Rust dependency scan"
    fi
else
    print_skip "Rust backend not found"
fi

# TypeScript/Node dependencies
if [ "$TYPESCRIPT_EXISTS" = true ]; then
    echo "Running npm audit for TypeScript/Node dependencies..."
    cd ponder-indexers

    if ! command_exists pnpm; then
        print_error "pnpm not found. Install with: npm install -g pnpm"
        exit 1
    fi

    # Install dependencies if needed
    if [ ! -d "node_modules" ]; then
        echo "Installing dependencies..."
        pnpm install --frozen-lockfile
    fi

    if pnpm audit --audit-level=moderate 2>&1 | tee /tmp/npm-audit.txt; then
        print_success "No moderate/high vulnerabilities in TypeScript dependencies"
    else
        AUDIT_EXIT=$?
        if [ $AUDIT_EXIT -ne 0 ]; then
            print_warning "Found vulnerabilities in TypeScript dependencies"
            cat /tmp/npm-audit.txt
        fi
    fi

    cd ..
else
    print_skip "TypeScript/Ponder indexers not found"
fi

echo ""

# ============================================================================
# Docker Image Security Scanning
# ============================================================================
print_header "Docker Image Security Scanning"

if command_exists trivy; then
    if [ -f "docker-compose.yml" ]; then
        echo "Scanning Docker images with Trivy..."

        # Extract image names from docker-compose.yml
        IMAGES=$(grep "image:" docker-compose.yml | awk '{print $2}' | sort -u)

        if [ -z "$IMAGES" ]; then
            print_skip "No images found in docker-compose.yml"
        else
            for image in $IMAGES; do
                echo "  Scanning $image..."
                if trivy image --severity HIGH,CRITICAL --exit-code 1 "$image" 2>&1 | tee "/tmp/trivy-$(echo "$image" | tr '/:' '_').txt"; then
                    print_success "$image has no HIGH/CRITICAL vulnerabilities"
                else
                    print_warning "$image has vulnerabilities - review report"
                fi
            done
        fi
    else
        print_skip "docker-compose.yml not found"
    fi
else
    print_skip "trivy not installed - skipping Docker image scanning"
fi

echo ""

# ============================================================================
# Secrets Detection
# ============================================================================
print_header "Secrets Detection"

if command_exists gitleaks; then
    echo "Scanning repository for secrets with Gitleaks..."
    if gitleaks detect --source . --verbose --exit-code 1 2>&1 | tee /tmp/gitleaks.txt; then
        print_success "No secrets detected in repository"
    else
        print_error "Potential secrets detected - IMMEDIATE ACTION REQUIRED"
        cat /tmp/gitleaks.txt
    fi
else
    print_skip "gitleaks not installed - skipping secrets detection"
fi

echo ""

# ============================================================================
# Dockerfile Linting
# ============================================================================
print_header "Dockerfile Security Linting"

if command_exists hadolint; then
    DOCKERFILES=$(find . -name "Dockerfile*" -type f 2>/dev/null)

    if [ -z "$DOCKERFILES" ]; then
        print_skip "No Dockerfiles found"
    else
        for dockerfile in $DOCKERFILES; do
            echo "  Linting $dockerfile..."
            if hadolint "$dockerfile" 2>&1 | tee "/tmp/hadolint-$(basename "$dockerfile").txt"; then
                print_success "$dockerfile passed hadolint"
            else
                print_warning "$dockerfile has issues - review recommendations"
            fi
        done
    fi
else
    print_skip "hadolint not installed - skipping Dockerfile linting"
fi

echo ""

# ============================================================================
# Configuration Security Checks
# ============================================================================
print_header "Configuration Security Checks"

# Check .env file security
echo "Checking .env file security..."
if [ -f ".env" ]; then
    print_success ".env file exists (good for local development)"

    # Check if .env is in .gitignore
    if grep -q "^\.env$" .gitignore 2>/dev/null; then
        print_success ".env is properly excluded from git"
    else
        print_error ".env is NOT in .gitignore - SECURITY RISK!"
    fi

    # Check for weak passwords
    if grep -i "password.*=.*password\|password.*=.*123" .env 2>/dev/null | grep -q .; then
        print_warning "Weak passwords detected in .env file"
    else
        print_success "No obvious weak passwords in .env"
    fi
else
    print_warning ".env file not found - using defaults or environment variables"
fi

# Check .env.example
echo "Checking .env.example..."
if [ -f ".env.example" ]; then
    print_success ".env.example exists"

    # Ensure .env.example doesn't contain real secrets
    if grep -E "password.*=.{20,}|api.*key.*=.{20,}" .env.example 2>/dev/null | grep -v "CHANGE_THIS\|YOUR_API_KEY\|your_.*_key" | grep -q .; then
        print_error ".env.example may contain real secrets!"
    else
        print_success ".env.example contains only placeholders"
    fi
else
    print_error ".env.example is missing"
fi

# Check for hardcoded secrets in code
echo "Checking for hardcoded secrets in code..."
HARDCODED_SECRETS=$(grep -r -i "password\s*=\s*['\"].\{8,\}['\"]" --include="*.rs" --include="*.ts" --include="*.js" . 2>/dev/null | grep -v "test\|example\|placeholder" | wc -l | tr -d ' ')
if [ "$HARDCODED_SECRETS" -eq 0 ]; then
    print_success "No obvious hardcoded secrets in code"
else
    print_warning "Found $HARDCODED_SECRETS potential hardcoded secret(s) - review manually"
fi

# Check CORS configuration
if [ "$RUST_EXISTS" = true ]; then
    echo "Checking CORS configuration..."
    if [ -f "rust-backend/crates/api-gateway/src/middleware.rs" ]; then
        if grep -q "ALLOWED_ORIGINS" rust-backend/crates/api-gateway/src/middleware.rs; then
            print_success "CORS uses environment-based whitelist"
        else
            print_warning "CORS configuration should use ALLOWED_ORIGINS"
        fi
    fi
fi

# Check JWT configuration
if [ "$RUST_EXISTS" = true ]; then
    echo "Checking JWT configuration..."
    if [ -f "rust-backend/crates/shared/src/config.rs" ]; then
        if grep -q "JWT_SECRET.*must be set" rust-backend/crates/shared/src/config.rs; then
            print_success "JWT_SECRET is required in production"
        else
            print_warning "JWT_SECRET should be required in production"
        fi
    fi
fi

echo ""

# ============================================================================
# Summary
# ============================================================================
print_header "Security Audit Summary"
echo -e "${GREEN}Passed:   $PASSED${NC}"
echo -e "${RED}Failed:   $FAILED${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo -e "${YELLOW}Skipped:  $SKIPPED${NC}"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ CRITICAL SECURITY ISSUES FOUND! Address immediately before deployment.${NC}"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Security audit completed with warnings. Review before deployment.${NC}"
    exit 0
else
    echo -e "${GREEN}✅ All security checks passed! No issues detected.${NC}"
    exit 0
fi
