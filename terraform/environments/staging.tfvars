# =============================================================================
# Staging Environment Configuration
# =============================================================================
# Usage: terraform plan -var-file=environments/staging.tfvars

environment = "staging"
aws_region  = "us-east-1"

# Network
vpc_cidr                 = "10.0.0.0/16"
availability_zones_count = 2

# Domain
domain_name     = "staging-api.agentauri.ai"
certificate_arn = "arn:aws:acm:us-east-1:781863585732:certificate/8f05d65b-bb32-4497-915f-20e757ddd2f1"

# ECS - API Gateway
api_gateway_cpu           = 256
api_gateway_memory        = 512
api_gateway_desired_count = 1

# ECS - Event Processor
event_processor_cpu    = 256
event_processor_memory = 512

# ECS - Action Workers
action_workers_cpu           = 256
action_workers_memory        = 512
action_workers_desired_count = 1

# Container Image
container_image     = "781863585732.dkr.ecr.us-east-1.amazonaws.com/agentauri-backend"
container_image_tag = "latest"

# RDS PostgreSQL
db_instance_class    = "db.t3.micro"
db_allocated_storage = 20
db_multi_az          = false

# ElastiCache Redis
redis_node_type       = "cache.t3.micro"
redis_num_cache_nodes = 1

# =============================================================================
# Ponder Indexer (Shared Service)
# =============================================================================
# Ponder is deployed ONLY in staging as a shared service.
# It indexes blockchain events which are public and immutable - same data
# for both staging and production environments.
# The production backend reads from the same ponder schema in this RDS.

ponder_indexer_enabled   = true
ponder_indexer_cpu       = 512
ponder_indexer_memory    = 1024
ponder_indexer_image_tag = "latest"
ponder_database_schema   = "ponder"

# =============================================================================
# Monitoring & Alerts
# =============================================================================
# CloudWatch alarms and SNS notifications for Ponder indexer

ponder_monitoring_enabled = true
alert_email               = "matteo.scurati@agentauri.ai"

# =============================================================================
# Grafana (Monitoring Dashboard)
# =============================================================================
# Grafana deployed on ECS with EFS for persistent storage
# Accessible at https://staging-api.agentauri.ai/grafana/

grafana_enabled = true
grafana_cpu     = 256
grafana_memory  = 512
