# HTTPS Quick Start Guide

Get HTTPS running in 5 minutes for development or production.

## Development (Self-Signed Certificates)

### 1. Generate Certificate
```bash
cd docker/nginx/ssl
./generate-self-signed.sh
```

### 2. Start Nginx
```bash
# From project root
docker compose --profile development up -d nginx
```

### 3. Test HTTPS
```bash
# Test HTTP endpoint
curl http://localhost/health

# Test HTTPS endpoint (accept self-signed cert)
curl -k https://localhost/health

# Run full test suite
./scripts/test-https.sh localhost
```

### 4. Access API
```
HTTP:  http://localhost/api/v1/health
HTTPS: https://localhost/api/v1/health
Docs:  https://localhost/api-docs/

Note: Browser will show security warning - this is expected for self-signed certificates
```

## Production (Let's Encrypt)

### 1. Prerequisites
- Domain pointing to server (e.g., api.agentauri.ai)
- Ports 80 and 443 open
- Valid email for certificate notifications

### 2. Configure Environment
```bash
# Set in .env file
LETSENCRYPT_EMAIL=admin@agentauri.ai
DOMAIN=api.agentauri.ai
```

### 3. Initialize Let's Encrypt
```bash
# Test with staging first (recommended)
./scripts/init-letsencrypt.sh --staging

# Get production certificate
./scripts/init-letsencrypt.sh
```

### 4. Start Production Services
```bash
# Disable development override
mv docker-compose.override.yml docker-compose.override.yml.bak

# Start nginx and certbot
docker compose --profile production up -d nginx certbot
```

### 5. Test HTTPS
```bash
# HTTP should redirect to HTTPS
curl -I http://api.agentauri.ai/health

# Test HTTPS
curl https://api.agentauri.ai/health

# Run full test suite
./scripts/test-https.sh api.agentauri.ai
```

### 6. Verify SSL Rating
```
https://www.ssllabs.com/ssltest/analyze.html?d=api.agentauri.ai
```

Expected: A or A+ rating

## Troubleshooting

### Nginx won't start
```bash
# Check configuration syntax
docker compose exec nginx nginx -t

# Check logs
docker compose logs nginx
```

### Certificate errors
```bash
# Development: Regenerate self-signed cert
cd docker/nginx/ssl && ./generate-self-signed.sh

# Production: Renew Let's Encrypt cert
docker compose run --rm certbot renew --force-renewal
docker compose exec nginx nginx -s reload
```

### Port already in use
```bash
# Find process on port 80
sudo lsof -i :80

# Find process on port 443
sudo lsof -i :443

# Stop conflicting service
```

## Configuration Files

```
docker/nginx/
├── nginx.conf              # Main nginx config
├── conf.d/
│   ├── api.conf           # Production HTTPS config
│   └── api-dev.conf       # Development HTTP/HTTPS config
└── ssl/
    ├── generate-self-signed.sh
    ├── self-signed.crt    # Development cert
    └── self-signed.key    # Development key
```

## Security Features

### Enabled
- TLS 1.2 and 1.3 only
- Strong cipher suites (ECDHE, AES-GCM)
- HTTP/2 support
- HSTS headers (production)
- Security headers (X-Frame-Options, CSP, etc.)
- Automatic certificate renewal (production)

### Disabled
- TLS 1.0 and 1.1 (deprecated)
- Weak ciphers (DES, RC4, etc.)
- SSL compression (CRIME attack)
- Session tickets (forward secrecy)

## Next Steps

1. Review complete guide: `/docs/operations/HTTPS_SETUP.md`
2. Test with browsers and mobile devices
3. Monitor certificate expiration
4. Set up alerts for renewal failures
5. Review SSL Labs recommendations

---

**Need Help?** See `/docs/operations/HTTPS_SETUP.md` for detailed documentation.
