# =============================================================================
# Amazon ECS - Fargate Cluster and Services
# =============================================================================

# -----------------------------------------------------------------------------
# ECS Cluster
# -----------------------------------------------------------------------------

resource "aws_ecs_cluster" "main" {
  name = local.name_prefix

  setting {
    name  = "containerInsights"
    value = "disabled" # Cost optimization: saves ~$75/month, CloudWatch alarms provide monitoring
  }

  configuration {
    execute_command_configuration {
      logging = "OVERRIDE"
      log_configuration {
        cloud_watch_log_group_name = aws_cloudwatch_log_group.ecs.name
      }
    }
  }

  tags = {
    Name = local.name_prefix
  }
}

resource "aws_ecs_cluster_capacity_providers" "main" {
  cluster_name = aws_ecs_cluster.main.name

  capacity_providers = ["FARGATE", "FARGATE_SPOT"]

  default_capacity_provider_strategy {
    base              = 1
    weight            = 100
    capacity_provider = "FARGATE"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Log Groups
# -----------------------------------------------------------------------------

resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/${local.name_prefix}"
  retention_in_days = var.environment == "production" ? 30 : 7

  tags = {
    Name = "${local.name_prefix}-logs"
  }
}

resource "aws_cloudwatch_log_group" "api_gateway" {
  name              = "/ecs/${local.name_prefix}/api-gateway"
  retention_in_days = var.environment == "production" ? 30 : 7
}

resource "aws_cloudwatch_log_group" "event_processor" {
  name              = "/ecs/${local.name_prefix}/event-processor"
  retention_in_days = var.environment == "production" ? 30 : 7
}

resource "aws_cloudwatch_log_group" "action_workers" {
  name              = "/ecs/${local.name_prefix}/action-workers"
  retention_in_days = var.environment == "production" ? 30 : 7
}

resource "aws_cloudwatch_log_group" "ponder_indexer" {
  name              = "/ecs/${local.name_prefix}/ponder-indexer"
  retention_in_days = var.environment == "production" ? 14 : 7 # Cost optimization: 14 days sufficient for indexer
}

# -----------------------------------------------------------------------------
# Task Definitions
# -----------------------------------------------------------------------------

resource "aws_ecs_task_definition" "api_gateway" {
  family                   = "${local.name_prefix}-api-gateway"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.api_gateway_cpu
  memory                   = var.api_gateway_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  container_definitions = jsonencode([
    {
      name      = "api-gateway"
      image     = "${var.container_image}:${var.container_image_tag}"
      essential = true
      command   = ["api-gateway"]

      portMappings = [
        {
          containerPort = 8080
          hostPort      = 8080
          protocol      = "tcp"
        }
      ]

      environment = [
        { name = "RUST_LOG", value = "info" },
        { name = "LOG_FORMAT", value = "json" },
        { name = "SECRETS_BACKEND", value = "aws" },
        { name = "SECRETS_PREFIX", value = "agentauri/${var.environment}" },
        { name = "AWS_REGION", value = var.aws_region },
        { name = "ENVIRONMENT", value = var.environment },
        { name = "SERVER_HOST", value = "0.0.0.0" },
        { name = "SERVER_PORT", value = "8080" },
        { name = "DB_SSL_MODE", value = "require" }, # TODO: Use verify-full after adding RDS CA cert to image
        { name = "FRONTEND_URL", value = "https://${var.domain_name}" },
        { name = "GOOGLE_REDIRECT_URI", value = "https://api.${var.domain_name}/api/v1/auth/google/callback" },
        { name = "GITHUB_REDIRECT_URI", value = "https://api.${var.domain_name}/api/v1/auth/github/callback" }
      ]

      secrets = [
        {
          name      = "DB_HOST"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:host::"
        },
        {
          name      = "DB_PORT"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:port::"
        },
        {
          name      = "DB_NAME"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:database::"
        },
        {
          name      = "DB_USER"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:username::"
        },
        {
          name      = "DB_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:password::"
        },
        {
          name      = "JWT_SECRET"
          valueFrom = aws_secretsmanager_secret.jwt_secret.arn
        },
        {
          name      = "OAUTH_STATE_SECRET"
          valueFrom = aws_secretsmanager_secret.oauth_state_key.arn
        },
        {
          # Unified Redis URL - works with both ElastiCache and external Redis (Upstash)
          name      = "REDIS_URL"
          valueFrom = aws_secretsmanager_secret.redis_url.arn
        },
        {
          name      = "MONITORING_TOKEN"
          valueFrom = aws_secretsmanager_secret.monitoring_token.arn
        },
        {
          name      = "GOOGLE_CLIENT_ID"
          valueFrom = "${aws_secretsmanager_secret.google_oauth.arn}:client_id::"
        },
        {
          name      = "GOOGLE_CLIENT_SECRET"
          valueFrom = "${aws_secretsmanager_secret.google_oauth.arn}:client_secret::"
        },
        {
          name      = "GITHUB_CLIENT_ID"
          valueFrom = "${aws_secretsmanager_secret.github_oauth.arn}:client_id::"
        },
        {
          name      = "GITHUB_CLIENT_SECRET"
          valueFrom = "${aws_secretsmanager_secret.github_oauth.arn}:client_secret::"
        }
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.api_gateway.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }

      healthCheck = {
        command     = ["CMD-SHELL", "curl -f http://localhost:8080/api/v1/health || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }
    }
  ])

  tags = {
    Name = "${local.name_prefix}-api-gateway"
  }
}

resource "aws_ecs_task_definition" "event_processor" {
  family                   = "${local.name_prefix}-event-processor"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.event_processor_cpu
  memory                   = var.event_processor_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  container_definitions = jsonencode([
    {
      name      = "event-processor"
      image     = "${var.container_image}:${var.container_image_tag}"
      essential = true
      command   = ["event-processor"]

      environment = [
        { name = "RUST_LOG", value = "info,sqlx=warn,hyper=warn,tokio=warn" }, # Cost optimization: reduces log volume ~70%
        { name = "LOG_FORMAT", value = "json" },
        { name = "SECRETS_BACKEND", value = "aws" },
        { name = "SECRETS_PREFIX", value = "agentauri/${var.environment}" },
        { name = "AWS_REGION", value = var.aws_region },
        { name = "ENVIRONMENT", value = var.environment },
        { name = "DB_SSL_MODE", value = "require" } # TODO: Use verify-full after adding RDS CA cert to image
      ]

      secrets = [
        {
          name      = "DB_HOST"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:host::"
        },
        {
          name      = "DB_PORT"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:port::"
        },
        {
          name      = "DB_NAME"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:database::"
        },
        {
          name      = "DB_USER"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:username::"
        },
        {
          name      = "DB_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:password::"
        },
        {
          name      = "JWT_SECRET"
          valueFrom = aws_secretsmanager_secret.jwt_secret.arn
        },
        {
          # Unified Redis URL - works with both ElastiCache and external Redis (Upstash)
          name      = "REDIS_URL"
          valueFrom = aws_secretsmanager_secret.redis_url.arn
        }
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.event_processor.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  tags = {
    Name = "${local.name_prefix}-event-processor"
  }
}

resource "aws_ecs_task_definition" "action_workers" {
  family                   = "${local.name_prefix}-action-workers"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.action_workers_cpu
  memory                   = var.action_workers_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  container_definitions = jsonencode([
    {
      name      = "action-workers"
      image     = "${var.container_image}:${var.container_image_tag}"
      essential = true
      command   = ["action-workers"]

      environment = [
        { name = "RUST_LOG", value = "info" },
        { name = "LOG_FORMAT", value = "json" },
        { name = "SECRETS_BACKEND", value = "aws" },
        { name = "SECRETS_PREFIX", value = "agentauri/${var.environment}" },
        { name = "AWS_REGION", value = var.aws_region },
        { name = "ENVIRONMENT", value = var.environment },
        { name = "DB_SSL_MODE", value = "require" } # TODO: Use verify-full after adding RDS CA cert to image
      ]

      secrets = [
        {
          name      = "DB_HOST"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:host::"
        },
        {
          name      = "DB_PORT"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:port::"
        },
        {
          name      = "DB_NAME"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:database::"
        },
        {
          name      = "DB_USER"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:username::"
        },
        {
          name      = "DB_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:password::"
        },
        {
          name      = "JWT_SECRET"
          valueFrom = aws_secretsmanager_secret.jwt_secret.arn
        },
        {
          # Unified Redis URL - works with both ElastiCache and external Redis (Upstash)
          name      = "REDIS_URL"
          valueFrom = aws_secretsmanager_secret.redis_url.arn
        },
        {
          name      = "TELEGRAM_BOT_TOKEN"
          valueFrom = aws_secretsmanager_secret.telegram_bot_token.arn
        }
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.action_workers.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  tags = {
    Name = "${local.name_prefix}-action-workers"
  }
}

# -----------------------------------------------------------------------------
# ECS Services
# -----------------------------------------------------------------------------

# API Gateway: Fargate Spot + Public Subnet (no NAT needed)
# Uses API Gateway HTTP with VPC Link + Cloud Map for routing (ALB removed for cost savings)
resource "aws_ecs_service" "api_gateway" {
  name                               = "api-gateway"
  cluster                            = aws_ecs_cluster.main.id
  task_definition                    = aws_ecs_task_definition.api_gateway.arn
  desired_count                      = var.api_gateway_desired_count
  platform_version                   = "LATEST"
  deployment_minimum_healthy_percent = 100
  deployment_maximum_percent         = 200
  enable_execute_command             = true

  # Cost optimization: Fargate Spot saves ~70%
  capacity_provider_strategy {
    capacity_provider = "FARGATE_SPOT"
    weight            = 100
    base              = 0
  }

  # Cost optimization: Public subnet eliminates NAT Gateway (~$35/mese)
  network_configuration {
    subnets          = aws_subnet.public[*].id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = true
  }

  # Service Discovery for API Gateway HTTP routing via Cloud Map (SRV records enabled)
  service_registries {
    registry_arn   = aws_service_discovery_service.api_gateway.arn
    container_name = "api-gateway"
    container_port = 8080
  }

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }

  tags = {
    Name = "${local.name_prefix}-api-gateway"
  }

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# Event Processor: Fargate Spot + Public Subnet
resource "aws_ecs_service" "event_processor" {
  name                               = "event-processor"
  cluster                            = aws_ecs_cluster.main.id
  task_definition                    = aws_ecs_task_definition.event_processor.arn
  desired_count                      = 1
  platform_version                   = "LATEST"
  deployment_minimum_healthy_percent = 0
  deployment_maximum_percent         = 100
  # ECS Exec disabled in production for security (only api-gateway allows debug access)
  enable_execute_command = var.environment != "production"

  # Cost optimization: Fargate Spot saves ~70%
  capacity_provider_strategy {
    capacity_provider = "FARGATE_SPOT"
    weight            = 100
    base              = 0
  }

  # Cost optimization: Public subnet eliminates NAT Gateway
  network_configuration {
    subnets          = aws_subnet.public[*].id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = true
  }

  tags = {
    Name = "${local.name_prefix}-event-processor"
  }
}

# Action Workers: Fargate Spot + Public Subnet
resource "aws_ecs_service" "action_workers" {
  name                               = "action-workers"
  cluster                            = aws_ecs_cluster.main.id
  task_definition                    = aws_ecs_task_definition.action_workers.arn
  desired_count                      = var.action_workers_desired_count
  platform_version                   = "LATEST"
  deployment_minimum_healthy_percent = 50
  deployment_maximum_percent         = 200
  # ECS Exec disabled in production for security (only api-gateway allows debug access)
  enable_execute_command = var.environment != "production"

  # Cost optimization: Fargate Spot saves ~70%
  capacity_provider_strategy {
    capacity_provider = "FARGATE_SPOT"
    weight            = 100
    base              = 0
  }

  # Cost optimization: Public subnet eliminates NAT Gateway
  network_configuration {
    subnets          = aws_subnet.public[*].id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = true
  }

  tags = {
    Name = "${local.name_prefix}-action-workers"
  }

  lifecycle {
    ignore_changes = [desired_count]
  }
}

# -----------------------------------------------------------------------------
# Ponder Indexer Task Definition and Service
# -----------------------------------------------------------------------------
# ARCHITECTURE NOTE: Ponder Indexer
#
# Ponder indexes blockchain events which are PUBLIC and IMMUTABLE.
# It uses a dedicated PostgreSQL schema ('ponder') to isolate blockchain
# events from application data.
#
# Configuration:
# - Set ponder_indexer_enabled = true to deploy
# - Uses dedicated 'ponder' schema in the production RDS
# - Indexes events from multiple blockchain networks
#
# The Ponder code automatically configures search_path to use the 'ponder'
# schema via the DATABASE_URL connection string options.
# -----------------------------------------------------------------------------

resource "aws_ecs_task_definition" "ponder_indexer" {
  count = var.ponder_indexer_enabled ? 1 : 0

  family                   = "${local.name_prefix}-ponder-indexer"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.ponder_indexer_cpu
  memory                   = var.ponder_indexer_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  container_definitions = jsonencode([
    {
      name      = "ponder-indexer"
      image     = var.ponder_indexer_image != "" ? "${var.ponder_indexer_image}:${var.ponder_indexer_image_tag}" : "${aws_ecr_repository.backend.repository_url}:ponder-${var.ponder_indexer_image_tag}"
      essential = true

      portMappings = [
        {
          containerPort = 42069
          hostPort      = 42069
          protocol      = "tcp"
        }
      ]

      environment = [
        { name = "NODE_ENV", value = "production" },
        { name = "PONDER_LOG_LEVEL", value = "info" },
        { name = "ENVIRONMENT", value = "production" },
        { name = "PONDER_DATABASE_SCHEMA", value = var.ponder_database_schema },
        # TLS validation enabled - RDS CA bundle included in Docker image via NODE_EXTRA_CA_CERTS

        # RPC Rate Limits - reduced for free tier endpoints
        # Default is 30 req/s but free tiers only support ~5 req/s
        { name = "RPC_RATE_LIMIT_ANKR", value = "5" },
        { name = "RPC_RATE_LIMIT_INFURA", value = "5" },
        { name = "RPC_RATE_LIMIT_ALCHEMY", value = "5" },
        { name = "RPC_RATE_LIMIT_QUIKNODE", value = "5" },

        # ERC-8004 Contract Addresses - Ethereum Sepolia
        { name = "ETHEREUM_SEPOLIA_IDENTITY_ADDRESS", value = "0x8004a6090Cd10A7288092483047B097295Fb8847" },
        { name = "ETHEREUM_SEPOLIA_REPUTATION_ADDRESS", value = "0x8004B8FD1A363aa02fDC07635C0c5F94f6Af5B7E" },
        { name = "ETHEREUM_SEPOLIA_VALIDATION_ADDRESS", value = "0x8004CB39f29c09145F24Ad9dDe2A108C1A2cdfC5" },

        # ERC-8004 Contract Addresses - Base Sepolia
        { name = "BASE_SEPOLIA_IDENTITY_ADDRESS", value = "0x8004AA63c570c570eBF15376c0dB199918BFe9Fb" },
        { name = "BASE_SEPOLIA_REPUTATION_ADDRESS", value = "0x8004bd8daB57f14Ed299135749a5CB5c42d341BF" },
        { name = "BASE_SEPOLIA_VALIDATION_ADDRESS", value = "0x8004C269D0A5647E51E121FeB226200ECE932d55" },

        # ERC-8004 Contract Addresses - Linea Sepolia
        { name = "LINEA_SEPOLIA_IDENTITY_ADDRESS", value = "0x8004aa7C931bCE1233973a0C6A667f73F66282e7" },
        { name = "LINEA_SEPOLIA_REPUTATION_ADDRESS", value = "0x8004bd8483b99310df121c46ED8858616b2Bba02" },
        { name = "LINEA_SEPOLIA_VALIDATION_ADDRESS", value = "0x8004c44d1EFdd699B2A26e781eF7F77c56A9a4EB" }
      ]

      secrets = [
        {
          # DATABASE_URL from Secrets Manager - Ponder code adds schema option automatically
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.rds_password.arn}:url::"
        },
        # ---------------------------------------------------------------------
        # Multi-RPC Configuration
        # Ponder uses these for automatic failover with smart ranking:
        # - 2-3 providers: fallback with ranking (30% latency, 70% stability)
        # - 4+ providers: load-balanced pools
        # Naming: {CHAIN}_RPC_{PROVIDER} for Ankr, {CHAIN}_RPC_URL for fallback
        # ---------------------------------------------------------------------

        # Ethereum Sepolia - Primary (Ankr) and Fallback (public)
        {
          name      = "ETHEREUM_SEPOLIA_RPC_ANKR"
          valueFrom = aws_secretsmanager_secret.eth_sepolia_rpc_ankr.arn
        },
        {
          name      = "ETHEREUM_SEPOLIA_RPC_URL"
          valueFrom = aws_secretsmanager_secret.alchemy_api_key.arn
        },

        # Base Sepolia - Primary (Ankr) and Fallback (public)
        {
          name      = "BASE_SEPOLIA_RPC_ANKR"
          valueFrom = aws_secretsmanager_secret.base_sepolia_rpc_ankr.arn
        },
        {
          name      = "BASE_SEPOLIA_RPC_URL"
          valueFrom = aws_secretsmanager_secret.base_sepolia_rpc.arn
        },

        # Linea Sepolia - Only public RPC available (Ankr doesn't support Linea)
        {
          name      = "LINEA_SEPOLIA_RPC_URL"
          valueFrom = aws_secretsmanager_secret.linea_sepolia_rpc.arn
        }
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ponder_indexer.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }

      healthCheck = {
        command     = ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:42069/health || exit 1"]
        interval    = 30
        timeout     = 10
        retries     = 3
        startPeriod = 60
      }
    }
  ])

  tags = {
    Name        = "${local.name_prefix}-ponder-indexer"
    SharedBy    = "all-environments"
    Description = "Shared blockchain indexer for ERC-8004 events"
  }
}

# Ponder Indexer uses Fargate Spot for ~70% cost savings
# Indexer is fault-tolerant: missed events are re-indexed on restart
resource "aws_ecs_service" "ponder_indexer" {
  count = var.ponder_indexer_enabled ? 1 : 0

  name                               = "ponder-indexer"
  cluster                            = aws_ecs_cluster.main.id
  task_definition                    = aws_ecs_task_definition.ponder_indexer[0].arn
  desired_count                      = 1
  platform_version                   = "LATEST"
  deployment_minimum_healthy_percent = 0
  deployment_maximum_percent         = 100
  # ECS Exec disabled in production for security (only api-gateway allows debug access)
  enable_execute_command = var.environment != "production"

  # Cost optimization: Fargate Spot saves ~70%
  capacity_provider_strategy {
    capacity_provider = "FARGATE_SPOT"
    weight            = 100
    base              = 0
  }

  # Cost optimization: Public subnet eliminates NAT Gateway
  network_configuration {
    subnets          = aws_subnet.public[*].id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = true
  }

  tags = {
    Name        = "${local.name_prefix}-ponder-indexer"
    SharedBy    = "all-environments"
    Description = "Shared blockchain indexer for ERC-8004 events"
  }
}

# -----------------------------------------------------------------------------
# ECS Auto Scaling - API Gateway
# -----------------------------------------------------------------------------
# Automatically scales the API Gateway service based on CPU utilization.
# Scales out when CPU > 70%, scales in when CPU < 50%.
# Min: 1 task, Max: 4 tasks for cost control.

resource "aws_appautoscaling_target" "api_gateway" {
  max_capacity       = var.environment == "production" ? 4 : 2
  min_capacity       = 1
  resource_id        = "service/${aws_ecs_cluster.main.name}/api-gateway"
  scalable_dimension = "ecs:service:DesiredCount"
  service_namespace  = "ecs"

  depends_on = [aws_ecs_service.api_gateway]
}

resource "aws_appautoscaling_policy" "api_gateway_cpu" {
  name               = "${local.name_prefix}-api-gateway-cpu-scaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.api_gateway.resource_id
  scalable_dimension = aws_appautoscaling_target.api_gateway.scalable_dimension
  service_namespace  = aws_appautoscaling_target.api_gateway.service_namespace

  target_tracking_scaling_policy_configuration {
    target_value       = 70.0
    scale_in_cooldown  = 300 # 5 minutes before scale in
    scale_out_cooldown = 60  # 1 minute before scale out (respond quickly)

    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}

# Optional: Scale based on ALB request count per target
# Note: ALBRequestCountPerTarget autoscaling policy removed after ALB migration to API Gateway HTTP
# CPU-based autoscaling (api_gateway_cpu policy above) is still active
