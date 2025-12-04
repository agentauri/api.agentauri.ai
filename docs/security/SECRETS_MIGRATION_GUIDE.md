# Secrets Migration Guide

Step-by-step guide for migrating from `.env` files to AWS Secrets Manager or HashiCorp Vault.

## Table of Contents

- [Overview](#overview)
- [Pre-Migration Checklist](#pre-migration-checklist)
- [Migration Path A: AWS Secrets Manager](#migration-path-a-aws-secrets-manager)
- [Migration Path B: HashiCorp Vault](#migration-path-b-hashicorp-vault)
- [Testing the Migration](#testing-the-migration)
- [Rollback Procedures](#rollback-procedures)
- [Post-Migration Cleanup](#post-migration-cleanup)

---

## Overview

This guide helps you migrate from storing secrets in `.env` files to a production-grade secrets management system.

**Time Required**: 1-2 hours
**Downtime**: Zero (with proper planning)
**Risk Level**: Low (fully reversible)

### Why Migrate?

| Issue with .env | Solution with Secrets Manager |
|-----------------|-------------------------------|
| Plain text secrets | AES-256 encryption at rest |
| No rotation | Automatic rotation support |
| No audit trail | Complete access logging |
| No access control | Fine-grained IAM/policy control |
| Risk of git commits | Centralized, secure storage |

---

## Pre-Migration Checklist

Before starting, ensure you have:

### 1. Inventory Current Secrets

```bash
# List all secrets in .env
grep -E '^[A-Z_]+=' .env | cut -d= -f1 | sort

# Expected output (example):
# API_ENCRYPTION_KEY
# BASE_SEPOLIA_RPC_URL
# DATABASE_URL
# ETHEREUM_SEPOLIA_RPC_URL
# JWT_SECRET
# REDIS_URL
# STRIPE_SECRET_KEY
# STRIPE_WEBHOOK_SECRET
# TELEGRAM_BOT_TOKEN
```

### 2. Backup Current Configuration

```bash
# Create timestamped backup
cp .env .env.backup_$(date +%Y%m%d_%H%M%S)

# Verify backup
diff .env .env.backup_*
```

### 3. Choose Backend

**Decision Matrix**:

| Scenario | Recommended Backend |
|----------|-------------------|
| Deploying on AWS (EC2, ECS, Lambda) | AWS Secrets Manager |
| Multi-cloud deployment | HashiCorp Vault |
| On-premises infrastructure | HashiCorp Vault |
| Tight budget constraints | HashiCorp Vault OSS |
| Need dynamic secrets (DB credentials) | HashiCorp Vault |

### 4. Verify Tools Installed

```bash
# For AWS Secrets Manager
aws --version  # Should be 2.x or later
aws sts get-caller-identity  # Verify credentials

# For HashiCorp Vault
vault --version  # Should be 1.15 or later
# Vault server must be accessible
```

### 5. Review Access Permissions

**AWS**: Ensure IAM user/role has:
- `secretsmanager:CreateSecret`
- `secretsmanager:PutSecretValue`
- `secretsmanager:GetSecretValue`

**Vault**: Ensure token has:
- `create`, `read`, `update` on `secret/data/agentauri/*`

---

## Migration Path A: AWS Secrets Manager

### Step 1: Set Up AWS Environment

```bash
# Configure AWS CLI (if not already done)
aws configure --profile agentauri-prod

# Set region
export AWS_REGION=us-east-1
export AWS_PROFILE=agentauri-prod

# Verify access
aws sts get-caller-identity
```

### Step 2: Create Secrets in AWS

**Option 1: Automated Script (Recommended)**

```bash
# Run interactive setup
./scripts/aws-secrets-setup.sh --region us-east-1 --profile agentauri-prod

# Follow prompts to enter each secret value
# Values from .env will be used as defaults
```

**Option 2: Manual Creation**

```bash
# Example: Create database_url secret
aws secretsmanager create-secret \
  --name agentauri/database_url \
  --description "PostgreSQL connection string" \
  --secret-string "postgresql://user:password@host:5432/db" \
  --region us-east-1

# Repeat for each secret
```

### Step 3: Verify Secrets Created

```bash
# List all agentauri secrets
aws secretsmanager list-secrets \
  --filters Key=name,Values=agentauri/ \
  --region us-east-1 \
  --query 'SecretList[].Name' \
  --output table

# Test retrieving a secret
aws secretsmanager get-secret-value \
  --secret-id agentauri/database_url \
  --region us-east-1 \
  --query 'SecretString' \
  --output text
```

### Step 4: Create IAM Policy for Application

Create `agentauri-secrets-policy.json`:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "secretsmanager:GetSecretValue",
                "secretsmanager:DescribeSecret"
            ],
            "Resource": "arn:aws:secretsmanager:us-east-1:ACCOUNT_ID:secret:agentauri/*"
        },
        {
            "Effect": "Allow",
            "Action": "kms:Decrypt",
            "Resource": "*"
        }
    ]
}
```

Replace `ACCOUNT_ID` with your AWS account ID:

```bash
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
sed -i "s/ACCOUNT_ID/$ACCOUNT_ID/g" agentauri-secrets-policy.json
```

Apply policy:

```bash
# Create policy
aws iam create-policy \
  --policy-name agentauri-secrets-policy \
  --policy-document file://agentauri-secrets-policy.json

# Attach to EC2 instance role (example)
aws iam attach-role-policy \
  --role-name agentauri-ec2-role \
  --policy-arn arn:aws:iam::ACCOUNT_ID:policy/agentauri-secrets-policy
```

### Step 5: Update Application Configuration

**For production (EC2/ECS)**:

```bash
# Add to environment variables (systemd, ECS task definition, etc.)
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1
# No need for AWS_PROFILE - uses IAM role
```

**For local testing**:

```bash
# Add to .env or shell profile
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1
export AWS_PROFILE=agentauri-prod
```

### Step 6: Update Cargo.toml

Enable AWS secrets feature:

```toml
[dependencies]
shared = { path = "../shared", features = ["aws-secrets"] }
```

### Step 7: Build and Test

```bash
# Build with AWS feature
cargo build --release --features aws-secrets

# Test locally
SECRETS_BACKEND=aws AWS_REGION=us-east-1 cargo run --bin api-gateway

# Check logs for "Loading secrets from AWS Secrets Manager"
```

---

## Migration Path B: HashiCorp Vault

### Step 1: Set Up Vault Server

**Development (Local)**:

```bash
# Start Vault dev server with Docker
docker-compose -f docker-compose.vault.yml up -d

# Verify Vault is running
curl -s http://localhost:8200/v1/sys/health | jq
```

**Production (Existing Vault)**:

```bash
# Set Vault address and token
export VAULT_ADDR='https://vault.example.com:8200'
export VAULT_TOKEN='s.YourProductionToken'

# Verify connection
vault status
```

### Step 2: Enable KV Secrets Engine

```bash
# Enable KV v2 secrets engine (if not already enabled)
vault secrets enable -version=2 -path=secret kv

# Verify
vault secrets list
```

### Step 3: Create Secrets in Vault

**Option 1: Manual Entry**

```bash
# Create each secret individually
vault kv put secret/agentauri/database_url \
  value="postgresql://user:password@host:5432/db"

vault kv put secret/agentauri/redis_url \
  value="redis://redis:6379"

vault kv put secret/agentauri/jwt_secret \
  value="your_jwt_secret_32_characters_long"

# Repeat for all secrets...
```

**Option 2: Bulk Import from .env**

```bash
# Script to import all secrets from .env
while IFS='=' read -r key value; do
  # Skip comments and empty lines
  [[ $key =~ ^#.*$ ]] && continue
  [[ -z $key ]] && continue

  # Convert KEY_NAME to secret_name
  secret_name=$(echo "$key" | tr '[:upper:]' '[:lower:]')

  # Create secret in Vault
  vault kv put "secret/agentauri/$secret_name" value="$value"

  echo "✓ Created: secret/agentauri/$secret_name"
done < .env
```

### Step 4: Verify Secrets Created

```bash
# List all secrets
vault kv list secret/agentauri

# Get a specific secret
vault kv get secret/agentauri/database_url

# Get secret value only
vault kv get -field=value secret/agentauri/database_url
```

### Step 5: Create Vault Policy

Create `agentauri-app-policy.hcl`:

```hcl
# Read-only access to application secrets
path "secret/data/agentauri/*" {
  capabilities = ["read"]
}

path "secret/metadata/agentauri/*" {
  capabilities = ["list"]
}
```

Apply policy:

```bash
vault policy write agentauri-app agentauri-app-policy.hcl
```

### Step 6: Create Application Token

```bash
# Create token with agentauri-app policy
# TTL: 720 hours (30 days) - adjust as needed
vault token create \
  -policy=agentauri-app \
  -ttl=720h \
  -display-name="agentauri-api-gateway"

# Save the token securely!
# Example output:
# Key                  Value
# ---                  -----
# token                s.xxxxxxxxxxxxxxxxxxx
# token_accessor       yyyyyyyyyyyyyyyyyyyy
# token_duration       720h
```

### Step 7: Update Application Configuration

**For production**:

```bash
# Add to environment variables (systemd, Kubernetes, etc.)
export SECRETS_BACKEND=vault
export VAULT_ADDR='https://vault.example.com:8200'
export VAULT_TOKEN='s.xxxxxxxxxxxxxxxxxxx'
# Optional (Vault Enterprise only):
export VAULT_NAMESPACE='your-namespace'
```

**For local testing**:

```bash
# Add to .env (do NOT commit VAULT_TOKEN!)
export SECRETS_BACKEND=vault
export VAULT_ADDR='http://localhost:8200'
export VAULT_TOKEN='dev-root-token'
```

### Step 8: Update Cargo.toml

Enable Vault secrets feature:

```toml
[dependencies]
shared = { path = "../shared", features = ["vault-secrets"] }
```

### Step 9: Build and Test

```bash
# Build with Vault feature
cargo build --release --features vault-secrets

# Test locally
SECRETS_BACKEND=vault \
VAULT_ADDR='http://localhost:8200' \
VAULT_TOKEN='dev-root-token' \
cargo run --bin api-gateway

# Check logs for "Loading secrets from HashiCorp Vault"
```

---

## Testing the Migration

### Local Testing

**Step 1: Test with Development Vault**

```bash
# Start local Vault
docker-compose -f docker-compose.vault.yml up -d

# Run application
SECRETS_BACKEND=vault \
VAULT_ADDR='http://localhost:8200' \
VAULT_TOKEN='dev-root-token' \
cargo run --bin api-gateway

# Verify application starts successfully
# Check logs for successful secret loading
```

**Step 2: Verify Database Connection**

```bash
# Application should connect to database using secrets from Vault/AWS
# Check logs for "Database health check passed"
```

**Step 3: Test API Endpoints**

```bash
# Health check
curl http://localhost:8080/api/v1/health

# Should return 200 OK if secrets are loaded correctly
```

### Staging Testing

**Step 1: Deploy to Staging Environment**

```bash
# Update staging environment variables
# For AWS:
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1

# For Vault:
export SECRETS_BACKEND=vault
export VAULT_ADDR='https://vault-staging.example.com:8200'
export VAULT_TOKEN='s.staging-token'
```

**Step 2: Run Integration Tests**

```bash
# Run full integration test suite
cargo test --workspace --features aws-secrets  # or vault-secrets

# Verify all tests pass
```

**Step 3: Smoke Test Critical Flows**

- User authentication (JWT_SECRET)
- Database queries (DATABASE_URL)
- Redis caching (REDIS_URL)
- Stripe payments (STRIPE_SECRET_KEY)
- RPC calls (ETHEREUM_SEPOLIA_RPC_URL)

### Production Deployment

**Zero-Downtime Deployment Strategy**:

1. **Blue-Green Deployment**:
   ```bash
   # Deploy new version (green) with SECRETS_BACKEND=aws/vault
   # Keep old version (blue) with .env files running
   # Gradually shift traffic to green
   # Once verified, decommission blue
   ```

2. **Canary Deployment**:
   ```bash
   # Deploy to 10% of instances first
   # Monitor for errors
   # Gradually increase to 100%
   ```

---

## Rollback Procedures

### If AWS Secrets Manager Fails

**Step 1: Revert Environment Variables**

```bash
# Remove secrets backend configuration
unset SECRETS_BACKEND
# Application will fall back to .env

# Restart application
systemctl restart api-gateway
```

**Step 2: Restore .env File**

```bash
# Restore from backup
cp .env.backup_YYYYMMDD_HHMMSS .env

# Restart application
systemctl restart api-gateway
```

### If Vault Fails

**Step 1: Check Vault Status**

```bash
# Verify Vault is accessible
vault status

# Check if unsealed
# If sealed, unseal with unseal keys:
vault operator unseal
```

**Step 2: Verify Token**

```bash
# Check if token is valid
vault token lookup

# Renew if expired
vault token renew
```

**Step 3: Fall Back to .env**

```bash
# Same as AWS rollback
unset SECRETS_BACKEND
cp .env.backup_YYYYMMDD_HHMMSS .env
systemctl restart api-gateway
```

---

## Post-Migration Cleanup

### Step 1: Verify Production Stability

Monitor for **7 days** before cleanup:

```bash
# Check application logs for errors
journalctl -u api-gateway -f | grep -i "secret\|error"

# Monitor CloudTrail (AWS) or Vault audit logs
# Verify secrets are being accessed correctly
```

### Step 2: Remove .env from Production

```bash
# Archive .env securely
gpg --encrypt --recipient security@example.com .env
mv .env.gpg ~/secure-backups/

# Remove from server
rm .env

# Verify application still works
systemctl restart api-gateway
systemctl status api-gateway
```

### Step 3: Update .gitignore

```bash
# Ensure .env is in .gitignore
grep "^\.env$" .gitignore || echo ".env" >> .gitignore

# Verify no .env files in git
git status --ignored
```

### Step 4: Rotate All Secrets

Since secrets were temporarily in .env, rotate all Tier 1 secrets:

```bash
# Rotate critical secrets using rotation script
./scripts/rotate-secrets.sh --backend aws --generate jwt_secret
./scripts/rotate-secrets.sh --backend aws database_url
./scripts/rotate-secrets.sh --backend aws redis_url
# etc.
```

### Step 5: Enable Monitoring and Alerts

**AWS CloudWatch Alarms**:

```bash
# Alert on excessive secret accesses (potential leak)
aws cloudwatch put-metric-alarm \
  --alarm-name agentauri-secrets-high-access \
  --alarm-description "Alert on >1000 secret accesses/hour" \
  --metric-name GetSecretValue \
  --namespace AWS/SecretsManager \
  --statistic Sum \
  --period 3600 \
  --threshold 1000 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 1
```

**Vault Audit Alerts**:

Configure log aggregation (Loki, Elasticsearch) to alert on:
- Unauthorized access attempts
- Secret access from unknown IPs
- Failed authentication attempts

### Step 6: Document the Change

Update internal documentation:

- ✅ Secrets now managed in AWS Secrets Manager / Vault
- ✅ Rotation schedule: Tier 1 quarterly, Tier 2 annually
- ✅ Access procedures documented
- ✅ Disaster recovery plan updated

---

## Verification Checklist

Before considering migration complete:

- [ ] All secrets migrated to AWS/Vault
- [ ] Application successfully loads secrets
- [ ] All integration tests pass
- [ ] No .env files on production servers
- [ ] IAM policies / Vault policies configured
- [ ] Audit logging enabled
- [ ] Rotation schedule documented
- [ ] Team trained on new procedures
- [ ] Disaster recovery plan tested
- [ ] Monitoring and alerts configured

---

## Troubleshooting

### "Cannot connect to Vault"

**Symptom**: Application logs show `Vault error: connection refused`

**Solution**:
```bash
# Check Vault address
echo $VAULT_ADDR

# Verify Vault is accessible
curl -s $VAULT_ADDR/v1/sys/health

# Check firewall rules (must allow port 8200)
```

### "AWS Secrets Manager access denied"

**Symptom**: Application logs show `AccessDeniedException`

**Solution**:
```bash
# Verify IAM role attached to instance
aws sts get-caller-identity

# Test secret access manually
aws secretsmanager get-secret-value --secret-id agentauri/database_url

# Check IAM policy is correct (see Step 4 above)
```

### "Secret not found after migration"

**Symptom**: Application crashes with `Secret agentauri/database_url not found`

**Solution**:
```bash
# Verify secret name (case-sensitive!)
aws secretsmanager list-secrets --query 'SecretList[].Name'

# Check prefix is correct (must be agentauri/ not agentauri_)
```

---

## Additional Resources

- [AWS Secrets Manager Best Practices](https://docs.aws.amazon.com/secretsmanager/latest/userguide/best-practices.html)
- [HashiCorp Vault Production Hardening](https://www.vaultproject.io/docs/internals/security)
- [Secrets Management Documentation](./SECRETS_MANAGEMENT.md)

---

**Last Updated**: 2025-01-30
**Version**: 1.0.0
**Support**: security@example.com
