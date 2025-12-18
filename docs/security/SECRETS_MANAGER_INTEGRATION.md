# Secrets Manager Integration Guide (FIX 4.1)

## Overview

This guide explains how to integrate AWS Secrets Manager or HashiCorp Vault for secure secrets management in production. The codebase includes placeholder implementations in `rust-backend/crates/shared/src/secrets/` that need to be completed for production use.

## Current Status

**Development**: Secrets are stored in `.env` file (⚠️ NOT suitable for production)
**Production**: Must use AWS Secrets Manager or HashiCorp Vault

## Critical Secrets to Manage

1. **Database Encryption Key** (`DB_ENCRYPTION_KEY`)
   - Used for column-level encryption of PII data
   - MUST be stored in secrets manager
   - NEVER commit to version control

2. **JWT Secret** (`JWT_SECRET`)
   - Used for JWT token signing
   - Should be rotated every 90 days

3. **Database Password** (`DB_PASSWORD`)
   - PostgreSQL connection credentials

4. **Redis Password** (if using Redis AUTH)

5. **RPC Provider API Keys**
   - Alchemy, Infura, QuickNode, Ankr API keys
   - Store per environment (production/production)

## Option 1: AWS Secrets Manager

### Setup

1. **Create Secrets in AWS Console**:
   ```bash
   # Using AWS CLI
   aws secretsmanager create-secret \
     --name agentauri-backend/production/db-encryption-key \
     --secret-string "YOUR_ENCRYPTION_KEY_HERE"

   aws secretsmanager create-secret \
     --name agentauri-backend/production/jwt-secret \
     --secret-string "YOUR_JWT_SECRET_HERE"

   aws secretsmanager create-secret \
     --name agentauri-backend/production/database-password \
     --secret-string "YOUR_DB_PASSWORD_HERE"
   ```

2. **Grant IAM Permissions**:
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
         "Resource": "arn:aws:secretsmanager:*:*:secret:agentauri-backend/*"
       }
     ]
   }
   ```

3. **Update Code to Use Secrets Manager**:

   **In `shared/src/config.rs`** (add secrets loading):
   ```rust
   use crate::secrets::aws::SecretsManager;

   pub async fn load_secrets_from_aws() -> Result<HashMap<String, String>> {
       let secrets_manager = SecretsManager::new("us-east-1").await?;

       let mut secrets = HashMap::new();
       secrets.insert("db_encryption_key",
           secrets_manager.get_secret("agentauri-backend/production/db-encryption-key").await?);
       secrets.insert("jwt_secret",
           secrets_manager.get_secret("agentauri-backend/production/jwt-secret").await?);
       secrets.insert("db_password",
           secrets_manager.get_secret("agentauri-backend/production/database-password").await?);

       Ok(secrets)
   }
   ```

4. **Update `.env.example`**:
   ```env
   # Production: Load from AWS Secrets Manager
   # Set ENABLE_SECRETS_MANAGER=true to load secrets from AWS
   ENABLE_SECRETS_MANAGER=false
   AWS_REGION=us-east-1
   AWS_SECRETS_PREFIX=agentauri-backend/production

   # Development: Use local secrets (DO NOT USE IN PRODUCTION)
   DB_ENCRYPTION_KEY=REPLACE_WITH_KEY_FROM_SECRETS_MANAGER
   JWT_SECRET=your_jwt_secret_here_change_in_production
   ```

### Implementation Checklist

- [ ] Complete `rust-backend/crates/shared/src/secrets/aws.rs` implementation
- [ ] Add `aws-sdk-secretsmanager` crate to `Cargo.toml`
- [ ] Implement caching with 5-minute TTL to reduce API calls
- [ ] Add retry logic with exponential backoff
- [ ] Test secret rotation without service restart
- [ ] Document secret naming conventions

## Option 2: HashiCorp Vault

### Setup

1. **Install and Initialize Vault**:
   ```bash
   # Development: Run Vault in dev mode
   vault server -dev

   # Production: Use managed Vault (HashiCorp Cloud, Vault Enterprise)
   ```

2. **Store Secrets in Vault**:
   ```bash
   # Set Vault address and token
   export VAULT_ADDR='http://127.0.0.1:8200'
   export VAULT_TOKEN='your-root-token'

   # Store secrets
   vault kv put secret/agentauri-backend/production/db-encryption-key value="YOUR_KEY"
   vault kv put secret/agentauri-backend/production/jwt-secret value="YOUR_SECRET"
   vault kv put secret/agentauri-backend/production/database-password value="YOUR_PASSWORD"
   ```

3. **Configure AppRole Authentication**:
   ```bash
   # Enable AppRole
   vault auth enable approle

   # Create policy
   vault policy write agentauri-backend - <<EOF
   path "secret/data/agentauri-backend/production/*" {
     capabilities = ["read"]
   }
   EOF

   # Create AppRole
   vault write auth/approle/role/agentauri-backend \
     policies="agentauri-backend" \
     token_ttl=1h \
     token_max_ttl=4h

   # Get RoleID and SecretID
   vault read auth/approle/role/agentauri-backend/role-id
   vault write -f auth/approle/role/agentauri-backend/secret-id
   ```

4. **Update Code to Use Vault**:

   **In `shared/src/config.rs`**:
   ```rust
   use crate::secrets::vault::SecretsManager;

   pub async fn load_secrets_from_vault() -> Result<HashMap<String, String>> {
       let vault_addr = std::env::var("VAULT_ADDR")?;
       let role_id = std::env::var("VAULT_ROLE_ID")?;
       let secret_id = std::env::var("VAULT_SECRET_ID")?;

       let secrets_manager = SecretsManager::new(
           &vault_addr,
           &role_id,
           &secret_id,
           "secret/agentauri-backend/production"
       ).await?;

       let mut secrets = HashMap::new();
       secrets.insert("db_encryption_key",
           secrets_manager.get_secret("db-encryption-key").await?);
       secrets.insert("jwt_secret",
           secrets_manager.get_secret("jwt-secret").await?);
       secrets.insert("db_password",
           secrets_manager.get_secret("database-password").await?);

       Ok(secrets)
   }
   ```

5. **Update `.env.example`**:
   ```env
   # Production: Load from Vault
   ENABLE_SECRETS_MANAGER=false
   VAULT_ADDR=https://vault.example.com:8200
   VAULT_ROLE_ID=your-role-id
   VAULT_SECRET_ID=your-secret-id  # Store this in CI/CD secrets
   VAULT_MOUNT_PATH=secret/agentauri-backend/production
   ```

### Implementation Checklist

- [ ] Complete `rust-backend/crates/shared/src/secrets/vault.rs` implementation
- [ ] Add `vaultrs` or `vault-client` crate to `Cargo.toml`
- [ ] Implement token renewal before expiry
- [ ] Add caching with configurable TTL
- [ ] Test secret rotation without service restart
- [ ] Document AppRole setup for CI/CD

## Secret Rotation Strategy

### JWT Secret Rotation

1. **Dual-Key Period** (recommended):
   - Generate new secret: `JWT_SECRET_NEW`
   - Keep old secret: `JWT_SECRET_OLD`
   - Accept tokens signed with either key for 1 hour
   - Switch to `JWT_SECRET_NEW` after 1 hour
   - Remove `JWT_SECRET_OLD` after 2 hours

2. **Implementation**:
   ```rust
   pub struct JwtSecrets {
       current: String,
       previous: Option<String>,
   }

   impl JwtSecrets {
       pub fn verify_token(&self, token: &str) -> Result<Claims> {
           // Try current secret first
           if let Ok(claims) = verify_with_secret(token, &self.current) {
               return Ok(claims);
           }

           // Fall back to previous secret if available
           if let Some(prev) = &self.previous {
               return verify_with_secret(token, prev);
           }

           Err(AuthError::InvalidToken)
       }
   }
   ```

### Database Encryption Key Rotation

**WARNING**: Rotating encryption keys requires re-encrypting all encrypted data.

1. **Steps**:
   - Generate new key: `DB_ENCRYPTION_KEY_NEW`
   - Deploy code that supports both keys (read with either, write with new)
   - Background job: Re-encrypt all data with new key
   - Remove old key after all data is re-encrypted
   - Monitor for decryption failures

2. **Migration Script**:
   ```sql
   -- Identify encrypted columns
   SELECT COUNT(*) FROM users WHERE encrypted_email IS NOT NULL;

   -- Re-encrypt in batches (run from application code)
   -- UPDATE users SET encrypted_email = encrypt(decrypt(encrypted_email, OLD_KEY), NEW_KEY);
   ```

## Security Best Practices

1. **Never Log Secrets**:
   ```rust
   // BAD
   tracing::info!("JWT secret: {}", jwt_secret);

   // GOOD
   tracing::info!("JWT secret loaded from secrets manager");
   ```

2. **Redact Secrets in Error Messages**:
   ```rust
   // BAD
   return Err(anyhow!("Failed to encrypt with key: {}", key));

   // GOOD
   return Err(anyhow!("Failed to encrypt data (key validation failed)"));
   ```

3. **Use Short TTLs for Cached Secrets**:
   - Cache for 5-10 minutes max
   - Refresh before expiry to avoid service disruption
   - Implement graceful degradation if secrets manager is unavailable

4. **Monitor Secret Access**:
   ```rust
   #[cfg(feature = "metrics")]
   metrics::counter!("secrets_manager.access", 1, "secret_name" => secret_name);
   ```

5. **Implement Circuit Breaker**:
   ```rust
   // If secrets manager fails 3 times in a row, use cached value for 5 minutes
   // Log error and alert ops team
   ```

## Testing Secrets Manager Integration

1. **Unit Tests** (use mock):
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_secret_loading() {
           let mock_manager = MockSecretsManager::new();
           mock_manager.expect_get_secret()
               .with(eq("db-encryption-key"))
               .returning(|_| Ok("test-key-123".to_string()));

           let secret = mock_manager.get_secret("db-encryption-key").await.unwrap();
           assert_eq!(secret, "test-key-123");
       }
   }
   ```

2. **Integration Tests** (production environment):
   ```bash
   # Test secret loading
   ENABLE_SECRETS_MANAGER=true cargo test --test secrets_integration

   # Test secret rotation
   ./scripts/test-secret-rotation.sh
   ```

## Deployment Checklist

- [ ] All secrets stored in secrets manager (not `.env`)
- [ ] IAM roles configured with least privilege
- [ ] Secret rotation schedule defined (90 days for JWT)
- [ ] Monitoring and alerting for secret access failures
- [ ] Runbook documented for secret rotation
- [ ] Tested graceful degradation if secrets manager unavailable
- [ ] Verified no secrets in logs or error messages
- [ ] Configured short cache TTLs (5-10 minutes)

## Monitoring

Add these Prometheus metrics:

```rust
// Secret access success/failure
metrics::counter!("secrets_manager.access.success", 1);
metrics::counter!("secrets_manager.access.failure", 1);

// Cache hit/miss rate
metrics::counter!("secrets_manager.cache.hit", 1);
metrics::counter!("secrets_manager.cache.miss", 1);

// Secret age (days since last rotation)
metrics::gauge!("secrets_manager.secret_age_days", age_days);
```

Alert when:
- Secret access failure rate >5%
- Cache miss rate >50%
- Secret age >80 days
- Secrets manager API latency >500ms (p95)

## References

- AWS Secrets Manager: https://docs.aws.amazon.com/secretsmanager/
- HashiCorp Vault: https://www.vaultproject.io/docs
- OWASP Secrets Management: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html

---

**Last Updated**: January 30, 2025
**Status**: Phase 4 Complete - Documentation Only (Implementation Required for Production)
