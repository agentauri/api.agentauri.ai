# HTTPS/TLS Configuration Guide

Complete guide for HTTPS enforcement in the api.agentauri.ai project.

## Table of Contents

- [Overview](#overview)
- [Development Setup](#development-setup)
- [Production Setup](#production-setup)
- [Configuration Details](#configuration-details)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Security Best Practices](#security-best-practices)

## Overview

The API Gateway uses Nginx as a reverse proxy with TLS termination for HTTPS enforcement. The configuration supports:

- **Development**: Self-signed certificates, HTTP allowed for local testing
- **Production**: Let's Encrypt certificates, HTTPS enforced with HSTS
- **Security Headers**: HSTS, CSP, X-Frame-Options, X-Content-Type-Options
- **TLS 1.2+**: Modern cipher suites, forward secrecy
- **HTTP/2**: Enabled for performance
- **Automatic Certificate Renewal**: Via Certbot

## Development Setup

### Prerequisites

- Docker and Docker Compose installed
- OpenSSL installed (for certificate generation)

### Step 1: Generate Self-Signed Certificate

```bash
# Navigate to nginx ssl directory
cd docker/nginx/ssl

# Run certificate generation script
./generate-self-signed.sh
```

This creates:
- `self-signed.crt` - Self-signed certificate
- `self-signed.key` - Private key

**Certificate Details**:
- Valid for 365 days
- Domains: localhost, api.agentauri.local, 127.0.0.1
- 2048-bit RSA key

### Step 2: Start Nginx (Development)

```bash
# Start nginx with development profile
docker compose --profile development up -d nginx

# Verify nginx is running
docker compose ps nginx
```

### Step 3: Test HTTPS

```bash
# Test HTTP endpoint (allowed in development)
curl http://localhost/health

# Test HTTPS endpoint (self-signed, accept certificate)
curl -k https://localhost/health

# Run comprehensive test suite
./scripts/test-https.sh localhost
```

**Browser Access**:
1. Navigate to https://localhost/health
2. Accept security warning (expected for self-signed certificate)
3. Proceed to endpoint

### Development Configuration

The `docker-compose.override.yml` file automatically configures development mode:

```yaml
services:
  nginx:
    profiles:
      - development
    volumes:
      - ./docker/nginx/conf.d/api-dev.conf:/etc/nginx/conf.d/default.conf:ro
      - ./docker/nginx/ssl:/etc/nginx/ssl:ro
```

**Key Differences from Production**:
- HTTP access allowed (no redirect)
- Self-signed certificates
- Relaxed rate limiting
- No HSTS header (allows HTTP)
- Metrics endpoint accessible

## Production Setup

### Prerequisites

- Domain name (e.g., api.agentauri.ai) pointing to server
- Ports 80 and 443 open in firewall
- Valid email address for Let's Encrypt notifications
- Docker and Docker Compose installed

### Step 1: Configure Environment

```bash
# Set environment variables in .env file
LETSENCRYPT_EMAIL=admin@agentauri.ai
DOMAIN=api.agentauri.ai
```

### Step 2: Initialize Let's Encrypt

```bash
# Test with production environment (recommended first)
./scripts/init-letsencrypt.sh --production

# Verify production certificate works
curl -k https://api.agentauri.ai/health

# Get production certificate
./scripts/init-letsencrypt.sh
```

**What the script does**:
1. Downloads TLS parameters from Mozilla
2. Creates dummy certificate for nginx startup
3. Starts nginx with dummy certificate
4. Requests real certificate from Let's Encrypt
5. Reloads nginx with production certificate

### Step 3: Start Production Services

```bash
# Disable development override (if present)
mv docker-compose.override.yml docker-compose.override.yml.bak

# Start production services
docker compose --profile production up -d nginx certbot
```

### Step 4: Verify HTTPS

```bash
# Test HTTP redirect
curl -I http://api.agentauri.ai/health
# Should return: HTTP/1.1 301 Moved Permanently
# Location: https://api.agentauri.ai/health

# Test HTTPS endpoint
curl https://api.agentauri.ai/health

# Run comprehensive test suite
./scripts/test-https.sh api.agentauri.ai
```

### Certificate Renewal

Certificates are automatically renewed by the Certbot container:

```bash
# Check renewal status
docker compose logs certbot

# Manual renewal (if needed)
docker compose run --rm certbot renew

# Reload nginx after manual renewal
docker compose exec nginx nginx -s reload
```

**Renewal Schedule**:
- Certbot checks every 12 hours
- Renews certificates <30 days from expiration
- Let's Encrypt certificates valid for 90 days

## Configuration Details

### Nginx Configuration Files

```
docker/nginx/
├── nginx.conf              # Main nginx configuration
├── conf.d/
│   ├── api.conf           # Production HTTPS configuration
│   └── api-dev.conf       # Development HTTP/HTTPS configuration
└── ssl/
    ├── generate-self-signed.sh
    ├── self-signed.crt    # Development certificate
    └── self-signed.key    # Development private key
```

### TLS Configuration

**Protocols**:
- TLS 1.2 (minimum)
- TLS 1.3 (preferred)
- TLS 1.0/1.1 disabled (deprecated)

**Cipher Suites** (Mozilla Intermediate):
```nginx
ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384';
```

**Session Configuration**:
```nginx
ssl_session_timeout 1d;
ssl_session_cache shared:SSL:50m;
ssl_session_tickets off;  # Forward secrecy
```

**OCSP Stapling**:
```nginx
ssl_stapling on;
ssl_stapling_verify on;
```

### Security Headers

**Production (HTTPS)**:
```nginx
# HSTS: Force HTTPS for 1 year
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload

# CSP: Restrictive policy for API
Content-Security-Policy: default-src 'none'; frame-ancestors 'none'

# Frame protection
X-Frame-Options: DENY

# MIME type sniffing protection
X-Content-Type-Options: nosniff

# Permissions policy
Permissions-Policy: geolocation=(), microphone=(), camera=()
```

**Development (HTTP/HTTPS)**:
- No HSTS (allows HTTP access)
- Same other headers for testing

### Rate Limiting

**Production**:
- General API: 100 requests/second per IP
- Auth endpoints: 5 requests/minute per IP

**Development**:
- General API: 1000 requests/second per IP
- Auth endpoints: 50 requests/minute per IP

## Testing

### Automated Test Suite

Run comprehensive HTTPS tests:

```bash
# Test development (localhost)
./scripts/test-https.sh localhost

# Test production
./scripts/test-https.sh api.agentauri.ai
```

**Tests Performed**:
1. HTTP to HTTPS redirect (production only)
2. HTTPS endpoint accessibility
3. SSL certificate validity
4. TLS protocol versions (1.0, 1.1, 1.2, 1.3)
5. Security headers (HSTS, CSP, X-Frame-Options, etc.)
6. Cipher suite strength
7. HTTP/2 support
8. API endpoints over HTTPS

### Manual Testing

**Test Certificate**:
```bash
# View certificate details
openssl s_client -servername api.agentauri.ai -connect api.agentauri.ai:443 </dev/null | openssl x509 -noout -text

# Check certificate expiration
echo | openssl s_client -servername api.agentauri.ai -connect api.agentauri.ai:443 2>/dev/null | openssl x509 -noout -dates
```

**Test TLS Version**:
```bash
# Test TLS 1.3
openssl s_client -tls1_3 -connect api.agentauri.ai:443

# Test TLS 1.2
openssl s_client -tls1_2 -connect api.agentauri.ai:443

# Verify TLS 1.1 rejected
openssl s_client -tls1_1 -connect api.agentauri.ai:443
```

**Test Security Headers**:
```bash
# Check all headers
curl -I https://api.agentauri.ai/health

# Check specific header
curl -I https://api.agentauri.ai/health | grep -i "strict-transport-security"
```

**Test Redirect**:
```bash
# HTTP should redirect to HTTPS (production)
curl -I http://api.agentauri.ai/health
```

### Online SSL Testing

**SSL Labs Test** (comprehensive):
```
https://www.ssllabs.com/ssltest/analyze.html?d=api.agentauri.ai
```

**Expected Grade**: A or A+

**SecurityHeaders.com**:
```
https://securityheaders.com/?q=https://api.agentauri.ai
```

**Expected Grade**: A or A+

## Troubleshooting

### Certificate Issues

**Problem**: Certificate not found
```bash
# Check certificate files exist
ls -la docker/certbot/conf/live/api.agentauri.ai/

# Regenerate if missing
./scripts/init-letsencrypt.sh
```

**Problem**: Certificate expired
```bash
# Manual renewal
docker compose run --rm certbot renew --force-renewal

# Reload nginx
docker compose exec nginx nginx -s reload
```

**Problem**: Self-signed certificate error (development)
```bash
# Regenerate self-signed certificate
cd docker/nginx/ssl
./generate-self-signed.sh

# Restart nginx
docker compose --profile development restart nginx
```

### Nginx Issues

**Problem**: Nginx fails to start
```bash
# Check nginx configuration syntax
docker compose exec nginx nginx -t

# Check logs
docker compose logs nginx

# Common issues:
# - Certificate files not found (check paths)
# - Port 80/443 already in use (stop conflicting service)
# - Invalid nginx syntax (review conf files)
```

**Problem**: "address already in use" error
```bash
# Find process using port 80
sudo lsof -i :80

# Find process using port 443
sudo lsof -i :443

# Stop conflicting service (example)
sudo systemctl stop apache2
```

### Let's Encrypt Issues

**Problem**: Rate limit exceeded
```
Error: too many certificates already issued for: api.agentauri.ai
```

**Solution**:
- Wait 7 days for rate limit reset
- Use production environment for testing: `./scripts/init-letsencrypt.sh --production`
- Let's Encrypt limits: 5 certificates per domain per week

**Problem**: ACME challenge fails
```bash
# Ensure port 80 is accessible
curl http://api.agentauri.ai/.well-known/acme-challenge/test

# Check nginx is serving ACME challenge
docker compose logs nginx | grep acme-challenge

# Verify DNS points to server
dig +short api.agentauri.ai
```

**Problem**: Domain not accessible
```bash
# Check DNS resolution
nslookup api.agentauri.ai

# Test from different location
curl -I http://api.agentauri.ai/

# Check firewall rules
sudo ufw status
```

### Connection Issues

**Problem**: SSL handshake errors
```bash
# Test with openssl
openssl s_client -connect api.agentauri.ai:443 -servername api.agentauri.ai

# Check for:
# - Certificate chain issues
# - Cipher mismatch
# - Protocol version incompatibility
```

**Problem**: Mixed content warnings (browser)
```bash
# Ensure all API requests use HTTPS
# Check browser console for mixed content errors
# Update API base URL to https://
```

## Security Best Practices

### Certificate Management

1. **Use Let's Encrypt in production** (never self-signed)
2. **Monitor certificate expiration** (automated alerts recommended)
3. **Use strong key size** (2048-bit minimum, 4096-bit recommended)
4. **Enable OCSP stapling** (improves performance and privacy)
5. **Implement certificate pinning** (optional, for mobile apps)

### TLS Configuration

1. **Disable old protocols** (TLS 1.0/1.1 deprecated)
2. **Use strong cipher suites** (AEAD ciphers preferred)
3. **Enable forward secrecy** (ECDHE/DHE key exchange)
4. **Disable session tickets** (for forward secrecy)
5. **Regular updates** (follow Mozilla SSL Configuration Generator)

### Security Headers

1. **HSTS with preload** (prevents downgrade attacks)
2. **CSP header** (mitigates XSS attacks)
3. **X-Frame-Options** (prevents clickjacking)
4. **X-Content-Type-Options** (prevents MIME sniffing)
5. **Referrer-Policy** (controls referrer information)

### Monitoring

1. **Certificate expiration alerts** (30 days before expiry)
2. **SSL Labs monitoring** (weekly scans)
3. **Log analysis** (failed handshakes, protocol errors)
4. **Rate limit monitoring** (detect abuse)
5. **HSTS preload status** (if using preload)

### Regular Maintenance

1. **Update TLS parameters** (quarterly review)
2. **Review cipher suites** (remove deprecated)
3. **Test certificate renewal** (monthly)
4. **Update nginx** (apply security patches)
5. **Review security headers** (follow best practices)

## References

- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [OWASP TLS Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)
- [SSL Labs Best Practices](https://github.com/ssllabs/research/wiki/SSL-and-TLS-Deployment-Best-Practices)
- [HSTS Preload](https://hstspreload.org/)
- [SecurityHeaders.com](https://securityheaders.com/)

---

**Last Updated**: December 6, 2025
**Version**: 1.0.0
