<!-- STATUS: PHASE 6 - DESIGN DOCUMENT -->
<!-- This is a design document for Phase 6 (Production Deployment) -->

# Secrets Management

Complete guide to secrets management for the ERC-8004 backend infrastructure.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Backend Options](#backend-options)
  - [Environment Variables (Development)](#environment-variables-development)
  - [AWS Secrets Manager (Production)](#aws-secrets-manager-production)
  - [HashiCorp Vault (Production)](#hashicorp-vault-production)
- [Setup Instructions](#setup-instructions)
- [Secret Rotation](#secret-rotation)
- [Access Control](#access-control)
- [Audit Logging](#audit-logging)
- [Disaster Recovery](#disaster-recovery)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

The ERC-8004 backend uses a **unified secrets management interface** that supports multiple backends:

- **Development**: `.env` files (local only)
- **Production**: AWS Secrets Manager or HashiCorp Vault

This design provides:

- **Flexibility**: Choose the backend that fits your infrastructure
- **Security**: Industry-standard encryption and access control
- **Rotation**: Automated and manual secret rotation support
- **Auditability**: Complete audit trail of secret access
- **Caching**: In-memory caching with configurable TTL
- **Validation**: Automatic validation of secret formats

### Secret Inventory

Secrets are organized into three tiers based on rotation frequency:

#### Tier 1: Critical (Rotate Quarterly)

| Secret | Description | Format |
|--------|-------------|--------|
| `database_url` | PostgreSQL connection string | `postgresql://user:password@host:port/database` |
| `redis_url` | Redis connection string | `redis://[:password@]host:port` |
| `jwt_secret` | JWT signing key | Base64, minimum 32 characters (256 bits) |
| `stripe_secret_key` | Payment processing key | `sk_live_xxx` or `sk_test_xxx` |
| `stripe_webhook_secret` | Webhook signature verification | `whsec_xxx` |

#### Tier 2: Important (Rotate Annually)

| Secret | Description | Format |
|--------|-------------|--------|
| `ethereum_sepolia_rpc_url` | Ethereum RPC endpoint | HTTPS URL |
| `base_sepolia_rpc_url` | Base RPC endpoint | HTTPS URL |
| `linea_sepolia_rpc_url` | Linea RPC endpoint (optional) | HTTPS URL |
| `api_encryption_key` | API key hashing (Argon2id) | Base64, 32 bytes |
| `telegram_bot_token` | Telegram bot authentication (optional) | `123456:ABC-DEF...` |

#### Tier 3: Configuration (Non-Secret)

These can remain in `.env` files:

- `BASE_URL` - Public API URL
- `CORS_ALLOWED_ORIGINS` - CORS whitelist
- `DOMAIN` - Domain name
- `ENABLE_HTTPS` - Boolean flag
- Contract addresses (public)

---

## Architecture

### Unified Interface

```rust
use shared::secrets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Automatically selects backend based on SECRETS_BACKEND env var
    let secrets = secrets::load_secrets().await?;

    // Use secrets
    println!("Database URL: {}", secrets.database_url);

    Ok(())
}
```

### Backend Selection

The `SECRETS_BACKEND` environment variable determines which backend to use:

- `env` (default) → `.env` files
- `aws` → AWS Secrets Manager
- `vault` → HashiCorp Vault

### Caching Strategy

All backends use in-memory caching to reduce API calls and improve performance:

- **Default TTL**: 1 hour (3600 seconds)
- **Configurable**: Set `SECRETS_CACHE_TTL_SECONDS` environment variable
- **Cache invalidation**: Automatic on TTL expiration or manual via API
- **Thread-safe**: Uses `Arc<RwLock<HashMap>>`

---

## Backend Options

### Environment Variables (Development)

**Use case**: Local development and testing only.

**Security warning**: NOT suitable for production. Secrets are stored in plain text.

#### Setup

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Set `SECRETS_BACKEND=env` (or leave unset):
   ```bash
   export SECRETS_BACKEND=env
   ```

3. Run the application:
   ```bash
   cargo run --bin api-gateway
   ```

#### Advantages

- Simple setup
- No external dependencies
- Fast iteration

#### Disadvantages

- No encryption at rest
- No rotation support
- No audit logging
- Risk of accidental commits
- No access control

---

### AWS Secrets Manager (Production)

**Use case**: Production deployments on AWS infrastructure.

**Security features**:
- AES-256 encryption at rest (AWS KMS)
- Automatic rotation via Lambda
- CloudTrail audit logging
- IAM-based access control
- Regional replication
- Versioning

#### Prerequisites

1. AWS account with appropriate permissions
2. AWS CLI installed: `brew install awscli` (macOS) or `apt install awscli` (Ubuntu)
3. AWS credentials configured: `aws configure`

#### Setup

**Step 1: Create Secrets**

Run the automated setup script:

```bash
./scripts/aws-secrets-setup.sh --region us-east-1 --profile default
```

This script will:
- Verify AWS credentials
- Create all required secrets with `agentauri/` prefix
- Prompt for secret values (or use defaults)
- Handle both new secret creation and updates

**Step 2: Configure IAM Permissions**

Attach this policy to your application's IAM role (EC2, ECS, Lambda):

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

Replace `ACCOUNT_ID` with your AWS account ID.

**Step 3: Configure Application**

Set environment variables:

```bash
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1
# AWS_PROFILE not needed if using IAM role
```

For local development with AWS CLI credentials:

```bash
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1
export AWS_PROFILE=default
```

**Step 4: Update Dependencies**

Add AWS SDK to `Cargo.toml`:

```toml
[dependencies]
shared = { path = "../shared", features = ["aws-secrets"] }
```

#### Cost Estimate

- **Secret storage**: $0.40 per secret/month
- **API calls**: $0.05 per 10,000 calls
- **Example**: 10 secrets + 1M API calls/month = $4.00 + $5.00 = **$9.00/month**

With 1-hour caching, API calls are minimized (one fetch per service restart).

#### Automatic Rotation

AWS Secrets Manager supports automatic rotation for database credentials via Lambda:

1. Create rotation Lambda function (see `infrastructure/lambda/secret-rotation.py`)
2. Configure rotation schedule (e.g., 30 days)
3. Lambda updates secret and rotates database password
4. Application automatically picks up new value after cache TTL

---

### HashiCorp Vault (Production)

**Use case**: Multi-cloud, hybrid, or cost-sensitive deployments.

**Security features**:
- AES-256-GCM encryption at rest
- Dynamic secrets (database, cloud credentials)
- Encryption as a service
- Detailed audit logs
- Fine-grained policies (ACL)
- Open source (free) or Enterprise
- Multi-cloud support

#### Prerequisites

1. Vault server (self-hosted or HCP Vault)
2. Vault CLI installed: `brew install vault` (macOS) or `apt install vault` (Ubuntu)
3. Valid authentication token

#### Setup (Local Development)

**Step 1: Start Vault Dev Server**

Use Docker Compose:

```bash
docker-compose -f docker-compose.vault.yml up -d
```

This starts:
- Vault server on `http://localhost:8200`
- Vault UI (access with root token: `dev-root-token`)
- Auto-initialization with example secrets

**Step 2: Verify Vault**

```bash
export VAULT_ADDR='http://localhost:8200'
export VAULT_TOKEN='dev-root-token'

# List secrets
vault kv list secret/agentauri

# Get a secret
vault kv get secret/agentauri/database_url
```

**Step 3: Configure Application**

Set environment variables:

```bash
export SECRETS_BACKEND=vault
export VAULT_ADDR='http://localhost:8200'
export VAULT_TOKEN='dev-root-token'
```

**Step 4: Update Dependencies**

Add Vault client to `Cargo.toml`:

```toml
[dependencies]
shared = { path = "../shared", features = ["vault-secrets"] }
```

#### Setup (Production)

**Step 1: Deploy Vault Server**

Option A: Self-hosted (Docker/Kubernetes)

```bash
# See official Vault documentation for production deployment
# https://www.vaultproject.io/docs/platform/k8s
```

Option B: HCP Vault (Managed)

```bash
# Sign up at https://portal.cloud.hashicorp.com/
```

**Step 2: Initialize and Unseal Vault**

```bash
vault operator init
# Save unseal keys and root token securely!

vault operator unseal
# Repeat with 3 different unseal keys
```

**Step 3: Enable KV Secrets Engine**

```bash
vault secrets enable -version=2 -path=secret kv
```

**Step 4: Create Secrets**

```bash
# Example: Create database_url secret
vault kv put secret/agentauri/database_url \
  value="postgresql://user:password@host:5432/db"

# Verify
vault kv get secret/agentauri/database_url
```

**Step 5: Create Policy for Application**

```hcl
# agentauri-app-policy.hcl
path "secret/data/agentauri/*" {
  capabilities = ["read"]
}
```

Apply policy:

```bash
vault policy write agentauri-app agentauri-app-policy.hcl
```

**Step 6: Create Token for Application**

```bash
vault token create -policy=agentauri-app -ttl=720h
# Save token securely - this is the VAULT_TOKEN for the app
```

**Step 7: Configure Application**

```bash
export SECRETS_BACKEND=vault
export VAULT_ADDR='https://vault.example.com:8200'
export VAULT_TOKEN='s.YourTokenHere'
# Optional (Vault Enterprise only):
export VAULT_NAMESPACE='your-namespace'
```

#### Cost Estimate

- **Open Source**: Free (self-hosted infrastructure costs only)
- **HCP Vault Starter**: $0.03/hour = ~$22/month
- **Vault Enterprise**: Contact HashiCorp sales

---

## Setup Instructions

### Migration from .env to Secrets Manager

**Step 1: Inventory Current Secrets**

```bash
grep -E '^[A-Z_]+=' .env | cut -d= -f1
```

**Step 2: Choose Backend**

- AWS infrastructure? → Use AWS Secrets Manager
- Multi-cloud or hybrid? → Use HashiCorp Vault
- Cost-sensitive or on-prem? → Use HashiCorp Vault (OSS)

**Step 3: Populate Secrets**

Use the appropriate setup script:

```bash
# AWS Secrets Manager
./scripts/aws-secrets-setup.sh --region us-east-1

# HashiCorp Vault
docker-compose -f docker-compose.vault.yml up -d
# Then manually create secrets via Vault CLI or UI
```

**Step 4: Update Application Configuration**

```bash
# Update .env or environment variables
export SECRETS_BACKEND=aws  # or vault
export AWS_REGION=us-east-1  # for AWS
# OR
export VAULT_ADDR='http://localhost:8200'  # for Vault
export VAULT_TOKEN='your-token'
```

**Step 5: Test Locally**

```bash
cargo run --bin api-gateway
# Check logs for "Loading secrets from AWS Secrets Manager" or Vault
```

**Step 6: Deploy to Production**

- **AWS**: Attach IAM role with SecretsManager permissions
- **Vault**: Set `VAULT_ADDR` and `VAULT_TOKEN` in environment

**Step 7: Remove .env File**

```bash
# Backup first!
cp .env .env.backup_$(date +%Y%m%d)

# Remove from production servers
rm .env

# Add to .gitignore (should already be there)
echo ".env" >> .gitignore
```

---

## Secret Rotation

### Rotation Frequency

- **Tier 1 (Critical)**: Rotate every 90 days (quarterly)
- **Tier 2 (Important)**: Rotate every 365 days (annually)
- **Emergency**: Rotate immediately if compromised

### Manual Rotation

Use the rotation script:

```bash
# Auto-generate new JWT secret
./scripts/rotate-secrets.sh --backend aws --generate jwt_secret

# Manual entry for database password
./scripts/rotate-secrets.sh --backend vault database_url

# Dry-run mode (preview changes)
./scripts/rotate-secrets.sh --backend aws --dry-run --generate api_encryption_key
```

### Automated Rotation (AWS)

**Step 1: Create Rotation Lambda**

See `infrastructure/lambda/secret-rotation.py` for example Lambda function.

**Step 2: Configure Rotation Schedule**

```bash
aws secretsmanager rotate-secret \
  --secret-id agentauri/database_url \
  --rotation-lambda-arn arn:aws:lambda:us-east-1:ACCOUNT:function:rotate-secret \
  --rotation-rules AutomaticallyAfterDays=30
```

**Step 3: Test Rotation**

```bash
aws secretsmanager rotate-secret \
  --secret-id agentauri/database_url
```

### Automated Rotation (Vault)

Vault supports dynamic secrets for databases:

```bash
# Enable database secrets engine
vault secrets enable database

# Configure PostgreSQL connection
vault write database/config/agentauri-db \
  plugin_name=postgresql-database-plugin \
  allowed_roles="*" \
  connection_url="postgresql://{{username}}:{{password}}@postgres:5432/agentauri_backend" \
  username="vaultadmin" \
  password="vaultpassword"

# Create role with TTL
vault write database/roles/agentauri-app \
  db_name=agentauri-db \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}';" \
  default_ttl="1h" \
  max_ttl="24h"

# Application reads dynamic credentials
vault read database/creds/agentauri-app
```

---

## Access Control

### AWS IAM Policies

**Principle of Least Privilege**:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "secretsmanager:GetSecretValue"
            ],
            "Resource": "arn:aws:secretsmanager:us-east-1:*:secret:agentauri/*"
        },
        {
            "Effect": "Deny",
            "Action": [
                "secretsmanager:DeleteSecret",
                "secretsmanager:PutSecretValue"
            ],
            "Resource": "*"
        }
    ]
}
```

This grants:
- ✅ Read access to `agentauri/*` secrets
- ❌ No write or delete access

### Vault Policies

**Application Policy** (read-only):

```hcl
# agentauri-app-policy.hcl
path "secret/data/agentauri/*" {
  capabilities = ["read"]
}

path "secret/metadata/agentauri/*" {
  capabilities = ["list"]
}
```

**Admin Policy** (full access):

```hcl
# agentauri-admin-policy.hcl
path "secret/data/agentauri/*" {
  capabilities = ["create", "read", "update", "delete"]
}

path "secret/metadata/agentauri/*" {
  capabilities = ["list", "read", "delete"]
}
```

Apply policies:

```bash
vault policy write agentauri-app agentauri-app-policy.hcl
vault policy write agentauri-admin agentauri-admin-policy.hcl
```

---

## Audit Logging

### AWS CloudTrail

All Secrets Manager API calls are logged to CloudTrail:

**Enable CloudTrail**:

```bash
aws cloudtrail create-trail \
  --name agentauri-secrets-trail \
  --s3-bucket-name my-cloudtrail-bucket

aws cloudtrail start-logging --name agentauri-secrets-trail
```

**Query Logs** (CloudWatch Insights):

```sql
fields @timestamp, eventName, userIdentity.principalId, requestParameters.secretId
| filter eventSource = "secretsmanager.amazonaws.com"
| filter requestParameters.secretId like /agentauri/
| sort @timestamp desc
```

### Vault Audit Logs

**Enable File Audit Device**:

```bash
vault audit enable file file_path=/var/log/vault/audit.log
```

**Query Logs**:

```bash
# View recent secret accesses
jq 'select(.request.path | startswith("secret/data/agentauri"))' \
  /var/log/vault/audit.log | tail -n 20
```

**Alert on Suspicious Access**:

Use log aggregation tools (Loki, Elasticsearch) to alert on:
- Access from unknown IP addresses
- Access outside business hours
- Repeated failed authentication attempts

---

## Disaster Recovery

### Backup Strategy

#### AWS Secrets Manager

Secrets are automatically replicated across AWS availability zones. No manual backup needed.

**Optional**: Export to S3 for cross-region backup:

```bash
# Export all secrets
for secret in $(aws secretsmanager list-secrets --query 'SecretList[].Name' --output text); do
  aws secretsmanager get-secret-value --secret-id "$secret" \
    --query 'SecretString' --output text > "backup_${secret}.txt"
done

# Upload to S3
aws s3 cp backup_*.txt s3://my-backup-bucket/secrets/
```

#### HashiCorp Vault

**Snapshot Vault Data**:

```bash
# Create snapshot
vault operator raft snapshot save vault-snapshot-$(date +%Y%m%d).snap

# Upload to S3
aws s3 cp vault-snapshot-*.snap s3://my-backup-bucket/vault/
```

**Schedule Automated Snapshots** (cron):

```bash
# /etc/cron.daily/vault-backup
#!/bin/bash
SNAPSHOT_FILE="/backups/vault-$(date +%Y%m%d).snap"
vault operator raft snapshot save "$SNAPSHOT_FILE"
aws s3 cp "$SNAPSHOT_FILE" s3://my-backup-bucket/vault/
find /backups -name "vault-*.snap" -mtime +30 -delete
```

### Recovery Procedures

#### Restore from AWS Secrets Manager

Secrets are never lost (AWS handles redundancy). To restore a deleted secret:

```bash
# Restore within 7-30 day recovery window
aws secretsmanager restore-secret --secret-id agentauri/database_url
```

#### Restore from Vault Snapshot

```bash
# Stop Vault
systemctl stop vault

# Restore snapshot
vault operator raft snapshot restore vault-snapshot-20250130.snap

# Start Vault
systemctl start vault

# Unseal Vault (requires 3 unseal keys)
vault operator unseal
```

---

## Best Practices

### Security

1. **Never commit secrets to version control**
   - Use `.gitignore` for `.env` files
   - Use pre-commit hooks to scan for secrets (e.g., `gitleaks`)

2. **Use environment-specific secrets**
   - Separate secrets for dev/staging/production
   - Never use production secrets in non-production environments

3. **Rotate secrets regularly**
   - Tier 1: Quarterly (90 days)
   - Tier 2: Annually (365 days)
   - Emergency: Immediately if compromised

4. **Principle of least privilege**
   - Grant only necessary permissions
   - Use separate credentials for different services

5. **Monitor secret access**
   - Enable audit logging
   - Alert on suspicious access patterns
   - Review logs regularly

### Performance

1. **Use caching wisely**
   - Default 1-hour TTL is reasonable
   - Increase TTL for stable secrets (RPC URLs)
   - Decrease TTL for frequently rotated secrets (database passwords)

2. **Batch secret fetching**
   - Fetch all secrets at startup (one API call per secret)
   - Use parallel fetching (`tokio::try_join!`)

3. **Handle failures gracefully**
   - Retry with exponential backoff
   - Fall back to cached values if backend is unavailable
   - Alert on prolonged failures

### Operational

1. **Document secret ownership**
   - Who created each secret?
   - Who is responsible for rotation?
   - What services use each secret?

2. **Test rotation procedures**
   - Dry-run rotations regularly
   - Document zero-downtime rotation steps
   - Practice emergency rotation

3. **Automate where possible**
   - Use automatic rotation for database credentials
   - Use infrastructure as code (Terraform) for secret creation
   - Integrate secret validation into CI/CD

---

## Troubleshooting

### Common Issues

#### "Secret not found"

**Cause**: Secret does not exist in backend.

**Solution**:

```bash
# AWS: List secrets
aws secretsmanager list-secrets --query 'SecretList[].Name'

# Vault: List secrets
vault kv list secret/agentauri
```

#### "Access denied" (AWS)

**Cause**: Insufficient IAM permissions.

**Solution**: Verify IAM role has `secretsmanager:GetSecretValue` permission:

```bash
# Check current IAM role (from EC2 instance)
aws sts get-caller-identity

# Test secret access
aws secretsmanager get-secret-value --secret-id agentauri/database_url
```

#### "Permission denied" (Vault)

**Cause**: Token lacks read permission.

**Solution**: Check token capabilities:

```bash
# Check token info
vault token lookup

# Check policy
vault policy read agentauri-app
```

#### "Cache always empty"

**Cause**: TTL too short or cache invalidation too aggressive.

**Solution**: Increase cache TTL:

```bash
export SECRETS_CACHE_TTL_SECONDS=7200  # 2 hours
```

#### "Application won't start without secrets"

**Cause**: Required secret is missing.

**Solution**: Check secret validation in logs:

```bash
# Search for validation errors
journalctl -u api-gateway | grep "Secret.*not found"
```

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=shared::secrets=debug

cargo run --bin api-gateway
```

This logs:
- Backend selection
- Secret fetching
- Cache hits/misses
- Validation errors

---

## Comparison Matrix

| Feature | .env Files | AWS Secrets Manager | HashiCorp Vault |
|---------|-----------|---------------------|-----------------|
| **Encryption at rest** | ❌ No | ✅ AES-256 (KMS) | ✅ AES-256-GCM |
| **Automatic rotation** | ❌ No | ✅ Yes (Lambda) | ✅ Yes (dynamic secrets) |
| **Audit logging** | ❌ No | ✅ CloudTrail | ✅ Audit devices |
| **Access control** | ❌ File permissions | ✅ IAM policies | ✅ Vault policies |
| **Cost** | Free | ~$9/month | Free (OSS) / $22/month (HCP) |
| **Multi-cloud** | ✅ Yes | ❌ AWS only | ✅ Yes |
| **Setup complexity** | Low | Medium | High |
| **Dynamic secrets** | ❌ No | ❌ No | ✅ Yes |
| **Encryption as a service** | ❌ No | ❌ No | ✅ Yes |
| **Replication** | ❌ Manual | ✅ Multi-region | ✅ Multi-region |

### Recommendation

- **Development**: Use `.env` files (simplicity)
- **Production (AWS-only)**: Use AWS Secrets Manager (integration)
- **Production (multi-cloud)**: Use HashiCorp Vault (flexibility)
- **Production (cost-sensitive)**: Use HashiCorp Vault OSS (free)

---

## Additional Resources

- [AWS Secrets Manager Documentation](https://docs.aws.amazon.com/secretsmanager/)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [12-Factor App: Config](https://12factor.net/config)

---

**Last Updated**: 2025-01-30
**Version**: 1.0.0
**Maintainers**: Security Team
