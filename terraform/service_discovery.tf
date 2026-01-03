# =============================================================================
# AWS Cloud Map - Service Discovery
# =============================================================================
# Enables API Gateway HTTP to route traffic to ECS services via VPC Link.
# This replaces ALB for ~$15-17/month cost savings.

# -----------------------------------------------------------------------------
# Private DNS Namespace
# -----------------------------------------------------------------------------

resource "aws_service_discovery_private_dns_namespace" "main" {
  name        = "agentauri.local"
  description = "Private namespace for AgentAuri ECS services"
  vpc         = aws_vpc.main.id

  tags = {
    Name = "${local.name_prefix}-namespace"
  }
}

# -----------------------------------------------------------------------------
# Service Discovery Service for API Gateway
# -----------------------------------------------------------------------------

resource "aws_service_discovery_service" "api_gateway" {
  name = "api-gateway"

  dns_config {
    namespace_id   = aws_service_discovery_private_dns_namespace.main.id
    routing_policy = "MULTIVALUE"

    dns_records {
      ttl  = 10
      type = "A"
    }

    # SRV record for port discovery - required for API Gateway HTTP integration
    dns_records {
      ttl  = 10
      type = "SRV"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }

  tags = {
    Name = "${local.name_prefix}-api-gateway-discovery"
  }
}
