#!/bin/bash
# =============================================================================
# Development HTTPS Setup - One-Command Setup
# =============================================================================
# Sets up HTTPS for local development with self-signed certificates
#
# Usage:
#   ./scripts/setup-https-dev.sh
# =============================================================================

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=========================================================================="
echo -e "${BLUE}Development HTTPS Setup${NC}"
echo "=========================================================================="
echo ""

# Step 1: Generate self-signed certificate
echo -e "${BLUE}[1/3]${NC} Generating self-signed SSL certificate..."
cd "$PROJECT_ROOT/docker/nginx/ssl"
if [ -f "self-signed.crt" ] && [ -f "self-signed.key" ]; then
    echo "  ✓ Certificate already exists, skipping generation"
else
    ./generate-self-signed.sh
fi
echo ""

# Step 2: Switch to development configuration
echo -e "${BLUE}[2/3]${NC} Switching to development nginx configuration..."
cd "$PROJECT_ROOT/docker/nginx/conf.d"
./switch-config.sh development
echo ""

# Step 3: Start nginx
echo -e "${BLUE}[3/3]${NC} Starting nginx..."
cd "$PROJECT_ROOT"
docker compose --profile development up -d nginx

# Wait for nginx to be ready
sleep 3

# Verify nginx is running
if docker compose ps nginx | grep -q "Up"; then
    echo -e "  ${GREEN}✓${NC} Nginx started successfully"
else
    echo -e "  ${YELLOW}⚠${NC} Nginx may not be running, check logs: docker compose logs nginx"
fi
echo ""

# Test endpoints
echo "=========================================================================="
echo -e "${GREEN}Setup Complete!${NC}"
echo "=========================================================================="
echo ""
echo "HTTPS is now enabled for development:"
echo ""
echo "  HTTP:  http://localhost/health"
echo "  HTTPS: https://localhost/health"
echo "  Docs:  https://localhost/api-docs/"
echo ""
echo "Note: Your browser will show a security warning for the self-signed"
echo "      certificate. This is expected - click 'Advanced' and proceed."
echo ""
echo "Test endpoints:"
echo "  curl http://localhost/health              # Test HTTP"
echo "  curl -k https://localhost/health          # Test HTTPS"
echo "  ./scripts/test-https.sh localhost         # Run full test suite"
echo ""
echo "=========================================================================="
