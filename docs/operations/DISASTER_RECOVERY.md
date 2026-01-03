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
  --db-instance-identifier agentauri-production \
  --db-snapshot-identifier agentauri-pre-migration-$(date +%Y%m%d)
```

### ECR Images

All deployed images use immutable tags:
- Format: `{sha}` or `{version}`
- Location: `{account}.dkr.ecr.us-east-1.amazonaws.com/agentauri-production-{service}`

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
  --db-instance-identifier agentauri-production \
  --query 'DBSnapshots[*].[DBSnapshotIdentifier,SnapshotCreateTime]' \
  --output table

# 2. Restore to new instance
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier agentauri-production-restored \
  --db-snapshot-identifier <snapshot-id> \
  --db-instance-class db.t3.micro \
  --vpc-security-group-ids sg-xxx \
  --db-subnet-group-name agentauri-production

# 3. Wait for instance to be available
aws rds wait db-instance-available \
  --db-instance-identifier agentauri-production-restored

# 4. Get new endpoint
aws rds describe-db-instances \
  --db-instance-identifier agentauri-production-restored \
  --query 'DBInstances[0].Endpoint.Address'

# 5. Update secrets with new endpoint
aws secretsmanager update-secret \
  --secret-id agentauri/production/database-url \
  --secret-string "postgres://user:pass@NEW_ENDPOINT:5432/agentauri_backend"

# 6. Restart ECS services
aws ecs update-service --cluster agentauri-production --service api-gateway --force-new-deployment
aws ecs update-service --cluster agentauri-production --service event-processor --force-new-deployment
aws ecs update-service --cluster agentauri-production --service action-workers --force-new-deployment
```

### Procedure 2: Point-in-Time Recovery

```bash
# Restore to specific point in time (within backup retention window)
aws rds restore-db-instance-to-point-in-time \
  --source-db-instance-identifier agentauri-production \
  --target-db-instance-identifier agentauri-production-pit \
  --restore-time 2025-12-28T10:00:00Z \
  --db-instance-class db.t3.micro \
  --vpc-security-group-ids sg-xxx \
  --db-subnet-group-name agentauri-production
```

### Procedure 3: Secret Rotation

```bash
# 1. Generate new secret
NEW_SECRET=$(openssl rand -base64 32)

# 2. Update in Secrets Manager
aws secretsmanager update-secret \
  --secret-id agentauri/production/jwt-secret \
  --secret-string "$NEW_SECRET"

# 3. Force ECS service restart to pick up new secret
aws ecs update-service --cluster agentauri-production --service api-gateway --force-new-deployment
```

### Procedure 4: Full Infrastructure Recovery

If infrastructure is destroyed, recover from Terraform:

```bash
# 1. Ensure Terraform state is available (S3 backend)
cd terraform

# 2. Initialize
terraform init

# 3. Select workspace
terraform workspace select production

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

## DR Testing Procedures

### Test 1: RDS Snapshot Restore Test (Quarterly)

**Purpose**: Verify that database backups are valid and can be restored.

**Pre-requisites**:
- AWS CLI configured with appropriate permissions
- At least one automated snapshot available

**Procedure**:

```bash
#!/bin/bash
# dr-test-snapshot-restore.sh
# Duration: ~20-30 minutes

set -e

DATE=$(date +%Y%m%d)
TEST_INSTANCE="agentauri-dr-test-${DATE}"
SNAPSHOT_ID=$(aws rds describe-db-snapshots \
  --db-instance-identifier agentauri-production \
  --query 'DBSnapshots | sort_by(@, &SnapshotCreateTime) | [-1].DBSnapshotIdentifier' \
  --output text)

echo "=== DR Test: Snapshot Restore ==="
echo "Date: $(date)"
echo "Snapshot: ${SNAPSHOT_ID}"
echo ""

# Step 1: Restore snapshot to test instance
echo "[1/5] Restoring snapshot to test instance..."
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier "${TEST_INSTANCE}" \
  --db-snapshot-identifier "${SNAPSHOT_ID}" \
  --db-instance-class db.t3.micro \
  --no-publicly-accessible

# Step 2: Wait for instance to be available
echo "[2/5] Waiting for instance to be available (this takes ~15 minutes)..."
aws rds wait db-instance-available \
  --db-instance-identifier "${TEST_INSTANCE}"

# Step 3: Get endpoint and test connection
echo "[3/5] Testing database connectivity..."
ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier "${TEST_INSTANCE}" \
  --query 'DBInstances[0].Endpoint.Address' \
  --output text)

echo "Endpoint: ${ENDPOINT}"

# Step 4: Verify data integrity (example queries)
echo "[4/5] Verifying data integrity..."
# Note: Replace with actual password or use IAM auth
PGPASSWORD="${RDS_PASSWORD}" psql -h "${ENDPOINT}" -U agentauri_admin -d agentauri_backend -c "
  SELECT 'users' as table_name, COUNT(*) as row_count FROM users
  UNION ALL
  SELECT 'organizations', COUNT(*) FROM organizations
  UNION ALL
  SELECT 'triggers', COUNT(*) FROM triggers
  UNION ALL
  SELECT 'events', COUNT(*) FROM events;
"

# Step 5: Cleanup test instance
echo "[5/5] Cleaning up test instance..."
aws rds delete-db-instance \
  --db-instance-identifier "${TEST_INSTANCE}" \
  --skip-final-snapshot \
  --delete-automated-backups

echo ""
echo "=== DR Test Complete ==="
echo "Result: SUCCESS"
echo "Documented at: docs/operations/dr-tests/${DATE}-snapshot-restore.md"
```

**Success Criteria**:
- [ ] Snapshot restored successfully
- [ ] Database accessible
- [ ] All tables present with expected row counts
- [ ] No data corruption detected

**Post-Test**:
1. Document results in `docs/operations/dr-tests/`
2. Update "Last Tested" date in this document
3. Create ticket for any issues found

---

### Test 2: Multi-AZ Failover Test (Quarterly)

**Purpose**: Verify that RDS Multi-AZ failover works correctly.

**Pre-requisites**:
- RDS Multi-AZ enabled (currently: ✅)
- Non-production time window (failover takes ~1-2 minutes)

**Procedure**:

```bash
#!/bin/bash
# dr-test-failover.sh
# Duration: ~5 minutes

set -e

echo "=== DR Test: Multi-AZ Failover ==="
echo "Date: $(date)"
echo ""

# Step 1: Get current AZ
echo "[1/4] Current configuration..."
aws rds describe-db-instances \
  --db-instance-identifier agentauri-production \
  --query 'DBInstances[0].{AZ:AvailabilityZone,MultiAZ:MultiAZ,Status:DBInstanceStatus}'

# Step 2: Initiate failover
echo "[2/4] Initiating failover..."
aws rds reboot-db-instance \
  --db-instance-identifier agentauri-production \
  --force-failover

# Step 3: Wait for instance to be available
echo "[3/4] Waiting for failover to complete..."
aws rds wait db-instance-available \
  --db-instance-identifier agentauri-production

# Step 4: Verify new AZ
echo "[4/4] New configuration..."
aws rds describe-db-instances \
  --db-instance-identifier agentauri-production \
  --query 'DBInstances[0].{AZ:AvailabilityZone,MultiAZ:MultiAZ,Status:DBInstanceStatus}'

echo ""
echo "=== Failover Test Complete ==="
```

**Success Criteria**:
- [ ] Failover completed within 2 minutes
- [ ] Application reconnected automatically
- [ ] No data loss
- [ ] AZ changed from primary to standby

**Monitoring During Test**:
- Watch CloudWatch for connection drops
- Monitor application health endpoint
- Check ECS service logs for reconnection

---

### Test 3: Secret Rotation Test (Monthly)

**Purpose**: Verify that secrets can be rotated without service disruption.

**Procedure**:

```bash
#!/bin/bash
# dr-test-secret-rotation.sh

set -e

echo "=== DR Test: Secret Rotation ==="
echo "Date: $(date)"
echo ""

# Rotate JWT secret (forces all users to re-login)
echo "[1/3] Generating new JWT secret..."
NEW_JWT=$(openssl rand -base64 32)

echo "[2/3] Updating secret in Secrets Manager..."
aws secretsmanager update-secret \
  --secret-id agentauri/production/jwt-secret \
  --secret-string "${NEW_JWT}"

echo "[3/3] Forcing ECS service restart..."
aws ecs update-service \
  --cluster agentauri-production \
  --service api-gateway \
  --force-new-deployment

echo ""
echo "=== Secret Rotation Complete ==="
echo "Note: All existing JWT tokens are now invalid."
```

**Success Criteria**:
- [ ] New secret stored successfully
- [ ] ECS tasks restarted with new secret
- [ ] New authentication working
- [ ] Old tokens correctly rejected

---

### Test 4: Full DR Drill (Annually)

**Purpose**: Complete infrastructure recovery simulation.

**Scenario**: Simulate complete region failure.

**Steps**:

1. **Preparation** (1 week before)
   - Schedule maintenance window
   - Notify stakeholders
   - Prepare backup region (if available)

2. **Drill Execution** (4-6 hours)
   - Take final snapshot of production
   - Document current state
   - "Destroy" infrastructure (terraform destroy --target specific resources)
   - Time the recovery process
   - Restore from Terraform + snapshots
   - Verify all services operational

3. **Documentation**
   - Record all steps taken
   - Document any issues encountered
   - Calculate actual RTO/RPO
   - Update procedures based on learnings

**Success Criteria**:
- [ ] RTO met (< 1 hour)
- [ ] RPO met (< 24 hours data loss)
- [ ] All services restored
- [ ] Data integrity verified
- [ ] External connectivity confirmed

---

## DR Test Results Log

| Date | Test Type | Result | RTO Actual | Notes | Tester |
|------|-----------|--------|------------|-------|--------|
| TBD | Snapshot Restore | - | - | - | - |
| TBD | Multi-AZ Failover | - | - | - | - |
| TBD | Secret Rotation | - | - | - | - |
| TBD | Full DR Drill | - | - | - | - |

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
