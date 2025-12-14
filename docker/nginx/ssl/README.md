# SSL Certificate Directory

This directory contains SSL/TLS certificates for development HTTPS testing.

## Development Certificates

For local HTTPS testing, use self-signed certificates:

```bash
# Generate self-signed certificate
./generate-self-signed.sh
```

This creates:
- `self-signed.crt` - Certificate (valid 365 days)
- `self-signed.key` - Private key

**Domains**: localhost, api.agentauri.local, 127.0.0.1

## Production Certificates

Production uses Let's Encrypt certificates managed by Certbot.

**Certificate Location**: `docker/certbot/conf/live/api.agentauri.ai/`

**Setup**:
```bash
./scripts/init-letsencrypt.sh
```

## Security

- Certificate files are ignored by Git (see `.gitignore`)
- Never commit private keys to version control
- Use self-signed certificates for development only
- Production must use Let's Encrypt or trusted CA

## Files

- `generate-self-signed.sh` - Certificate generation script
- `self-signed.crt` - Self-signed certificate (gitignored)
- `self-signed.key` - Private key (gitignored)
- `.gitignore` - Excludes certificate files from Git
- `README.md` - This file

## Documentation

See `/docs/operations/HTTPS_SETUP.md` for complete HTTPS configuration guide.
