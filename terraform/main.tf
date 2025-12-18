# =============================================================================
# AgentAuri Infrastructure - Main Configuration
# =============================================================================
# Terraform configuration for AWS ECS/Fargate deployment
#
# Usage:
#   cd terraform
#   terraform init
#   terraform plan -var-file="environments/production.tfvars"
#   terraform apply -var-file="environments/production.tfvars"
# =============================================================================

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Remote state storage (uncomment and configure for team use)
  # backend "s3" {
  #   bucket         = "agentauri-terraform-state"
  #   key            = "infrastructure/terraform.tfstate"
  #   region         = "us-east-1"
  #   encrypt        = true
  #   dynamodb_table = "agentauri-terraform-locks"
  # }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "AgentAuri"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

# -----------------------------------------------------------------------------
# Data Sources
# -----------------------------------------------------------------------------

data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

data "aws_availability_zones" "available" {
  state = "available"
}

# -----------------------------------------------------------------------------
# Local Values
# -----------------------------------------------------------------------------

locals {
  name_prefix = "agentauri-${var.environment}"

  common_tags = {
    Project     = "AgentAuri"
    Environment = var.environment
  }
}
