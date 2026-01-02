# =============================================================================
# Amazon ElastiCache - Redis Cluster
# =============================================================================

# -----------------------------------------------------------------------------
# ElastiCache Subnet Group
# -----------------------------------------------------------------------------

resource "aws_elasticache_subnet_group" "main" {
  name       = local.name_prefix
  subnet_ids = aws_subnet.private[*].id

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# ElastiCache Parameter Group
# -----------------------------------------------------------------------------

resource "aws_elasticache_parameter_group" "main" {
  family = "redis7"
  name   = local.name_prefix

  parameter {
    name  = "maxmemory-policy"
    value = "volatile-lru"
  }

  parameter {
    name  = "notify-keyspace-events"
    value = "Ex" # Enable keyspace notifications for expiring keys
  }

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# ElastiCache Replication Group (Redis Cluster)
# -----------------------------------------------------------------------------

resource "aws_elasticache_replication_group" "main" {
  replication_group_id = local.name_prefix
  description          = "Redis cluster for AgentAuri ${var.environment}"

  # Engine configuration
  engine               = "redis"
  engine_version       = "7.1"
  node_type            = var.redis_node_type
  port                 = 6379
  parameter_group_name = aws_elasticache_parameter_group.main.name

  # Cluster configuration
  num_cache_clusters         = var.redis_num_cache_nodes
  automatic_failover_enabled = var.redis_num_cache_nodes > 1
  apply_immediately          = true

  # Network configuration
  subnet_group_name  = aws_elasticache_subnet_group.main.name
  security_group_ids = [aws_security_group.redis.id]

  # Encryption
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token                 = random_password.redis_auth_token.result

  # Maintenance
  maintenance_window       = "sun:05:00-sun:06:00"
  snapshot_window          = "04:00-05:00"
  snapshot_retention_limit = var.environment == "production" ? 7 : 1

  # Auto minor version upgrade
  auto_minor_version_upgrade = true

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# Random Auth Token for Redis
# -----------------------------------------------------------------------------

resource "random_password" "redis_auth_token" {
  length  = 64
  special = false # Redis auth token only allows alphanumeric
}

# -----------------------------------------------------------------------------
# Store Redis Auth Token in Secrets Manager
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "redis_auth_token" {
  name                    = "agentauri/${var.environment}/redis-auth-token"
  description             = "Redis auth token for AgentAuri ${var.environment}"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-redis-auth-token"
  }
}

resource "aws_secretsmanager_secret_version" "redis_auth_token" {
  secret_id = aws_secretsmanager_secret.redis_auth_token.id
  secret_string = jsonencode({
    auth_token       = random_password.redis_auth_token.result
    primary_endpoint = aws_elasticache_replication_group.main.primary_endpoint_address
    reader_endpoint  = aws_elasticache_replication_group.main.reader_endpoint_address
    port             = 6379
    url              = "rediss://:${random_password.redis_auth_token.result}@${aws_elasticache_replication_group.main.primary_endpoint_address}:6379"
  })
}
