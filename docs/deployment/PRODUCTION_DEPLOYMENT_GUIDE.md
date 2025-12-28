# Production Deployment Guide - api.agentauri.ai

> **Note**: As of December 2025, api.agentauri.ai is deployed on **AWS ECS Fargate** in `us-east-1`.
> See `terraform/` for infrastructure-as-code and `docs/deployment/AWS_DEPLOYMENT.md` for ECS-specific
> deployment procedures. This guide is maintained for reference and manual deployments.

This guide provides step-by-step instructions for deploying the api.agentauri.ai API Gateway to production.

## Prerequisites

- Linux server (Ubuntu 22.04 LTS recommended)
- Docker and Docker Compose installed
- Nginx installed
- Domain with SSL certificate (Let's Encrypt recommended)
- PostgreSQL 15+ with TimescaleDB extension
- Redis 7+

## Environment Variables (REQUIRED)

All sensitive configuration must be provided via environment variables. **Never commit secrets to git.**

### Required Variables

```bash
# JWT Configuration (CRITICAL - MUST BE STRONG)
JWT_SECRET="<generate with: openssl rand -base64 32>"
# Must be at least 32 characters, use cryptographically secure random

# Database Configuration
DB_USER="postgres"
DB_PASSWORD="<strong-random-password>"
DB_NAME="agentauri_backend"
DB_HOST="localhost"
DB_PORT="5432"

# Redis Configuration
REDIS_PASSWORD="<strong-random-password>"

# CORS Configuration (Production)
ALLOWED_ORIGINS="https://app.yourdomain.com,https://www.yourdomain.com"
# Comma-separated list of allowed frontend origins

# Server Configuration
SERVER_HOST="0.0.0.0"
SERVER_PORT="8080"
```

### Generating Strong Secrets

```bash
# Generate JWT_SECRET
openssl rand -base64 32

# Generate database password
openssl rand -base64 24

# Generate Redis password
openssl rand -base64 24
```

## Infrastructure Setup

### 1. PostgreSQL with TimescaleDB

```bash
# Install PostgreSQL 15
sudo apt update
sudo apt install postgresql-15 postgresql-contrib-15

# Install TimescaleDB
sudo add-apt-repository ppa:timescale/timescaledb-ppa
sudo apt update
sudo apt install timescaledb-2-postgresql-15

# Configure TimescaleDB
sudo timescaledb-tune --quiet --yes

# Restart PostgreSQL
sudo systemctl restart postgresql
```

### 2. Redis

```bash
# Install Redis
sudo apt install redis-server

# Configure Redis password
sudo nano /etc/redis/redis.conf
# Add: requirepass <your-redis-password>

# Restart Redis
sudo systemctl restart redis-server
```

### 3. Nginx with Rate Limiting

Create `/etc/nginx/conf.d/api.agentauri.ai.conf`:

```nginx
# Rate limiting zones
limit_req_zone $binary_remote_addr zone=auth_limit:10m rate=3r/m;
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;

# Upstream API Gateway
upstream api_gateway {
    server 127.0.0.1:8080;
}

server {
    listen 443 ssl http2;
    server_name api.yourdomain.com;

    # SSL Configuration (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/api.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.yourdomain.com/privkey.pem;

    # Security Headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Request size limit (defense in depth - app also limits to 1MB)
    client_max_body_size 1M;

    # Authentication endpoints (strict rate limiting)
    location /api/v1/auth {
        limit_req zone=auth_limit burst=5 nodelay;
        proxy_pass http://api_gateway;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # General API endpoints
    location /api/v1 {
        limit_req zone=api_limit burst=20 nodelay;
        proxy_pass http://api_gateway;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Health check (no rate limiting for load balancer)
    location /api/v1/health {
        proxy_pass http://api_gateway;
        access_log off;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name api.yourdomain.com;
    return 301 https://$server_name$request_uri;
}
```

Reload nginx:
```bash
sudo nginx -t
sudo systemctl reload nginx
```

## Application Deployment

### 1. Build Release Binary

```bash
cd /path/to/api.agentauri.ai/rust-backend
cargo build --release --package api-gateway
```

Binary location: `target/release/api-gateway`

### 2. Database Migrations

```bash
# Run all migrations
for migration in /path/to/api.agentauri.ai/database/migrations/*.sql; do
    echo "Running: $(basename $migration)"
    PGPASSWORD="$DB_PASSWORD" psql -h localhost -U postgres -d agentauri_backend -f "$migration"
done
```

### 3. Systemd Service

Create `/etc/systemd/system/api-gateway.service`:

```ini
[Unit]
Description=API Gateway for api.agentauri.ai
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=apigateway
WorkingDirectory=/opt/api.agentauri.ai
ExecStart=/opt/api.agentauri.ai/api-gateway

# Environment variables
Environment="JWT_SECRET=<your-jwt-secret>"
Environment="DB_PASSWORD=<your-db-password>"
Environment="REDIS_PASSWORD=<your-redis-password>"
Environment="ALLOWED_ORIGINS=https://app.yourdomain.com"
Environment="SERVER_HOST=127.0.0.1"
Environment="SERVER_PORT=8080"
Environment="RUST_LOG=info"

# Security hardening
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/api.agentauri.ai/logs

# Restart policy
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable api-gateway
sudo systemctl start api-gateway
sudo systemctl status api-gateway
```

## Health Checks

### Liveness Probe
```bash
curl http://localhost:8080/api/v1/health
```

Expected response:
```json
{
  "status": "healthy",
  "timestamp": "2025-11-24T...",
  "database": "connected"
}
```

### Readiness Probe
Same as liveness probe - the health endpoint checks database connectivity.

## Security Checklist

Before going live, verify:

- [ ] JWT_SECRET is set (not using default)
- [ ] JWT_SECRET is at least 32 characters
- [ ] DB_PASSWORD is strong (not default)
- [ ] REDIS_PASSWORD is set
- [ ] ALLOWED_ORIGINS configured (no wildcard)
- [ ] Nginx rate limiting configured
- [ ] SSL certificate valid and not expired
- [ ] Firewall configured (only ports 80, 443 open)
- [ ] Database backups configured
- [ ] Log rotation configured
- [ ] Monitoring/alerting configured (Phase 3)

## Monitoring

### Basic Logging

Logs location: `/var/log/api-gateway/`

View logs:
```bash
sudo journalctl -u api-gateway -f
```

### Database Monitoring

```bash
# Active connections
PGPASSWORD="$DB_PASSWORD" psql -h localhost -U postgres -d agentauri_backend \
    -c "SELECT count(*) FROM pg_stat_activity;"

# Table sizes
PGPASSWORD="$DB_PASSWORD" psql -h localhost -U postgres -d agentauri_backend \
    -c "SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) FROM pg_tables WHERE schemaname = 'public' ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;"
```

## Post-Deployment Verification

### 1. Test Registration
```bash
curl -X POST https://api.yourdomain.com/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","email":"test@example.com","password":"testpass123"}'
```

### 2. Test Login
```bash
curl -X POST https://api.yourdomain.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","password":"testpass123"}'
```

Save the JWT token from response.

### 3. Test Protected Endpoint
```bash
curl https://api.yourdomain.com/api/v1/triggers \
  -H "Authorization: Bearer <your-jwt-token>"
```

### 4. Test Rate Limiting
```bash
# Send 10 rapid login attempts (should be rate limited after 3)
for i in {1..10}; do
    curl -X POST https://api.yourdomain.com/api/v1/auth/login \
      -H "Content-Type: application/json" \
      -d '{"username":"test","password":"wrong"}' &
done
wait
```

Should see 429 Too Many Requests after hitting rate limit.

### 5. Test Payload Size Limit
```bash
# Try to send 2MB payload (should be rejected)
python3 -c "import requests; requests.post('https://api.yourdomain.com/api/v1/triggers', headers={'Authorization': 'Bearer <token>', 'Content-Type': 'application/json'}, json={'name': 'A'*2000000})"
```

Should return 413 Payload Too Large.

## Troubleshooting

### Issue: API Gateway won't start

**Check logs**:
```bash
sudo journalctl -u api-gateway -n 50
```

**Common causes**:
- JWT_SECRET not set (should see panic message)
- Database connection failed (check DB_PASSWORD, DB_HOST)
- Port already in use (check with `lsof -i :8080`)

### Issue: 401 Unauthorized on all requests

**Possible causes**:
- JWT_SECRET mismatch between services
- Token expired (check token exp claim with jwt.io)
- CORS issue (check ALLOWED_ORIGINS)

### Issue: Rate limiting too aggressive

**Solution**: Adjust nginx config rate limits in `/etc/nginx/conf.d/api.agentauri.ai.conf`

## Backup and Recovery

### Database Backup

```bash
# Daily backup script
#!/bin/bash
BACKUP_DIR="/var/backups/postgresql"
DATE=$(date +%Y%m%d_%H%M%S)
PGPASSWORD="$DB_PASSWORD" pg_dump -h localhost -U postgres agentauri_backend | gzip > "$BACKUP_DIR/agentauri_backend_$DATE.sql.gz"
find "$BACKUP_DIR" -type f -mtime +7 -delete  # Keep 7 days
```

Add to crontab:
```bash
0 2 * * * /path/to/backup-script.sh
```

### Database Restore

```bash
gunzip < backup.sql.gz | PGPASSWORD="$DB_PASSWORD" psql -h localhost -U postgres agentauri_backend
```

## Phase 3 Enhancements (Post-MVP)

Items deferred to Phase 3 for production hardening:

1. **Application-Level Rate Limiting** (4-6 hours)
   - Implement actix-governor for per-user rate limiting
   - Store rate limit state in Redis
   - Fine-grained limits per endpoint

2. **Token Refresh Pattern** (3-4 hours)
   - Add /api/v1/auth/refresh endpoint
   - Store refresh tokens in database
   - Implement token revocation

3. **Enhanced Password Validation** (1 hour)
   - Complexity requirements (uppercase, lowercase, number, special char)
   - Password strength meter in frontend

4. **Comprehensive Monitoring** (8 hours)
   - Prometheus metrics endpoint
   - Grafana dashboards
   - Alert rules for critical issues

5. **Request Correlation IDs** (2 hours)
   - X-Request-ID header middleware
   - Include in all logs
   - Return in response headers

---

**Document Version**: 1.0
**Last Updated**: November 24, 2025
**Status**: Production-Ready
