#!/usr/bin/env bash
#
# AWS Secrets Manager Setup Script
#
# This script creates all required secrets in AWS Secrets Manager
# for the ERC-8004 backend infrastructure.
#
# Prerequisites:
# - AWS CLI installed and configured
# - IAM permissions: secretsmanager:CreateSecret, secretsmanager:PutSecretValue
# - Valid AWS credentials (aws configure)
#
# Usage:
#   ./scripts/aws-secrets-setup.sh [--region us-east-1] [--profile default]
#
# Options:
#   --region REGION    AWS region (default: us-east-1)
#   --profile PROFILE  AWS CLI profile (default: default)
#   --dry-run          Show commands without executing
#   --help             Show this help message

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
AWS_REGION="${AWS_REGION:-us-east-1}"
AWS_PROFILE="${AWS_PROFILE:-default}"
DRY_RUN=false
SECRET_PREFIX="agentauri"

# Parse command-line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --profile)
            AWS_PROFILE="$2"
            shift 2
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
            echo -e "${RED}Unknown option: $1${NC}" >&2
            echo "Use --help for usage information" >&2
            exit 1
            ;;
    esac
done

# Print configuration
echo -e "${BLUE}=== AWS Secrets Manager Setup ===${NC}"
echo "Region: $AWS_REGION"
echo "Profile: $AWS_PROFILE"
echo "Prefix: $SECRET_PREFIX"
echo "Dry run: $DRY_RUN"
echo ""

# Check AWS CLI is installed
if ! command -v aws &> /dev/null; then
    echo -e "${RED}ERROR: AWS CLI is not installed${NC}" >&2
    echo "Install from: https://aws.amazon.com/cli/" >&2
    exit 1
fi

# Verify AWS credentials
echo -e "${BLUE}Verifying AWS credentials...${NC}"
if ! aws sts get-caller-identity --profile "$AWS_PROFILE" &> /dev/null; then
    echo -e "${RED}ERROR: AWS credentials are not configured correctly${NC}" >&2
    echo "Run: aws configure --profile $AWS_PROFILE" >&2
    exit 1
fi

ACCOUNT_ID=$(aws sts get-caller-identity --profile "$AWS_PROFILE" --query Account --output text)
echo -e "${GREEN}Authenticated as AWS account: $ACCOUNT_ID${NC}"
echo ""

# Function to create or update a secret
create_secret() {
    local secret_name="$1"
    local secret_description="$2"
    local prompt_message="$3"
    local default_value="${4:-}"

    echo -e "${BLUE}--- $secret_name ---${NC}"
    echo "Description: $secret_description"

    # Prompt for secret value
    if [ -n "$default_value" ]; then
        echo -e "${YELLOW}$prompt_message${NC}"
        read -r -p "Value (press Enter for default): " secret_value
        secret_value="${secret_value:-$default_value}"
    else
        echo -e "${YELLOW}$prompt_message${NC}"
        read -r -s -p "Value (hidden): " secret_value
        echo ""
    fi

    if [ -z "$secret_value" ]; then
        echo -e "${RED}ERROR: Secret value cannot be empty${NC}" >&2
        exit 1
    fi

    # Create or update secret
    local full_secret_name="${SECRET_PREFIX}/${secret_name}"

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN] Would create/update secret: $full_secret_name${NC}"
    else
        # Check if secret exists
        if aws secretsmanager describe-secret \
            --secret-id "$full_secret_name" \
            --region "$AWS_REGION" \
            --profile "$AWS_PROFILE" &> /dev/null; then

            # Update existing secret
            echo "Updating existing secret..."
            aws secretsmanager put-secret-value \
                --secret-id "$full_secret_name" \
                --secret-string "$secret_value" \
                --region "$AWS_REGION" \
                --profile "$AWS_PROFILE" > /dev/null

            echo -e "${GREEN}✓ Secret updated: $full_secret_name${NC}"
        else
            # Create new secret
            echo "Creating new secret..."
            aws secretsmanager create-secret \
                --name "$full_secret_name" \
                --description "$secret_description" \
                --secret-string "$secret_value" \
                --region "$AWS_REGION" \
                --profile "$AWS_PROFILE" > /dev/null

            echo -e "${GREEN}✓ Secret created: $full_secret_name${NC}"
        fi
    fi

    echo ""
}

# Create all secrets
echo -e "${BLUE}=== Creating Secrets (Tier 1: Critical) ===${NC}"
echo ""

create_secret \
    "database_url" \
    "PostgreSQL connection string" \
    "Enter DATABASE_URL (format: postgresql://user:password@host:port/database)"

create_secret \
    "redis_url" \
    "Redis connection string" \
    "Enter REDIS_URL (format: redis://[:password@]host:port)"

create_secret \
    "jwt_secret" \
    "JWT signing secret (minimum 32 characters)" \
    "Enter JWT_SECRET (generate with: openssl rand -base64 32)"

create_secret \
    "stripe_secret_key" \
    "Stripe secret key for payment processing" \
    "Enter STRIPE_SECRET_KEY (format: sk_live_xxx or sk_test_xxx)"

create_secret \
    "stripe_webhook_secret" \
    "Stripe webhook secret for signature verification" \
    "Enter STRIPE_WEBHOOK_SECRET (format: whsec_xxx)"

echo -e "${BLUE}=== Creating Secrets (Tier 2: Important) ===${NC}"
echo ""

create_secret \
    "ethereum_sepolia_rpc_url" \
    "Ethereum Sepolia RPC endpoint URL" \
    "Enter ETHEREUM_SEPOLIA_RPC_URL" \
    "https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY"

create_secret \
    "base_sepolia_rpc_url" \
    "Base Sepolia RPC endpoint URL" \
    "Enter BASE_SEPOLIA_RPC_URL" \
    "https://base-sepolia.g.alchemy.com/v2/YOUR_API_KEY"

create_secret \
    "linea_sepolia_rpc_url" \
    "Linea Sepolia RPC endpoint URL (optional)" \
    "Enter LINEA_SEPOLIA_RPC_URL (or leave empty)" \
    ""

create_secret \
    "api_encryption_key" \
    "API key encryption key (Argon2id)" \
    "Enter API_ENCRYPTION_KEY (generate with: openssl rand -base64 32)"

create_secret \
    "telegram_bot_token" \
    "Telegram bot token (optional)" \
    "Enter TELEGRAM_BOT_TOKEN (or leave empty)" \
    ""

# Summary
echo -e "${GREEN}=== Setup Complete ===${NC}"
echo ""
echo "All secrets have been created in AWS Secrets Manager."
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Update your application configuration:"
echo "   export SECRETS_BACKEND=aws"
echo "   export AWS_REGION=$AWS_REGION"
echo ""
echo "2. For EC2/ECS, attach IAM role with this policy:"
echo "   {
        \"Version\": \"2012-10-17\",
        \"Statement\": [
            {
                \"Effect\": \"Allow\",
                \"Action\": [
                    \"secretsmanager:GetSecretValue\",
                    \"secretsmanager:DescribeSecret\"
                ],
                \"Resource\": \"arn:aws:secretsmanager:$AWS_REGION:$ACCOUNT_ID:secret:${SECRET_PREFIX}/*\"
            },
            {
                \"Effect\": \"Allow\",
                \"Action\": \"kms:Decrypt\",
                \"Resource\": \"*\"
            }
        ]
    }"
echo ""
echo "3. For local development with AWS CLI credentials:"
echo "   No additional configuration needed (uses AWS_PROFILE)"
echo ""
echo -e "${YELLOW}Security Reminder:${NC}"
echo "- Rotate secrets quarterly (Tier 1) and annually (Tier 2)"
echo "- Enable CloudTrail for audit logging"
echo "- Use AWS Secrets Manager automatic rotation for database credentials"
echo "- Never commit secrets to version control"
echo ""
