# =============================================================================
# AWS Secrets Manager - Automatic Secret Rotation
# =============================================================================
# Phase 6: Security Hardening - Implements automatic rotation for:
# - RDS password (using AWS SAR-managed Lambda)
# - JWT secret (using custom Lambda)
# - API key salt (using custom Lambda)
# - OAuth state key (using custom Lambda)
#
# Rotation schedules:
# - RDS password: 30 days (database credential)
# - JWT secret: 30 days (session tokens valid max 1 hour)
# - OAuth state key: 30 days (short-lived state tokens)
# - API key salt: 90 days (affects existing API keys - requires graceful migration)

# -----------------------------------------------------------------------------
# Security Group for Lambda Rotation Functions
# -----------------------------------------------------------------------------

resource "aws_security_group" "lambda_rotation" {
  name        = "${local.name_prefix}-lambda-rotation"
  description = "Security group for secret rotation Lambda functions"
  vpc_id      = aws_vpc.main.id

  # Outbound to RDS
  egress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.rds.id]
    description     = "PostgreSQL access to RDS"
  }

  # Outbound to Secrets Manager (via NAT or VPC Endpoint)
  egress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTPS for Secrets Manager API"
  }

  tags = {
    Name = "${local.name_prefix}-lambda-rotation"
  }
}

# Allow Lambda rotation to connect to RDS
resource "aws_security_group_rule" "rds_from_lambda_rotation" {
  type                     = "ingress"
  from_port                = 5432
  to_port                  = 5432
  protocol                 = "tcp"
  source_security_group_id = aws_security_group.lambda_rotation.id
  security_group_id        = aws_security_group.rds.id
  description              = "PostgreSQL from Lambda rotation"
}

# -----------------------------------------------------------------------------
# RDS Password Rotation (AWS Serverless Application Repository)
# -----------------------------------------------------------------------------
# Uses AWS-maintained Lambda for PostgreSQL single-user rotation.
# This handles password generation, RDS update, and secret staging.

resource "aws_serverlessapplicationrepository_cloudformation_stack" "rds_rotation" {
  name           = "${local.name_prefix}-rds-rotation"
  application_id = "arn:aws:serverlessrepo:us-east-1:297356227824:applications/SecretsManagerRDSPostgreSQLRotationSingleUser"

  capabilities = [
    "CAPABILITY_IAM",
    "CAPABILITY_RESOURCE_POLICY"
  ]

  parameters = {
    endpoint            = "https://secretsmanager.${var.aws_region}.amazonaws.com"
    functionName        = "${local.name_prefix}-rds-rotation"
    vpcSubnetIds        = join(",", aws_subnet.private[*].id)
    vpcSecurityGroupIds = aws_security_group.lambda_rotation.id
  }

  tags = {
    Name    = "${local.name_prefix}-rds-rotation"
    Purpose = "RDS password rotation"
  }
}

# Allow Secrets Manager to invoke the RDS rotation Lambda
resource "aws_lambda_permission" "rds_rotation" {
  statement_id  = "AllowSecretsManagerInvocation"
  action        = "lambda:InvokeFunction"
  function_name = aws_serverlessapplicationrepository_cloudformation_stack.rds_rotation.outputs["RotationLambdaARN"]
  principal     = "secretsmanager.amazonaws.com"
}

# RDS Password Rotation Schedule (every 30 days)
resource "aws_secretsmanager_secret_rotation" "rds_password" {
  secret_id           = aws_secretsmanager_secret.rds_password.id
  rotation_lambda_arn = aws_serverlessapplicationrepository_cloudformation_stack.rds_rotation.outputs["RotationLambdaARN"]

  rotation_rules {
    automatically_after_days = 30
  }

  depends_on = [aws_lambda_permission.rds_rotation]
}

# -----------------------------------------------------------------------------
# Application Secrets Rotation Lambda
# -----------------------------------------------------------------------------
# Rotates simple secrets that don't require external system updates:
# - JWT signing secret
# - API key hashing salt
# - OAuth state signing key

resource "aws_lambda_function" "app_secrets_rotation" {
  function_name = "${local.name_prefix}-app-secrets-rotation"
  description   = "Rotates application secrets (JWT, API key salt, OAuth state)"

  filename         = data.archive_file.app_secrets_rotation.output_path
  source_code_hash = data.archive_file.app_secrets_rotation.output_base64sha256

  handler = "index.handler"
  runtime = "python3.11"
  timeout = 30

  role = aws_iam_role.app_secrets_rotation.arn

  environment {
    variables = {
      LOG_LEVEL = "INFO"
    }
  }

  tags = {
    Name    = "${local.name_prefix}-app-secrets-rotation"
    Purpose = "Application secrets rotation"
  }
}

# Lambda source code
data "archive_file" "app_secrets_rotation" {
  type        = "zip"
  output_path = "${path.module}/lambda/app_secrets_rotation.zip"

  source {
    content  = <<-PYTHON
import json
import boto3
import secrets
import string
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

def handler(event, context):
    """
    Rotates application secrets by generating new random values.
    Follows AWS Secrets Manager rotation protocol.
    """
    arn = event['SecretId']
    token = event['ClientRequestToken']
    step = event['Step']

    sm = boto3.client('secretsmanager')
    logger.info(f"Rotation step: {step} for secret: {arn}")

    metadata = sm.describe_secret(SecretId=arn)
    secret_name = metadata['Name']

    if step == "createSecret":
        create_secret(sm, arn, token, secret_name)
    elif step == "setSecret":
        # No external system to update for these secrets
        logger.info("setSecret: No external system to update")
    elif step == "testSecret":
        test_secret(sm, arn, token)
    elif step == "finishSecret":
        finish_secret(sm, arn, token)
    else:
        raise ValueError(f"Invalid step: {step}")

    return {"statusCode": 200}

def create_secret(sm, arn, token, secret_name):
    """Generate and store new secret value."""
    try:
        sm.get_secret_value(SecretId=arn, VersionId=token, VersionStage="AWSPENDING")
        logger.info("createSecret: AWSPENDING already exists")
        return
    except sm.exceptions.ResourceNotFoundException:
        pass

    # Secret length based on type
    if 'jwt-secret' in secret_name or 'oauth-state-key' in secret_name:
        length = 64
    elif 'api-key-salt' in secret_name:
        length = 32
    else:
        length = 32

    # Generate cryptographically secure random string
    alphabet = string.ascii_letters + string.digits
    new_secret = ''.join(secrets.choice(alphabet) for _ in range(length))

    sm.put_secret_value(
        SecretId=arn,
        ClientRequestToken=token,
        SecretString=new_secret,
        VersionStages=['AWSPENDING']
    )
    logger.info(f"createSecret: Generated {length}-char secret")

def test_secret(sm, arn, token):
    """Validate the pending secret."""
    secret = sm.get_secret_value(SecretId=arn, VersionId=token, VersionStage="AWSPENDING")
    value = secret['SecretString']

    if not value or len(value) < 32:
        raise ValueError(f"Secret too short: {len(value)} chars")

    logger.info(f"testSecret: Validated ({len(value)} chars)")

def finish_secret(sm, arn, token):
    """Promote AWSPENDING to AWSCURRENT."""
    metadata = sm.describe_secret(SecretId=arn)
    versions = metadata.get('VersionIdsToStages', {})

    # Check if already current
    if token in versions and 'AWSCURRENT' in versions[token]:
        logger.info("finishSecret: Already current")
        return

    # Find current version
    current_version = None
    for vid, stages in versions.items():
        if 'AWSCURRENT' in stages:
            current_version = vid
            break

    # Promote pending to current
    sm.update_secret_version_stage(
        SecretId=arn,
        VersionStage='AWSCURRENT',
        MoveToVersionId=token,
        RemoveFromVersionId=current_version
    )
    logger.info("finishSecret: Promoted to AWSCURRENT")
PYTHON
    filename = "index.py"
  }
}

# IAM Role for Application Secrets Rotation
resource "aws_iam_role" "app_secrets_rotation" {
  name = "${local.name_prefix}-app-secrets-rotation"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "${local.name_prefix}-app-secrets-rotation"
  }
}

resource "aws_iam_role_policy_attachment" "app_secrets_rotation_basic" {
  role       = aws_iam_role.app_secrets_rotation.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy" "app_secrets_rotation" {
  name = "secrets-manager-rotation"
  role = aws_iam_role.app_secrets_rotation.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:DescribeSecret",
          "secretsmanager:GetSecretValue",
          "secretsmanager:PutSecretValue",
          "secretsmanager:UpdateSecretVersionStage"
        ]
        Resource = [
          aws_secretsmanager_secret.jwt_secret.arn,
          aws_secretsmanager_secret.api_key_salt.arn,
          aws_secretsmanager_secret.oauth_state_key.arn
        ]
      },
      {
        Effect   = "Allow"
        Action   = "secretsmanager:GetRandomPassword"
        Resource = "*"
      }
    ]
  })
}

# Allow Secrets Manager to invoke the Lambda
resource "aws_lambda_permission" "app_secrets_rotation" {
  statement_id  = "AllowSecretsManagerInvocation"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.app_secrets_rotation.function_name
  principal     = "secretsmanager.amazonaws.com"
}

# -----------------------------------------------------------------------------
# Rotation Schedules for Application Secrets
# -----------------------------------------------------------------------------

# JWT Secret Rotation (every 30 days)
# Safe to rotate frequently - JWT tokens are short-lived (1 hour)
resource "aws_secretsmanager_secret_rotation" "jwt_secret" {
  secret_id           = aws_secretsmanager_secret.jwt_secret.id
  rotation_lambda_arn = aws_lambda_function.app_secrets_rotation.arn

  rotation_rules {
    automatically_after_days = 30
  }

  depends_on = [aws_lambda_permission.app_secrets_rotation]
}

# OAuth State Key Rotation (every 30 days)
# Safe to rotate - state tokens are very short-lived (< 5 min)
resource "aws_secretsmanager_secret_rotation" "oauth_state_key" {
  secret_id           = aws_secretsmanager_secret.oauth_state_key.id
  rotation_lambda_arn = aws_lambda_function.app_secrets_rotation.arn

  rotation_rules {
    automatically_after_days = 30
  }

  depends_on = [aws_lambda_permission.app_secrets_rotation]
}

# API Key Salt Rotation (every 90 days)
# IMPORTANT: Longer period because rotation affects API key verification.
# Application handles graceful migration with dual-salt support during rotation.
resource "aws_secretsmanager_secret_rotation" "api_key_salt" {
  secret_id           = aws_secretsmanager_secret.api_key_salt.id
  rotation_lambda_arn = aws_lambda_function.app_secrets_rotation.arn

  rotation_rules {
    # 90 days - less frequent due to impact on existing API keys
    automatically_after_days = 90
  }

  depends_on = [aws_lambda_permission.app_secrets_rotation]
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "rds_rotation_lambda_arn" {
  description = "ARN of the RDS password rotation Lambda"
  value       = aws_serverlessapplicationrepository_cloudformation_stack.rds_rotation.outputs["RotationLambdaARN"]
}

output "app_secrets_rotation_lambda_arn" {
  description = "ARN of the application secrets rotation Lambda"
  value       = aws_lambda_function.app_secrets_rotation.arn
}
