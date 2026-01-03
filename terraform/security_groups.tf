# =============================================================================
# Security Groups
# =============================================================================

# -----------------------------------------------------------------------------
# ECS Tasks Security Group
# -----------------------------------------------------------------------------

resource "aws_security_group" "ecs_tasks" {
  name        = "${local.name_prefix}-ecs-tasks"
  description = "Security group for ECS tasks"
  vpc_id      = aws_vpc.main.id

  # Note: API Gateway VPC Link ingress added via separate rule in api_gateway.tf

  egress {
    description = "Allow all outbound traffic"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name_prefix}-ecs-tasks"
  }
}

# API Gateway VPC Link to ECS ingress rule (separate to avoid circular dependency)
resource "aws_security_group_rule" "ecs_from_api_gw_vpc_link" {
  type                     = "ingress"
  from_port                = 8080
  to_port                  = 8080
  protocol                 = "tcp"
  security_group_id        = aws_security_group.ecs_tasks.id
  source_security_group_id = aws_security_group.api_gateway_vpc_link.id
  description              = "HTTP from API Gateway VPC Link"
}

# -----------------------------------------------------------------------------
# RDS Security Group
# -----------------------------------------------------------------------------

resource "aws_security_group" "rds" {
  name        = "${local.name_prefix}-rds"
  description = "Security group for RDS PostgreSQL"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "PostgreSQL from ECS tasks"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }

  tags = {
    Name = "${local.name_prefix}-rds"
  }
}

# -----------------------------------------------------------------------------
# ElastiCache Security Group (Optional - only when redis_enabled = true)
# -----------------------------------------------------------------------------

resource "aws_security_group" "redis" {
  count = var.redis_enabled ? 1 : 0

  name        = "${local.name_prefix}-redis"
  description = "Security group for ElastiCache Redis"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "Redis from ECS tasks"
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }

  tags = {
    Name = "${local.name_prefix}-redis"
  }
}
