#!/bin/bash
# =============================================================================
# Nginx Configuration Switcher
# =============================================================================
# Switches between development and production nginx configurations
#
# Usage:
#   ./switch-config.sh development
#   ./switch-config.sh production
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-development}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=========================================================================="
echo "Nginx Configuration Switcher"
echo "=========================================================================="

case "$MODE" in
    development|dev)
        echo "Switching to DEVELOPMENT configuration..."
        ln -sf api.conf.development "$SCRIPT_DIR/api.conf"
        echo -e "${GREEN}✓${NC} Linked api.conf → api.conf.development"
        echo ""
        echo "Development mode:"
        echo "  - HTTP allowed (no redirect)"
        echo "  - Self-signed certificates"
        echo "  - Relaxed rate limiting"
        echo "  - Metrics endpoint accessible"
        ;;

    production|prod)
        echo "Switching to PRODUCTION configuration..."
        ln -sf api.conf.production "$SCRIPT_DIR/api.conf"
        echo -e "${GREEN}✓${NC} Linked api.conf → api.conf.production"
        echo ""
        echo "Production mode:"
        echo "  - HTTP→HTTPS redirect enforced"
        echo "  - Let's Encrypt certificates"
        echo "  - Strict rate limiting"
        echo "  - HSTS headers enabled"
        ;;

    *)
        echo "Error: Invalid mode '$MODE'"
        echo ""
        echo "Usage: $0 [development|production]"
        exit 1
        ;;
esac

echo ""
echo "Current configuration:"
ls -l "$SCRIPT_DIR/api.conf" | awk '{print "  " $9, $10, $11}'
echo ""
echo -e "${YELLOW}Note:${NC} Restart nginx for changes to take effect:"
echo "  docker compose restart nginx"
echo "=========================================================================="
