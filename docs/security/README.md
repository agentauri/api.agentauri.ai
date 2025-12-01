# Security Documentation

Comprehensive security documentation for the ERC-8004 backend infrastructure.

## Quick Links

### Secrets Management

- **[Secrets Management Guide](./SECRETS_MANAGEMENT.md)** - Complete guide to secrets management (1,050 lines)
  - Backend options (AWS, Vault, .env)
  - Setup instructions
  - Secret rotation
  - Access control
  - Disaster recovery

- **[Migration Guide](./SECRETS_MIGRATION_GUIDE.md)** - Step-by-step migration from .env (850 lines)
  - Pre-migration checklist
  - AWS Secrets Manager setup
  - HashiCorp Vault setup
  - Testing procedures
  - Rollback procedures

- **[Implementation Summary](./SECRETS_IMPLEMENTATION_SUMMARY.md)** - Technical overview
  - Architecture details
  - AWS vs Vault comparison
  - Cost analysis
  - Performance metrics
  - Security improvements

## Quick Start

### Local Development (Default)

```bash
# Use .env files (no setup required)
cargo run --bin api-gateway
```

### Local Development with Vault

```bash
# Start Vault dev server
docker-compose -f docker-compose.vault.yml up -d

# Configure application
export SECRETS_BACKEND=vault
export VAULT_ADDR='http://localhost:8200'
export VAULT_TOKEN='dev-root-token'

# Run application
cargo run --bin api-gateway
```

### Production (AWS Secrets Manager)

```bash
# Create secrets
./scripts/aws-secrets-setup.sh --region us-east-1

# Configure application
export SECRETS_BACKEND=aws
export AWS_REGION=us-east-1

# Build with AWS feature
cargo build --release --features aws-secrets

# Run application
./target/release/api-gateway
```

### Production (HashiCorp Vault)

```bash
# Vault must be deployed and accessible
export SECRETS_BACKEND=vault
export VAULT_ADDR='https://vault.example.com:8200'
export VAULT_TOKEN='s.YourProductionToken'

# Build with Vault feature
cargo build --release --features vault-secrets

# Run application
./target/release/api-gateway
```

## Secret Rotation

```bash
# Auto-generate new secret
./scripts/rotate-secrets.sh --backend aws --generate jwt_secret

# Manual entry
./scripts/rotate-secrets.sh --backend vault database_url

# Dry-run (preview changes)
./scripts/rotate-secrets.sh --backend aws --dry-run api_encryption_key
```

## Backend Comparison

| Feature | .env | AWS | Vault |
|---------|------|-----|-------|
| **Production Ready** | ❌ No | ✅ Yes | ✅ Yes |
| **Encryption** | ❌ None | ✅ AES-256 | ✅ AES-256-GCM |
| **Rotation** | ❌ Manual | ✅ Automatic | ✅ Dynamic |
| **Audit Logs** | ❌ None | ✅ CloudTrail | ✅ Audit devices |
| **Cost** | Free | ~$4-9/month | Free (OSS) / $22/month |
| **Setup Time** | 1 min | 30 min | 60 min |

## Security Best Practices

1. **Never commit secrets to git**
   - `.env` is in `.gitignore`
   - Use pre-commit hooks (gitleaks)

2. **Rotate secrets regularly**
   - Tier 1 (Critical): Every 90 days
   - Tier 2 (Important): Every 365 days

3. **Use least privilege**
   - IAM policies (AWS) or Vault policies
   - Read-only access for application

4. **Enable audit logging**
   - CloudTrail (AWS)
   - Vault audit devices

5. **Monitor secret access**
   - Alert on unusual patterns
   - Review logs regularly

## Troubleshooting

### "Secret not found"

```bash
# AWS: List secrets
aws secretsmanager list-secrets --query 'SecretList[].Name'

# Vault: List secrets
vault kv list secret/erc8004
```

### "Access denied"

```bash
# AWS: Check IAM role
aws sts get-caller-identity

# Vault: Check token
vault token lookup
```

### Application won't start

```bash
# Check logs
journalctl -u api-gateway -f | grep -i "secret\|error"

# Verify backend configuration
echo $SECRETS_BACKEND
echo $AWS_REGION  # for AWS
echo $VAULT_ADDR  # for Vault
```

## Additional Resources

- [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [AWS Secrets Manager Documentation](https://docs.aws.amazon.com/secretsmanager/)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [12-Factor App: Config](https://12factor.net/config)

---

**Last Updated**: 2025-01-30
**Version**: 1.0.0
**Maintainers**: Security Team
