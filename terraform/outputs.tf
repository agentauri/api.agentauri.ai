# =============================================================================
# Terraform Outputs
# =============================================================================

# -----------------------------------------------------------------------------
# VPC Outputs
# -----------------------------------------------------------------------------

output "vpc_id" {
  description = "The ID of the VPC"
  value       = aws_vpc.main.id
}

output "public_subnet_ids" {
  description = "List of public subnet IDs"
  value       = aws_subnet.public[*].id
}

output "private_subnet_ids" {
  description = "List of private subnet IDs"
  value       = aws_subnet.private[*].id
}

# -----------------------------------------------------------------------------
# ECR Outputs
# -----------------------------------------------------------------------------

output "ecr_repository_url" {
  description = "The URL of the ECR repository"
  value       = aws_ecr_repository.backend.repository_url
}

output "ecr_repository_arn" {
  description = "The ARN of the ECR repository"
  value       = aws_ecr_repository.backend.arn
}

# -----------------------------------------------------------------------------
# ECS Outputs
# -----------------------------------------------------------------------------

output "ecs_cluster_id" {
  description = "The ID of the ECS cluster"
  value       = aws_ecs_cluster.main.id
}

output "ecs_cluster_name" {
  description = "The name of the ECS cluster"
  value       = aws_ecs_cluster.main.name
}

output "ecs_service_api_gateway" {
  description = "The name of the API Gateway ECS service"
  value       = aws_ecs_service.api_gateway.name
}

output "ecs_service_event_processor" {
  description = "The name of the Event Processor ECS service"
  value       = aws_ecs_service.event_processor.name
}

output "ecs_service_action_workers" {
  description = "The name of the Action Workers ECS service"
  value       = aws_ecs_service.action_workers.name
}

output "ecs_service_ponder_indexer" {
  description = "The name of the Ponder Indexer ECS service (empty if disabled)"
  value       = var.ponder_indexer_enabled ? aws_ecs_service.ponder_indexer[0].name : ""
}

# -----------------------------------------------------------------------------
# ALB Outputs
# -----------------------------------------------------------------------------

output "alb_dns_name" {
  description = "The DNS name of the Application Load Balancer"
  value       = aws_lb.main.dns_name
}

output "alb_zone_id" {
  description = "The zone ID of the Application Load Balancer (for Route53)"
  value       = aws_lb.main.zone_id
}

output "alb_arn" {
  description = "The ARN of the Application Load Balancer"
  value       = aws_lb.main.arn
}

# -----------------------------------------------------------------------------
# RDS Outputs
# -----------------------------------------------------------------------------

output "rds_endpoint" {
  description = "The endpoint of the RDS instance"
  value       = aws_db_instance.main.endpoint
}

output "rds_address" {
  description = "The hostname of the RDS instance"
  value       = aws_db_instance.main.address
}

output "rds_port" {
  description = "The port of the RDS instance"
  value       = aws_db_instance.main.port
}

output "rds_database_name" {
  description = "The name of the database"
  value       = aws_db_instance.main.db_name
}

output "rds_password_secret_arn" {
  description = "The ARN of the RDS password secret in Secrets Manager"
  value       = aws_secretsmanager_secret.rds_password.arn
}

# -----------------------------------------------------------------------------
# ElastiCache Outputs
# -----------------------------------------------------------------------------

output "redis_primary_endpoint" {
  description = "The primary endpoint of the Redis cluster"
  value       = aws_elasticache_replication_group.main.primary_endpoint_address
}

output "redis_reader_endpoint" {
  description = "The reader endpoint of the Redis cluster"
  value       = aws_elasticache_replication_group.main.reader_endpoint_address
}

output "redis_port" {
  description = "The port of the Redis cluster"
  value       = 6379
}

output "redis_auth_token_secret_arn" {
  description = "The ARN of the Redis auth token secret in Secrets Manager"
  value       = aws_secretsmanager_secret.redis_auth_token.arn
}

# -----------------------------------------------------------------------------
# IAM Outputs
# -----------------------------------------------------------------------------

output "github_actions_role_arn" {
  description = "The ARN of the GitHub Actions IAM role for OIDC authentication"
  value       = aws_iam_role.github_actions.arn
}

output "ecs_execution_role_arn" {
  description = "The ARN of the ECS execution role"
  value       = aws_iam_role.ecs_execution.arn
}

output "ecs_task_role_arn" {
  description = "The ARN of the ECS task role"
  value       = aws_iam_role.ecs_task.arn
}

# -----------------------------------------------------------------------------
# Secrets Manager Outputs
# -----------------------------------------------------------------------------

output "secrets_arns" {
  description = "Map of secret names to their ARNs"
  value = {
    rds_password     = aws_secretsmanager_secret.rds_password.arn
    redis_auth_token = aws_secretsmanager_secret.redis_auth_token.arn
    jwt_secret       = aws_secretsmanager_secret.jwt_secret.arn
    api_key_salt     = aws_secretsmanager_secret.api_key_salt.arn
    oauth_state_key  = aws_secretsmanager_secret.oauth_state_key.arn
    telegram_bot     = aws_secretsmanager_secret.telegram_bot_token.arn
    stripe_keys      = aws_secretsmanager_secret.stripe_keys.arn
    google_oauth     = aws_secretsmanager_secret.google_oauth.arn
    github_oauth     = aws_secretsmanager_secret.github_oauth.arn
    alchemy_api_key  = aws_secretsmanager_secret.alchemy_api_key.arn
  }
}

# -----------------------------------------------------------------------------
# Useful Connection Strings (for reference only - use Secrets Manager in app)
# -----------------------------------------------------------------------------

output "api_url" {
  description = "The API URL (use with your custom domain)"
  value       = "https://${var.domain_name}"
}

output "health_check_url" {
  description = "The health check URL via ALB DNS"
  value       = "https://${aws_lb.main.dns_name}/api/v1/health"
}

# -----------------------------------------------------------------------------
# Monitoring Outputs
# -----------------------------------------------------------------------------

output "ponder_alerts_sns_topic_arn" {
  description = "The ARN of the Ponder alerts SNS topic"
  value       = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? aws_sns_topic.ponder_alerts[0].arn : ""
}
