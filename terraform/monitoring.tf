# =============================================================================
# Ponder Indexer Monitoring - CloudWatch Alarms & SNS Notifications
# =============================================================================
# This module provides monitoring for the Ponder blockchain indexer including:
# - SNS topic for alert notifications (email/SMS)
# - CloudWatch alarms for task health, CPU, memory
# - Log metric filters for RPC errors and sync issues
# =============================================================================

# -----------------------------------------------------------------------------
# SNS Topic for Ponder Alerts
# -----------------------------------------------------------------------------

resource "aws_sns_topic" "ponder_alerts" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  name = "${local.name_prefix}-ponder-alerts"

  tags = {
    Name        = "${local.name_prefix}-ponder-alerts"
    Service     = "ponder-indexer"
    Description = "Alert notifications for Ponder blockchain indexer"
  }
}

resource "aws_sns_topic_subscription" "ponder_email" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled && var.alert_email != "" ? 1 : 0

  topic_arn = aws_sns_topic.ponder_alerts[0].arn
  protocol  = "email"
  endpoint  = var.alert_email
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - ECS Task Health
# -----------------------------------------------------------------------------

# CRITICAL: Ponder task is down (no running tasks)
resource "aws_cloudwatch_metric_alarm" "ponder_task_down" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-task-down"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "RunningTaskCount"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 1
  alarm_description   = "CRITICAL: Ponder indexer task is not running. Blockchain indexing has stopped."
  treat_missing_data  = "breaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "ponder-indexer"
  }

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]
  ok_actions    = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-task-down"
    Severity = "critical"
    Service  = "ponder-indexer"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Alarms - Resource Utilization
# -----------------------------------------------------------------------------

# WARNING: High CPU usage (>80% for 5 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_high_cpu" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "CpuUtilized"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "WARNING: Ponder indexer CPU utilization is above 80% for 5 minutes."
  treat_missing_data  = "notBreaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "ponder-indexer"
  }

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-high-cpu"
    Severity = "warning"
    Service  = "ponder-indexer"
  }
}

# CRITICAL: Very high CPU usage (>95% for 2 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_critical_cpu" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-critical-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "CpuUtilized"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 95
  alarm_description   = "CRITICAL: Ponder indexer CPU utilization is above 95%. Service may be degraded."
  treat_missing_data  = "notBreaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "ponder-indexer"
  }

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]
  ok_actions    = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-critical-cpu"
    Severity = "critical"
    Service  = "ponder-indexer"
  }
}

# WARNING: High memory usage (>80% for 5 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_high_memory" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-high-memory"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "MemoryUtilized"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "WARNING: Ponder indexer memory utilization is above 80% for 5 minutes."
  treat_missing_data  = "notBreaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "ponder-indexer"
  }

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-high-memory"
    Severity = "warning"
    Service  = "ponder-indexer"
  }
}

# CRITICAL: Very high memory usage (>95% for 2 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_critical_memory" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-critical-memory"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "MemoryUtilized"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = 95
  alarm_description   = "CRITICAL: Ponder indexer memory utilization is above 95%. OOM risk imminent."
  treat_missing_data  = "notBreaching"

  dimensions = {
    ClusterName = aws_ecs_cluster.main.name
    ServiceName = "ponder-indexer"
  }

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]
  ok_actions    = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-critical-memory"
    Severity = "critical"
    Service  = "ponder-indexer"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Log Metric Filters - RPC Errors
# -----------------------------------------------------------------------------

# Metric filter for RPC errors in logs
resource "aws_cloudwatch_log_metric_filter" "ponder_rpc_errors" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  name           = "${local.name_prefix}-ponder-rpc-errors"
  pattern        = "?\"RPC error\" ?\"provider failed\" ?\"circuit breaker\" ?\"OPEN\""
  log_group_name = aws_cloudwatch_log_group.ponder_indexer.name

  metric_transformation {
    name          = "PonderRPCErrors"
    namespace     = "AgentAuri/Ponder"
    value         = "1"
    default_value = "0"
  }
}

# Alarm for RPC errors (>10 errors in 5 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_rpc_errors" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-rpc-errors"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "PonderRPCErrors"
  namespace           = "AgentAuri/Ponder"
  period              = 300
  statistic           = "Sum"
  threshold           = 10
  alarm_description   = "WARNING: Ponder is experiencing RPC provider errors. Check provider health and connectivity."
  treat_missing_data  = "notBreaching"

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-rpc-errors"
    Severity = "warning"
    Service  = "ponder-indexer"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Log Metric Filters - Sync Activity
# -----------------------------------------------------------------------------

# Metric filter for successful block syncs
resource "aws_cloudwatch_log_metric_filter" "ponder_sync_activity" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  name           = "${local.name_prefix}-ponder-sync-activity"
  pattern        = "\"Synced block\""
  log_group_name = aws_cloudwatch_log_group.ponder_indexer.name

  metric_transformation {
    name          = "PonderSyncActivity"
    namespace     = "AgentAuri/Ponder"
    value         = "1"
    default_value = "0"
  }
}

# Alarm for sync stalled (no syncs for 10 minutes)
resource "aws_cloudwatch_metric_alarm" "ponder_sync_stalled" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  alarm_name          = "${local.name_prefix}-ponder-sync-stalled"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "PonderSyncActivity"
  namespace           = "AgentAuri/Ponder"
  period              = 300
  statistic           = "Sum"
  threshold           = 1
  alarm_description   = "CRITICAL: Ponder has not synced any blocks in 10 minutes. Blockchain indexing may be stalled."
  treat_missing_data  = "breaching"

  alarm_actions = [aws_sns_topic.ponder_alerts[0].arn]
  ok_actions    = [aws_sns_topic.ponder_alerts[0].arn]

  tags = {
    Name     = "${local.name_prefix}-ponder-sync-stalled"
    Severity = "critical"
    Service  = "ponder-indexer"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Log Metric Filters - Events Indexed
# -----------------------------------------------------------------------------

# Metric filter for events processed
resource "aws_cloudwatch_log_metric_filter" "ponder_events_indexed" {
  count = var.ponder_indexer_enabled && var.ponder_monitoring_enabled ? 1 : 0

  name           = "${local.name_prefix}-ponder-events-indexed"
  pattern        = "\"Event processed\""
  log_group_name = aws_cloudwatch_log_group.ponder_indexer.name

  metric_transformation {
    name          = "PonderEventsIndexed"
    namespace     = "AgentAuri/Ponder"
    value         = "1"
    default_value = "0"
  }
}
