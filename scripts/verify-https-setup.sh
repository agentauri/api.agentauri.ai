#!/bin/bash

# Verification script for HTTPS/TLS setup completeness
# Checks all configuration files, scripts, and documentation

echo "=========================================================================="
echo "HTTPS/TLS Setup Verification"
echo "=========================================================================="
echo ""

ERRORS=0
WARNINGS=0

check_file() {
    if [ -f "$1" ]; then
        echo "✓ $1"
    else
        echo "✗ $1 (MISSING)"
        ERRORS=$((ERRORS + 1))
    fi
}

check_dir() {
    if [ -d "$1" ]; then
        echo "✓ $1/"
    else
        echo "✗ $1/ (MISSING)"
        ERRORS=$((ERRORS + 1))
    fi
}

echo "Nginx Configuration Files:"
check_file "docker/nginx/nginx.conf"
check_file "docker/nginx/conf.d/api.conf"
echo ""

echo "Docker Configuration:"
check_file "docker-compose.yml"
grep -q "nginx:" docker-compose.yml && echo "✓ nginx service configured" || { echo "✗ nginx service missing"; ERRORS=$((ERRORS + 1)); }
grep -q "certbot:" docker-compose.yml && echo "✓ certbot service configured" || { echo "✗ certbot service missing"; ERRORS=$((ERRORS + 1)); }
echo ""

echo "Automation Scripts:"
check_file "scripts/init-letsencrypt.sh"
test -x "scripts/init-letsencrypt.sh" && echo "  ✓ executable" || { echo "  ✗ not executable"; WARNINGS=$((WARNINGS + 1)); }
check_file "scripts/test-https.sh"
test -x "scripts/test-https.sh" && echo "  ✓ executable" || { echo "  ✗ not executable"; WARNINGS=$((WARNINGS + 1)); }
check_file "scripts/monitor-ssl.sh"
test -x "scripts/monitor-ssl.sh" && echo "  ✓ executable" || { echo "  ✗ not executable"; WARNINGS=$((WARNINGS + 1)); }
echo ""

echo "Documentation:"
check_file "docs/deployment/HTTPS_SETUP.md"
check_file "docs/deployment/HTTPS_QUICK_REFERENCE.md"
check_file "docs/deployment/HTTPS_IMPLEMENTATION_SUMMARY.md"
echo ""

echo "Certificate Directories:"
check_dir "docker/certbot/conf"
check_dir "docker/certbot/www"
check_dir "docker/nginx/conf.d"
echo ""

echo "Environment Configuration:"
check_file ".env.example"
grep -q "DOMAIN=" .env.example && echo "✓ DOMAIN variable documented" || { echo "✗ DOMAIN variable missing"; WARNINGS=$((WARNINGS + 1)); }
grep -q "LETSENCRYPT_EMAIL=" .env.example && echo "✓ LETSENCRYPT_EMAIL variable documented" || { echo "✗ LETSENCRYPT_EMAIL missing"; WARNINGS=$((WARNINGS + 1)); }
grep -q "ENABLE_HTTPS=" .env.example && echo "✓ ENABLE_HTTPS variable documented" || { echo "✗ ENABLE_HTTPS missing"; WARNINGS=$((WARNINGS + 1)); }
echo ""

echo "=========================================================================="
echo "Verification Summary"
echo "=========================================================================="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo "✓ HTTPS/TLS setup is complete and ready for deployment!"
    exit 0
elif [ $ERRORS -eq 0 ]; then
    echo "⚠ Setup complete with warnings. Review above."
    exit 0
else
    echo "✗ Setup incomplete. Fix errors above."
    exit 1
fi
