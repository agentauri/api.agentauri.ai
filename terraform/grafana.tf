# =============================================================================
# Grafana - Monitoring Dashboard on AWS ECS
# =============================================================================
# Grafana deployment with:
# - EFS for persistent storage (dashboards, plugins, database)
# - CloudWatch as datasource (complements CloudWatch Alarms)
# - ALB routing on /grafana path
# - Secure admin password from Secrets Manager
# =============================================================================

# -----------------------------------------------------------------------------
# Variables
# -----------------------------------------------------------------------------

variable "grafana_enabled" {
  description = "Enable Grafana deployment"
  type        = bool
  default     = false
}

variable "grafana_cpu" {
  description = "CPU units for Grafana (1024 = 1 vCPU)"
  type        = number
  default     = 256
}

variable "grafana_memory" {
  description = "Memory in MB for Grafana"
  type        = number
  default     = 512
}

variable "grafana_image" {
  description = "Docker image for custom Grafana (full ECR URI)"
  type        = string
  default     = "781863585732.dkr.ecr.us-east-1.amazonaws.com/agentauri-grafana"
}

variable "grafana_image_tag" {
  description = "Docker image tag for custom Grafana image"
  type        = string
  default     = "v1.0.0"
}

# -----------------------------------------------------------------------------
# EFS Filesystem for Grafana Persistent Storage
# -----------------------------------------------------------------------------

resource "aws_efs_file_system" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  creation_token = "${local.name_prefix}-grafana"
  encrypted      = true

  performance_mode = "generalPurpose"
  throughput_mode  = "bursting"

  lifecycle_policy {
    transition_to_ia = "AFTER_30_DAYS"
  }

  tags = {
    Name = "${local.name_prefix}-grafana"
  }
}

resource "aws_efs_mount_target" "grafana" {
  count = var.grafana_enabled ? length(aws_subnet.private) : 0

  file_system_id  = aws_efs_file_system.grafana[0].id
  subnet_id       = aws_subnet.private[count.index].id
  security_groups = [aws_security_group.efs_grafana[0].id]
}

# EFS Access Point for Grafana (sets correct POSIX permissions)
# Grafana runs as UID 472, GID 0 inside the container
resource "aws_efs_access_point" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  file_system_id = aws_efs_file_system.grafana[0].id

  posix_user {
    uid = 472 # grafana user
    gid = 0   # root group
  }

  root_directory {
    path = "/grafana"
    creation_info {
      owner_uid   = 472
      owner_gid   = 0
      permissions = "755"
    }
  }

  tags = {
    Name = "${local.name_prefix}-grafana-ap"
  }
}

resource "aws_security_group" "efs_grafana" {
  count = var.grafana_enabled ? 1 : 0

  name        = "${local.name_prefix}-efs-grafana"
  description = "Security group for Grafana EFS mount targets"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "NFS from ECS tasks"
    from_port       = 2049
    to_port         = 2049
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name_prefix}-efs-grafana"
  }
}

# -----------------------------------------------------------------------------
# Grafana Admin Password Secret
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "grafana_admin" {
  count = var.grafana_enabled ? 1 : 0

  name                    = "agentauri/${var.environment}/grafana-admin"
  description             = "Grafana admin password"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-grafana-admin"
  }
}

resource "random_password" "grafana_admin" {
  count = var.grafana_enabled ? 1 : 0

  length  = 24
  special = false
}

resource "aws_secretsmanager_secret_version" "grafana_admin" {
  count = var.grafana_enabled ? 1 : 0

  secret_id     = aws_secretsmanager_secret.grafana_admin[0].id
  secret_string = random_password.grafana_admin[0].result
}

# -----------------------------------------------------------------------------
# CloudWatch Log Group
# -----------------------------------------------------------------------------

resource "aws_cloudwatch_log_group" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  name              = "/ecs/${local.name_prefix}/grafana"
  retention_in_days = var.environment == "production" ? 30 : 7

  tags = {
    Name = "${local.name_prefix}-grafana-logs"
  }
}

# -----------------------------------------------------------------------------
# ECS Task Definition
# -----------------------------------------------------------------------------

resource "aws_ecs_task_definition" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  family                   = "${local.name_prefix}-grafana"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.grafana_cpu
  memory                   = var.grafana_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.grafana_task[0].arn

  volume {
    name = "grafana-data"
    efs_volume_configuration {
      file_system_id     = aws_efs_file_system.grafana[0].id
      transit_encryption = "ENABLED"
      authorization_config {
        access_point_id = aws_efs_access_point.grafana[0].id
        iam             = "ENABLED"
      }
    }
  }

  container_definitions = jsonencode([
    {
      name      = "grafana"
      image     = "${var.grafana_image}:${var.grafana_image_tag}"
      essential = true

      portMappings = [
        {
          containerPort = 3000
          hostPort      = 3000
          protocol      = "tcp"
        }
      ]

      environment = [
        # Server settings
        { name = "GF_SERVER_ROOT_URL", value = "https://${var.domain_name}/grafana/" },
        { name = "GF_SERVER_SERVE_FROM_SUB_PATH", value = "true" },

        # Security
        { name = "GF_SECURITY_ADMIN_USER", value = "admin" },
        { name = "GF_SECURITY_DISABLE_GRAVATAR", value = "true" },

        # Auth - disable anonymous access
        { name = "GF_AUTH_ANONYMOUS_ENABLED", value = "false" },

        # Plugins
        { name = "GF_INSTALL_PLUGINS", value = "grafana-clock-panel,grafana-piechart-panel" },

        # AWS CloudWatch datasource (uses task role)
        { name = "GF_AWS_default_REGION", value = var.aws_region },
        { name = "AWS_SDK_LOAD_CONFIG", value = "true" },

        # Paths
        { name = "GF_PATHS_DATA", value = "/var/lib/grafana" },
        { name = "GF_PATHS_LOGS", value = "/var/log/grafana" },
        { name = "GF_PATHS_PLUGINS", value = "/var/lib/grafana/plugins" },
        { name = "GF_PATHS_PROVISIONING", value = "/etc/grafana/provisioning" }
      ]

      secrets = [
        {
          name      = "GF_SECURITY_ADMIN_PASSWORD"
          valueFrom = aws_secretsmanager_secret.grafana_admin[0].arn
        }
      ]

      mountPoints = [
        {
          sourceVolume  = "grafana-data"
          containerPath = "/var/lib/grafana"
          readOnly      = false
        }
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.grafana[0].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }

      healthCheck = {
        command     = ["CMD-SHELL", "wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }
    }
  ])

  tags = {
    Name = "${local.name_prefix}-grafana"
  }
}

# -----------------------------------------------------------------------------
# IAM Role for Grafana Task (CloudWatch access)
# -----------------------------------------------------------------------------

resource "aws_iam_role" "grafana_task" {
  count = var.grafana_enabled ? 1 : 0

  name = "${local.name_prefix}-grafana-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "${local.name_prefix}-grafana-task"
  }
}

resource "aws_iam_role_policy" "grafana_cloudwatch" {
  count = var.grafana_enabled ? 1 : 0

  name = "${local.name_prefix}-grafana-cloudwatch"
  role = aws_iam_role.grafana_task[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:DescribeAlarmsForMetric",
          "cloudwatch:DescribeAlarmHistory",
          "cloudwatch:DescribeAlarms",
          "cloudwatch:ListMetrics",
          "cloudwatch:GetMetricData",
          "cloudwatch:GetMetricStatistics",
          "cloudwatch:GetInsightRuleReport"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "logs:DescribeLogGroups",
          "logs:GetLogGroupFields",
          "logs:StartQuery",
          "logs:StopQuery",
          "logs:GetQueryResults",
          "logs:GetLogEvents"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "ec2:DescribeTags",
          "ec2:DescribeInstances",
          "ec2:DescribeRegions"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "tag:GetResources"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "elasticfilesystem:ClientMount",
          "elasticfilesystem:ClientWrite",
          "elasticfilesystem:ClientRootAccess"
        ]
        Resource = aws_efs_file_system.grafana[0].arn
        Condition = {
          StringEquals = {
            "elasticfilesystem:AccessPointArn" = aws_efs_access_point.grafana[0].arn
          }
        }
      }
    ]
  })
}

# -----------------------------------------------------------------------------
# ECS Service
# -----------------------------------------------------------------------------

resource "aws_ecs_service" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  name                               = "grafana"
  cluster                            = aws_ecs_cluster.main.id
  task_definition                    = aws_ecs_task_definition.grafana[0].arn
  desired_count                      = 1
  launch_type                        = "FARGATE"
  platform_version                   = "1.4.0" # Required for EFS
  health_check_grace_period_seconds  = 120
  deployment_minimum_healthy_percent = 100
  deployment_maximum_percent         = 200

  network_configuration {
    subnets          = aws_subnet.private[*].id
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.grafana[0].arn
    container_name   = "grafana"
    container_port   = 3000
  }

  depends_on = [
    aws_lb_listener_rule.grafana,
    aws_efs_mount_target.grafana
  ]

  tags = {
    Name = "${local.name_prefix}-grafana"
  }
}

# -----------------------------------------------------------------------------
# ALB Target Group and Listener Rule
# -----------------------------------------------------------------------------

resource "aws_lb_target_group" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  name        = "${local.name_prefix}-grafana"
  port        = 3000
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/api/health"
    port                = "traffic-port"
    protocol            = "HTTP"
    timeout             = 5
    unhealthy_threshold = 3
  }

  tags = {
    Name = "${local.name_prefix}-grafana"
  }
}

resource "aws_lb_listener_rule" "grafana" {
  count = var.grafana_enabled ? 1 : 0

  listener_arn = aws_lb_listener.https.arn
  priority     = 10

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.grafana[0].arn
  }

  condition {
    path_pattern {
      values = ["/grafana", "/grafana/*"]
    }
  }

  tags = {
    Name = "${local.name_prefix}-grafana-rule"
  }
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "grafana_url" {
  description = "Grafana dashboard URL"
  value       = var.grafana_enabled ? "https://${var.domain_name}/grafana/" : ""
}

output "grafana_admin_secret_arn" {
  description = "ARN of the Grafana admin password secret"
  value       = var.grafana_enabled ? aws_secretsmanager_secret.grafana_admin[0].arn : ""
}
