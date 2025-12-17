# =============================================================================
# AgentAuri Infrastructure - Variables
# =============================================================================

# -----------------------------------------------------------------------------
# General
# -----------------------------------------------------------------------------

variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name (staging, production)"
  type        = string
  validation {
    condition     = contains(["staging", "production"], var.environment)
    error_message = "Environment must be 'staging' or 'production'."
  }
}

# -----------------------------------------------------------------------------
# Networking
# -----------------------------------------------------------------------------

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones_count" {
  description = "Number of availability zones to use"
  type        = number
  default     = 2
}

# -----------------------------------------------------------------------------
# ECS Configuration
# -----------------------------------------------------------------------------

variable "api_gateway_cpu" {
  description = "CPU units for API Gateway (1024 = 1 vCPU)"
  type        = number
  default     = 512
}

variable "api_gateway_memory" {
  description = "Memory in MB for API Gateway"
  type        = number
  default     = 1024
}

variable "api_gateway_desired_count" {
  description = "Desired number of API Gateway tasks"
  type        = number
  default     = 2
}

variable "event_processor_cpu" {
  description = "CPU units for Event Processor"
  type        = number
  default     = 256
}

variable "event_processor_memory" {
  description = "Memory in MB for Event Processor"
  type        = number
  default     = 512
}

variable "action_workers_cpu" {
  description = "CPU units for Action Workers"
  type        = number
  default     = 256
}

variable "action_workers_memory" {
  description = "Memory in MB for Action Workers"
  type        = number
  default     = 512
}

variable "action_workers_desired_count" {
  description = "Desired number of Action Worker tasks"
  type        = number
  default     = 2
}

variable "ponder_indexer_cpu" {
  description = "CPU units for Ponder Indexer"
  type        = number
  default     = 512
}

variable "ponder_indexer_memory" {
  description = "Memory in MB for Ponder Indexer"
  type        = number
  default     = 1024
}

variable "ponder_indexer_image" {
  description = "Docker image for Ponder Indexer"
  type        = string
  default     = ""
}

variable "ponder_indexer_image_tag" {
  description = "Docker image tag for Ponder Indexer"
  type        = string
  default     = "latest"
}

variable "ponder_indexer_enabled" {
  description = "Enable Ponder Indexer deployment. Set to true only in ONE workspace (staging) since Ponder indexes public blockchain data that is shared across all environments."
  type        = bool
  default     = false
}

variable "ponder_database_schema" {
  description = "PostgreSQL schema for Ponder tables (Event, Checkpoint). Ponder uses a dedicated schema to isolate blockchain events from application data."
  type        = string
  default     = "ponder"
}

# -----------------------------------------------------------------------------
# Database (RDS)
# -----------------------------------------------------------------------------

variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "db_allocated_storage" {
  description = "Allocated storage in GB"
  type        = number
  default     = 20
}

variable "db_max_allocated_storage" {
  description = "Max allocated storage for autoscaling"
  type        = number
  default     = 100
}

variable "db_multi_az" {
  description = "Enable Multi-AZ deployment"
  type        = bool
  default     = true
}

variable "db_backup_retention_period" {
  description = "Backup retention period in days"
  type        = number
  default     = 7
}

# -----------------------------------------------------------------------------
# Redis (ElastiCache)
# -----------------------------------------------------------------------------

variable "redis_node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t3.micro"
}

variable "redis_num_cache_nodes" {
  description = "Number of cache nodes"
  type        = number
  default     = 1
}

# -----------------------------------------------------------------------------
# Domain & SSL
# -----------------------------------------------------------------------------

variable "domain_name" {
  description = "Domain name for the API"
  type        = string
  default     = "api.agentauri.ai"
}

variable "certificate_arn" {
  description = "ACM certificate ARN for HTTPS"
  type        = string
  default     = ""
}

# -----------------------------------------------------------------------------
# Container Image
# -----------------------------------------------------------------------------

variable "container_image" {
  description = "Docker image URI"
  type        = string
}

variable "container_image_tag" {
  description = "Docker image tag"
  type        = string
  default     = "latest"
}

# -----------------------------------------------------------------------------
# Monitoring & Alerts
# -----------------------------------------------------------------------------

variable "alert_email" {
  description = "Email address for alert notifications (SNS)"
  type        = string
  default     = ""
}

variable "ponder_monitoring_enabled" {
  description = "Enable CloudWatch alarms and SNS notifications for Ponder indexer"
  type        = bool
  default     = true
}

# -----------------------------------------------------------------------------
# Documentation Site (docs.agentauri.ai)
# -----------------------------------------------------------------------------

variable "docs_enabled" {
  description = "Enable documentation site infrastructure (S3 + CloudFront)"
  type        = bool
  default     = false
}
