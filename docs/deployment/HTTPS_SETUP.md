# HTTPS/TLS Setup Guide

Complete guide for enabling HTTPS/TLS on api.8004.dev with automatic Let's Encrypt certificate management.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Configuration Details](#configuration-details)
- [Testing & Validation](#testing--validation)
- [Certificate Management](#certificate-management)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)
- [Monitoring & Alerts](#monitoring--alerts)
- [Rollback Procedure](#rollback-procedure)

## Overview

The ERC-8004 backend uses **Nginx** as a reverse proxy for HTTPS termination with **Let's Encrypt** for automatic SSL/TLS certificate management.

### Architecture

```
Internet → Nginx (HTTPS termination) → API Gateway (HTTP localhost:8080)
         ↓                            → Ponder Indexers (HTTP localhost:42069)
    Let's Encrypt
```

**Key Features**:
- Automatic certificate issuance and renewal
- TLS 1.2+ with modern cipher suites (Mozilla Intermediate profile)
- HTTP → HTTPS redirect (301)
- HSTS with preload support
- Comprehensive security headers
- OCSP stapling for performance
- Rate limiting per endpoint type

## Prerequisites

### 1. Domain Configuration

**Required**:
- Domain name purchased and configured (e.g., `api.8004.dev`)
- DNS A record pointing to production server IP
- Domain propagation complete (verify with `host api.8004.dev`)

**DNS Configuration Example**:
```
Type: A
Name: api.8004.dev
Value: <YOUR_SERVER_IP>
TTL: 3600 (1 hour)
```

**Verify DNS**:
```bash
host api.8004.dev
# Should show: api.8004.dev has address <YOUR_SERVER_IP>
```

### 2. Server Requirements

**Firewall Configuration**:
```bash
# Allow HTTP (for Let's Encrypt ACME challenge)
sudo ufw allow 80/tcp

# Allow HTTPS
sudo ufw allow 443/tcp

# Verify firewall status
sudo ufw status
```

**Port Availability**:
```bash
# Check if ports are free
sudo lsof -i :80
sudo lsof -i :443

# If blocked, stop conflicting services
sudo systemctl stop apache2  # Example: Apache
sudo systemctl stop nginx    # Example: existing Nginx
```

### 3. Docker Environment

**Required**:
- Docker Engine 20.10+
- Docker Compose 2.0+

**Verify**:
```bash
docker --version
docker compose version
```

### 4. Email Address

**Required for Let's Encrypt**:
- Valid email for certificate expiry notifications
- Set in `.env` file: `LETSENCRYPT_EMAIL=admin@8004.dev`

## Quick Start

### Step 1: Update Configuration

Edit `.env` file:
```bash
# HTTPS Configuration
DOMAIN=api.8004.dev
LETSENCRYPT_EMAIL=your-email@example.com  # CHANGE THIS
ENABLE_HTTPS=true
BASE_URL=https://api.8004.dev
```

### Step 2: Initialize Let's Encrypt

Run initialization script:
```bash
# Production certificate (trusted by browsers)
./scripts/init-letsencrypt.sh

# OR for testing (staging environment, not trusted)
./scripts/init-letsencrypt.sh --staging
```

**What this does**:
1. Downloads TLS parameters (Diffie-Hellman, SSL options)
2. Creates dummy certificate for initial nginx startup
3. Starts nginx container
4. Requests real certificate from Let's Encrypt
5. Reloads nginx with production certificate

**Expected output**:
```
==========================================================================
SUCCESS! HTTPS/TLS setup complete for api.8004.dev
==========================================================================

Certificate details:
  Domain: api.8004.dev
  Email: admin@8004.dev
  Issuer: Let's Encrypt
  Environment: PRODUCTION

Next steps:
  1. Test HTTPS: curl https://api.8004.dev/health
  2. Run test script: ./scripts/test-https.sh
```

### Step 3: Start Production Services

```bash
# Start all services with HTTPS enabled
docker compose --profile production up -d

# Verify services are running
docker compose --profile production ps
```

### Step 4: Validate HTTPS

Run comprehensive tests:
```bash
./scripts/test-https.sh
```

**Expected result**: All tests pass (✓ PASS)

## Configuration Details

### Nginx Configuration

**Main configuration** (`/docker/nginx/nginx.conf`):
- Worker processes: auto (CPU-optimized)
- Worker connections: 1024
- Gzip compression: enabled (6 levels)
- Rate limiting zones:
  - `api_limit`: 100 req/sec (API endpoints)
  - `login_limit`: 5 req/min (authentication)
- Connection limit: 10 per IP
- Global security headers

**Site configuration** (`/docker/nginx/conf.d/api.conf`):
- HTTP server (port 80): Redirect to HTTPS + ACME challenge
- HTTPS server (port 443):
  - TLS 1.2 and TLS 1.3
  - Modern cipher suites (AEAD priority)
  - HSTS with 1-year max-age
  - OCSP stapling
  - CSP, X-Frame-Options, etc.

### TLS Configuration

**Protocols**: TLS 1.2, TLS 1.3 (TLS 1.0/1.1 disabled)

**Cipher Suites** (Mozilla Intermediate profile):
```
ECDHE-ECDSA-AES128-GCM-SHA256
ECDHE-RSA-AES128-GCM-SHA256
ECDHE-ECDSA-AES256-GCM-SHA384
ECDHE-RSA-AES256-GCM-SHA384
ECDHE-ECDSA-CHACHA20-POLY1305
ECDHE-RSA-CHACHA20-POLY1305
DHE-RSA-AES128-GCM-SHA256
DHE-RSA-AES256-GCM-SHA384
```

**Session Management**:
- Timeout: 1 day
- Cache: 50MB (shared, ~200k sessions)
- Session tickets: disabled (forward secrecy)

### Security Headers

**Applied to all HTTPS responses**:

| Header | Value | Purpose |
|--------|-------|---------|
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains; preload` | Force HTTPS for 1 year |
| `Content-Security-Policy` | `default-src 'none'; frame-ancestors 'none'` | Prevent injection attacks |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-XSS-Protection` | `1; mode=block` | XSS protection (legacy) |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Control referrer info |
| `Permissions-Policy` | All features disabled | Disable browser features |

### Rate Limiting

**API endpoints** (`/api/*`):
- Limit: 100 requests/second per IP
- Burst: 20 requests
- Mode: nodelay (immediate rejection)

**Authentication endpoints** (`/api/v1/auth/*`):
- Limit: 5 requests/minute per IP
- Burst: 3 requests
- Mode: nodelay

**Health check** (`/health`):
- No rate limiting
- No access logging

## Testing & Validation

### Automated Testing

Run comprehensive test suite:
```bash
./scripts/test-https.sh [domain]
```

**Tests performed** (11 total):
1. DNS resolution
2. HTTP → HTTPS redirect (301)
3. HTTPS connectivity
4. HSTS header (1-year, includeSubDomains, preload)
5. Security headers (X-Frame-Options, CSP, etc.)
6. TLS version (1.2+, no 1.0/1.1)
7. Certificate validity and expiry
8. Cipher suite strength (AEAD)
9. OCSP stapling
10. HTTP/2 support
11. API endpoint access

**Expected output**:
```
==========================================================================
Test Summary
==========================================================================

Domain: api.8004.dev

Passed:   11
Warnings: 0
Failed:   0

✓ All tests passed! HTTPS configuration is excellent.
```

### Manual Testing

**Test HTTPS connection**:
```bash
curl -v https://api.8004.dev/health
```

**Test HTTP redirect**:
```bash
curl -I http://api.8004.dev/
# Should return: 301 Moved Permanently
# Location: https://api.8004.dev/
```

**Test security headers**:
```bash
curl -I https://api.8004.dev/ | grep -i "strict-transport-security"
```

**Test TLS version**:
```bash
openssl s_client -connect api.8004.dev:443 -tls1_2 < /dev/null
# Should show: Protocol : TLSv1.2
```

**Test certificate**:
```bash
openssl s_client -connect api.8004.dev:443 -servername api.8004.dev < /dev/null 2>&1 | openssl x509 -noout -text
```

### External Validation

**SSL Labs** (comprehensive TLS audit):
```
https://www.ssllabs.com/ssltest/analyze.html?d=api.8004.dev
```
**Target**: A or A+ rating

**SecurityHeaders.com** (header analysis):
```
https://securityheaders.com/?q=https://api.8004.dev
```
**Target**: A rating

**Mozilla Observatory** (security assessment):
```
https://observatory.mozilla.org/analyze/api.8004.dev
```
**Target**: B+ or higher

## Certificate Management

### Automatic Renewal

**Certbot container** runs continuous renewal loop:
- Checks every 12 hours
- Renews if <30 days until expiry
- Automatically reloads nginx

**Verify renewal cron**:
```bash
docker compose --profile production logs certbot
```

### Manual Renewal

**Force renewal** (testing):
```bash
docker compose --profile production run --rm certbot renew --force-renewal
docker compose --profile production exec nginx nginx -s reload
```

**Dry-run** (test renewal without requesting certificate):
```bash
docker compose --profile production run --rm certbot renew --dry-run
```

### Certificate Expiry Check

**Check expiry date**:
```bash
openssl s_client -connect api.8004.dev:443 -servername api.8004.dev < /dev/null 2>&1 | openssl x509 -noout -enddate
```

**Alert if <30 days**:
```bash
openssl s_client -connect api.8004.dev:443 -servername api.8004.dev < /dev/null 2>&1 | openssl x509 -noout -checkend 2592000 || echo "WARNING: Certificate expires soon!"
```

### Certificate Revocation

**If compromised**:
```bash
docker compose --profile production run --rm certbot revoke --cert-path /etc/letsencrypt/live/api.8004.dev/fullchain.pem
./scripts/init-letsencrypt.sh  # Request new certificate
```

## Troubleshooting

### Issue: nginx fails to start

**Symptom**: `docker compose logs nginx` shows error

**Common causes**:
1. **Port 80/443 already in use**:
   ```bash
   sudo lsof -i :80
   sudo lsof -i :443
   # Stop conflicting service
   ```

2. **Certificate files missing**:
   ```bash
   ls -la docker/certbot/conf/live/api.8004.dev/
   # Should show: fullchain.pem, privkey.pem
   # Fix: Run ./scripts/init-letsencrypt.sh
   ```

3. **Configuration syntax error**:
   ```bash
   docker compose --profile production run --rm nginx nginx -t
   # Should show: syntax is ok
   ```

### Issue: Let's Encrypt certificate request fails

**Symptom**: `init-letsencrypt.sh` fails with "too many failed authorizations"

**Causes**:
1. **DNS not configured**: Verify `host api.8004.dev` resolves to server IP
2. **Firewall blocks port 80**: `sudo ufw allow 80/tcp`
3. **Rate limit hit**: Use staging environment (`--staging` flag)
4. **Port 80 not accessible**: Test `curl http://<SERVER_IP>/.well-known/test`

**Fix for rate limit**:
```bash
# Wait 1 hour, then use staging
./scripts/init-letsencrypt.sh --staging
# Test with staging certificate
# Once working, request production certificate
./scripts/init-letsencrypt.sh
```

### Issue: HSTS warnings in browser

**Symptom**: Browser shows "NET::ERR_CERT_AUTHORITY_INVALID"

**Cause**: Using staging certificate (not trusted)

**Fix**:
```bash
# Request production certificate
docker compose --profile production down
./scripts/init-letsencrypt.sh  # Without --staging flag
```

**Clear HSTS cache** (Chrome):
```
chrome://net-internals/#hsts
# Delete domain security policies for api.8004.dev
```

### Issue: Certificate renewal fails

**Symptom**: Certificate expired, renewal logs show errors

**Debug**:
```bash
# Check certbot logs
docker compose --profile production logs certbot

# Test renewal
docker compose --profile production run --rm certbot renew --dry-run

# Check ACME challenge accessibility
curl http://api.8004.dev/.well-known/acme-challenge/test
# Should return: 404 (not 301 redirect)
```

**Fix**:
1. Ensure nginx allows ACME challenge (HTTP-01)
2. Check firewall rules (port 80)
3. Verify nginx configuration doesn't redirect `/.well-known/acme-challenge/`

### Issue: SSL Labs rating below A

**Check cipher suites**:
```bash
docker compose --profile production exec nginx nginx -T | grep ssl_ciphers
```

**Check TLS version**:
```bash
docker compose --profile production exec nginx nginx -T | grep ssl_protocols
```

**Verify OCSP stapling**:
```bash
openssl s_client -connect api.8004.dev:443 -status < /dev/null 2>&1 | grep "OCSP Response Status"
# Should show: successful
```

## Security Considerations

### Certificate Security

**Private key protection**:
- Stored in `docker/certbot/conf/live/` (git-ignored)
- Never commit to version control
- File permissions: 600 (read/write owner only)
- Rotated with certificate renewal (every 90 days)

**Backup strategy**:
```bash
# Backup certificates (encrypted)
tar -czf cert-backup-$(date +%Y%m%d).tar.gz docker/certbot/conf/
gpg --encrypt cert-backup-$(date +%Y%m%d).tar.gz
# Store encrypted backup off-server
```

### HSTS Considerations

**HSTS Preload** (`preload` directive):
- Domain permanently in browser HSTS list
- Cannot be easily revoked (requires manual removal request)
- Only enable if 100% confident in HTTPS-only operation

**Disable preload** (if needed):
```nginx
# In docker/nginx/conf.d/api.conf
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
# Remove "preload" directive
```

### Rate Limiting

**Adjust limits** (if needed):
```nginx
# In docker/nginx/nginx.conf
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=200r/s;  # Increase to 200 req/s
```

**Whitelist trusted IPs**:
```nginx
# In docker/nginx/conf.d/api.conf, location /api/
geo $limit {
    default 1;
    10.0.0.0/8 0;     # Internal network
    203.0.113.5 0;    # Trusted monitoring IP
}
map $limit $limit_key {
    0 "";
    1 $binary_remote_addr;
}
limit_req_zone $limit_key zone=api_limit:10m rate=100r/s;
```

### Certificate Transparency

**Monitor certificate issuance**:
- Subscribe to https://crt.sh/?q=api.8004.dev
- Receive alerts for any certificate issued for your domain
- Detect unauthorized certificate requests

## Monitoring & Alerts

### Certificate Expiry Monitoring

**Cron job** (daily check):
```bash
# Add to crontab: crontab -e
0 0 * * * /usr/bin/docker compose -f /path/to/api.8004.dev/docker-compose.yml --profile production run --rm certbot renew --quiet

# Alert if <30 days
0 0 * * * /usr/bin/openssl s_client -connect api.8004.dev:443 -servername api.8004.dev </dev/null 2>/dev/null | /usr/bin/openssl x509 -noout -checkend 2592000 || echo "WARNING: Certificate expires soon!" | mail -s "SSL Certificate Expiry Alert" admin@8004.dev
```

### Nginx Metrics

**Prometheus exporter** (optional):
```yaml
# Add to docker-compose.yml
nginx-exporter:
  image: nginx/nginx-prometheus-exporter:latest
  command:
    - '-nginx.scrape-uri=http://nginx:8080/stub_status'
  ports:
    - "9113:9113"
```

**Enable stub_status**:
```nginx
# In docker/nginx/conf.d/api.conf
location /stub_status {
    stub_status;
    allow 127.0.0.1;
    deny all;
}
```

### SSL/TLS Alerts

**Monitor SSL Labs rating** (weekly):
```bash
# Use SSL Labs API
curl -s "https://api.ssllabs.com/api/v3/analyze?host=api.8004.dev" | jq '.endpoints[0].grade'
# Alert if grade < A
```

## Rollback Procedure

### Emergency Rollback (Disable HTTPS)

**If HTTPS causes production issues**:

1. **Stop nginx and certbot**:
   ```bash
   docker compose --profile production stop nginx certbot
   ```

2. **Update environment** (`.env`):
   ```bash
   ENABLE_HTTPS=false
   BASE_URL=http://api.8004.dev
   ```

3. **Restart without HTTPS**:
   ```bash
   docker compose up -d  # Without --profile production
   ```

4. **Update DNS** (if needed):
   - Point to load balancer without TLS termination
   - Or expose API Gateway directly (port 8080 → 80)

### Restore from Backup

**If certificate corrupted**:
```bash
# Stop services
docker compose --profile production stop nginx certbot

# Restore from backup
gpg --decrypt cert-backup-20250130.tar.gz.gpg | tar -xzf -

# Restart services
docker compose --profile production up -d nginx
```

### Revert to Self-Signed Certificate

**For development/testing**:
```bash
# Generate self-signed certificate
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout docker/certbot/conf/live/api.8004.dev/privkey.pem \
  -out docker/certbot/conf/live/api.8004.dev/fullchain.pem \
  -subj "/CN=api.8004.dev"

# Restart nginx
docker compose --profile production restart nginx
```

**Warning**: Browsers will show certificate warnings.

## Additional Resources

### Documentation
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [Nginx SSL/TLS Module](https://nginx.org/en/docs/http/ngx_http_ssl_module.html)
- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)

### Security Standards
- [OWASP Transport Layer Protection Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)
- [RFC 8996: TLS 1.0/1.1 Deprecation](https://www.rfc-editor.org/rfc/rfc8996.html)
- [RFC 6797: HTTP Strict Transport Security](https://www.rfc-editor.org/rfc/rfc6797.html)

### Tools
- [SSL Labs Server Test](https://www.ssllabs.com/ssltest/)
- [SecurityHeaders.com](https://securityheaders.com/)
- [Mozilla Observatory](https://observatory.mozilla.org/)
- [testssl.sh](https://testssl.sh/) - Command-line SSL/TLS tester

---

**Last Updated**: January 30, 2025
**Version**: 1.0.0
**Maintainer**: DevOps Team
