# HTTPS/TLS Quick Reference

Quick commands and configurations for managing HTTPS/TLS on api.agentauri.ai.

## Quick Start

```bash
# 1. Configure environment
cp .env.example .env
# Edit .env: Set DOMAIN, LETSENCRYPT_EMAIL, ENABLE_HTTPS=true

# 2. Initialize Let's Encrypt
./scripts/init-letsencrypt.sh

# 3. Start production services
docker compose --profile production up -d

# 4. Test HTTPS
./scripts/test-https.sh
```

## Common Commands

### Certificate Management

```bash
# Check certificate expiry
openssl s_client -connect api.agentauri.ai:443 -servername api.agentauri.ai < /dev/null 2>&1 | openssl x509 -noout -enddate

# Force certificate renewal
docker compose --profile production run --rm certbot renew --force-renewal
docker compose --profile production exec nginx nginx -s reload

# Test renewal (dry-run)
docker compose --profile production run --rm certbot renew --dry-run

# View certificate details
openssl s_client -connect api.agentauri.ai:443 -servername api.agentauri.ai < /dev/null 2>&1 | openssl x509 -noout -text

# Revoke compromised certificate
docker compose --profile production run --rm certbot revoke --cert-path /etc/letsencrypt/live/api.agentauri.ai/fullchain.pem
```

### Nginx Operations

```bash
# Test configuration syntax
docker compose --profile production run --rm nginx nginx -t

# Reload configuration (no downtime)
docker compose --profile production exec nginx nginx -s reload

# View full configuration
docker compose --profile production exec nginx nginx -T

# Check nginx logs
docker compose --profile production logs nginx --tail=100 -f

# Restart nginx
docker compose --profile production restart nginx

# Stop nginx
docker compose --profile production stop nginx

# Start nginx
docker compose --profile production start nginx
```

### Service Management

```bash
# Start all services (development, no HTTPS)
docker compose up -d

# Start all services (production, with HTTPS)
docker compose --profile production up -d

# Stop all services
docker compose --profile production down

# View service status
docker compose --profile production ps

# View all logs
docker compose --profile production logs -f

# View specific service logs
docker compose --profile production logs nginx -f
docker compose --profile production logs certbot -f
```

### Testing & Validation

```bash
# Run comprehensive HTTPS tests
./scripts/test-https.sh

# Monitor SSL/TLS health
./scripts/monitor-ssl.sh api.agentauri.ai admin@agentauri.ai

# Test HTTP → HTTPS redirect
curl -I http://api.agentauri.ai/

# Test HTTPS connection
curl -v https://api.agentauri.ai/health

# Test security headers
curl -I https://api.agentauri.ai/ | grep -i "strict-transport-security"

# Test TLS version
openssl s_client -connect api.agentauri.ai:443 -tls1_2 < /dev/null

# Check SSL Labs rating (browser)
# https://www.ssllabs.com/ssltest/analyze.html?d=api.agentauri.ai
```

## Configuration Files

### Environment Variables (.env)

```bash
# HTTPS Configuration
DOMAIN=api.agentauri.ai
LETSENCRYPT_EMAIL=admin@agentauri.ai
ENABLE_HTTPS=true
BASE_URL=https://api.agentauri.ai
```

### Nginx Configuration

**Main config**: `/docker/nginx/nginx.conf`
- Worker settings
- Global security headers
- Rate limiting zones
- Gzip compression

**Site config**: `/docker/nginx/conf.d/api.conf`
- HTTP server (port 80): Redirects to HTTPS
- HTTPS server (port 443): TLS termination
- Rate limiting per endpoint
- Proxy configuration

**Edit site config**:
```bash
vim docker/nginx/conf.d/api.conf

# Test configuration
docker compose --profile production run --rm nginx nginx -t

# Apply changes
docker compose --profile production exec nginx nginx -s reload
```

## Troubleshooting Quick Fixes

### Certificate Request Fails

```bash
# Check DNS
host api.agentauri.ai

# Check port 80 accessibility
curl http://<SERVER_IP>/.well-known/acme-challenge/test

# Check firewall
sudo ufw status
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Use staging environment (testing)
./scripts/init-letsencrypt.sh --staging

# Check certbot logs
docker compose --profile production logs certbot
```

### Nginx Won't Start

```bash
# Check ports
sudo lsof -i :80
sudo lsof -i :443

# Check certificate files
ls -la docker/certbot/conf/live/api.agentauri.ai/

# Test configuration
docker compose --profile production run --rm nginx nginx -t

# Check nginx logs
docker compose --profile production logs nginx
```

### HSTS Errors in Browser

```bash
# Clear HSTS cache (Chrome)
# 1. Go to: chrome://net-internals/#hsts
# 2. Delete domain security policies for api.agentauri.ai
# 3. Clear browser cache

# Or use incognito/private mode
```

### Certificate Renewal Fails

```bash
# Test renewal
docker compose --profile production run --rm certbot renew --dry-run

# Check ACME challenge
curl http://api.agentauri.ai/.well-known/acme-challenge/test
# Should return: 404 (not 301 redirect)

# Force renewal
docker compose --profile production run --rm certbot renew --force-renewal

# Check certbot logs
docker compose --profile production logs certbot --tail=50
```

## Security Checklist

- [ ] Domain DNS configured (A record)
- [ ] Ports 80 and 443 open in firewall
- [ ] Valid email for Let's Encrypt notifications
- [ ] Certificate issued successfully
- [ ] HTTP → HTTPS redirect (301)
- [ ] HSTS header present (1-year max-age)
- [ ] All security headers present
- [ ] TLS 1.2+ enabled (1.0/1.1 disabled)
- [ ] Strong cipher suites only
- [ ] OCSP stapling enabled
- [ ] SSL Labs rating A or A+
- [ ] Certificate auto-renewal working
- [ ] Monitoring/alerts configured

## Monitoring Setup

### Certificate Expiry Cron Job

Add to crontab (`crontab -e`):

```bash
# Certificate renewal (daily)
0 0 * * * docker compose -f /path/to/api.agentauri.ai/docker-compose.yml --profile production run --rm certbot renew --quiet

# Certificate expiry alert (<30 days)
0 0 * * * /path/to/api.agentauri.ai/scripts/monitor-ssl.sh api.agentauri.ai admin@agentauri.ai

# SSL Labs rating check (weekly, Monday at 2 AM)
0 2 * * 1 curl -s "https://api.ssllabs.com/api/v3/analyze?host=api.agentauri.ai" | jq -r '.endpoints[0].grade' | grep -q 'A' || echo "SSL Labs rating below A" | mail -s "SSL Rating Alert" admin@agentauri.ai
```

### Log Monitoring

```bash
# Watch nginx error logs
docker compose --profile production logs nginx -f | grep -i error

# Watch access logs
docker compose --profile production logs nginx -f | grep -v health

# Count requests by endpoint
docker compose --profile production logs nginx | grep "GET /api/" | awk '{print $7}' | sort | uniq -c | sort -rn
```

## Performance Tuning

### Increase Rate Limits

Edit `docker/nginx/nginx.conf`:
```nginx
# Increase API rate limit from 100 to 200 req/s
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=200r/s;
```

### Adjust Worker Connections

Edit `docker/nginx/nginx.conf`:
```nginx
events {
    worker_connections 2048;  # Increase from 1024
}
```

### Enable HTTP/2 Server Push

Edit `docker/nginx/conf.d/api.conf`:
```nginx
location /api/ {
    http2_push_preload on;
    # ...
}
```

### Add Response Caching

```nginx
# Add to http block
proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=api_cache:10m max_size=100m;

# Add to location block
proxy_cache api_cache;
proxy_cache_valid 200 5m;
proxy_cache_key "$scheme$request_method$host$request_uri";
```

## Emergency Rollback

```bash
# 1. Stop nginx and certbot
docker compose --profile production stop nginx certbot

# 2. Update .env
echo "ENABLE_HTTPS=false" >> .env

# 3. Restart without HTTPS
docker compose up -d

# 4. (Optional) Restore from backup
gpg --decrypt cert-backup-YYYYMMDD.tar.gz.gpg | tar -xzf -
```

## External Resources

- **SSL Labs**: https://www.ssllabs.com/ssltest/
- **SecurityHeaders**: https://securityheaders.com/
- **Mozilla Observatory**: https://observatory.mozilla.org/
- **Let's Encrypt Status**: https://letsencrypt.status.io/
- **Nginx SSL Module**: https://nginx.org/en/docs/http/ngx_http_ssl_module.html

## Support

For detailed documentation, see:
- `docs/deployment/HTTPS_SETUP.md` - Complete setup guide
- `docker/nginx/nginx.conf` - Main nginx configuration
- `docker/nginx/conf.d/api.conf` - Site-specific configuration
- `scripts/init-letsencrypt.sh` - Certificate initialization script
- `scripts/test-https.sh` - Comprehensive test suite
- `scripts/monitor-ssl.sh` - Certificate monitoring script

---

**Last Updated**: January 30, 2025
**Version**: 1.0.0
