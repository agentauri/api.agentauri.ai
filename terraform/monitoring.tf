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

# =============================================================================
# RDS Monitoring - CloudWatch Alarms for Database Health
# =============================================================================
# These alarms monitor the primary RDS instance and prepare for multi-region
# by establishing baseline metrics and alerting patterns.
# =============================================================================

# -----------------------------------------------------------------------------
# SNS Topic for RDS Alerts (reuses existing or creates new)
# -----------------------------------------------------------------------------

resource "aws_sns_topic" "rds_alerts" {
  name = "${local.name_prefix}-rds-alerts"

  tags = {
    Name        = "${local.name_prefix}-rds-alerts"
    Service     = "rds"
    Description = "Alert notifications for RDS database"
  }
}

resource "aws_sns_topic_subscription" "rds_email" {
  count = var.alert_email != "" ? 1 : 0

  topic_arn = aws_sns_topic.rds_alerts.arn
  protocol  = "email"
  endpoint  = var.alert_email
}

# -----------------------------------------------------------------------------
# RDS Alarms - CPU Utilization
# -----------------------------------------------------------------------------

# WARNING: High CPU (>70% for 5 minutes)
resource "aws_cloudwatch_metric_alarm" "rds_high_cpu" {
  alarm_name          = "${local.name_prefix}-rds-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 70
  alarm_description   = "WARNING: RDS CPU utilization above 70% for 5 minutes. Consider query optimization or scaling."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-high-cpu"
    Severity = "warning"
    Service  = "rds"
  }
}

# CRITICAL: Very high CPU (>90% for 2 minutes)
resource "aws_cloudwatch_metric_alarm" "rds_critical_cpu" {
  alarm_name          = "${local.name_prefix}-rds-critical-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 90
  alarm_description   = "CRITICAL: RDS CPU utilization above 90%. Database performance severely impacted."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]
  ok_actions    = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-critical-cpu"
    Severity = "critical"
    Service  = "rds"
  }
}

# -----------------------------------------------------------------------------
# RDS Alarms - Storage
# -----------------------------------------------------------------------------

# WARNING: Low free storage (<20% of allocated)
resource "aws_cloudwatch_metric_alarm" "rds_low_storage" {
  alarm_name          = "${local.name_prefix}-rds-low-storage"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 1
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  # 20% of allocated storage (var.db_allocated_storage in GB, convert to bytes)
  threshold          = var.db_allocated_storage * 1024 * 1024 * 1024 * 0.2
  alarm_description  = "WARNING: RDS free storage below 20%. Storage autoscaling should handle this, but monitor closely."
  treat_missing_data = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-low-storage"
    Severity = "warning"
    Service  = "rds"
  }
}

# CRITICAL: Very low free storage (<10% of allocated)
resource "aws_cloudwatch_metric_alarm" "rds_critical_storage" {
  alarm_name          = "${local.name_prefix}-rds-critical-storage"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 1
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = var.db_allocated_storage * 1024 * 1024 * 1024 * 0.1
  alarm_description   = "CRITICAL: RDS free storage below 10%. Immediate action required."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]
  ok_actions    = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-critical-storage"
    Severity = "critical"
    Service  = "rds"
  }
}

# -----------------------------------------------------------------------------
# RDS Alarms - Connections
# -----------------------------------------------------------------------------

# WARNING: High connection count (>80% of max)
# db.t3.medium has ~150 max connections
resource "aws_cloudwatch_metric_alarm" "rds_high_connections" {
  alarm_name          = "${local.name_prefix}-rds-high-connections"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 120 # ~80% of 150
  alarm_description   = "WARNING: RDS connection count above 80% of max. Check for connection leaks or scale up."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-high-connections"
    Severity = "warning"
    Service  = "rds"
  }
}

# -----------------------------------------------------------------------------
# RDS Alarms - Memory
# -----------------------------------------------------------------------------

# WARNING: Low freeable memory (<256MB)
resource "aws_cloudwatch_metric_alarm" "rds_low_memory" {
  alarm_name          = "${local.name_prefix}-rds-low-memory"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 3
  metric_name         = "FreeableMemory"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 256 * 1024 * 1024 # 256 MB in bytes
  alarm_description   = "WARNING: RDS freeable memory below 256MB. Consider scaling up instance class."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-low-memory"
    Severity = "warning"
    Service  = "rds"
  }
}

# -----------------------------------------------------------------------------
# RDS Alarms - Read/Write Latency (Multi-Region Prep)
# -----------------------------------------------------------------------------

# WARNING: High read latency (>20ms average)
resource "aws_cloudwatch_metric_alarm" "rds_high_read_latency" {
  alarm_name          = "${local.name_prefix}-rds-high-read-latency"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "ReadLatency"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 0.02 # 20ms in seconds
  alarm_description   = "WARNING: RDS read latency above 20ms. Consider read replica for read-heavy workloads."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-high-read-latency"
    Severity = "warning"
    Service  = "rds"
  }
}

# WARNING: High write latency (>50ms average)
resource "aws_cloudwatch_metric_alarm" "rds_high_write_latency" {
  alarm_name          = "${local.name_prefix}-rds-high-write-latency"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "WriteLatency"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 0.05 # 50ms in seconds
  alarm_description   = "WARNING: RDS write latency above 50ms. Check for lock contention or I/O bottlenecks."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-high-write-latency"
    Severity = "warning"
    Service  = "rds"
  }
}

# -----------------------------------------------------------------------------
# RDS Alarms - IOPS (Multi-Region Prep)
# -----------------------------------------------------------------------------

# WARNING: High read IOPS (>80% of provisioned for gp3)
# gp3 baseline: 3000 IOPS
resource "aws_cloudwatch_metric_alarm" "rds_high_read_iops" {
  alarm_name          = "${local.name_prefix}-rds-high-read-iops"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 5
  metric_name         = "ReadIOPS"
  namespace           = "AWS/RDS"
  period              = 60
  statistic           = "Average"
  threshold           = 2400 # 80% of 3000
  alarm_description   = "WARNING: RDS read IOPS approaching gp3 baseline limit. Read replica would help."
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = [aws_sns_topic.rds_alerts.arn]

  tags = {
    Name     = "${local.name_prefix}-rds-high-read-iops"
    Severity = "warning"
    Service  = "rds"
  }
}
