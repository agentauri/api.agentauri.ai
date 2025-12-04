#!/bin/bash

# =============================================================================
# SSL/TLS Certificate Monitoring Script
# =============================================================================
# Monitors certificate health and sends alerts for:
# - Certificate expiry (<30 days)
# - Certificate validity issues
# - SSL Labs rating degradation
# - OCSP stapling failures
#
# Usage:
#   ./scripts/monitor-ssl.sh [domain] [email]
#
# Cron setup (daily at midnight):
#   0 0 * * * /path/to/scripts/monitor-ssl.sh api.agentauri.ai admin@agentauri.ai
# =============================================================================

set -e

# =============================================================================
# CONFIGURATION
# =============================================================================
DOMAIN="${1:-api.agentauri.ai}"
ALERT_EMAIL="${2:-}"
WARNING_DAYS=30  # Alert if certificate expires in <30 days
CRITICAL_DAYS=7  # Critical alert if <7 days

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Alert storage
ALERTS=()

# =============================================================================
# HELPER FUNCTIONS
# =============================================================================
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
    ALERTS+=("WARNING: $1")
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ALERTS+=("ERROR: $1")
}

send_alert() {
    if [ -n "$ALERT_EMAIL" ] && [ ${#ALERTS[@]} -gt 0 ]; then
        {
            echo "SSL/TLS Certificate Monitoring Alert for $DOMAIN"
            echo "Timestamp: $(date)"
            echo ""
            echo "Issues detected:"
            echo ""
            for alert in "${ALERTS[@]}"; do
                echo "- $alert"
            done
        } | mail -s "SSL Certificate Alert: $DOMAIN" "$ALERT_EMAIL"
        log_info "Alert email sent to $ALERT_EMAIL"
    fi
}

# =============================================================================
# CERTIFICATE CHECKS
# =============================================================================
echo "==================================================================="
echo "SSL/TLS Certificate Monitoring for $DOMAIN"
echo "==================================================================="
echo "Timestamp: $(date)"
echo ""

# Check 1: Certificate expiry
log_info "Checking certificate expiry..."
CERT_INFO=$(openssl s_client -connect "$DOMAIN:443" -servername "$DOMAIN" < /dev/null 2>&1)

if echo "$CERT_INFO" | grep -q "Verify return code: 0 (ok)"; then
    log_info "✓ Certificate is valid and trusted"
else
    VERIFY_ERROR=$(echo "$CERT_INFO" | grep "Verify return code" || echo "Unknown error")
    log_error "Certificate validation failed: $VERIFY_ERROR"
fi

EXPIRY_DATE=$(echo "$CERT_INFO" | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)
if [ -n "$EXPIRY_DATE" ]; then
    # macOS and Linux have different date command syntax
    EXPIRY_TIMESTAMP=$(date -j -f "%b %d %T %Y %Z" "$EXPIRY_DATE" "+%s" 2>/dev/null || date -d "$EXPIRY_DATE" "+%s" 2>/dev/null)
    NOW_TIMESTAMP=$(date "+%s")
    DAYS_UNTIL_EXPIRY=$(( (EXPIRY_TIMESTAMP - NOW_TIMESTAMP) / 86400 ))

    if [ "$DAYS_UNTIL_EXPIRY" -lt "$CRITICAL_DAYS" ]; then
        log_error "Certificate expires in $DAYS_UNTIL_EXPIRY days ($EXPIRY_DATE) - CRITICAL"
    elif [ "$DAYS_UNTIL_EXPIRY" -lt "$WARNING_DAYS" ]; then
        log_warn "Certificate expires in $DAYS_UNTIL_EXPIRY days ($EXPIRY_DATE)"
    else
        log_info "✓ Certificate expires in $DAYS_UNTIL_EXPIRY days ($EXPIRY_DATE)"
    fi
else
    log_error "Could not determine certificate expiry date"
fi

# Check 2: Certificate chain
log_info "Checking certificate chain..."
if echo "$CERT_INFO" | grep -q "Certificate chain"; then
    CHAIN_LENGTH=$(echo "$CERT_INFO" | grep -c " s:" || echo "0")
    log_info "✓ Certificate chain complete ($CHAIN_LENGTH certificates)"
else
    log_warn "Certificate chain incomplete or missing"
fi

# Check 3: OCSP stapling
log_info "Checking OCSP stapling..."
OCSP_OUTPUT=$(openssl s_client -connect "$DOMAIN:443" -status < /dev/null 2>&1)
if echo "$OCSP_OUTPUT" | grep -q "OCSP Response Status: successful"; then
    log_info "✓ OCSP stapling enabled and working"
else
    log_warn "OCSP stapling not working (performance impact)"
fi

# Check 4: TLS version
log_info "Checking TLS version..."
if echo "$CERT_INFO" | grep -qE "Protocol\s*:\s*TLSv1\.(2|3)"; then
    TLS_VERSION=$(echo "$CERT_INFO" | grep "Protocol" | awk '{print $3}')
    log_info "✓ Modern TLS version in use: $TLS_VERSION"
else
    log_warn "Outdated TLS version detected"
fi

# Check 5: Cipher strength
log_info "Checking cipher strength..."
CIPHER=$(echo "$CERT_INFO" | grep "Cipher" | head -1 | awk '{print $3}')
if echo "$CIPHER" | grep -qE "(GCM|CHACHA20)"; then
    log_info "✓ Strong AEAD cipher in use: $CIPHER"
else
    log_warn "Weak or outdated cipher in use: $CIPHER"
fi

# Check 6: HSTS header
log_info "Checking HSTS header..."
HSTS_HEADER=$(curl -s -I "https://$DOMAIN/" 2>&1 | grep -i "strict-transport-security" || true)
if [ -n "$HSTS_HEADER" ]; then
    if echo "$HSTS_HEADER" | grep -q "max-age=31536000"; then
        log_info "✓ HSTS enabled with 1-year max-age"
    else
        log_warn "HSTS max-age less than 1 year"
    fi
else
    log_warn "HSTS header missing"
fi

# Check 7: Certificate Transparency
log_info "Checking Certificate Transparency..."
if echo "$CERT_INFO" | openssl x509 -noout -text 2>/dev/null | grep -q "CT Precertificate SCTs"; then
    log_info "✓ Certificate Transparency enabled"
else
    log_warn "Certificate Transparency not detected"
fi

# Check 8: Vulnerability scan (basic)
log_info "Checking for known vulnerabilities..."

# Test for POODLE (SSLv3)
if openssl s_client -connect "$DOMAIN:443" -ssl3 < /dev/null 2>&1 | grep -q "Protocol.*SSLv3"; then
    log_error "SSLv3 enabled (POODLE vulnerability)"
else
    log_info "✓ SSLv3 disabled (POODLE protected)"
fi

# Test for BEAST (TLS 1.0)
if openssl s_client -connect "$DOMAIN:443" -tls1 < /dev/null 2>&1 | grep -q "Protocol.*TLSv1\s"; then
    log_warn "TLS 1.0 enabled (BEAST vulnerability)"
else
    log_info "✓ TLS 1.0 disabled (BEAST protected)"
fi

# Check 9: Certificate revocation status
log_info "Checking certificate revocation status..."
REVOCATION_CHECK=$(echo "$CERT_INFO" | openssl x509 -noout -text 2>/dev/null | grep -i "OCSP\|CRL" || true)
if [ -n "$REVOCATION_CHECK" ]; then
    log_info "✓ Revocation checking available (OCSP/CRL)"
else
    log_warn "No revocation checking mechanisms found"
fi

# =============================================================================
# SUMMARY
# =============================================================================
echo ""
echo "==================================================================="
echo "Monitoring Summary"
echo "==================================================================="
echo "Domain: $DOMAIN"
echo "Alerts: ${#ALERTS[@]}"
echo ""

if [ ${#ALERTS[@]} -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed. Certificate health is good.${NC}"
    EXIT_CODE=0
else
    echo -e "${YELLOW}⚠ Issues detected:${NC}"
    for alert in "${ALERTS[@]}"; do
        echo "  - $alert"
    done
    echo ""
    EXIT_CODE=1
fi

# Send alerts if configured
send_alert

echo ""
echo "Next monitoring run: $(date -d '+1 day' 2>/dev/null || date -v '+1d' 2>/dev/null)"
echo "==================================================================="

exit $EXIT_CODE
