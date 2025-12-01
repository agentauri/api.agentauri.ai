#!/bin/bash

# =============================================================================
# HTTPS/TLS Testing Script for api.8004.dev
# =============================================================================
# Comprehensive test suite validating SSL/TLS configuration:
# - HTTP → HTTPS redirect
# - HTTPS connectivity
# - Security headers (HSTS, CSP, X-Frame-Options, etc.)
# - TLS protocol version (TLS 1.2+)
# - Certificate validity
# - OCSP stapling
# - Cipher suite strength
#
# Usage:
#   ./scripts/test-https.sh [domain]
#
# Arguments:
#   domain    Domain to test (default: api.8004.dev)
# =============================================================================

set -e

# =============================================================================
# CONFIGURATION
# =============================================================================
DOMAIN="${1:-api.8004.dev}"
PORT_HTTP=80
PORT_HTTPS=443

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0
WARNINGS=0

# =============================================================================
# HELPER FUNCTIONS
# =============================================================================
test_pass() {
    echo -e "${GREEN}✓ PASS${NC} $1"
    PASSED=$((PASSED + 1))
}

test_fail() {
    echo -e "${RED}✗ FAIL${NC} $1"
    FAILED=$((FAILED + 1))
}

test_warn() {
    echo -e "${YELLOW}⚠ WARN${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

print_header() {
    echo ""
    echo "=========================================================================="
    echo "$1"
    echo "=========================================================================="
}

# =============================================================================
# PREFLIGHT CHECKS
# =============================================================================
print_header "HTTPS/TLS Testing for $DOMAIN"
echo ""

# Check required tools
REQUIRED_TOOLS="curl openssl host"
for tool in $REQUIRED_TOOLS; do
    if ! command -v "$tool" &> /dev/null; then
        echo -e "${RED}ERROR:${NC} Required tool '$tool' not found"
        echo "Install with: brew install $tool (macOS) or apt install $tool (Linux)"
        exit 1
    fi
done

# =============================================================================
# TEST 1: DNS RESOLUTION
# =============================================================================
echo "Test 1: DNS Resolution"
if host "$DOMAIN" > /dev/null 2>&1; then
    IP=$(host "$DOMAIN" | grep "has address" | head -1 | awk '{print $4}')
    test_pass "DNS resolves to $IP"
else
    test_fail "DNS resolution failed for $DOMAIN"
fi

# =============================================================================
# TEST 2: HTTP → HTTPS REDIRECT
# =============================================================================
echo "Test 2: HTTP → HTTPS Redirect (301)"
HTTP_RESPONSE=$(curl -s -I -L "http://$DOMAIN/" 2>&1 || true)
if echo "$HTTP_RESPONSE" | grep -q "301 Moved Permanently"; then
    test_pass "HTTP redirects to HTTPS with 301"
elif echo "$HTTP_RESPONSE" | grep -q "302 Found"; then
    test_warn "HTTP redirects with 302 (should be 301 for SEO)"
else
    test_fail "HTTP → HTTPS redirect not working"
fi

# =============================================================================
# TEST 3: HTTPS CONNECTIVITY
# =============================================================================
echo "Test 3: HTTPS Connectivity"
if curl -s --max-time 10 "https://$DOMAIN/health" > /dev/null 2>&1; then
    test_pass "HTTPS connection successful"
else
    test_fail "Cannot connect via HTTPS"
fi

# =============================================================================
# TEST 4: HSTS HEADER
# =============================================================================
echo "Test 4: HSTS Header (Strict-Transport-Security)"
HSTS_HEADER=$(curl -s -I "https://$DOMAIN/" 2>&1 | grep -i "strict-transport-security" || true)
if [ -n "$HSTS_HEADER" ]; then
    if echo "$HSTS_HEADER" | grep -q "max-age=31536000"; then
        test_pass "HSTS enabled with 1-year max-age"
    else
        test_warn "HSTS enabled but max-age < 1 year"
    fi
    if echo "$HSTS_HEADER" | grep -q "includeSubDomains"; then
        test_pass "HSTS includes subdomains"
    fi
    if echo "$HSTS_HEADER" | grep -q "preload"; then
        test_pass "HSTS preload enabled"
    fi
else
    test_fail "HSTS header not present"
fi

# =============================================================================
# TEST 5: SECURITY HEADERS
# =============================================================================
echo "Test 5: Security Headers"

# X-Frame-Options
if curl -s -I "https://$DOMAIN/" | grep -qi "X-Frame-Options.*DENY"; then
    test_pass "X-Frame-Options: DENY"
else
    test_fail "X-Frame-Options header missing or incorrect"
fi

# X-Content-Type-Options
if curl -s -I "https://$DOMAIN/" | grep -qi "X-Content-Type-Options.*nosniff"; then
    test_pass "X-Content-Type-Options: nosniff"
else
    test_fail "X-Content-Type-Options header missing"
fi

# X-XSS-Protection
if curl -s -I "https://$DOMAIN/" | grep -qi "X-XSS-Protection"; then
    test_pass "X-XSS-Protection present"
else
    test_warn "X-XSS-Protection header missing (deprecated but good to have)"
fi

# Content-Security-Policy
if curl -s -I "https://$DOMAIN/" | grep -qi "Content-Security-Policy"; then
    test_pass "Content-Security-Policy present"
else
    test_warn "Content-Security-Policy header missing"
fi

# Referrer-Policy
if curl -s -I "https://$DOMAIN/" | grep -qi "Referrer-Policy"; then
    test_pass "Referrer-Policy present"
else
    test_warn "Referrer-Policy header missing"
fi

# =============================================================================
# TEST 6: TLS VERSION
# =============================================================================
echo "Test 6: TLS Protocol Version"

# Test TLS 1.2
if openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -tls1_2 < /dev/null 2>&1 | grep -q "Protocol.*TLSv1.2"; then
    test_pass "TLS 1.2 supported"
else
    test_fail "TLS 1.2 not supported"
fi

# Test TLS 1.3
if openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -tls1_3 < /dev/null 2>&1 | grep -q "Protocol.*TLSv1.3"; then
    test_pass "TLS 1.3 supported"
else
    test_warn "TLS 1.3 not supported (optional but recommended)"
fi

# Test TLS 1.1 (should fail)
if openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -tls1_1 < /dev/null 2>&1 | grep -q "Protocol.*TLSv1.1"; then
    test_fail "TLS 1.1 still enabled (deprecated, should be disabled)"
else
    test_pass "TLS 1.1 disabled (good)"
fi

# Test TLS 1.0 (should fail)
if openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -tls1 < /dev/null 2>&1 | grep -q "Protocol.*TLSv1\s"; then
    test_fail "TLS 1.0 still enabled (deprecated, should be disabled)"
else
    test_pass "TLS 1.0 disabled (good)"
fi

# =============================================================================
# TEST 7: CERTIFICATE VALIDITY
# =============================================================================
echo "Test 7: Certificate Validity"

CERT_INFO=$(openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -servername "$DOMAIN" < /dev/null 2>&1)

# Check if certificate is valid
if echo "$CERT_INFO" | grep -q "Verify return code: 0 (ok)"; then
    test_pass "Certificate is valid and trusted"
else
    VERIFY_ERROR=$(echo "$CERT_INFO" | grep "Verify return code" || echo "Unknown error")
    test_fail "Certificate validation failed: $VERIFY_ERROR"
fi

# Check certificate expiry
EXPIRY_DATE=$(echo "$CERT_INFO" | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)
if [ -n "$EXPIRY_DATE" ]; then
    EXPIRY_TIMESTAMP=$(date -j -f "%b %d %T %Y %Z" "$EXPIRY_DATE" "+%s" 2>/dev/null || date -d "$EXPIRY_DATE" "+%s" 2>/dev/null)
    NOW_TIMESTAMP=$(date "+%s")
    DAYS_UNTIL_EXPIRY=$(( (EXPIRY_TIMESTAMP - NOW_TIMESTAMP) / 86400 ))

    if [ "$DAYS_UNTIL_EXPIRY" -gt 30 ]; then
        test_pass "Certificate expires in $DAYS_UNTIL_EXPIRY days ($EXPIRY_DATE)"
    elif [ "$DAYS_UNTIL_EXPIRY" -gt 0 ]; then
        test_warn "Certificate expires soon: $DAYS_UNTIL_EXPIRY days ($EXPIRY_DATE)"
    else
        test_fail "Certificate expired! ($EXPIRY_DATE)"
    fi
fi

# =============================================================================
# TEST 8: CIPHER SUITE
# =============================================================================
echo "Test 8: Cipher Suite Strength"

CIPHER=$(openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -cipher "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256" < /dev/null 2>&1 | grep "Cipher" | head -1)

if echo "$CIPHER" | grep -q "GCM"; then
    test_pass "Strong AEAD cipher in use (GCM)"
elif echo "$CIPHER" | grep -q "CHACHA20"; then
    test_pass "Strong AEAD cipher in use (CHACHA20-POLY1305)"
else
    test_warn "Cipher suite could be stronger: $CIPHER"
fi

# =============================================================================
# TEST 9: OCSP STAPLING
# =============================================================================
echo "Test 9: OCSP Stapling"

OCSP_OUTPUT=$(openssl s_client -connect "$DOMAIN:$PORT_HTTPS" -status < /dev/null 2>&1)

if echo "$OCSP_OUTPUT" | grep -q "OCSP Response Status: successful"; then
    test_pass "OCSP stapling enabled and working"
else
    test_warn "OCSP stapling not detected (optional optimization)"
fi

# =============================================================================
# TEST 10: HTTP/2 SUPPORT
# =============================================================================
echo "Test 10: HTTP/2 Support"

if curl -s -I --http2 "https://$DOMAIN/" 2>&1 | grep -q "HTTP/2"; then
    test_pass "HTTP/2 enabled"
else
    test_warn "HTTP/2 not enabled (recommended for performance)"
fi

# =============================================================================
# TEST 11: API ENDPOINTS
# =============================================================================
echo "Test 11: API Endpoint Access"

# Test health endpoint
if curl -s "https://$DOMAIN/health" 2>&1 | grep -q "ok"; then
    test_pass "Health endpoint accessible via HTTPS"
else
    test_warn "Health endpoint not responding (may be normal if backend not running)"
fi

# Test discovery endpoint
if curl -s "https://$DOMAIN/.well-known/agent.json" 2>&1 | grep -q "name\|version"; then
    test_pass "Discovery endpoint accessible via HTTPS"
else
    test_warn "Discovery endpoint not responding (may be normal if backend not running)"
fi

# =============================================================================
# SUMMARY
# =============================================================================
print_header "Test Summary"
echo ""
echo "Domain: $DOMAIN"
echo ""
echo -e "${GREEN}Passed:${NC}   $PASSED"
echo -e "${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "${RED}Failed:${NC}   $FAILED"
echo ""

if [ "$FAILED" -eq 0 ] && [ "$WARNINGS" -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed! HTTPS configuration is excellent.${NC}"
    EXIT_CODE=0
elif [ "$FAILED" -eq 0 ]; then
    echo -e "${YELLOW}⚠ All critical tests passed, but there are warnings.${NC}"
    echo "Review warnings above and consider improvements."
    EXIT_CODE=0
else
    echo -e "${RED}✗ Some tests failed. HTTPS configuration needs attention.${NC}"
    echo "Review failures above and fix issues."
    EXIT_CODE=1
fi

echo ""
echo "Additional testing:"
echo "  1. SSL Labs: https://www.ssllabs.com/ssltest/analyze.html?d=$DOMAIN"
echo "  2. SecurityHeaders.com: https://securityheaders.com/?q=https://$DOMAIN"
echo "  3. Mozilla Observatory: https://observatory.mozilla.org/analyze/$DOMAIN"
echo ""

exit $EXIT_CODE
