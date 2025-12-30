# Multi-Region Strategy

Last Updated: 2025-12-30
Phase: 7 - Scaling

## Current State

AgentAuri runs in a single region: **us-east-1** (N. Virginia)

### Infrastructure
- **ECS Fargate**: api-gateway, event-processor, action-workers, ponder-indexer
- **RDS PostgreSQL**: Multi-AZ enabled (standby in different AZ)
- **ElastiCache Redis**: Single node (can be upgraded to cluster mode)
- **ALB**: Internet-facing, multi-AZ

### Availability
- Current setup provides **high availability within a single region**
- Multi-AZ for RDS and ALB ensures AZ-level fault tolerance
- ECS tasks distributed across multiple AZs

---

## Multi-Region Architecture

### Target Architecture

```
                    ┌─────────────────┐
                    │   Route 53      │
                    │   (Latency/GEO) │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
       ┌──────────┐   ┌──────────┐   ┌──────────┐
       │us-east-1 │   │eu-west-1 │   │ap-south-1│
       │(Primary) │   │(Europe)  │   │(Asia)    │
       └────┬─────┘   └────┬─────┘   └────┬─────┘
            │              │              │
            ▼              ▼              ▼
       ┌─────────┐   ┌─────────┐   ┌─────────┐
       │   ALB   │   │   ALB   │   │   ALB   │
       └────┬────┘   └────┬────┘   └────┬────┘
            │              │              │
            ▼              ▼              ▼
       ┌─────────┐   ┌─────────┐   ┌─────────┐
       │   ECS   │   │   ECS   │   │   ECS   │
       │ Cluster │   │ Cluster │   │ Cluster │
       └────┬────┘   └────┬────┘   └────┬────┘
            │              │              │
            ▼              ▼              ▼
       ┌─────────┐   ┌─────────┐   ┌─────────┐
       │RDS (R/W)│◄──│RDS(Read)│   │RDS(Read)│
       │ Primary │   │ Replica │   │ Replica │
       └─────────┘   └─────────┘   └─────────┘
```

---

## Phase 7.1: Read Replicas (Recommended First Step)

### Goal
Improve read performance and prepare for multi-region by adding RDS read replicas.

### Implementation

```hcl
# terraform/rds_replicas.tf

resource "aws_db_instance" "read_replica_eu" {
  identifier             = "${local.name_prefix}-replica-eu"
  replicate_source_db    = aws_db_instance.main.identifier
  instance_class         = "db.t3.medium"
  publicly_accessible    = false
  vpc_security_group_ids = [aws_security_group.rds.id]

  # Read replicas inherit encryption from source
  storage_encrypted = true

  # No backups for read replicas
  backup_retention_period = 0

  # Can be promoted to standalone if needed
  auto_minor_version_upgrade = true

  tags = {
    Name = "${local.name_prefix}-replica-eu"
    Role = "read-replica"
  }
}
```

### Application Changes
- Use read replica for:
  - Event queries (`GET /api/v1/ponder/events`)
  - Statistics endpoints (`GET /api/v1/organizations/{id}/api-keys/stats`)
  - Dashboard data
- Keep writes on primary (triggers, API keys, auth)

---

## Phase 7.2: Active-Passive Multi-Region

### Goal
Full regional redundancy with automatic failover.

### Components

| Component | Primary (us-east-1) | Secondary (eu-west-1) |
|-----------|---------------------|----------------------|
| ECS Services | Active | Warm standby |
| RDS | Writer | Read replica |
| Redis | Active | Passive (cold) |
| ALB | Active | Active (receives traffic) |
| Route 53 | Health checks | Failover routing |

### Terraform Module Structure

```
terraform/
├── modules/
│   └── region/
│       ├── main.tf          # VPC, subnets, NAT
│       ├── ecs.tf           # Cluster, services
│       ├── rds.tf           # Instance or replica
│       ├── redis.tf         # ElastiCache
│       ├── alb.tf           # Load balancer
│       └── variables.tf
├── environments/
│   ├── us-east-1/
│   │   ├── main.tf
│   │   └── terraform.tfvars
│   └── eu-west-1/
│       ├── main.tf
│       └── terraform.tfvars
└── global/
    ├── route53.tf
    ├── acm.tf
    └── s3.tf
```

### Route 53 Configuration

```hcl
# terraform/global/route53.tf

resource "aws_route53_health_check" "primary" {
  fqdn              = "api-us.agentauri.ai"
  port              = 443
  type              = "HTTPS"
  resource_path     = "/api/v1/health"
  failure_threshold = 3
  request_interval  = 30

  tags = {
    Name = "primary-health-check"
  }
}

resource "aws_route53_record" "api" {
  zone_id = data.aws_route53_zone.main.zone_id
  name    = "api.agentauri.ai"
  type    = "A"

  failover_routing_policy {
    type = "PRIMARY"
  }

  set_identifier  = "primary"
  health_check_id = aws_route53_health_check.primary.id

  alias {
    name                   = aws_lb.primary.dns_name
    zone_id                = aws_lb.primary.zone_id
    evaluate_target_health = true
  }
}
```

---

## Phase 7.3: Active-Active Multi-Region

### Goal
Serve traffic from multiple regions simultaneously with lowest latency.

### Challenges

| Challenge | Solution |
|-----------|----------|
| Write conflicts | Single writer region + global DB |
| Read consistency | Eventually consistent reads acceptable |
| Session state | Stateless JWT tokens |
| Redis sync | Global Datastore or regional caches |

### Database Options

1. **Aurora Global Database** (Recommended)
   - Sub-second replication lag
   - Automatic failover
   - Up to 5 read regions
   - Higher cost (~$500+/month for global)

2. **RDS Cross-Region Read Replicas**
   - Manual failover
   - Higher replication lag (~minutes)
   - Lower cost

### Redis Strategy

```hcl
# Option 1: Regional Redis with write-through
# Each region has its own Redis, writes propagate to primary

# Option 2: Global Datastore (Recommended for active-active)
resource "aws_elasticache_global_replication_group" "main" {
  global_replication_group_id_suffix = "agentauri"
  primary_replication_group_id       = aws_elasticache_replication_group.primary.id
}

resource "aws_elasticache_replication_group" "secondary" {
  replication_group_id       = "${local.name_prefix}-redis-eu"
  description                = "Secondary Redis in EU"
  global_replication_group_id = aws_elasticache_global_replication_group.main.id

  # Secondary inherits settings from primary
}
```

---

## Cost Analysis

### Current (Single Region)
| Resource | Monthly Cost |
|----------|-------------|
| ECS Fargate (4 tasks) | ~$150 |
| RDS db.t3.medium (Multi-AZ) | ~$120 |
| ElastiCache t3.micro | ~$15 |
| ALB | ~$20 |
| NAT Gateway | ~$35 |
| **Total** | **~$340/month** |

### Phase 7.1: Read Replicas
| Addition | Monthly Cost |
|----------|-------------|
| RDS Read Replica | +$60 |
| **Total** | **~$400/month** |

### Phase 7.2: Active-Passive
| Addition | Monthly Cost |
|----------|-------------|
| Second region (warm) | +$200 |
| Route 53 health checks | +$5 |
| **Total** | **~$545/month** |

### Phase 7.3: Active-Active
| Change | Monthly Cost |
|--------|-------------|
| Aurora Global Database | +$300 |
| Second region (active) | +$250 |
| Global Datastore Redis | +$50 |
| **Total** | **~$900/month** |

---

## Implementation Timeline

### Phase 7.1 (2 weeks)
- [ ] Add RDS read replica in same region
- [ ] Update application to use read replica for queries
- [ ] Test failover scenarios

### Phase 7.2 (4 weeks)
- [ ] Create Terraform modules for regional deployment
- [ ] Deploy secondary region (eu-west-1)
- [ ] Configure Route 53 failover
- [ ] Test regional failover

### Phase 7.3 (6 weeks)
- [ ] Migrate to Aurora Global Database
- [ ] Implement Global Datastore for Redis
- [ ] Configure latency-based routing
- [ ] Load testing for multi-region

---

## Decision Matrix

| Factor | Read Replicas | Active-Passive | Active-Active |
|--------|--------------|----------------|---------------|
| Cost | Low | Medium | High |
| Complexity | Low | Medium | High |
| Read latency | Improved | Improved | Best |
| Write latency | Same | Same | Same (single writer) |
| Failover time | Manual | ~1 min | Automatic |
| Data loss risk | Minimal | Minimal | Minimal |

### Recommendation

**Start with Phase 7.1 (Read Replicas)** because:
1. Lowest cost and complexity
2. Immediate performance benefits
3. Prepares architecture for multi-region
4. Can be implemented quickly

Proceed to Phase 7.2 when:
- User base grows significantly in other regions
- Compliance requires data residency
- SLA requires <1 minute failover

---

## Monitoring Additions

For multi-region, add these CloudWatch alarms:

```hcl
# Cross-region replication lag
resource "aws_cloudwatch_metric_alarm" "rds_replica_lag" {
  alarm_name          = "rds-replica-lag-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "ReplicaLag"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 60  # 60 seconds
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.read_replica.identifier
  }
}
```

---

## References

- [AWS Global Infrastructure](https://aws.amazon.com/about-aws/global-infrastructure/)
- [Aurora Global Database](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/aurora-global-database.html)
- [ElastiCache Global Datastore](https://docs.aws.amazon.com/AmazonElastiCache/latest/red-ug/Redis-Global-Datastore.html)
- [Route 53 Failover Routing](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/routing-policy-failover.html)
