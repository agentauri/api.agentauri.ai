# Rollback Procedures

**Project**: api.agentauri.ai
**Last Updated**: 2025-12-28
**Environment**: AWS ECS Fargate (us-east-1)

This document provides step-by-step rollback procedures for ECS deployments, database migrations, and configuration changes.

---

## Table of Contents

- [Quick Reference](#quick-reference)
- [ECS Service Rollback](#ecs-service-rollback)
- [Database Migration Rollback](#database-migration-rollback)
- [Configuration Rollback](#configuration-rollback)
- [Complete Rollback Checklist](#complete-rollback-checklist)

---

## Quick Reference

### Emergency Rollback Commands

```bash
# Rollback API Gateway to previous task definition
aws ecs update-service \
  --cluster agentauri-production \
  --service api-gateway-v2 \
  --task-definition agentauri-production-api-gateway:PREVIOUS_REVISION

# Force new deployment with current (stable) image
aws ecs update-service \
  --cluster agentauri-production \
  --service api-gateway-v2 \
  --force-new-deployment
```

### Services

| Service | ECS Service Name | Task Definition Family |
|---------|------------------|------------------------|
| API Gateway | api-gateway-v2 | agentauri-production-api-gateway |
| Event Processor | event-processor | agentauri-production-event-processor |
| Action Workers | action-workers | agentauri-production-action-workers |
| Ponder Indexer | ponder-indexer | agentauri-production-ponder-indexer |

---

## ECS Service Rollback

### Step 1: Identify Current and Previous Revisions

```bash
# List recent task definition revisions
aws ecs list-task-definitions \
  --family-prefix agentauri-production-api-gateway \
  --sort DESC \
  --max-items 5

# Output example:
# arn:aws:ecs:us-east-1:xxx:task-definition/agentauri-production-api-gateway:15  (current)
# arn:aws:ecs:us-east-1:xxx:task-definition/agentauri-production-api-gateway:14  (previous)
```

### Step 2: Verify Previous Revision

```bash
# Check what image/config was in previous revision
aws ecs describe-task-definition \
  --task-definition agentauri-production-api-gateway:14 \
  --query 'taskDefinition.containerDefinitions[0].image'
```

### Step 3: Execute Rollback

```bash
# Rollback to specific revision
aws ecs update-service \
  --cluster agentauri-production \
  --service api-gateway-v2 \
  --task-definition agentauri-production-api-gateway:14

# Wait for deployment to complete
aws ecs wait services-stable \
  --cluster agentauri-production \
  --services api-gateway-v2
```

### Step 4: Verify Rollback

```bash
# Check running task uses correct revision
aws ecs describe-services \
  --cluster agentauri-production \
  --services api-gateway-v2 \
  --query 'services[0].deployments'

# Verify health
curl -s https://api.agentauri.ai/health | jq
```

### Rollback All Services

If you need to rollback all services to a previous known-good state:

```bash
#!/bin/bash
# rollback-all.sh

CLUSTER="agentauri-production"
PREVIOUS_TAG="abc123"  # Known good commit SHA

services=(
  "api-gateway-v2:agentauri-production-api-gateway"
  "event-processor:agentauri-production-event-processor"
  "action-workers:agentauri-production-action-workers"
)

for svc in "${services[@]}"; do
  SERVICE_NAME="${svc%%:*}"
  TASK_FAMILY="${svc##*:}"

  # Find revision with the known-good image tag
  REVISION=$(aws ecs list-task-definitions \
    --family-prefix "$TASK_FAMILY" \
    --query "taskDefinitionArns[?contains(@, '$PREVIOUS_TAG')]" \
    --output text | head -1)

  if [ -n "$REVISION" ]; then
    echo "Rolling back $SERVICE_NAME to $REVISION"
    aws ecs update-service \
      --cluster "$CLUSTER" \
      --service "$SERVICE_NAME" \
      --task-definition "$REVISION"
  else
    echo "WARNING: No revision found for $TASK_FAMILY with tag $PREVIOUS_TAG"
  fi
done

# Wait for all services to stabilize
aws ecs wait services-stable \
  --cluster "$CLUSTER" \
  --services api-gateway-v2 event-processor action-workers
```

---

## Database Migration Rollback

### Before Running Migrations

Always create a snapshot before applying migrations:

```bash
# Create pre-migration snapshot
aws rds create-db-snapshot \
  --db-instance-identifier agentauri-production \
  --db-snapshot-identifier pre-migration-$(date +%Y%m%d-%H%M%S)
```

### Rollback Strategy Options

#### Option 1: Revert Migration (If Down Migration Exists)

Check if migration has a down script:

```bash
ls database/migrations/
# Example: 20251221000001_create_a2a_tasks_table.sql

# If there's a corresponding down migration, apply it
# (Note: SQLx doesn't support automatic down migrations - manual required)
```

#### Option 2: Restore from Snapshot

```bash
# 1. Stop ECS services to prevent new writes
aws ecs update-service --cluster agentauri-production --service api-gateway-v2 --desired-count 0
aws ecs update-service --cluster agentauri-production --service event-processor --desired-count 0
aws ecs update-service --cluster agentauri-production --service action-workers --desired-count 0

# 2. Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier agentauri-production-rollback \
  --db-snapshot-identifier pre-migration-20251228-143000 \
  --db-instance-class db.t3.micro \
  --vpc-security-group-ids sg-xxx \
  --db-subnet-group-name agentauri-production

# 3. Wait for restore
aws rds wait db-instance-available \
  --db-instance-identifier agentauri-production-rollback

# 4. Get new endpoint
NEW_ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier agentauri-production-rollback \
  --query 'DBInstances[0].Endpoint.Address' \
  --output text)

# 5. Update secret with new endpoint
aws secretsmanager update-secret \
  --secret-id agentauri/production/database-url \
  --secret-string "postgres://user:pass@$NEW_ENDPOINT:5432/agentauri_backend"

# 6. Restart services
aws ecs update-service --cluster agentauri-production --service api-gateway-v2 --desired-count 1 --force-new-deployment
aws ecs update-service --cluster agentauri-production --service event-processor --desired-count 1 --force-new-deployment
aws ecs update-service --cluster agentauri-production --service action-workers --desired-count 1 --force-new-deployment
```

#### Option 3: Manual SQL Rollback

For simple schema changes, manually revert:

```sql
-- Connect to database
PGPASSWORD="xxx" psql -h endpoint -U user -d agentauri_backend

-- Example: Drop newly added column
ALTER TABLE triggers DROP COLUMN IF EXISTS new_column;

-- Example: Drop newly added table
DROP TABLE IF EXISTS new_table CASCADE;

-- Update _sqlx_migrations table to remove migration record
DELETE FROM _sqlx_migrations WHERE version = 20251228000001;
```

---

## Configuration Rollback

### Secrets Manager

```bash
# List secret versions
aws secretsmanager list-secret-version-ids \
  --secret-id agentauri/production/jwt-secret

# Restore previous version
aws secretsmanager update-secret-version-stage \
  --secret-id agentauri/production/jwt-secret \
  --version-stage AWSCURRENT \
  --move-to-version-id PREVIOUS_VERSION_ID \
  --remove-from-version-id CURRENT_VERSION_ID

# Force service restart to pick up old secret
aws ecs update-service --cluster agentauri-production --service api-gateway-v2 --force-new-deployment
```

### Terraform Configuration

```bash
cd terraform

# Check current state
terraform show

# Rollback to previous git commit
git checkout HEAD~1 -- .

# Plan changes (review carefully!)
terraform plan

# Apply rollback
terraform apply
```

### Environment Variables

If environment variables were changed in task definition:

```bash
# 1. Find previous task definition revision
aws ecs list-task-definitions \
  --family-prefix agentauri-production-api-gateway \
  --sort DESC

# 2. Compare environment variables
aws ecs describe-task-definition \
  --task-definition agentauri-production-api-gateway:14 \
  --query 'taskDefinition.containerDefinitions[0].environment'

aws ecs describe-task-definition \
  --task-definition agentauri-production-api-gateway:15 \
  --query 'taskDefinition.containerDefinitions[0].environment'

# 3. Rollback to previous revision
aws ecs update-service \
  --cluster agentauri-production \
  --service api-gateway-v2 \
  --task-definition agentauri-production-api-gateway:14
```

---

## Complete Rollback Checklist

### Before Rollback

- [ ] Identify the issue and confirm rollback is needed
- [ ] Note current versions/revisions for all affected services
- [ ] Create database snapshot if applicable
- [ ] Notify team of rollback

### During Rollback

- [ ] Stop affected services (if data integrity at risk)
- [ ] Execute rollback commands
- [ ] Wait for services to stabilize
- [ ] Verify health endpoints
- [ ] Check CloudWatch logs for errors

### After Rollback

- [ ] Verify application functionality
- [ ] Test critical user flows
- [ ] Monitor error rates
- [ ] Document what was rolled back and why
- [ ] Create ticket for proper fix

### Verification Commands

```bash
# Health check
curl -s https://api.agentauri.ai/health | jq

# Check service status
aws ecs describe-services \
  --cluster agentauri-production \
  --services api-gateway-v2 \
  --query 'services[0].{status:status,running:runningCount,desired:desiredCount,deployments:deployments[*].status}'

# Check recent logs for errors
aws logs tail /ecs/agentauri-production/api-gateway --since 10m --filter-pattern "ERROR"

# Check CloudWatch alarm status
aws cloudwatch describe-alarms \
  --alarm-name-prefix agentauri \
  --state-value ALARM
```

---

## Common Rollback Scenarios

### Scenario: Bad Deploy - 5xx Errors

1. Check logs: `aws logs tail /ecs/agentauri-production/api-gateway --since 5m`
2. Identify last working revision
3. Rollback ECS service to previous revision
4. Verify health

### Scenario: Database Migration Broke Queries

1. Stop ECS services (prevent more errors)
2. Restore from pre-migration snapshot
3. Update DATABASE_URL secret
4. Rollback ECS to version before migration
5. Restart services

### Scenario: Wrong Configuration Deployed

1. Identify what changed (diff task definitions)
2. Rollback to previous task definition
3. Verify configuration is correct

### Scenario: Security Issue - Compromised Secrets

1. Rotate affected secrets immediately
2. Force new ECS deployment
3. Invalidate user sessions if JWT_SECRET changed
4. Audit access logs

---

## Related Documentation

- [Disaster Recovery](./DISASTER_RECOVERY.md)
- [Production Deployment Guide](../deployment/PRODUCTION_DEPLOYMENT_GUIDE.md)
- [Monitoring Setup](./MONITORING.md)
