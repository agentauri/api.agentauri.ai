#!/bin/bash

# =============================================================================
# Let's Encrypt Initialization Script for api.8004.dev
# =============================================================================
# This script initializes HTTPS/TLS with automatic certificate management:
# 1. Downloads recommended TLS parameters
# 2. Creates dummy certificate for initial nginx startup
# 3. Requests real certificate from Let's Encrypt
# 4. Reloads nginx with production certificate
#
# Usage:
#   ./scripts/init-letsencrypt.sh [--staging]
#
# Options:
#   --staging    Use Let's Encrypt staging environment (for testing)
#
# Prerequisites:
#   - Domain name (api.8004.dev) pointing to this server
#   - Ports 80 and 443 open in firewall
#   - Docker and Docker Compose installed
#   - Valid email address for certificate notifications
# =============================================================================

set -e  # Exit on error

# =============================================================================
# CONFIGURATION
# =============================================================================
DOMAIN="${DOMAIN:-api.8004.dev}"
EMAIL="${LETSENCRYPT_EMAIL:-admin@8004.dev}"  # Change this!
STAGING=0  # Set to 1 for testing with staging environment

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CERTBOT_DIR="$PROJECT_ROOT/docker/certbot"
NGINX_CONF="$PROJECT_ROOT/docker/nginx/conf.d/api.conf"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# =============================================================================
# HELPER FUNCTIONS
# =============================================================================
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# =============================================================================
# PARSE ARGUMENTS
# =============================================================================
while [[ $# -gt 0 ]]; do
    case $1 in
        --staging)
            STAGING=1
            log_warn "Using Let's Encrypt STAGING environment (certificates will not be trusted)"
            shift
            ;;
        --help)
            echo "Usage: $0 [--staging]"
            echo "Initialize Let's Encrypt SSL/TLS certificates for $DOMAIN"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--staging]"
            exit 1
            ;;
    esac
done

# =============================================================================
# PREFLIGHT CHECKS
# =============================================================================
log_info "Starting Let's Encrypt initialization for $DOMAIN"
echo ""

# Check if running from project root
if [ ! -f "$PROJECT_ROOT/docker-compose.yml" ]; then
    log_error "docker-compose.yml not found. Run this script from the project root."
    exit 1
fi

# Check if domain is reachable
log_info "Checking DNS resolution for $DOMAIN..."
if ! host "$DOMAIN" > /dev/null 2>&1; then
    log_warn "DNS resolution failed for $DOMAIN"
    log_warn "Make sure your domain points to this server before continuing"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Aborted by user"
        exit 1
    fi
fi

# Validate email
if [[ ! "$EMAIL" =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
    log_error "Invalid email address: $EMAIL"
    log_error "Set LETSENCRYPT_EMAIL in .env file or update this script"
    exit 1
fi

# =============================================================================
# DIRECTORY SETUP
# =============================================================================
log_info "Creating certificate directories..."
mkdir -p "$CERTBOT_DIR/conf/live/$DOMAIN"
mkdir -p "$CERTBOT_DIR/www"

# =============================================================================
# DOWNLOAD TLS PARAMETERS
# =============================================================================
if [ ! -e "$CERTBOT_DIR/conf/options-ssl-nginx.conf" ]; then
    log_info "Downloading recommended TLS parameters..."
    curl -s https://raw.githubusercontent.com/certbot/certbot/master/certbot-nginx/certbot_nginx/_internal/tls_configs/options-ssl-nginx.conf \
        > "$CERTBOT_DIR/conf/options-ssl-nginx.conf"
    log_info "✓ Downloaded options-ssl-nginx.conf"
fi

if [ ! -e "$CERTBOT_DIR/conf/ssl-dhparams.pem" ]; then
    log_info "Downloading Diffie-Hellman parameters (2048-bit)..."
    curl -s https://ssl-config.mozilla.org/ffdhe2048.txt \
        > "$CERTBOT_DIR/conf/ssl-dhparams.pem"
    log_info "✓ Downloaded ssl-dhparams.pem"
fi

# =============================================================================
# CREATE DUMMY CERTIFICATE
# =============================================================================
log_info "Creating dummy certificate for nginx initial startup..."

# Remove existing dummy certificate if present
if [ -d "$CERTBOT_DIR/conf/live/$DOMAIN" ]; then
    log_warn "Removing existing certificate directory..."
    rm -rf "$CERTBOT_DIR/conf/live/$DOMAIN"
    rm -rf "$CERTBOT_DIR/conf/archive/$DOMAIN"
    rm -rf "$CERTBOT_DIR/conf/renewal/$DOMAIN.conf"
fi

mkdir -p "$CERTBOT_DIR/conf/live/$DOMAIN"

# Generate dummy certificate
docker compose -f "$PROJECT_ROOT/docker-compose.yml" run --rm --entrypoint "\
    openssl req -x509 -nodes -newkey rsa:2048 -days 1 \
    -keyout '/etc/letsencrypt/live/$DOMAIN/privkey.pem' \
    -out '/etc/letsencrypt/live/$DOMAIN/fullchain.pem' \
    -subj '/CN=localhost'" certbot

log_info "✓ Dummy certificate created"

# =============================================================================
# START NGINX
# =============================================================================
log_info "Starting nginx with dummy certificate..."
docker compose -f "$PROJECT_ROOT/docker-compose.yml" up -d nginx

# Wait for nginx to start
sleep 5

# Check if nginx is running
if ! docker compose -f "$PROJECT_ROOT/docker-compose.yml" ps nginx | grep -q "Up"; then
    log_error "Nginx failed to start. Check logs with: docker compose logs nginx"
    exit 1
fi

log_info "✓ Nginx started successfully"

# =============================================================================
# REQUEST REAL CERTIFICATE
# =============================================================================
log_info "Deleting dummy certificate..."
docker compose -f "$PROJECT_ROOT/docker-compose.yml" run --rm --entrypoint "\
    rm -rf /etc/letsencrypt/live/$DOMAIN && \
    rm -rf /etc/letsencrypt/archive/$DOMAIN && \
    rm -rf /etc/letsencrypt/renewal/$DOMAIN.conf" certbot

log_info "Requesting Let's Encrypt certificate for $DOMAIN..."
echo "  Email: $EMAIL"
if [ $STAGING != "0" ]; then
    echo "  Environment: STAGING (not trusted)"
    STAGING_ARG="--staging"
else
    echo "  Environment: PRODUCTION"
    STAGING_ARG=""
fi
echo ""

# Request certificate
docker compose -f "$PROJECT_ROOT/docker-compose.yml" run --rm --entrypoint "\
    certbot certonly --webroot -w /var/www/certbot \
    $STAGING_ARG \
    --email $EMAIL \
    --agree-tos \
    --no-eff-email \
    --force-renewal \
    -d $DOMAIN" certbot

# Check if certificate was issued
if [ ! -f "$CERTBOT_DIR/conf/live/$DOMAIN/fullchain.pem" ]; then
    log_error "Certificate request failed!"
    log_error "Check logs with: docker compose logs certbot"
    exit 1
fi

log_info "✓ Certificate issued successfully"

# =============================================================================
# RELOAD NGINX
# =============================================================================
log_info "Reloading nginx with production certificate..."
docker compose -f "$PROJECT_ROOT/docker-compose.yml" exec nginx nginx -s reload

log_info "✓ Nginx reloaded"

# =============================================================================
# SUCCESS
# =============================================================================
echo ""
echo "=========================================================================="
echo -e "${GREEN}SUCCESS!${NC} HTTPS/TLS setup complete for $DOMAIN"
echo "=========================================================================="
echo ""
echo "Certificate details:"
echo "  Domain: $DOMAIN"
echo "  Email: $EMAIL"
echo "  Issuer: Let's Encrypt"
if [ $STAGING != "0" ]; then
    echo -e "  ${YELLOW}Environment: STAGING (not trusted by browsers)${NC}"
    echo -e "  ${YELLOW}Run without --staging flag for production certificate${NC}"
else
    echo "  Environment: PRODUCTION"
fi
echo ""
echo "Certificate files:"
echo "  Fullchain: $CERTBOT_DIR/conf/live/$DOMAIN/fullchain.pem"
echo "  Private Key: $CERTBOT_DIR/conf/live/$DOMAIN/privkey.pem"
echo ""
echo "Next steps:"
echo "  1. Test HTTPS: curl https://$DOMAIN/health"
echo "  2. Test redirect: curl -I http://$DOMAIN/"
echo "  3. Check SSL rating: https://www.ssllabs.com/ssltest/analyze.html?d=$DOMAIN"
echo "  4. Run test script: ./scripts/test-https.sh"
echo ""
echo "Certificate renewal:"
echo "  Automatic renewal runs daily via certbot container"
echo "  Manual renewal: docker compose run --rm certbot renew"
echo ""
echo "=========================================================================="
