---
title: Billing & Credits
description: Manage credits, subscriptions, and payments
sidebar:
  order: 5
---

AgentAuri uses a credits-based billing system. Credits are consumed when triggers fire and actions execute.

## Credit Balance

Check your organization's credit balance:

```bash
curl "https://api.agentauri.ai/api/v1/billing/credits" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Organization-Id: ORG_ID"
```

Response:
```json
{
  "organization_id": "org_abc123",
  "balance": 10000,
  "currency": "credits",
  "updated_at": "2024-01-15T10:00:00Z"
}
```

## Credit Costs

| Action Type | Cost |
|-------------|------|
| Trigger evaluation | 1 credit |
| Telegram notification | 2 credits |
| REST webhook | 2 credits |
| MCP update | 5 credits |

### Query Tier Multipliers

For API queries, costs vary by tier:

| Tier | Description | Cost Multiplier |
|------|-------------|-----------------|
| Tier 0 | Basic queries (feedbacks, validations) | 1x |
| Tier 1 | Aggregated data (reputation summary) | 2x |
| Tier 2 | Analysis (client analysis, comparison) | 5x |
| Tier 3 | AI-powered insights | 10x |

## Purchasing Credits

```bash
curl -X POST "https://api.agentauri.ai/api/v1/billing/credits/purchase" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "organization_id": "org_abc123",
    "amount": 10000,
    "payment_method": "stripe"
  }'
```

Response:
```json
{
  "transaction_id": "txn_xyz789",
  "amount": 10000,
  "price_usd": 10.00,
  "status": "completed",
  "new_balance": 20000
}
```

### Credit Packages

| Package | Credits | Price | Per-Credit |
|---------|---------|-------|------------|
| Starter | 1,000 | $1.00 | $0.001 |
| Standard | 10,000 | $8.00 | $0.0008 |
| Pro | 100,000 | $60.00 | $0.0006 |
| Enterprise | 1,000,000 | $400.00 | $0.0004 |

## Transaction History

View credit transactions:

```bash
curl "https://api.agentauri.ai/api/v1/billing/transactions?limit=20" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Organization-Id: ORG_ID"
```

Response:
```json
{
  "data": [
    {
      "id": "txn_abc123",
      "type": "purchase",
      "amount": 10000,
      "balance_after": 20000,
      "description": "Credit purchase via Stripe",
      "created_at": "2024-01-15T10:00:00Z"
    },
    {
      "id": "txn_def456",
      "type": "usage",
      "amount": -5,
      "balance_after": 19995,
      "description": "Trigger: Agent Alert (telegram)",
      "created_at": "2024-01-15T10:05:00Z"
    }
  ],
  "total": 2
}
```

## Subscriptions

View your current subscription:

```bash
curl "https://api.agentauri.ai/api/v1/billing/subscription" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Organization-Id: ORG_ID"
```

Response:
```json
{
  "organization_id": "org_abc123",
  "plan": "pro",
  "status": "active",
  "current_period_start": "2024-01-01T00:00:00Z",
  "current_period_end": "2024-02-01T00:00:00Z",
  "monthly_credits": 50000,
  "credits_used_this_period": 12500
}
```

## Stripe Integration

AgentAuri integrates with Stripe for secure payment processing.

### Setting Up Payment Methods

1. Navigate to your organization settings
2. Click "Add Payment Method"
3. Enter card details (processed securely by Stripe)

### Webhooks

AgentAuri receives Stripe webhooks for:
- Successful payments
- Failed payments
- Subscription updates
- Refunds

## Low Balance Alerts

Configure alerts when credits run low:

```bash
curl -X POST "https://api.agentauri.ai/api/v1/billing/alerts" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "organization_id": "org_abc123",
    "threshold": 1000,
    "notify_email": true,
    "notify_webhook": "https://your-server.com/billing-alert"
  }'
```

## Best Practices

1. **Monitor usage** - Check transaction history regularly
2. **Set alerts** - Configure low balance notifications
3. **Use appropriate tiers** - Don't use Tier 3 queries when Tier 0 suffices
4. **Bulk operations** - Combine actions when possible
5. **Test on free tier** - Validate triggers before production use
