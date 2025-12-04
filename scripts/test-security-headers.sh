#!/bin/bash
#
# Security Headers Testing Script
#
# This script tests that all security headers are properly set on API responses.
# Run this after starting the API Gateway locally or against a deployed instance.
#
# Usage:
#   ./scripts/test-security-headers.sh [URL]
#
# Examples:
#   ./scripts/test-security-headers.sh                              # Test localhost:8080
#   ./scripts/test-security-headers.sh https://api.agentauri.ai         # Test production

set -euo pipefail

# Default to localhost if no URL provided
BASE_URL="${1:-http://localhost:8080}"

echo "========================================"
echo "Security Headers Test"
echo "========================================"
echo "Target: $BASE_URL"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test endpoint
ENDPOINT="/api/v1/health"
FULL_URL="$BASE_URL$ENDPOINT"

echo "Testing endpoint: $FULL_URL"
echo ""

# Make request and capture headers
HEADERS=$(curl -sI "$FULL_URL" 2>&1)

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Failed to connect to $FULL_URL${NC}"
    echo "Make sure the API Gateway is running:"
    echo "  cd rust-backend && cargo run --bin api-gateway"
    exit 1
fi

# Function to check if header exists
check_header() {
    local header_name="$1"
    local expected_value="${2:-}"

    if echo "$HEADERS" | grep -qi "^$header_name:"; then
        local actual_value
        actual_value=$(echo "$HEADERS" | grep -i "^$header_name:" | cut -d' ' -f2- | tr -d '\r')

        if [ -z "$expected_value" ]; then
            echo -e "${GREEN}✓${NC} $header_name: $actual_value"
            return 0
        elif echo "$actual_value" | grep -qi "$expected_value"; then
            echo -e "${GREEN}✓${NC} $header_name: $actual_value"
            return 0
        else
            echo -e "${YELLOW}⚠${NC} $header_name: $actual_value (expected: $expected_value)"
            return 1
        fi
    else
        echo -e "${RED}✗${NC} $header_name: MISSING"
        return 1
    fi
}

# Track failures
FAILURES=0

echo "Checking OWASP Recommended Headers:"
echo "------------------------------------"

check_header "X-Content-Type-Options" "nosniff" || ((FAILURES++))
check_header "X-Frame-Options" "DENY" || ((FAILURES++))
check_header "X-XSS-Protection" "1; mode=block" || ((FAILURES++))
check_header "Referrer-Policy" "strict-origin-when-cross-origin" || ((FAILURES++))
check_header "Permissions-Policy" || ((FAILURES++))

echo ""
echo "Checking Cross-Origin Policies (Spectre/Meltdown mitigation):"
echo "--------------------------------------------------------------"

check_header "Cross-Origin-Embedder-Policy" "require-corp" || ((FAILURES++))
check_header "Cross-Origin-Opener-Policy" "same-origin" || ((FAILURES++))
check_header "Cross-Origin-Resource-Policy" "same-origin" || ((FAILURES++))

echo ""
echo "Checking HSTS (production only):"
echo "---------------------------------"

if echo "$BASE_URL" | grep -q "^https://"; then
    if check_header "Strict-Transport-Security"; then
        echo -e "  ${GREEN}✓${NC} HSTS enabled (HTTPS connection detected)"
    else
        echo -e "  ${RED}✗${NC} HSTS missing on HTTPS connection!"
        ((FAILURES++))
    fi
else
    if echo "$HEADERS" | grep -qi "^Strict-Transport-Security:"; then
        echo -e "  ${YELLOW}⚠${NC} HSTS enabled on HTTP (should be disabled in development)"
    else
        echo -e "  ${GREEN}✓${NC} HSTS disabled (HTTP connection, correct for development)"
    fi
fi

echo ""
echo "Checking Content-Security-Policy:"
echo "----------------------------------"

if echo "$HEADERS" | grep -qi "^Content-Security-Policy:"; then
    echo -e "${YELLOW}⚠${NC} CSP enabled (should be disabled for API endpoints)"
    echo "  Consider using SecurityHeaders::for_api() instead of SecurityHeaders::default()"
else
    echo -e "${GREEN}✓${NC} CSP disabled (correct for API endpoints)"
fi

echo ""
echo "========================================"
echo "Test Summary"
echo "========================================"

if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}All security headers are properly configured!${NC}"
    echo ""
    echo "SecurityHeaders.com Grade Estimation: A+"
    echo ""
    echo "To verify with SecurityHeaders.com:"
    echo "  1. Deploy to a public HTTPS domain"
    echo "  2. Visit https://securityheaders.com"
    echo "  3. Enter your domain URL"
    echo "  4. Expected grade: A+"
    exit 0
else
    echo -e "${RED}$FAILURES header(s) missing or incorrect${NC}"
    echo ""
    echo "Please check the configuration in:"
    echo "  rust-backend/crates/api-gateway/src/middleware/security_headers.rs"
    exit 1
fi
