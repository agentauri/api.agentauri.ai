# =============================================================================
# ECS Auto Scaling & CloudWatch Alarms
# =============================================================================
# Phase 7: Scaling - Comprehensive auto-scaling and monitoring
#
# Services scaled:
# - API Gateway: CPU, Memory, Request count (existing + enhanced)
# - Action Workers: CPU, Memory, Queue depth
# - Event Processor: Single instance (no scaling - state machine)
# - Ponder Indexer: Single instance (blockchain sync)
#
# Alarm categories:
# - Service health (task failures, restarts)
# - Resource utilization (CPU, memory thresholds)
# - Application performance (response times, error rates)
# - Database health (connections, CPU, storage)

# -----------------------------------------------------------------------------
# SNS Topic for Alerts
# -----------------------------------------------------------------------------

resource "aws_sns_topic" "alerts" {
  name = "${local.name_prefix}-alerts"

  tags = {
    Name = "${local.name_prefix}-alerts"
  }
}

resource "aws_sns_topic_subscription" "email_alerts" {
  count     = var.alert_email != "" ? 1 : 0
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

# -----------------------------------------------------------------------------
# Action Workers Auto Scaling
# -----------------------------------------------------------------------------

resource "aws_appautoscaling_target" "action_workers" {
  max_capacity       = var.environment == "production" ? 6 : 2
  min_capacity       = 1
  resource_id        = "service/${aws_ecs_cluster.main.name}/action-workers"
  scalable_dimension = "ecs:service:DesiredCount"
  service_namespace  = "ecs"

  depends_on = [aws_ecs_service.action_workers]
}

# Scale based on CPU utilization
resource "aws_appautoscaling_policy" "action_workers_cpu" {
  name               = "${local.name_prefix}-action-workers-cpu-scaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.action_workers.resource_id
  scalable_dimension = aws_appautoscaling_target.action_workers.scalable_dimension
  service_namespace  = aws_appautoscaling_target.action_workers.service_namespace

  target_tracking_scaling_policy_configuration {
    target_value       = 70.0
    scale_in_cooldown  = 300
    scale_out_cooldown = 60

    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}

# Scale based on memory utilization
resource "aws_appautoscaling_policy" "action_workers_memory" {
  name               = "${local.name_prefix}-action-workers-memory-scaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.action_workers.resource_id
  scalable_dimension = aws_appautoscaling_target.action_workers.scalable_dimension
  service_namespace  = aws_appautoscaling_target.action_workers.service_namespace

  target_tracking_scaling_policy_configuration {
    target_value       = 80.0
    scale_in_cooldown  = 300
    scale_out_cooldown = 60

    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageMemoryUtilization"
    }
  }
}

# -----------------------------------------------------------------------------
# API Gateway Memory Scaling (supplement to existing CPU scaling)
# -----------------------------------------------------------------------------

resource "aws_appautoscaling_policy" "api_gateway_memory" {
  name               = "${local.name_prefix}-api-gateway-memory-scaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.api_gateway.resource_id
  scalable_dimension = aws_appautoscaling_target.api_gateway.scalable_dimension
  service_namespace  = aws_appautoscaling_target.api_gateway.service_namespace

  target_tracking_scaling_policy_configuration {
    target_value       = 80.0
    scale_in_cooldown  = 300
    scale_out_cooldown = 60

    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageMemoryUtilization"
    }
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - Service Health
# -----------------------------------------------------------------------------

# API Gateway - High CPU
resource "aws_cloudwatch_metric_alarm" "api_gateway_cpu_high" {
  alarm_name          = "${local.name_prefix}-api-gateway-cpu-high"
  alarm_description   = "API Gateway CPU utilization is critically high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ECS"
  period              = 60
  statistic           = "Average"
  threshold           = 85
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "api-gateway"
  }

  tags = {
    Name = "${local.name_prefix}-api-gateway-cpu-high"
  }
}

# API Gateway - High Memory
resource "aws_cloudwatch_metric_alarm" "api_gateway_memory_high" {
  alarm_name          = "${local.name_prefix}-api-gateway-memory-high"
  alarm_description   = "API Gateway memory utilization is critically high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "MemoryUtilization"
  namespace           = "AWS/ECS"
  period              = 60
  statistic           = "Average"
  threshold           = 90
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "api-gateway"
  }

  tags = {
    Name = "${local.name_prefix}-api-gateway-memory-high"
  }
}

# API Gateway - No Running Tasks
resource "aws_cloudwatch_metric_alarm" "api_gateway_no_tasks" {
  alarm_name          = "${local.name_prefix}-api-gateway-no-tasks"
  alarm_description   = "CRITICAL: API Gateway has no running tasks"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "RunningTaskCount"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 1
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]
  treat_missing_data  = "breaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "api-gateway"
  }

  tags = {
    Name     = "${local.name_prefix}-api-gateway-no-tasks"
    Severity = "critical"
  }
}

# Event Processor - No Running Tasks
resource "aws_cloudwatch_metric_alarm" "event_processor_no_tasks" {
  alarm_name          = "${local.name_prefix}-event-processor-no-tasks"
  alarm_description   = "CRITICAL: Event Processor has no running tasks"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "RunningTaskCount"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 1
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]
  treat_missing_data  = "breaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "event-processor"
  }

  tags = {
    Name     = "${local.name_prefix}-event-processor-no-tasks"
    Severity = "critical"
  }
}

# Action Workers - No Running Tasks
resource "aws_cloudwatch_metric_alarm" "action_workers_no_tasks" {
  alarm_name          = "${local.name_prefix}-action-workers-no-tasks"
  alarm_description   = "CRITICAL: Action Workers has no running tasks"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "RunningTaskCount"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 1
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]
  treat_missing_data  = "breaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "action-workers"
  }

  tags = {
    Name     = "${local.name_prefix}-action-workers-no-tasks"
    Severity = "critical"
  }
}

# Action Workers - High CPU
resource "aws_cloudwatch_metric_alarm" "action_workers_cpu_high" {
  alarm_name          = "${local.name_prefix}-action-workers-cpu-high"
  alarm_description   = "Action Workers CPU utilization is critically high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ECS"
  period              = 60
  statistic           = "Average"
  threshold           = 85
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "action-workers"
  }

  tags = {
    Name = "${local.name_prefix}-action-workers-cpu-high"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - API Gateway HTTP (replaces ALB alarms)
# -----------------------------------------------------------------------------
# Note: API Gateway HTTP metrics available under AWS/ApiGateway namespace
# Metrics: Count, 4XXError, 5XXError, Latency, IntegrationLatency

# API Gateway - High 5xx Error Rate
resource "aws_cloudwatch_metric_alarm" "api_gw_5xx_high" {
  alarm_name          = "${local.name_prefix}-api-gw-5xx-high"
  alarm_description   = "API Gateway 5xx error rate is high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "5XXError"
  namespace           = "AWS/ApiGateway"
  period              = 60
  statistic           = "Sum"
  threshold           = 10
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]
  treat_missing_data  = "notBreaching"

  dimensions = {
    ApiId = aws_apigatewayv2_api.main.id
  }

  tags = {
    Name = "${local.name_prefix}-api-gw-5xx-high"
  }
}

# API Gateway - High Latency
resource "aws_cloudwatch_metric_alarm" "api_gw_latency_high" {
  alarm_name          = "${local.name_prefix}-api-gw-latency-high"
  alarm_description   = "API Gateway latency is high (>2s)"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "Latency"
  namespace           = "AWS/ApiGateway"
  period              = 60
  statistic           = "Average"
  threshold           = 2000.0 # milliseconds
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    ApiId = aws_apigatewayv2_api.main.id
  }

  tags = {
    Name = "${local.name_prefix}-api-gw-latency-high"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - RDS Health
# -----------------------------------------------------------------------------

# RDS - High CPU
resource "aws_cloudwatch_metric_alarm" "rds_cpu_high" {
  alarm_name          = "${local.name_prefix}-rds-cpu-high"
  alarm_description   = "RDS CPU utilization is high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 80
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  tags = {
    Name = "${local.name_prefix}-rds-cpu-high"
  }
}

# RDS - Low Free Storage
resource "aws_cloudwatch_metric_alarm" "rds_storage_low" {
  alarm_name          = "${local.name_prefix}-rds-storage-low"
  alarm_description   = "RDS free storage is low (<5GB)"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = 5368709120 # 5GB in bytes
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  tags = {
    Name     = "${local.name_prefix}-rds-storage-low"
    Severity = "warning"
  }
}

# RDS - High Connection Count
resource "aws_cloudwatch_metric_alarm" "rds_connections_high" {
  alarm_name          = "${local.name_prefix}-rds-connections-high"
  alarm_description   = "RDS connection count is high (>80% of max)"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 80 # db.t3.medium has ~100 max connections
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  tags = {
    Name = "${local.name_prefix}-rds-connections-high"
  }
}

# RDS - Low Freeable Memory
resource "aws_cloudwatch_metric_alarm" "rds_memory_low" {
  alarm_name          = "${local.name_prefix}-rds-memory-low"
  alarm_description   = "RDS freeable memory is low (<256MB)"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 3
  metric_name         = "FreeableMemory"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 268435456 # 256MB in bytes
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  tags = {
    Name     = "${local.name_prefix}-rds-memory-low"
    Severity = "warning"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - Redis Health (only when using ElastiCache)
# -----------------------------------------------------------------------------

# Redis - High CPU
resource "aws_cloudwatch_metric_alarm" "redis_cpu_high" {
  count = var.redis_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-redis-cpu-high"
  alarm_description   = "Redis CPU utilization is high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ElastiCache"
  period              = 60
  statistic           = "Average"
  threshold           = 80
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    CacheClusterId = aws_elasticache_replication_group.main[0].id
  }

  tags = {
    Name = "${local.name_prefix}-redis-cpu-high"
  }
}

# Redis - High Memory
resource "aws_cloudwatch_metric_alarm" "redis_memory_high" {
  count = var.redis_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-redis-memory-high"
  alarm_description   = "Redis memory utilization is high (>80%)"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "DatabaseMemoryUsagePercentage"
  namespace           = "AWS/ElastiCache"
  period              = 60
  statistic           = "Average"
  threshold           = 80
  alarm_actions       = [aws_sns_topic.alerts.arn]
  ok_actions          = [aws_sns_topic.alerts.arn]

  dimensions = {
    CacheClusterId = aws_elasticache_replication_group.main[0].id
  }

  tags = {
    Name     = "${local.name_prefix}-redis-memory-high"
    Severity = "warning"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Dashboard
# -----------------------------------------------------------------------------

locals {
  # Base dashboard widgets (always present)
  dashboard_base_widgets = [
    # Row 1: ECS Service Health
    {
      type   = "metric"
      x      = 0
      y      = 0
      width  = 8
      height = 6
      properties = {
        title  = "ECS Running Tasks"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["ECS/ContainerInsights", "RunningTaskCount", "ClusterName", aws_ecs_cluster.main.name, "ServiceName", "api-gateway"],
          ["...", "event-processor"],
          ["...", "action-workers"]
        ]
        period = 60
      }
    },
    {
      type   = "metric"
      x      = 8
      y      = 0
      width  = 8
      height = 6
      properties = {
        title  = "ECS CPU Utilization"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ECS", "CPUUtilization", "ClusterName", aws_ecs_cluster.main.name, "ServiceName", "api-gateway"],
          ["...", "event-processor"],
          ["...", "action-workers"]
        ]
        period = 60
      }
    },
    {
      type   = "metric"
      x      = 16
      y      = 0
      width  = 8
      height = 6
      properties = {
        title  = "ECS Memory Utilization"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ECS", "MemoryUtilization", "ClusterName", aws_ecs_cluster.main.name, "ServiceName", "api-gateway"],
          ["...", "event-processor"],
          ["...", "action-workers"]
        ]
        period = 60
      }
    },
    # Row 2: API Gateway Metrics
    {
      type   = "metric"
      x      = 0
      y      = 6
      width  = 8
      height = 6
      properties = {
        title  = "API Gateway Request Count"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ApiGateway", "Count", "ApiId", aws_apigatewayv2_api.main.id]
        ]
        period = 60
        stat   = "Sum"
      }
    },
    {
      type   = "metric"
      x      = 8
      y      = 6
      width  = 8
      height = 6
      properties = {
        title  = "API Gateway Latency"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ApiGateway", "Latency", "ApiId", aws_apigatewayv2_api.main.id],
          [".", "IntegrationLatency", ".", "."]
        ]
        period = 60
        stat   = "Average"
      }
    },
    {
      type   = "metric"
      x      = 16
      y      = 6
      width  = 8
      height = 6
      properties = {
        title  = "API Gateway Error Rates"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ApiGateway", "4XXError", "ApiId", aws_apigatewayv2_api.main.id],
          [".", "5XXError", ".", "."]
        ]
        period = 60
        stat   = "Sum"
      }
    },
    # Row 3: RDS Metrics
    {
      type   = "metric"
      x      = 0
      y      = 12
      width  = 8
      height = 6
      properties = {
        title  = "RDS CPU & Connections"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/RDS", "CPUUtilization", "DBInstanceIdentifier", aws_db_instance.main.identifier],
          [".", "DatabaseConnections", ".", ".", { yAxis = "right" }]
        ]
        period = 60
      }
    },
    {
      type   = "metric"
      x      = 8
      y      = 12
      width  = 8
      height = 6
      properties = {
        title  = "RDS Memory & Storage"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/RDS", "FreeableMemory", "DBInstanceIdentifier", aws_db_instance.main.identifier],
          [".", "FreeStorageSpace", ".", ".", { yAxis = "right" }]
        ]
        period = 60
      }
    }
  ]

  # Redis widget (only when using ElastiCache)
  dashboard_redis_widget = var.redis_enabled ? [
    {
      type   = "metric"
      x      = 16
      y      = 12
      width  = 8
      height = 6
      properties = {
        title  = "Redis CPU & Memory"
        view   = "timeSeries"
        region = var.aws_region
        metrics = [
          ["AWS/ElastiCache", "CPUUtilization", "CacheClusterId", aws_elasticache_replication_group.main[0].id],
          [".", "DatabaseMemoryUsagePercentage", ".", ".", { yAxis = "right" }]
        ]
        period = 60
      }
    }
  ] : []

  # Combined dashboard widgets
  dashboard_widgets = concat(local.dashboard_base_widgets, local.dashboard_redis_widget)
}

resource "aws_cloudwatch_dashboard" "main" {
  dashboard_name = "${local.name_prefix}-overview"

  dashboard_body = jsonencode({
    widgets = local.dashboard_widgets
  })
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "sns_alerts_topic_arn" {
  description = "ARN of the SNS topic for alerts"
  value       = aws_sns_topic.alerts.arn
}

output "cloudwatch_dashboard_url" {
  description = "URL for the CloudWatch dashboard"
  value       = "https://${var.aws_region}.console.aws.amazon.com/cloudwatch/home?region=${var.aws_region}#dashboards:name=${aws_cloudwatch_dashboard.main.dashboard_name}"
}
