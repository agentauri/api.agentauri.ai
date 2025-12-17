# =============================================================================
# AWS Secrets Manager - Application Secrets
# =============================================================================
# This file manages application-level secrets that need to be created
# and populated manually or through a separate process.
# Database and Redis secrets are auto-generated in their respective files.

# -----------------------------------------------------------------------------
# JWT Secret
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "jwt_secret" {
  name                    = "agentauri/${var.environment}/jwt-secret"
  description             = "JWT signing secret for AgentAuri ${var.environment}"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-jwt-secret"
  }
}

resource "random_password" "jwt_secret" {
  length  = 64
  special = false
}

resource "aws_secretsmanager_secret_version" "jwt_secret" {
  secret_id     = aws_secretsmanager_secret.jwt_secret.id
  secret_string = random_password.jwt_secret.result
}

# -----------------------------------------------------------------------------
# API Key Hashing Salt
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "api_key_salt" {
  name                    = "agentauri/${var.environment}/api-key-salt"
  description             = "Salt for API key hashing in AgentAuri ${var.environment}"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-api-key-salt"
  }
}

resource "random_password" "api_key_salt" {
  length  = 32
  special = false
}

resource "aws_secretsmanager_secret_version" "api_key_salt" {
  secret_id     = aws_secretsmanager_secret.api_key_salt.id
  secret_string = random_password.api_key_salt.result
}

# -----------------------------------------------------------------------------
# OAuth State Signing Key
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "oauth_state_key" {
  name                    = "agentauri/${var.environment}/oauth-state-key"
  description             = "HMAC key for OAuth state signing in AgentAuri ${var.environment}"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-oauth-state-key"
  }
}

resource "random_password" "oauth_state_key" {
  length  = 64
  special = false
}

resource "aws_secretsmanager_secret_version" "oauth_state_key" {
  secret_id     = aws_secretsmanager_secret.oauth_state_key.id
  secret_string = random_password.oauth_state_key.result
}

# -----------------------------------------------------------------------------
# Telegram Bot Token (Placeholder - populate manually)
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "telegram_bot_token" {
  name                    = "agentauri/${var.environment}/telegram-bot-token"
  description             = "Telegram bot token for notifications - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-telegram-bot-token"
  }
}

# -----------------------------------------------------------------------------
# Stripe API Keys (Placeholder - populate manually)
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "stripe_keys" {
  name                    = "agentauri/${var.environment}/stripe-keys"
  description             = "Stripe API keys for payment processing - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-stripe-keys"
  }
}

# -----------------------------------------------------------------------------
# Google OAuth Credentials (Placeholder - populate manually)
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "google_oauth" {
  name                    = "agentauri/${var.environment}/google-oauth"
  description             = "Google OAuth client ID and secret - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-google-oauth"
  }
}

# -----------------------------------------------------------------------------
# GitHub OAuth Credentials (Placeholder - populate manually)
# -----------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "github_oauth" {
  name                    = "agentauri/${var.environment}/github-oauth"
  description             = "GitHub OAuth client ID and secret - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-github-oauth"
  }
}

# -----------------------------------------------------------------------------
# RPC URLs for Blockchain Networks (Placeholder - populate manually)
# -----------------------------------------------------------------------------

# Ethereum Sepolia RPC URL
resource "aws_secretsmanager_secret" "alchemy_api_key" {
  name                    = "agentauri/${var.environment}/alchemy-api-key"
  description             = "Ethereum Sepolia RPC URL - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-alchemy-api-key"
  }
}

# Base Sepolia RPC URL
resource "aws_secretsmanager_secret" "base_sepolia_rpc" {
  name                    = "agentauri/${var.environment}/base-sepolia-rpc"
  description             = "Base Sepolia RPC URL - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-base-sepolia-rpc"
  }
}

# Linea Sepolia RPC URL
resource "aws_secretsmanager_secret" "linea_sepolia_rpc" {
  name                    = "agentauri/${var.environment}/linea-sepolia-rpc"
  description             = "Linea Sepolia RPC URL - POPULATE MANUALLY"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-linea-sepolia-rpc"
  }
}

# -----------------------------------------------------------------------------
# Multi-RPC Provider URLs (for failover and load balancing)
# -----------------------------------------------------------------------------
# These secrets enable multi-RPC configuration in Ponder with automatic
# failover and smart ranking based on latency (30%) and stability (70%).
#
# Naming convention: {chain}-rpc-{provider}
# Ponder expects env vars: {CHAIN}_RPC_{PROVIDER} (e.g., ETHEREUM_SEPOLIA_RPC_ANKR)

# Ethereum Sepolia - Ankr RPC
resource "aws_secretsmanager_secret" "eth_sepolia_rpc_ankr" {
  name                    = "agentauri/${var.environment}/eth-sepolia-rpc-ankr"
  description             = "Ethereum Sepolia Ankr RPC URL for multi-RPC failover"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-eth-sepolia-rpc-ankr"
  }
}

# Base Sepolia - Ankr RPC
resource "aws_secretsmanager_secret" "base_sepolia_rpc_ankr" {
  name                    = "agentauri/${var.environment}/base-sepolia-rpc-ankr"
  description             = "Base Sepolia Ankr RPC URL for multi-RPC failover"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-base-sepolia-rpc-ankr"
  }
}

# -----------------------------------------------------------------------------
# Monitoring Token (for bypassing rate limits on monitoring endpoints)
# -----------------------------------------------------------------------------
# This token allows Grafana, Prometheus, and health checkers to access
# API endpoints without being subject to rate limiting.
# Requests with X-Monitoring-Token header matching this value bypass rate limits.

resource "aws_secretsmanager_secret" "monitoring_token" {
  name                    = "agentauri/${var.environment}/monitoring-token"
  description             = "Token for monitoring systems to bypass rate limiting"
  recovery_window_in_days = var.environment == "production" ? 30 : 0

  tags = {
    Name = "${local.name_prefix}-monitoring-token"
  }

  # Note: Secret value is managed manually via AWS Console or CLI.
  # To update: aws secretsmanager put-secret-value --secret-id "agentauri/<env>/monitoring-token" --secret-string "<new-token>"
}
