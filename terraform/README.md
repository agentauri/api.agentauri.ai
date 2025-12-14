# AgentAuri Infrastructure - Terraform

This directory contains Terraform configurations for deploying AgentAuri backend services to AWS.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              AWS Cloud                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                         VPC (10.0.0.0/16)                        │   │
│  │  ┌─────────────────────┐    ┌─────────────────────┐             │   │
│  │  │   Public Subnet A   │    │   Public Subnet B   │             │   │
│  │  │  ┌───────────────┐  │    │  ┌───────────────┐  │             │   │
│  │  │  │      ALB      │  │    │  │      ALB      │  │             │   │
│  │  │  └───────────────┘  │    │  └───────────────┘  │             │   │
│  │  │  ┌───────────────┐  │    │                     │             │   │
│  │  │  │  NAT Gateway  │  │    │                     │             │   │
│  │  │  └───────────────┘  │    │                     │             │   │
│  │  └─────────────────────┘    └─────────────────────┘             │   │
│  │  ┌─────────────────────┐    ┌─────────────────────┐             │   │
│  │  │  Private Subnet A   │    │  Private Subnet B   │             │   │
│  │  │  ┌───────────────┐  │    │  ┌───────────────┐  │             │   │
│  │  │  │  ECS Tasks    │  │    │  │  ECS Tasks    │  │             │   │
│  │  │  │ (api-gateway) │  │    │  │ (api-gateway) │  │             │   │
│  │  │  │ (event-proc)  │  │    │  │ (workers)     │  │             │   │
│  │  │  └───────────────┘  │    │  └───────────────┘  │             │   │
│  │  │  ┌───────────────┐  │    │  ┌───────────────┐  │             │   │
│  │  │  │ RDS Primary   │──────│  │ RDS Standby   │  │             │   │
│  │  │  │ (PostgreSQL)  │  │    │  │ (Multi-AZ)    │  │             │   │
│  │  │  └───────────────┘  │    │  └───────────────┘  │             │   │
│  │  │  ┌───────────────┐  │    │  ┌───────────────┐  │             │   │
│  │  │  │ ElastiCache   │──────│  │ ElastiCache   │  │             │   │
│  │  │  │ (Redis)       │  │    │  │ (Replica)     │  │             │   │
│  │  │  └───────────────┘  │    │  └───────────────┘  │             │   │
│  │  └─────────────────────┘    └─────────────────────┘             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │
│  │     ECR      │  │   Secrets    │  │  CloudWatch  │                 │
│  │  (Images)    │  │   Manager    │  │   (Logs)     │                 │
│  └──────────────┘  └──────────────┘  └──────────────┘                 │
└─────────────────────────────────────────────────────────────────────────┘
```

## Prerequisites

1. **AWS CLI** configured with appropriate credentials
2. **Terraform** >= 1.5.0
3. **ACM Certificate** for your domain (must be in us-east-1 for ALB)
4. **Route53 Hosted Zone** for your domain (optional, for DNS)

## Quick Start

### 1. Initialize Terraform

```bash
cd terraform
terraform init
```

### 2. Configure Backend (Recommended for Team Use)

Uncomment the S3 backend configuration in `main.tf` and create the S3 bucket:

```bash
aws s3 mb s3://agentauri-terraform-state-ACCOUNT_ID
aws s3api put-bucket-versioning \
  --bucket agentauri-terraform-state-ACCOUNT_ID \
  --versioning-configuration Status=Enabled
```

### 3. Create ACM Certificate

```bash
aws acm request-certificate \
  --domain-name api.agentauri.ai \
  --subject-alternative-names staging-api.agentauri.ai \
  --validation-method DNS \
  --region us-east-1
```

Update `environments/staging.tfvars` and `environments/production.tfvars` with the certificate ARN.

### 4. Deploy Staging Environment

```bash
terraform workspace new staging || terraform workspace select staging
terraform plan -var-file=environments/staging.tfvars -out=staging.tfplan
terraform apply staging.tfplan
```

### 5. Deploy Production Environment

```bash
terraform workspace new production || terraform workspace select production
terraform plan -var-file=environments/production.tfvars -out=production.tfplan
terraform apply production.tfplan
```

## Configuration Files

| File | Description |
|------|-------------|
| `main.tf` | Provider configuration and backend |
| `variables.tf` | Input variable definitions |
| `vpc.tf` | VPC, subnets, NAT gateway, routing |
| `security_groups.tf` | Security groups for ALB, ECS, RDS, Redis |
| `ecr.tf` | Container registry |
| `ecs.tf` | ECS cluster, task definitions, services |
| `alb.tf` | Application Load Balancer, target groups |
| `rds.tf` | PostgreSQL database |
| `elasticache.tf` | Redis cluster |
| `iam.tf` | IAM roles for ECS, GitHub Actions |
| `secrets.tf` | Secrets Manager secrets |
| `outputs.tf` | Output values |

## Environment Variables

After deployment, these environment variables are automatically available to ECS tasks:

| Variable | Source |
|----------|--------|
| `DATABASE_URL` | Secrets Manager (`agentauri/{env}/rds-password`) |
| `REDIS_URL` | Secrets Manager (`agentauri/{env}/redis-auth-token`) |
| `JWT_SECRET` | Secrets Manager (`agentauri/{env}/jwt-secret`) |
| `AWS_REGION` | Task definition environment |
| `ENVIRONMENT` | Task definition environment |
| `SECRETS_BACKEND` | Set to `aws` in ECS |

## Manual Secret Population

Some secrets need to be populated manually after initial deployment:

```bash
# Telegram Bot Token
aws secretsmanager put-secret-value \
  --secret-id agentauri/staging/telegram-bot-token \
  --secret-string "YOUR_TELEGRAM_BOT_TOKEN"

# Stripe Keys
aws secretsmanager put-secret-value \
  --secret-id agentauri/staging/stripe-keys \
  --secret-string '{"publishable_key":"pk_test_...","secret_key":"sk_test_..."}'

# Google OAuth
aws secretsmanager put-secret-value \
  --secret-id agentauri/staging/google-oauth \
  --secret-string '{"client_id":"...","client_secret":"..."}'

# GitHub OAuth
aws secretsmanager put-secret-value \
  --secret-id agentauri/staging/github-oauth \
  --secret-string '{"client_id":"...","client_secret":"..."}'

# Alchemy API Key
aws secretsmanager put-secret-value \
  --secret-id agentauri/staging/alchemy-api-key \
  --secret-string "YOUR_ALCHEMY_API_KEY"
```

## GitHub Actions Integration

The Terraform configuration creates an IAM role for GitHub Actions using OIDC authentication. After deployment:

1. Get the role ARN from Terraform output:
   ```bash
   terraform output github_actions_role_arn
   ```

2. Add to GitHub repository secrets:
   - `AWS_DEPLOY_ROLE_ARN`: The role ARN from above

## Monitoring & Logs

- **CloudWatch Logs**: `/ecs/agentauri-{env}/api-gateway`, `/ecs/agentauri-{env}/event-processor`, `/ecs/agentauri-{env}/action-workers`
- **RDS Performance Insights**: Enabled by default
- **ECS Container Insights**: Enabled by default

## Cost Estimation (Staging)

| Resource | Type | Monthly Cost (est.) |
|----------|------|---------------------|
| ECS Fargate | 3 tasks (256 CPU, 512 MB) | ~$30 |
| RDS | db.t3.micro | ~$15 |
| ElastiCache | cache.t3.micro | ~$12 |
| NAT Gateway | 1 | ~$32 |
| ALB | 1 | ~$16 |
| **Total** | | **~$105/month** |

## Cost Estimation (Production)

| Resource | Type | Monthly Cost (est.) |
|----------|------|---------------------|
| ECS Fargate | 6 tasks (512 CPU, 1024 MB) | ~$120 |
| RDS Multi-AZ | db.t3.small | ~$50 |
| ElastiCache | cache.t3.small x2 | ~$50 |
| NAT Gateway | 3 (multi-AZ) | ~$96 |
| ALB | 1 | ~$16 |
| **Total** | | **~$332/month** |

## Destroying Infrastructure

```bash
# Select environment
terraform workspace select staging

# Destroy (with confirmation)
terraform destroy -var-file=environments/staging.tfvars
```

**Warning**: Production has `deletion_protection = true` on RDS and ALB. You must manually disable this before destroying.
