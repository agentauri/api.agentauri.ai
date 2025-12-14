#!/bin/bash
# =============================================================================
# Generate Self-Signed SSL Certificate for Development
# =============================================================================
# Creates a self-signed certificate for local HTTPS testing
# Valid for 365 days, suitable for development only
#
# Usage:
#   ./generate-self-signed.sh
#
# Output:
#   - self-signed.key (private key)
#   - self-signed.crt (certificate)
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_FILE="${SCRIPT_DIR}/self-signed.crt"
KEY_FILE="${SCRIPT_DIR}/self-signed.key"

echo "==================================================================="
echo "Generating Self-Signed SSL Certificate for Development"
echo "==================================================================="

# Check if certificate already exists
if [ -f "$CERT_FILE" ] && [ -f "$KEY_FILE" ]; then
    echo "⚠️  Self-signed certificate already exists!"
    echo "   Certificate: $CERT_FILE"
    echo "   Private Key: $KEY_FILE"
    echo ""
    read -p "Do you want to regenerate? (y/N): " -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "✅ Using existing certificate"
        exit 0
    fi
    echo "🔄 Regenerating certificate..."
fi

# Generate private key
echo "📝 Generating private key..."
openssl genrsa -out "$KEY_FILE" 2048

# Generate self-signed certificate
echo "📝 Generating self-signed certificate..."
openssl req -new -x509 -sha256 -key "$KEY_FILE" -out "$CERT_FILE" -days 365 \
    -subj "/C=US/ST=Development/L=Local/O=AgentAuri/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,DNS:api.agentauri.local,IP:127.0.0.1"

# Set appropriate permissions
chmod 600 "$KEY_FILE"
chmod 644 "$CERT_FILE"

echo ""
echo "==================================================================="
echo "✅ Self-Signed Certificate Generated Successfully"
echo "==================================================================="
echo "Certificate: $CERT_FILE"
echo "Private Key: $KEY_FILE"
echo "Valid for:   365 days"
echo "Domains:     localhost, api.agentauri.local, 127.0.0.1"
echo ""
echo "⚠️  SECURITY WARNING:"
echo "   This is a self-signed certificate for DEVELOPMENT ONLY"
echo "   Browsers will show a security warning - this is expected"
echo "   NEVER use self-signed certificates in production"
echo ""
echo "📋 Next Steps:"
echo "   1. Start nginx: docker compose --profile development up -d nginx"
echo "   2. Access API: https://localhost/api/v1/health"
echo "   3. Accept browser security warning for localhost"
echo "==================================================================="
