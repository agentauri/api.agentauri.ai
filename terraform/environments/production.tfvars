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
container_image_tag = "latest"

# RDS PostgreSQL
# Note: Using free tier compatible settings. Upgrade later:
# - db_instance_class = "db.t3.small" (or larger)
# - db_multi_az = true
db_instance_class    = "db.t3.micro"
db_allocated_storage = 20
db_multi_az          = false

# ElastiCache Redis
redis_node_type       = "cache.t3.small"
redis_num_cache_nodes = 2

# =============================================================================
# Ponder Indexer
# =============================================================================
# Ponder indexes blockchain events from ERC-8004 registries into PostgreSQL

ponder_indexer_enabled   = true
ponder_indexer_cpu       = 512
ponder_indexer_memory    = 1024
ponder_indexer_image     = "781863585732.dkr.ecr.us-east-1.amazonaws.com/agentauri-ponder"
ponder_indexer_image_tag = "v1.1.0"
ponder_database_schema   = "ponder"

# =============================================================================
# Monitoring & Alerts
# =============================================================================

ponder_monitoring_enabled = true
alert_email               = "matteo.scurati@agentauri.ai"

# =============================================================================
# Grafana (Monitoring Dashboard)
# =============================================================================

grafana_enabled = true
grafana_cpu     = 256
grafana_memory  = 512

# =============================================================================
# Documentation Site (docs.agentauri.ai)
# =============================================================================

docs_enabled = true
