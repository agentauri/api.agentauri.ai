# Disaster Recovery Procedures

**Project**: api.agentauri.ai
**Last Updated**: 2025-12-28
**Environment**: AWS ECS (us-east-1)

This document outlines disaster recovery procedures for the api.agentauri.ai production infrastructure.

---

## Table of Contents

- [Infrastructure Overview](#infrastructure-overview)
- [Recovery Time Objectives](#recovery-time-objectives)
- [Backup Configuration](#backup-configuration)
- [Disaster Scenarios](#disaster-scenarios)
- [Recovery Procedures](#recovery-procedures)
- [Verification Checklist](#verification-checklist)
- [Contacts](#contacts)

---

## Infrastructure Overview

### Production Components

| Component | AWS Service | Region | Backup Strategy |
|-----------|-------------|--------|-----------------|
| Database | RDS PostgreSQL 15 | us-east-1 | Automated snapshots |
| Cache | ElastiCache Redis | us-east-1 | No persistence (cache-only) |
| Application | ECS Fargate | us-east-1 | Stateless (ECR images) |
| Load Balancer | Application LB | us-east-1 | Stateless |
| Secrets | Secrets Manager | us-east-1 | Versioned, cross-region replication |
| Container Images | ECR | us-east-1 | Immutable image tags |
| Infrastructure | Terraform | Git | terraform/ directory |

### Data Classification

| Data Type | Location | Criticality | Backup? |
|-----------|----------|-------------|---------|
| User data | RDS | Critical | Yes |
| Triggers/Actions | RDS | Critical | Yes |
| Events (TimescaleDB) | RDS | Important | Yes |
| Session cache | Redis | Low | No |
| Rate limit counters | Redis | Low | No |
| Container images | ECR | Critical | Yes (immutable) |
| Secrets | Secrets Manager | Critical | Yes (versioned) |

---

## Recovery Time Objectives

| Metric | Target | Current Capability |
|--------|--------|-------------------|
| RTO (Recovery Time Objective) | < 1 hour | ~30 minutes |
| RPO (Recovery Point Objective) | < 24 hours | 24 hours (daily backup) |
| MTTR (Mean Time To Recover) | < 2 hours | Depends on scenario |

---

## Backup Configuration

### RDS PostgreSQL

```hcl
# Current configuration (terraform/rds.tf)
backup_retention_period   = 1      # 1 day retention
backup_window             = "03:00-04:00"  # UTC
maintenance_window        = "Mon:04:00-Mon:05:00"
copy_tags_to_snapshot     = true
deletion_protection       = true   # Production only
```

**Recommendation**: Increase `backup_retention_period` to 7-30 days for production.

### Manual Snapshot

Create on-demand snapshot before major changes:

```bash
aws rds create-db-snapshot \
  --db-instance-identifier agentauri-staging \
  --db-snapshot-identifier agentauri-pre-migration-$(date +%Y%m%d)
```

### ECR Images

All deployed images use immutable tags:
- Format: `{sha}` or `{version}`
- Location: `{account}.dkr.ecr.us-east-1.amazonaws.com/agentauri-staging-{service}`

---

## Disaster Scenarios

### Scenario 1: Database Corruption/Loss

**Symptoms**:
- 5xx errors from API
- "Connection refused" or "relation does not exist" errors
- Data inconsistencies reported by users

**Recovery**:
1. Identify last known good snapshot
2. Restore from RDS snapshot (see procedure below)
3. Update ECS services to use new endpoint (if changed)
4. Verify data integrity

### Scenario 2: ECS Service Failure

**Symptoms**:
- Health checks failing
- 502/503 errors from ALB
- No tasks running in ECS cluster

**Recovery**:
1. Check CloudWatch logs for error cause
2. Rollback to previous task definition (see Rollback Procedures)
3. Force new deployment if needed

### Scenario 3: Region-Wide Outage

**Symptoms**:
- All AWS services in us-east-1 unavailable
- No connectivity to any resources

**Recovery**:
1. Wait for AWS to restore services (most common)
2. If prolonged (>4 hours): Initiate cross-region recovery
3. Deploy from Terraform to backup region

### Scenario 4: Secrets Compromise

**Symptoms**:
- Unauthorized access detected
- API keys/tokens used from unknown sources

**Recovery**:
1. Rotate all secrets immediately
2. Revoke all existing API keys
3. Invalidate all JWT tokens (change JWT_SECRET)
4. Audit access logs

### Scenario 5: Accidental Data Deletion

**Symptoms**:
- Missing triggers, users, or organizations
- User reports data loss

**Recovery**:
1. Identify time of deletion
2. Restore from point-in-time recovery (if within retention)
3. Or restore from snapshot

---

## Recovery Procedures

### Procedure 1: RDS Snapshot Restore

```bash
# 1. List available snapshots
aws rds describe-db-snapshots \
  --db-instance-identifier agentauri-staging \
  --query 'DBSnapshots[*].[DBSnapshotIdentifier,SnapshotCreateTime]' \
  --output table

# 2. Restore to new instance
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier agentauri-staging-restored \
  --db-snapshot-identifier <snapshot-id> \
  --db-instance-class db.t3.micro \
  --vpc-security-group-ids sg-xxx \
  --db-subnet-group-name agentauri-staging

# 3. Wait for instance to be available
aws rds wait db-instance-available \
  --db-instance-identifier agentauri-staging-restored

# 4. Get new endpoint
aws rds describe-db-instances \
  --db-instance-identifier agentauri-staging-restored \
  --query 'DBInstances[0].Endpoint.Address'

# 5. Update secrets with new endpoint
aws secretsmanager update-secret \
  --secret-id agentauri/staging/database-url \
  --secret-string "postgres://user:pass@NEW_ENDPOINT:5432/agentauri_backend"

# 6. Restart ECS services
aws ecs update-service --cluster agentauri-staging --service api-gateway --force-new-deployment
aws ecs update-service --cluster agentauri-staging --service event-processor --force-new-deployment
aws ecs update-service --cluster agentauri-staging --service action-workers --force-new-deployment
```

### Procedure 2: Point-in-Time Recovery

```bash
# Restore to specific point in time (within backup retention window)
aws rds restore-db-instance-to-point-in-time \
  --source-db-instance-identifier agentauri-staging \
  --target-db-instance-identifier agentauri-staging-pit \
  --restore-time 2025-12-28T10:00:00Z \
  --db-instance-class db.t3.micro \
  --vpc-security-group-ids sg-xxx \
  --db-subnet-group-name agentauri-staging
```

### Procedure 3: Secret Rotation

```bash
# 1. Generate new secret
NEW_SECRET=$(openssl rand -base64 32)

# 2. Update in Secrets Manager
aws secretsmanager update-secret \
  --secret-id agentauri/staging/jwt-secret \
  --secret-string "$NEW_SECRET"

# 3. Force ECS service restart to pick up new secret
aws ecs update-service --cluster agentauri-staging --service api-gateway --force-new-deployment
```

### Procedure 4: Full Infrastructure Recovery

If infrastructure is destroyed, recover from Terraform:

```bash
# 1. Ensure Terraform state is available (S3 backend)
cd terraform

# 2. Initialize
terraform init

# 3. Select workspace
terraform workspace select staging

# 4. Plan and apply
terraform plan -out=recovery.tfplan
terraform apply recovery.tfplan

# 5. Restore database from latest snapshot
# (See Procedure 1)

# 6. Deploy latest application images
# (Images preserved in ECR)
```

---

## Verification Checklist

After any recovery, verify:

- [ ] **API Health**: `curl https://api.agentauri.ai/health`
- [ ] **Database**: Check connection and data integrity
- [ ] **Authentication**: Test login flow
- [ ] **Triggers**: Verify existing triggers are intact
- [ ] **Event Processing**: Check event-processor logs
- [ ] **Action Workers**: Verify Telegram/webhook actions work
- [ ] **Rate Limiting**: Test Redis connectivity
- [ ] **Monitoring**: CloudWatch dashboards showing data

### Quick Verification Script

```bash
#!/bin/bash
# verify-recovery.sh

API_URL="https://api.agentauri.ai"

echo "=== Recovery Verification ==="

# Health check
echo -n "Health check: "
curl -s "$API_URL/health" | jq -r '.status'

# Database
echo -n "Database: "
curl -s "$API_URL/health" | jq -r '.database'

# Check a protected endpoint (replace with valid token)
echo -n "Auth working: "
curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/api/v1/auth/me" | jq -r '.id // "FAILED"'

echo "=== Done ==="
```

---

## Preventive Measures

### Automated Backups

1. RDS automated snapshots (daily)
2. ECR image immutability
3. Secrets Manager versioning

### Monitoring Alerts

CloudWatch alarms configured for:
- Database CPU > 80%
- Database storage < 20%
- ECS task failures
- 5xx error rate > 5%

### Regular Testing

| Test | Frequency | Last Tested |
|------|-----------|-------------|
| Snapshot restore | Quarterly | TBD |
| Secret rotation | Monthly | TBD |
| Failover (if multi-AZ) | Quarterly | TBD |
| Full DR drill | Annually | TBD |

---

## Contacts

| Role | Contact | Responsibility |
|------|---------|---------------|
| On-call Engineer | TBD | First responder |
| Database Admin | TBD | RDS recovery |
| DevOps Lead | TBD | Infrastructure decisions |
| Product Owner | TBD | Business impact assessment |

---

## Related Documentation

- [Rollback Procedures](./ROLLBACK_PROCEDURES.md)
- [Production Deployment Guide](../deployment/PRODUCTION_DEPLOYMENT_GUIDE.md)
- [Secrets Management](../security/SECRETS_MANAGEMENT.md)
- [Monitoring Setup](./MONITORING.md)
