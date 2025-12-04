# Database Encryption Quick Start

Get PostgreSQL encryption running in **5 minutes**.

## Prerequisites

- Docker and Docker Compose installed
- OpenSSL installed (`brew install openssl` or `apt-get install openssl`)

## Step 1: Generate TLS Certificates (1 minute)

```bash
# Generate self-signed certificates for development
./scripts/generate-pg-certs.sh
```

**Output**:
```
========================================================================
PostgreSQL TLS Certificate Generation
========================================================================

Step 1: Generating Certificate Authority (CA)...
✓ CA certificate generated (valid for 3650 days)

Step 2: Generating server certificate...
✓ Server certificate generated (valid for 365 days)

Step 3: Verifying certificates...
✓ Certificate verification successful

========================================================================
Certificate Summary
========================================================================

Files generated in ./docker/postgres/certs:
  - root.crt     : CA certificate (for client verification)
  - root.key     : CA private key (keep secure!)
  - server.crt   : Server certificate
  - server.key   : Server private key (keep secure!)

Certificate Details:
----------------------------------------
CA Certificate:
subject=CN = PostgreSQL CA
issuer=CN = PostgreSQL CA
notBefore=Nov 30 12:00:00 2025 GMT
notAfter=Nov 27 12:00:00 2035 GMT

Server Certificate:
subject=CN = localhost
issuer=CN = PostgreSQL CA
notBefore=Nov 30 12:00:00 2025 GMT
notAfter=Nov 29 12:00:00 2026 GMT

Subject Alternative Names (SAN):
Subject Alternative Name:
    DNS:localhost, DNS:*.localhost, DNS:postgres, DNS:agentauri-postgres, IP Address:127.0.0.1, IP Address:0:0:0:0:0:0:0:1

========================================================================
SECURITY WARNINGS
========================================================================
⚠  These are self-signed certificates for DEVELOPMENT ONLY
⚠  For production, use certificates from a trusted CA
⚠  Keep root.key and server.key secure (chmod 600)
⚠  Never commit certificates to version control

✓ Added certificates to .gitignore

========================================================================
Next Steps
========================================================================
1. Start PostgreSQL with TLS enabled:
   docker compose up -d postgres

2. Test TLS connection:
   ./scripts/test-pg-tls.sh

3. Update DATABASE_URL in .env:
   DATABASE_URL=postgresql://postgres:password@localhost:5432/agentauri_backend?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt

Certificate generation complete!
```

## Step 2: Update Environment Variables (30 seconds)

Edit `.env`:

```bash
# Before (no encryption)
DATABASE_URL=postgresql://postgres:YOUR_PASSWORD@localhost:5432/agentauri_backend

# After (with TLS)
DATABASE_URL=postgresql://postgres:YOUR_PASSWORD@localhost:5432/agentauri_backend?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt

# Add encryption key (from secrets manager in production)
DB_ENCRYPTION_KEY=your_secret_key_here_change_in_production
```

## Step 3: Start PostgreSQL with TLS (1 minute)

```bash
# Stop existing container (if running)
docker compose down postgres

# Start with TLS enabled
docker compose up -d postgres

# Wait for PostgreSQL to be ready
docker compose logs -f postgres
# Look for: "database system is ready to accept connections"
```

## Step 4: Run Database Migrations (1 minute)

```bash
# Apply migrations (includes pgcrypto extension + encryption functions)
cd rust-backend
sqlx migrate run

# Verify pgcrypto extension is enabled
docker compose exec postgres psql -U postgres -d agentauri_backend -c "\dx pgcrypto"
```

**Expected output**:
```
                          List of installed extensions
   Name    | Version |   Schema   |                    Description
-----------+---------+------------+----------------------------------------------------
 pgcrypto  | 1.3     | public     | cryptographic functions
```

## Step 5: Test TLS Connection (1 minute)

```bash
# Run comprehensive TLS tests
./scripts/test-pg-tls.sh

# Expected output: All 8 tests should PASS
```

## Step 6: Verify Encryption Functions (30 seconds)

```bash
# Connect to database
docker compose exec postgres psql -U postgres -d agentauri_backend

# Test encryption/decryption
SELECT encrypt_text('hello@example.com', 'test_key');
-- Returns: base64 encrypted string (e.g., 'ww0ECQMCXx5...')

SELECT decrypt_text(encrypt_text('hello@example.com', 'test_key'), 'test_key');
-- Returns: hello@example.com

# Exit
\q
```

## Done!

Your database now has:

✅ **TLS 1.2+ encryption** (protects data in transit)
✅ **pgcrypto extension** (for column-level encryption)
✅ **Encryption functions** (encrypt_text, decrypt_text)
✅ **Strong ciphers** (AES-256 only)
✅ **Certificate validation** (self-signed for dev)

## Next Steps

### For Development

1. **Use encrypted connections in your app**:
   ```rust
   let pool = PgPool::connect(
       "postgresql://postgres:password@localhost:5432/agentauri_backend?sslmode=require&sslrootcert=./docker/postgres/certs/root.crt"
   ).await?;
   ```

2. **Encrypt sensitive columns** (optional):
   ```sql
   -- Example: Encrypt email addresses
   UPDATE users
   SET email = encrypt_text(email, 'key_from_vault')
   WHERE email IS NOT NULL;
   ```

### For Production

1. **Use CA-signed certificates** (Let's Encrypt, commercial CA)
2. **Use managed PostgreSQL** (AWS RDS, Azure, GCP - includes TDE)
3. **Store encryption keys in secrets manager** (AWS Secrets Manager, Vault)
4. **Enable audit logging** for encrypted data access
5. **Set up automatic certificate renewal**
6. **Use `sslmode=verify-full`** instead of `sslmode=require`

## Troubleshooting

### Problem: TLS connection fails

```bash
ERROR: connection requires a valid SSL connection
```

**Solution**:
```bash
# Verify certificates exist
ls -la ./docker/postgres/certs/

# Regenerate if missing
./scripts/generate-pg-certs.sh

# Restart PostgreSQL
docker compose restart postgres
```

### Problem: Certificate expired

```bash
ERROR: SSL certificate verify failed: certificate has expired
```

**Solution**:
```bash
# Check expiry
openssl x509 -in ./docker/postgres/certs/server.crt -noout -enddate

# Regenerate certificates (valid for 1 year)
./scripts/generate-pg-certs.sh

# Restart PostgreSQL
docker compose restart postgres
```

### Problem: Performance is slow

**Solution**:
```bash
# Use connection pooling (in .env)
DB_MAX_CONNECTIONS=20

# Cache decrypted data in application
# Only decrypt once per request, store in memory
```

## Full Documentation

See [DATABASE_ENCRYPTION.md](./DATABASE_ENCRYPTION.md) for:
- Production setup (managed PostgreSQL, CA certificates)
- Column-level encryption guide
- Key management and rotation
- Compliance (GDPR, HIPAA, PCI DSS)
- Performance benchmarks
- Disaster recovery procedures

## Security Checklist

Development:
- [x] TLS certificates generated
- [x] PostgreSQL configured with TLS
- [x] Connection string uses `sslmode=require`
- [x] pgcrypto extension enabled
- [ ] Encryption key stored securely (not in code)

Production (before going live):
- [ ] CA-signed certificates (not self-signed)
- [ ] Connection string uses `sslmode=verify-full`
- [ ] Encryption keys in secrets manager (AWS/Vault/GCP)
- [ ] TDE enabled (managed database)
- [ ] Column encryption for PII (email, phone, etc.)
- [ ] Audit logging enabled
- [ ] Certificate auto-renewal configured
- [ ] Backup encryption verified
- [ ] Disaster recovery tested

## Support

- Documentation: `docs/security/DATABASE_ENCRYPTION.md`
- Test script: `./scripts/test-pg-tls.sh`
- Certificate generation: `./scripts/generate-pg-certs.sh`
- GitHub Issues: https://github.com/erc-8004/api.agentauri.ai/issues
