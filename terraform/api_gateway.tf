# =============================================================================
# Amazon API Gateway HTTP API
# =============================================================================
# Replaces ALB for cost optimization:
# - ALB: ~$16-18/month base + LCU costs
# - API Gateway HTTP: $1/million requests (pay per use)
#
# For low traffic (<16M requests/month), API Gateway HTTP is cheaper.

# -----------------------------------------------------------------------------
# API Gateway HTTP API
# -----------------------------------------------------------------------------

resource "aws_apigatewayv2_api" "main" {
  name          = "${local.name_prefix}-api"
  protocol_type = "HTTP"
  description   = "AgentAuri API Gateway - replaces ALB for cost savings"

  cors_configuration {
    allow_headers = ["*"]
    allow_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"]
    allow_origins = [
      "https://agentauri.ai",
      "https://www.agentauri.ai",
      "https://app.agentauri.ai",
      "https://docs.agentauri.ai",
      "http://localhost:3000",
      "http://localhost:5173"
    ]
    expose_headers    = ["*"]
    allow_credentials = true
    max_age           = 86400
  }

  tags = {
    Name = "${local.name_prefix}-api"
  }
}

# -----------------------------------------------------------------------------
# VPC Link for Private Integration
# -----------------------------------------------------------------------------
# Allows API Gateway to route traffic to ECS tasks in private subnets
# via Cloud Map service discovery.

resource "aws_apigatewayv2_vpc_link" "main" {
  name               = "${local.name_prefix}-vpc-link"
  security_group_ids = [aws_security_group.api_gateway_vpc_link.id]
  # Use public subnets since ECS tasks are deployed there (no NAT Gateway)
  subnet_ids = aws_subnet.public[*].id

  tags = {
    Name = "${local.name_prefix}-vpc-link"
  }
}

# -----------------------------------------------------------------------------
# Security Group for VPC Link
# -----------------------------------------------------------------------------

resource "aws_security_group" "api_gateway_vpc_link" {
  name        = "${local.name_prefix}-api-gw-vpc-link"
  description = "Security group for API Gateway VPC Link"
  vpc_id      = aws_vpc.main.id

  tags = {
    Name = "${local.name_prefix}-api-gw-vpc-link"
  }
}

# Egress rule added separately to avoid circular dependency
resource "aws_security_group_rule" "api_gw_vpc_link_to_ecs" {
  type                     = "egress"
  from_port                = 8080
  to_port                  = 8080
  protocol                 = "tcp"
  security_group_id        = aws_security_group.api_gateway_vpc_link.id
  source_security_group_id = aws_security_group.ecs_tasks.id
  description              = "Allow outbound to ECS tasks on port 8080"
}

# -----------------------------------------------------------------------------
# Integration with ECS via Cloud Map
# -----------------------------------------------------------------------------

resource "aws_apigatewayv2_integration" "api_gateway" {
  api_id             = aws_apigatewayv2_api.main.id
  integration_type   = "HTTP_PROXY"
  integration_uri    = aws_service_discovery_service.api_gateway.arn
  integration_method = "ANY"
  connection_type    = "VPC_LINK"
  connection_id      = aws_apigatewayv2_vpc_link.main.id
}

# -----------------------------------------------------------------------------
# Routes
# -----------------------------------------------------------------------------

# Default catch-all route
resource "aws_apigatewayv2_route" "default" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "$default"
  target    = "integrations/${aws_apigatewayv2_integration.api_gateway.id}"
}

# Health check route (for monitoring)
resource "aws_apigatewayv2_route" "health" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "GET /api/v1/health"
  target    = "integrations/${aws_apigatewayv2_integration.api_gateway.id}"
}

# -----------------------------------------------------------------------------
# Stage with Auto-Deploy
# -----------------------------------------------------------------------------

resource "aws_apigatewayv2_stage" "prod" {
  api_id      = aws_apigatewayv2_api.main.id
  name        = "$default"
  auto_deploy = true

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_gateway_access.arn
    format = jsonencode({
      requestId        = "$context.requestId"
      ip               = "$context.identity.sourceIp"
      requestTime      = "$context.requestTime"
      httpMethod       = "$context.httpMethod"
      routeKey         = "$context.routeKey"
      status           = "$context.status"
      protocol         = "$context.protocol"
      responseLength   = "$context.responseLength"
      integrationError = "$context.integrationErrorMessage"
      latency          = "$context.responseLatency"
    })
  }

  default_route_settings {
    throttling_burst_limit = 5000
    throttling_rate_limit  = 10000
  }

  tags = {
    Name = "${local.name_prefix}-prod-stage"
  }
}

# -----------------------------------------------------------------------------
# CloudWatch Log Group for API Gateway Access Logs
# -----------------------------------------------------------------------------

resource "aws_cloudwatch_log_group" "api_gateway_access" {
  name              = "/aws/apigateway/${local.name_prefix}"
  retention_in_days = var.environment == "production" ? 30 : 7

  tags = {
    Name = "${local.name_prefix}-api-gw-logs"
  }
}

# -----------------------------------------------------------------------------
# Custom Domain (uses same ACM certificate as ALB)
# -----------------------------------------------------------------------------

resource "aws_apigatewayv2_domain_name" "main" {
  domain_name = var.domain_name

  domain_name_configuration {
    certificate_arn = var.certificate_arn
    endpoint_type   = "REGIONAL"
    security_policy = "TLS_1_2"
  }

  tags = {
    Name = "${local.name_prefix}-domain"
  }
}

resource "aws_apigatewayv2_api_mapping" "main" {
  api_id      = aws_apigatewayv2_api.main.id
  domain_name = aws_apigatewayv2_domain_name.main.id
  stage       = aws_apigatewayv2_stage.prod.id
}
