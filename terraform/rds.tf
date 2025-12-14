# =============================================================================
# Amazon RDS - PostgreSQL with TimescaleDB
# =============================================================================

# -----------------------------------------------------------------------------
# DB Subnet Group
# -----------------------------------------------------------------------------

resource "aws_db_subnet_group" "main" {
  name       = local.name_prefix
  subnet_ids = aws_subnet.private[*].id

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# RDS Parameter Group (PostgreSQL 15 with TimescaleDB)
# -----------------------------------------------------------------------------

resource "aws_db_parameter_group" "main" {
  family = "postgres15"
  name   = local.name_prefix

  # TLS/SSL - Enforce encrypted connections in production
  parameter {
    name         = "rds.force_ssl"
    value        = var.environment == "production" ? "1" : "0"
    apply_method = "pending-reboot"
  }

  # Extensions (note: TimescaleDB not available on standard RDS)
  parameter {
    name  = "shared_preload_libraries"
    value = "pg_stat_statements"
  }

  # Logging
  parameter {
    name  = "log_min_duration_statement"
    value = "1000" # Log queries taking > 1 second
  }

  parameter {
    name  = "log_statement"
    value = "ddl" # Log DDL statements
  }

  # SSL logging for audit
  parameter {
    name  = "log_connections"
    value = "1"
  }

  parameter {
    name  = "log_disconnections"
    value = "1"
  }

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# RDS Instance
# -----------------------------------------------------------------------------

resource "aws_db_instance" "main" {
  identifier = local.name_prefix

  # Engine configuration
  engine                = "postgres"
  engine_version        = "15.10"
  instance_class        = var.db_instance_class
  allocated_storage     = var.db_allocated_storage
  max_allocated_storage = var.db_max_allocated_storage
  storage_type          = "gp3"
  storage_encrypted     = true

  # Database configuration
  db_name  = "agentauri_backend"
  username = "agentauri_admin"
  password = random_password.rds_password.result

  # Network configuration
  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  publicly_accessible    = false
  port                   = 5432

  # Parameter and option groups
  parameter_group_name = aws_db_parameter_group.main.name

  # Backup configuration (free tier allows max 7 days, staging uses 1 day)
  backup_retention_period   = var.environment == "production" ? 30 : 1
  backup_window             = "03:00-04:00"
  maintenance_window        = "Mon:04:00-Mon:05:00"
  copy_tags_to_snapshot     = true
  delete_automated_backups  = var.environment != "production"
  deletion_protection       = var.environment == "production"
  skip_final_snapshot       = var.environment != "production"
  final_snapshot_identifier = var.environment == "production" ? "${local.name_prefix}-final" : null

  # High availability
  multi_az = var.db_multi_az

  # IAM Database Authentication (optional, for passwordless auth from ECS)
  iam_database_authentication_enabled = var.environment == "production"

  # Certificate Authority (for SSL connections)
  ca_cert_identifier = "rds-ca-rsa2048-g1"

  # Performance Insights
  performance_insights_enabled          = true
  performance_insights_retention_period = var.environment == "production" ? 7 : 7

  # Monitoring
  monitoring_interval = 60
  monitoring_role_arn = aws_iam_role.rds_monitoring.arn

  tags = {
    Name = local.name_prefix
  }
}

# -----------------------------------------------------------------------------
# Random Password for RDS
# -----------------------------------------------------------------------------

resource "random_password" "rds_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

# -----------------------------------------------------------------------------
# Store RDS Password in Secrets Manager
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "rds_password" {
  name                    = "agentauri/${var.environment}/rds-password"
  description             = "RDS master password for AgentAuri ${var.environment}"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-rds-password"
  }
}

resource "aws_secretsmanager_secret_version" "rds_password" {
  secret_id = aws_secretsmanager_secret.rds_password.id
  secret_string = jsonencode({
    username = aws_db_instance.main.username
    password = random_password.rds_password.result
    host     = aws_db_instance.main.address
    port     = aws_db_instance.main.port
    database = aws_db_instance.main.db_name
    # Connection URL with TLS/SSL enforced (production uses sslmode=verify-full)
    url = var.environment == "production" ? (
      "postgres://${aws_db_instance.main.username}:${urlencode(random_password.rds_password.result)}@${aws_db_instance.main.address}:${aws_db_instance.main.port}/${aws_db_instance.main.db_name}?sslmode=verify-full&sslrootcert=/app/certs/aws-rds-ca.pem"
      ) : (
      "postgres://${aws_db_instance.main.username}:${urlencode(random_password.rds_password.result)}@${aws_db_instance.main.address}:${aws_db_instance.main.port}/${aws_db_instance.main.db_name}?sslmode=require"
    )
    # Additional SSL URLs for different verification levels
    url_require     = "postgres://${aws_db_instance.main.username}:${urlencode(random_password.rds_password.result)}@${aws_db_instance.main.address}:${aws_db_instance.main.port}/${aws_db_instance.main.db_name}?sslmode=require"
    url_verify_ca   = "postgres://${aws_db_instance.main.username}:${urlencode(random_password.rds_password.result)}@${aws_db_instance.main.address}:${aws_db_instance.main.port}/${aws_db_instance.main.db_name}?sslmode=verify-ca&sslrootcert=/app/certs/aws-rds-ca.pem"
    url_verify_full = "postgres://${aws_db_instance.main.username}:${urlencode(random_password.rds_password.result)}@${aws_db_instance.main.address}:${aws_db_instance.main.port}/${aws_db_instance.main.db_name}?sslmode=verify-full&sslrootcert=/app/certs/aws-rds-ca.pem"
  })
}

# -----------------------------------------------------------------------------
# RDS Enhanced Monitoring Role
# -----------------------------------------------------------------------------

resource "aws_iam_role" "rds_monitoring" {
  name = "${local.name_prefix}-rds-monitoring"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "monitoring.rds.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "${local.name_prefix}-rds-monitoring"
  }
}

resource "aws_iam_role_policy_attachment" "rds_monitoring" {
  role       = aws_iam_role.rds_monitoring.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}
