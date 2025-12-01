# PostgreSQL TLS Certificates

This directory contains TLS certificates for encrypting PostgreSQL connections.

## Development Certificates

Self-signed certificates for local development are generated automatically:

```bash
# Generate certificates
./scripts/generate-pg-certs.sh
```

This creates:
- `root.crt` - CA certificate (for client verification)
- `root.key` - CA private key
- `server.crt` - Server certificate (valid for 1 year)
- `server.key` - Server private key

## SECURITY WARNINGS

⚠️ **Development Only**: These self-signed certificates are for **LOCAL DEVELOPMENT ONLY**

⚠️ **Never Commit**: Certificates are automatically added to `.gitignore` to prevent accidental commits

⚠️ **Production**: Use CA-signed certificates from:
- Let's Encrypt (free, automated)
- DigiCert, GlobalSign, Sectigo (commercial)
- Managed database service (AWS RDS, Azure, GCP)

## Certificate Renewal

Development certificates expire after **1 year**. Renew by re-running:

```bash
./scripts/generate-pg-certs.sh
```

Production certificates should be renewed **automatically** via:
- Let's Encrypt Certbot (auto-renewal)
- Cloud provider certificate manager
- Manual renewal 30 days before expiry

## File Permissions

Certificates MUST have correct permissions:

```bash
chmod 644 root.crt server.crt   # Readable by all
chmod 600 root.key server.key   # Readable by owner only (postgres)
```

## Testing

Verify TLS is working:

```bash
./scripts/test-pg-tls.sh
```

Expected output:
```
========================================================================
PostgreSQL TLS Connection Tests
========================================================================

Test 1: TLS connection with certificate verification
✓ PASS: TLS connection successful

Test 2: Non-TLS connection (should fail if TLS enforced)
✓ PASS: Non-TLS connection rejected (TLS enforced)

...

✓ All tests passed! PostgreSQL TLS encryption is working correctly.
```

## Troubleshooting

### Certificate not found

```bash
ERROR: root certificate file "/path/to/root.crt" does not exist
```

**Solution**: Generate certificates with `./scripts/generate-pg-certs.sh`

### Permission denied

```bash
ERROR: private key file "/var/lib/postgresql/server.key" has group or world access
```

**Solution**: Fix permissions
```bash
chmod 600 docker/postgres/certs/server.key
```

### Certificate expired

```bash
ERROR: SSL certificate verify failed: certificate has expired
```

**Solution**: Regenerate certificates
```bash
./scripts/generate-pg-certs.sh
docker compose restart postgres
```

## Documentation

- [Database Encryption Guide](../../../docs/security/DATABASE_ENCRYPTION.md) - Complete encryption documentation
- [Quick Start](../../../docs/security/DATABASE_ENCRYPTION_QUICKSTART.md) - Get encryption running in 5 minutes
- [Production Setup](../../../docs/security/PRODUCTION_DB_SETUP.md) - Production deployment guides
