#!/bin/bash
# ============================================================================
# PostgreSQL TLS Connection Testing Script
# ============================================================================
# Tests TLS encryption and certificate validation
# ============================================================================

set -e

# Default connection parameters
DB_HOST="${1:-localhost}"
DB_PORT="${2:-5432}"
DB_NAME="${3:-agentauri_backend}"
DB_USER="${4:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-postgres}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "========================================================================"
echo "PostgreSQL TLS Connection Tests"
echo "========================================================================"
echo ""
echo "Testing connection to: $DB_HOST:$DB_PORT/$DB_NAME"
echo ""

# Check if psql is installed
if ! command -v psql &> /dev/null; then
    echo -e "${RED}ERROR: psql is not installed${NC}"
    echo "Install with: brew install postgresql (macOS) or apt-get install postgresql-client (Linux)"
    exit 1
fi

# Check if certificate exists
CERT_PATH="./docker/postgres/certs/root.crt"
if [ ! -f "$CERT_PATH" ]; then
    echo -e "${RED}ERROR: CA certificate not found at $CERT_PATH${NC}"
    echo "Run: ./scripts/generate-pg-certs.sh"
    exit 1
fi

# Test counter
PASSED=0
FAILED=0

# ----------------------------------------------------------------------------
# Test 1: TLS Connection with Certificate Verification
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 1: TLS connection with certificate verification${NC}"

PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -c "SELECT version();" > /dev/null 2>&1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ PASS: TLS connection successful${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL: TLS connection failed${NC}"
    echo "Check that PostgreSQL is running and TLS is enabled"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 2: Non-TLS Connection (Should Fail)
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 2: Non-TLS connection (should fail if TLS enforced)${NC}"

PGPASSWORD="$DB_PASSWORD" PGSSLMODE=disable \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -c "SELECT version();" > /dev/null 2>&1

if [ $? -ne 0 ]; then
    echo -e "${GREEN}✓ PASS: Non-TLS connection rejected (TLS enforced)${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ WARNING: Non-TLS connection allowed (TLS not enforced)${NC}"
    echo "Check pg_hba.conf - should use 'hostssl' instead of 'host'"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 3: Check TLS Version
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 3: TLS protocol version${NC}"

TLS_VERSION=$(PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -t -c "SHOW ssl_version;" 2>/dev/null | xargs)

if [[ "$TLS_VERSION" =~ ^TLSv1\.[23]$ ]]; then
    echo -e "${GREEN}✓ PASS: TLS version is $TLS_VERSION (secure)${NC}"
    ((PASSED++))
elif [ -z "$TLS_VERSION" ]; then
    echo -e "${RED}✗ FAIL: Could not determine TLS version${NC}"
    ((FAILED++))
else
    echo -e "${YELLOW}⚠ WARNING: TLS version is $TLS_VERSION (consider upgrading to TLS 1.2+)${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 4: Check pgcrypto Extension
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 4: pgcrypto extension (for column encryption)${NC}"

PGCRYPTO_VERSION=$(PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -t -c "SELECT installed_version FROM pg_available_extensions WHERE name = 'pgcrypto';" 2>/dev/null | xargs)

if [ -n "$PGCRYPTO_VERSION" ]; then
    echo -e "${GREEN}✓ PASS: pgcrypto extension available (version $PGCRYPTO_VERSION)${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL: pgcrypto extension not available${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 5: Check Cipher Suites
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 5: TLS cipher suites${NC}"

SSL_CIPHER=$(PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -t -c "SHOW ssl_ciphers;" 2>/dev/null | xargs)

if [ -n "$SSL_CIPHER" ]; then
    echo -e "${GREEN}✓ PASS: Cipher configuration: $SSL_CIPHER${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ WARNING: Could not determine cipher configuration${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 6: Password Encryption Method
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 6: Password encryption method${NC}"

PASSWORD_ENC=$(PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -t -c "SHOW password_encryption;" 2>/dev/null | xargs)

if [ "$PASSWORD_ENC" = "scram-sha-256" ]; then
    echo -e "${GREEN}✓ PASS: Password encryption is scram-sha-256 (secure)${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ WARNING: Password encryption is $PASSWORD_ENC (consider scram-sha-256)${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 7: Certificate Validity
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 7: Certificate validity period${NC}"

if [ -f "./docker/postgres/certs/server.crt" ]; then
    EXPIRY_DATE=$(openssl x509 -in "./docker/postgres/certs/server.crt" -noout -enddate | cut -d= -f2)
    EXPIRY_EPOCH=$(date -j -f "%b %d %T %Y %Z" "$EXPIRY_DATE" "+%s" 2>/dev/null || date -d "$EXPIRY_DATE" "+%s" 2>/dev/null)
    CURRENT_EPOCH=$(date "+%s")
    DAYS_UNTIL_EXPIRY=$(( ($EXPIRY_EPOCH - $CURRENT_EPOCH) / 86400 ))

    if [ $DAYS_UNTIL_EXPIRY -gt 30 ]; then
        echo -e "${GREEN}✓ PASS: Certificate valid for $DAYS_UNTIL_EXPIRY more days${NC}"
        ((PASSED++))
    elif [ $DAYS_UNTIL_EXPIRY -gt 0 ]; then
        echo -e "${YELLOW}⚠ WARNING: Certificate expires in $DAYS_UNTIL_EXPIRY days - consider renewal${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL: Certificate expired ${DAYS_UNTIL_EXPIRY#-} days ago${NC}"
        ((FAILED++))
    fi
else
    echo -e "${YELLOW}⚠ WARNING: Server certificate not found at ./docker/postgres/certs/server.crt${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Test 8: Test Encryption/Decryption Functions
# ----------------------------------------------------------------------------
echo -e "${BLUE}Test 8: pgcrypto encryption/decryption${NC}"

TEST_RESULT=$(PGPASSWORD="$DB_PASSWORD" PGSSLMODE=require PGSSLROOTCERT="$CERT_PATH" \
  psql "host=$DB_HOST port=$DB_PORT dbname=$DB_NAME user=$DB_USER" \
  -t -c "SELECT pgp_sym_decrypt(pgp_sym_encrypt('test', 'secret'), 'secret');" 2>/dev/null | xargs)

if [ "$TEST_RESULT" = "test" ]; then
    echo -e "${GREEN}✓ PASS: pgcrypto encryption/decryption working${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ FAIL: pgcrypto encryption/decryption failed${NC}"
    ((FAILED++))
fi
echo ""

# ----------------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------------
echo "========================================================================"
echo "Test Summary"
echo "========================================================================"
echo -e "Total tests: $(($PASSED + $FAILED))"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed! PostgreSQL TLS encryption is working correctly.${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Review the output above for details.${NC}"
    exit 1
fi
