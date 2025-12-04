#!/usr/bin/env bash
#
# Manual Secret Rotation Script
#
# This script provides manual secret rotation for emergency use or scheduled maintenance.
# It supports both AWS Secrets Manager and HashiCorp Vault backends.
#
# Prerequisites:
# - AWS CLI (for AWS backend) or Vault CLI (for Vault backend)
# - Valid credentials configured
#
# Usage:
#   ./scripts/rotate-secrets.sh [OPTIONS] <secret-name>
#
# Options:
#   --backend aws|vault    Secrets backend (default: aws)
#   --region REGION        AWS region (default: us-east-1)
#   --profile PROFILE      AWS CLI profile (default: default)
#   --vault-addr ADDR      Vault server address
#   --vault-token TOKEN    Vault authentication token
#   --generate             Auto-generate new secret value
#   --dry-run              Show changes without applying
#   --help                 Show this help message
#
# Examples:
#   # Rotate JWT secret in AWS Secrets Manager
#   ./scripts/rotate-secrets.sh --backend aws --generate jwt_secret
#
#   # Rotate database password in Vault (manual entry)
#   ./scripts/rotate-secrets.sh --backend vault database_url
#
#   # Rotate API encryption key with auto-generation
#   ./scripts/rotate-secrets.sh --backend aws --generate api_encryption_key

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
BACKEND="aws"
AWS_REGION="${AWS_REGION:-us-east-1}"
AWS_PROFILE="${AWS_PROFILE:-default}"
VAULT_ADDR="${VAULT_ADDR:-http://localhost:8200}"
VAULT_TOKEN="${VAULT_TOKEN:-}"
AUTO_GENERATE=false
DRY_RUN=false
SECRET_PREFIX="agentauri"
SECRET_NAME=""

# Parse command-line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --backend)
            BACKEND="$2"
            shift 2
            ;;
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --profile)
            AWS_PROFILE="$2"
            shift 2
            ;;
        --vault-addr)
            VAULT_ADDR="$2"
            shift 2
            ;;
        --vault-token)
            VAULT_TOKEN="$2"
            shift 2
            ;;
        --generate)
            AUTO_GENERATE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            grep '^#' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            if [ -z "$SECRET_NAME" ]; then
                SECRET_NAME="$1"
                shift
            else
                echo -e "${RED}Unknown option: $1${NC}" >&2
                echo "Use --help for usage information" >&2
                exit 1
            fi
            ;;
    esac
done

# Validate required arguments
if [ -z "$SECRET_NAME" ]; then
    echo -e "${RED}ERROR: Secret name is required${NC}" >&2
    echo "Usage: $0 [OPTIONS] <secret-name>" >&2
    exit 1
fi

# Print configuration
echo -e "${BLUE}=== Secret Rotation ===${NC}"
echo "Backend: $BACKEND"
echo "Secret: $SECRET_NAME"
echo "Auto-generate: $AUTO_GENERATE"
echo "Dry run: $DRY_RUN"
echo ""

# Function to generate a secure random value
generate_secret_value() {
    local secret_type="$1"

    case "$secret_type" in
        jwt_secret|api_encryption_key)
            # Generate 32-byte base64 encoded value
            openssl rand -base64 32
            ;;
        stripe_secret_key)
            # Generate Stripe-like key (not real, for testing)
            echo "sk_live_$(openssl rand -hex 24)"
            ;;
        stripe_webhook_secret)
            # Generate webhook secret
            echo "whsec_$(openssl rand -hex 24)"
            ;;
        *)
            echo -e "${YELLOW}Cannot auto-generate for type: $secret_type${NC}" >&2
            echo -e "${YELLOW}Manual input required.${NC}" >&2
            return 1
            ;;
    esac
}

# Function to rotate secret in AWS Secrets Manager
rotate_aws_secret() {
    local secret_name="$1"
    local new_value="$2"
    local full_name="${SECRET_PREFIX}/${secret_name}"

    echo -e "${BLUE}Rotating AWS secret: $full_name${NC}"

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN] Would update secret in AWS Secrets Manager${NC}"
        return 0
    fi

    # Update secret value
    aws secretsmanager put-secret-value \
        --secret-id "$full_name" \
        --secret-string "$new_value" \
        --region "$AWS_REGION" \
        --profile "$AWS_PROFILE" > /dev/null

    echo -e "${GREEN}✓ Secret rotated successfully in AWS${NC}"

    # Tag with rotation metadata
    aws secretsmanager tag-resource \
        --secret-id "$full_name" \
        --tags Key=LastRotated,Value="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
               Key=RotatedBy,Value="manual-script" \
        --region "$AWS_REGION" \
        --profile "$AWS_PROFILE" > /dev/null

    echo -e "${GREEN}✓ Rotation metadata updated${NC}"
}

# Function to rotate secret in HashiCorp Vault
rotate_vault_secret() {
    local secret_name="$1"
    local new_value="$2"
    local vault_path="secret/data/${SECRET_PREFIX}/${secret_name}"

    echo -e "${BLUE}Rotating Vault secret: $vault_path${NC}"

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN] Would update secret in Vault${NC}"
        return 0
    fi

    # Update secret value
    export VAULT_ADDR="$VAULT_ADDR"
    if [ -n "$VAULT_TOKEN" ]; then
        export VAULT_TOKEN="$VAULT_TOKEN"
    fi

    vault kv put "secret/${SECRET_PREFIX}/${secret_name}" \
        value="$new_value" \
        rotated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        rotated_by="manual-script" > /dev/null

    echo -e "${GREEN}✓ Secret rotated successfully in Vault${NC}"
}

# Main rotation logic
main() {
    # Get new secret value
    local new_value=""

    if [ "$AUTO_GENERATE" = true ]; then
        echo -e "${BLUE}Auto-generating new secret value...${NC}"
        if ! new_value=$(generate_secret_value "$SECRET_NAME"); then
            echo -e "${YELLOW}Falling back to manual input${NC}"
            AUTO_GENERATE=false
        else
            echo -e "${GREEN}✓ Generated new secret value${NC}"
        fi
    fi

    if [ "$AUTO_GENERATE" = false ]; then
        echo -e "${YELLOW}Enter new secret value:${NC}"
        read -r -s -p "New value (hidden): " new_value
        echo ""

        if [ -z "$new_value" ]; then
            echo -e "${RED}ERROR: Secret value cannot be empty${NC}" >&2
            exit 1
        fi

        # Confirm
        read -r -s -p "Confirm new value (hidden): " confirm_value
        echo ""

        if [ "$new_value" != "$confirm_value" ]; then
            echo -e "${RED}ERROR: Values do not match${NC}" >&2
            exit 1
        fi
    fi

    # Validate secret value
    case "$SECRET_NAME" in
        jwt_secret|api_encryption_key)
            if [ ${#new_value} -lt 32 ]; then
                echo -e "${RED}ERROR: $SECRET_NAME must be at least 32 characters${NC}" >&2
                exit 1
            fi
            ;;
        stripe_secret_key)
            if [[ ! "$new_value" =~ ^sk_ ]]; then
                echo -e "${YELLOW}WARNING: Stripe secret key should start with 'sk_'${NC}"
            fi
            ;;
        stripe_webhook_secret)
            if [[ ! "$new_value" =~ ^whsec_ ]]; then
                echo -e "${YELLOW}WARNING: Stripe webhook secret should start with 'whsec_'${NC}"
            fi
            ;;
    esac

    # Perform rotation based on backend
    case "$BACKEND" in
        aws)
            if ! command -v aws &> /dev/null; then
                echo -e "${RED}ERROR: AWS CLI is not installed${NC}" >&2
                exit 1
            fi
            rotate_aws_secret "$SECRET_NAME" "$new_value"
            ;;
        vault)
            if ! command -v vault &> /dev/null; then
                echo -e "${RED}ERROR: Vault CLI is not installed${NC}" >&2
                exit 1
            fi
            rotate_vault_secret "$SECRET_NAME" "$new_value"
            ;;
        *)
            echo -e "${RED}ERROR: Invalid backend: $BACKEND${NC}" >&2
            echo "Supported backends: aws, vault" >&2
            exit 1
            ;;
    esac

    # Post-rotation instructions
    echo ""
    echo -e "${GREEN}=== Rotation Complete ===${NC}"
    echo ""
    echo -e "${BLUE}Important next steps:${NC}"
    echo "1. Restart all services using this secret"
    echo "2. Verify services are functioning correctly"
    echo "3. Monitor logs for authentication errors"
    echo "4. Update any backup/disaster recovery documentation"
    echo ""
    echo -e "${YELLOW}For zero-downtime rotation:${NC}"
    echo "1. Deploy new secret to secrets manager"
    echo "2. Update services to read new secret (rolling restart)"
    echo "3. Verify all instances are using new secret"
    echo "4. Decommission old secret value"
    echo ""
}

# Run main function
main
