# Security Audit Checklist

Last Updated: 2025-12-30
Phase: 6 - Security Hardening

## Overview

This checklist documents the security controls implemented in AgentAuri's backend infrastructure and serves as a reference for internal reviews and external audits.

---

## 1. Encryption

### 1.1 Encryption at Rest

| Control | Status | Evidence |
|---------|--------|----------|
| RDS storage encryption | ✅ Enabled | `terraform/rds.tf` - `storage_encrypted = true` |
| KMS key management | ✅ AWS-managed | `aws/kms` default key |
| S3 bucket encryption | ✅ SSE-S3 | Default encryption policy |
| EBS volume encryption | ✅ Enabled | ECS Fargate managed |

### 1.2 Encryption in Transit

| Control | Status | Evidence |
|---------|--------|----------|
| TLS for RDS connections | ✅ Enforced | `rds.force_ssl = 1` parameter |
| Application uses SSL | ✅ Yes | `sslmode=verify-full` in connection strings |
| ALB HTTPS only | ✅ Enabled | `terraform/alb.tf` - HTTPS listener |
| Certificate management | ✅ ACM | AWS Certificate Manager |

---

## 2. Secret Management

### 2.1 Secrets Storage

| Secret | Location | Rotation |
|--------|----------|----------|
| RDS password | AWS Secrets Manager | 30 days (auto) |
| JWT signing secret | AWS Secrets Manager | 30 days (auto) |
| API key salt | AWS Secrets Manager | 90 days (auto) |
| OAuth state key | AWS Secrets Manager | 30 days (auto) |
| OAuth client secrets | AWS Secrets Manager | Manual |
| Telegram bot token | AWS Secrets Manager | Manual |

### 2.2 Secret Rotation

| Control | Status | Evidence |
|---------|--------|----------|
| Automatic rotation enabled | ✅ Yes | `terraform/secret_rotation.tf` |
| RDS rotation Lambda | ✅ AWS SAR | PostgreSQL single-user rotation |
| App secrets rotation Lambda | ✅ Custom | Python 3.11 Lambda |
| Rotation audit logging | ✅ CloudTrail | AWS-native |

### 2.3 Secret Access Controls

| Control | Status | Evidence |
|---------|--------|----------|
| IAM least privilege | ✅ Yes | Per-service IAM roles |
| ECS task roles | ✅ Scoped | Only required secrets per service |
| No hardcoded secrets | ✅ Verified | Environment variables from Secrets Manager |

---

## 3. Authentication & Authorization

### 3.1 API Authentication

| Method | Implementation | Notes |
|--------|----------------|-------|
| JWT tokens | HS256 signed | 1-hour expiry |
| API keys | Argon2id hashed | Prefix-based (`sk_live_`, `sk_test_`) |
| OAuth 2.0 | Google, GitHub | PKCE flow |
| Wallet signatures | EIP-191 | Layer 2 auth |

### 3.2 Authorization

| Control | Status | Evidence |
|---------|--------|----------|
| Organization-based access | ✅ Yes | `organization_members` table |
| Role-based permissions | ✅ Yes | Owner, Admin, Member, Viewer |
| API key scoping | ✅ Yes | Per-organization keys |
| Resource ownership validation | ✅ Yes | Handler-level checks |

---

## 4. Network Security

### 4.1 VPC Architecture

| Control | Status | Evidence |
|---------|--------|----------|
| Private subnets for RDS | ✅ Yes | `terraform/network.tf` |
| Private subnets for ECS | ✅ Yes | Fargate in private subnets |
| NAT Gateway for outbound | ✅ Yes | Internet access for services |
| No public RDS access | ✅ Yes | `publicly_accessible = false` |

### 4.2 Security Groups

| Resource | Inbound | Outbound |
|----------|---------|----------|
| ALB | 443 (HTTPS) | ECS (8080) |
| ECS Services | ALB only | RDS, Redis, Internet |
| RDS | ECS only | None |
| Redis | ECS only | None |
| Lambda Rotation | None | RDS (5432), HTTPS (443) |

### 4.3 Rate Limiting

| Layer | Implementation | Limits |
|-------|----------------|--------|
| IP-based | Redis sliding window | 100 req/min (anonymous) |
| API key | Redis sliding window | 1000 req/min |
| Authenticated | Redis sliding window | 500 req/min |

---

## 5. Application Security

### 5.1 Input Validation

| Control | Status | Evidence |
|---------|--------|----------|
| SQL injection prevention | ✅ Yes | SQLx parameterized queries |
| XSS prevention | ✅ N/A | API-only, no HTML rendering |
| Request size limits | ✅ Yes | Actix-web body limits |
| UUID validation | ✅ Yes | Rust type system |

### 5.2 Error Handling

| Control | Status | Evidence |
|---------|--------|----------|
| No stack traces in responses | ✅ Yes | Production error handling |
| Structured error codes | ✅ Yes | `docs/api/ERROR_CODES.md` |
| Sensitive data redaction | ✅ Yes | Secrets not logged |

### 5.3 Dependencies

| Control | Status | Evidence |
|---------|--------|----------|
| Rust security advisories | ✅ Monitored | `cargo audit` |
| Node.js security | ✅ Monitored | `pnpm audit` |
| Dependency updates | ⚠️ Manual | Renovate recommended |

---

## 6. Logging & Monitoring

### 6.1 Audit Logging

| Event | Logged | Location |
|-------|--------|----------|
| Authentication attempts | ✅ Yes | CloudWatch Logs |
| API key creation/revocation | ✅ Yes | `api_key_audit_log` table |
| Organization changes | ✅ Yes | Application logs |
| Trigger modifications | ✅ Yes | Application logs |

### 6.2 Security Monitoring

| Control | Status | Evidence |
|---------|--------|----------|
| CloudWatch Logs | ✅ Enabled | All ECS services |
| CloudTrail | ✅ Enabled | AWS API calls |
| RDS Performance Insights | ✅ Enabled | Query monitoring |
| Alerting | ⚠️ Basic | CloudWatch Alarms needed |

---

## 7. Disaster Recovery

### 7.1 Backup Strategy

| Resource | Backup | Retention |
|----------|--------|-----------|
| RDS | Automated snapshots | 7 days |
| Redis | AOF persistence | N/A (cache) |
| Secrets | Version history | 30 days |

### 7.2 Recovery Procedures

| Scenario | RTO | RPO | Documented |
|----------|-----|-----|------------|
| RDS failure | < 15 min | 5 min | ✅ Yes |
| Region failure | 4 hours | 1 hour | ⚠️ Partial |
| Secret compromise | < 30 min | N/A | ✅ Yes |

---

## 8. Compliance Considerations

### 8.1 Data Classification

| Data Type | Classification | Handling |
|-----------|----------------|----------|
| User credentials | Sensitive | Hashed (Argon2id) |
| API keys | Secret | Hashed, prefix visible |
| Wallet addresses | Public | Stored plaintext |
| Trigger configurations | Internal | User-owned |

### 8.2 Data Retention

| Data | Retention | Deletion |
|------|-----------|----------|
| User accounts | Until deleted | Hard delete |
| API keys | Until revoked | Soft delete |
| Audit logs | 90 days | Auto-purge |
| Events | 30 days | TimescaleDB compression |

---

## 9. Pre-Audit Action Items

### High Priority
- [ ] Enable CloudWatch Alarms for failed auth attempts
- [ ] Configure AWS WAF for ALB
- [ ] Set up GuardDuty for threat detection
- [ ] Implement dependency update automation

### Medium Priority
- [ ] Add request signing for webhooks
- [ ] Implement API key IP restrictions
- [ ] Enable cross-region backup replication
- [ ] Document incident response procedures

### Low Priority
- [ ] Consider mTLS for internal services
- [ ] Evaluate AWS Macie for data discovery
- [ ] Add security headers (CSP, HSTS) if web UI added

---

## 10. External Audit Scope

### Recommended Audit Areas
1. **Authentication/Authorization** - JWT, API keys, OAuth flows
2. **Secret Management** - Rotation, access controls, logging
3. **Network Architecture** - VPC, security groups, traffic flow
4. **API Security** - Input validation, error handling, rate limiting
5. **Infrastructure as Code** - Terraform security best practices
6. **Blockchain Integration** - Signature verification, event handling

### Out of Scope
- Frontend applications (API-only backend)
- Third-party service security (Google, GitHub OAuth)
- AWS infrastructure security (shared responsibility)

---

## Revision History

| Date | Author | Changes |
|------|--------|---------|
| 2025-12-30 | Claude | Initial checklist creation |
