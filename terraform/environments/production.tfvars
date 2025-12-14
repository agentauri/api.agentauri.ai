# =============================================================================
# Production Environment Configuration
# =============================================================================
# Usage: terraform plan -var-file=environments/production.tfvars

environment = "production"
aws_region  = "us-east-1"

# Network
vpc_cidr           = "10.1.0.0/16"
availability_zones = ["us-east-1a", "us-east-1b", "us-east-1c"]

# Domain
domain_name     = "api.agentauri.ai"
certificate_arn = "arn:aws:acm:us-east-1:781863585732:certificate/8f05d65b-bb32-4497-915f-20e757ddd2f1"

# ECS - API Gateway
api_gateway_cpu           = 512
api_gateway_memory        = 1024
api_gateway_desired_count = 2

# ECS - Event Processor
event_processor_cpu    = 512
event_processor_memory = 1024

# ECS - Action Workers
action_workers_cpu           = 512
action_workers_memory        = 1024
action_workers_desired_count = 2

# Container Image
container_image     = "781863585732.dkr.ecr.us-east-1.amazonaws.com/agentauri-backend"
container_image_tag = "v1.0.0"

# RDS PostgreSQL
rds_instance_class    = "db.t3.small"
rds_allocated_storage = 50
rds_multi_az          = true

# ElastiCache Redis
redis_node_type       = "cache.t3.small"
redis_num_cache_nodes = 2

# =============================================================================
# Ponder Indexer (Shared Service)
# =============================================================================
# IMPORTANT: Ponder is NOT deployed in production.
# The staging workspace runs a single shared Ponder instance that indexes
# blockchain events into the 'ponder' schema. Production reads from this schema.
# This avoids duplicate indexing of public blockchain data.

ponder_indexer_enabled = false
