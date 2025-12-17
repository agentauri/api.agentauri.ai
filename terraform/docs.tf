# =============================================================================
# Documentation Site Infrastructure (docs.agentauri.ai)
# =============================================================================
# Static documentation hosted via S3 + CloudFront
# Built with Docusaurus, deployed via GitHub Actions

# -----------------------------------------------------------------------------
# S3 Bucket for Static Site
# -----------------------------------------------------------------------------

resource "aws_s3_bucket" "docs" {
  count  = var.docs_enabled ? 1 : 0
  bucket = "agentauri-docs-${var.environment}"

  tags = {
    Name = "agentauri-docs-${var.environment}"
  }
}

resource "aws_s3_bucket_versioning" "docs" {
  count  = var.docs_enabled ? 1 : 0
  bucket = aws_s3_bucket.docs[0].id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_public_access_block" "docs" {
  count  = var.docs_enabled ? 1 : 0
  bucket = aws_s3_bucket.docs[0].id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# -----------------------------------------------------------------------------
# CloudFront Origin Access Control
# -----------------------------------------------------------------------------

resource "aws_cloudfront_origin_access_control" "docs" {
  count                             = var.docs_enabled ? 1 : 0
  name                              = "agentauri-docs-${var.environment}"
  description                       = "OAC for docs.agentauri.ai"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

# -----------------------------------------------------------------------------
# S3 Bucket Policy for CloudFront
# -----------------------------------------------------------------------------

resource "aws_s3_bucket_policy" "docs" {
  count  = var.docs_enabled ? 1 : 0
  bucket = aws_s3_bucket.docs[0].id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AllowCloudFrontServicePrincipal"
        Effect = "Allow"
        Principal = {
          Service = "cloudfront.amazonaws.com"
        }
        Action   = "s3:GetObject"
        Resource = "${aws_s3_bucket.docs[0].arn}/*"
        Condition = {
          StringEquals = {
            "AWS:SourceArn" = aws_cloudfront_distribution.docs[0].arn
          }
        }
      }
    ]
  })
}

# -----------------------------------------------------------------------------
# CloudFront Distribution
# -----------------------------------------------------------------------------

resource "aws_cloudfront_distribution" "docs" {
  count = var.docs_enabled ? 1 : 0

  enabled             = true
  is_ipv6_enabled     = true
  default_root_object = "index.html"
  comment             = "AgentAuri Documentation - ${var.environment}"
  price_class         = "PriceClass_100" # US, Canada, Europe

  aliases = var.environment == "production" ? ["docs.agentauri.ai"] : []

  origin {
    domain_name              = aws_s3_bucket.docs[0].bucket_regional_domain_name
    origin_id                = "S3-docs"
    origin_access_control_id = aws_cloudfront_origin_access_control.docs[0].id
  }

  default_cache_behavior {
    allowed_methods  = ["GET", "HEAD", "OPTIONS"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = "S3-docs"

    forwarded_values {
      query_string = false
      cookies {
        forward = "none"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 3600  # 1 hour
    max_ttl                = 86400 # 24 hours
    compress               = true
  }

  # Handle SPA routing - serve index.html for all paths
  custom_error_response {
    error_code         = 403
    response_code      = 200
    response_page_path = "/index.html"
  }

  custom_error_response {
    error_code         = 404
    response_code      = 200
    response_page_path = "/index.html"
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = var.environment == "production" ? "arn:aws:acm:us-east-1:781863585732:certificate/e8af92ac-7b78-44e6-bae0-85c6a096a111" : null
    ssl_support_method       = var.environment == "production" ? "sni-only" : null
    minimum_protocol_version = var.environment == "production" ? "TLSv1.2_2021" : null
    cloudfront_default_certificate = var.environment != "production"
  }

  tags = {
    Name = "agentauri-docs-${var.environment}"
  }
}

# -----------------------------------------------------------------------------
# Route53 Record (Skipped - DNS managed externally)
# -----------------------------------------------------------------------------
# DNS for agentauri.ai is managed externally.
# After deployment, create a CNAME record:
#   docs.agentauri.ai -> <cloudfront_domain_name>
# The CloudFront domain is output as docs_cloudfront_domain

# -----------------------------------------------------------------------------
# IAM Policy for GitHub Actions to deploy docs
# -----------------------------------------------------------------------------

resource "aws_iam_policy" "docs_deploy" {
  count = var.docs_enabled ? 1 : 0
  name  = "agentauri-docs-deploy-${var.environment}"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
          "s3:GetObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.docs[0].arn,
          "${aws_s3_bucket.docs[0].arn}/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "cloudfront:CreateInvalidation"
        ]
        Resource = aws_cloudfront_distribution.docs[0].arn
      }
    ]
  })
}

# Attach to existing GitHub Actions role
resource "aws_iam_role_policy_attachment" "github_actions_docs" {
  count      = var.docs_enabled ? 1 : 0
  role       = aws_iam_role.github_actions.name
  policy_arn = aws_iam_policy.docs_deploy[0].arn
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "docs_bucket_name" {
  description = "The name of the S3 bucket for documentation"
  value       = var.docs_enabled ? aws_s3_bucket.docs[0].id : ""
}

output "docs_cloudfront_distribution_id" {
  description = "The ID of the CloudFront distribution for documentation"
  value       = var.docs_enabled ? aws_cloudfront_distribution.docs[0].id : ""
}

output "docs_cloudfront_domain" {
  description = "The domain name of the CloudFront distribution"
  value       = var.docs_enabled ? aws_cloudfront_distribution.docs[0].domain_name : ""
}

output "docs_url" {
  description = "The URL of the documentation site"
  value       = var.docs_enabled && var.environment == "production" ? "https://docs.agentauri.ai" : (var.docs_enabled ? "https://${aws_cloudfront_distribution.docs[0].domain_name}" : "")
}
