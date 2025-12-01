#!/bin/bash
# ============================================================================
# PostgreSQL TLS Certificate Generation Script
# ============================================================================
# Generates self-signed certificates for local development
# For production, use certificates from a trusted CA (Let's Encrypt, DigiCert)
# ============================================================================

set -e

CERT_DIR="./docker/postgres/certs"
VALIDITY_DAYS_CA=3650    # 10 years for CA
VALIDITY_DAYS_SERVER=365 # 1 year for server cert

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================================================"
echo "PostgreSQL TLS Certificate Generation"
echo "========================================================================"
echo ""

# Check if OpenSSL is installed
if ! command -v openssl &> /dev/null; then
    echo -e "${RED}ERROR: OpenSSL is not installed${NC}"
    echo "Install with: brew install openssl (macOS) or apt-get install openssl (Linux)"
    exit 1
fi

# Create certificate directory
mkdir -p "$CERT_DIR"

echo -e "${YELLOW}Generating certificates in: $CERT_DIR${NC}"
echo ""

# ----------------------------------------------------------------------------
# Step 1: Generate Certificate Authority (CA)
# ----------------------------------------------------------------------------
echo "Step 1: Generating Certificate Authority (CA)..."

if [ -f "$CERT_DIR/root.crt" ]; then
    echo -e "${YELLOW}WARNING: root.crt already exists. Overwrite? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "Skipping CA generation. Using existing certificates."
        exit 0
    fi
fi

openssl req -new -x509 -days "$VALIDITY_DAYS_CA" -nodes -text \
  -out "$CERT_DIR/root.crt" \
  -keyout "$CERT_DIR/root.key" \
  -subj "/C=US/ST=California/L=San Francisco/O=ERC-8004 Dev/CN=PostgreSQL CA" \
  2>/dev/null

chmod 600 "$CERT_DIR/root.key"
chmod 644 "$CERT_DIR/root.crt"

echo -e "${GREEN}✓ CA certificate generated (valid for $VALIDITY_DAYS_CA days)${NC}"
echo ""

# ----------------------------------------------------------------------------
# Step 2: Generate Server Certificate
# ----------------------------------------------------------------------------
echo "Step 2: Generating server certificate..."

# Generate private key
openssl genrsa -out "$CERT_DIR/server.key" 2048 2>/dev/null
chmod 600 "$CERT_DIR/server.key"

# Create certificate signing request (CSR)
openssl req -new -key "$CERT_DIR/server.key" -text \
  -out "$CERT_DIR/server.csr" \
  -subj "/C=US/ST=California/L=San Francisco/O=ERC-8004 Dev/CN=localhost" \
  2>/dev/null

# Create SAN (Subject Alternative Name) configuration
cat > "$CERT_DIR/san.cnf" <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req

[req_distinguished_name]

[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = postgres
DNS.4 = erc8004-postgres
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Sign server certificate with CA
openssl x509 -req -in "$CERT_DIR/server.csr" \
  -text -days "$VALIDITY_DAYS_SERVER" \
  -CA "$CERT_DIR/root.crt" \
  -CAkey "$CERT_DIR/root.key" \
  -CAcreateserial \
  -out "$CERT_DIR/server.crt" \
  -extfile "$CERT_DIR/san.cnf" \
  -extensions v3_req \
  2>/dev/null

chmod 644 "$CERT_DIR/server.crt"

# Cleanup temporary files
rm -f "$CERT_DIR/server.csr" "$CERT_DIR/san.cnf"

echo -e "${GREEN}✓ Server certificate generated (valid for $VALIDITY_DAYS_SERVER days)${NC}"
echo ""

# ----------------------------------------------------------------------------
# Step 3: Verify Certificates
# ----------------------------------------------------------------------------
echo "Step 3: Verifying certificates..."

# Verify server certificate against CA
openssl verify -CAfile "$CERT_DIR/root.crt" "$CERT_DIR/server.crt" > /dev/null 2>&1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Certificate verification successful${NC}"
else
    echo -e "${RED}✗ Certificate verification failed${NC}"
    exit 1
fi

echo ""

# ----------------------------------------------------------------------------
# Step 4: Display Certificate Information
# ----------------------------------------------------------------------------
echo "========================================================================"
echo "Certificate Summary"
echo "========================================================================"
echo ""
echo "Files generated in $CERT_DIR:"
echo "  - root.crt     : CA certificate (for client verification)"
echo "  - root.key     : CA private key (keep secure!)"
echo "  - server.crt   : Server certificate"
echo "  - server.key   : Server private key (keep secure!)"
echo ""
echo "Certificate Details:"
echo "----------------------------------------"
echo "CA Certificate:"
openssl x509 -in "$CERT_DIR/root.crt" -noout -subject -issuer -dates
echo ""
echo "Server Certificate:"
openssl x509 -in "$CERT_DIR/server.crt" -noout -subject -issuer -dates
echo ""
echo "Subject Alternative Names (SAN):"
openssl x509 -in "$CERT_DIR/server.crt" -noout -text | grep -A 1 "Subject Alternative Name"
echo ""

# ----------------------------------------------------------------------------
# Step 5: Security Warnings
# ----------------------------------------------------------------------------
echo "========================================================================"
echo "SECURITY WARNINGS"
echo "========================================================================"
echo -e "${YELLOW}⚠  These are self-signed certificates for DEVELOPMENT ONLY${NC}"
echo -e "${YELLOW}⚠  For production, use certificates from a trusted CA${NC}"
echo -e "${YELLOW}⚠  Keep root.key and server.key secure (chmod 600)${NC}"
echo -e "${YELLOW}⚠  Never commit certificates to version control${NC}"
echo ""

# Add to .gitignore
if ! grep -q "docker/postgres/certs/" .gitignore 2>/dev/null; then
    echo "docker/postgres/certs/*.crt" >> .gitignore
    echo "docker/postgres/certs/*.key" >> .gitignore
    echo "docker/postgres/certs/*.srl" >> .gitignore
    echo -e "${GREEN}✓ Added certificates to .gitignore${NC}"
fi

echo ""
echo "========================================================================"
echo "Next Steps"
echo "========================================================================"
echo "1. Start PostgreSQL with TLS enabled:"
echo "   docker compose up -d postgres"
echo ""
echo "2. Test TLS connection:"
echo "   ./scripts/test-pg-tls.sh"
echo ""
echo "3. Update DATABASE_URL in .env:"
echo "   DATABASE_URL=postgresql://postgres:password@localhost:5432/erc8004_backend?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt"
echo ""
echo -e "${GREEN}Certificate generation complete!${NC}"
echo ""
