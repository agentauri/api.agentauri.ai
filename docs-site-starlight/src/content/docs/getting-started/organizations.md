---
title: Organizations
description: Manage multi-tenant organizations and team access
sidebar:
  order: 4
---

Organizations enable multi-tenant access to AgentAuri. Each organization has its own triggers, API keys, and billing.

## Creating an Organization

```bash
curl -X POST https://api.agentauri.ai/api/v1/organizations \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My AI Agents Team",
    "slug": "my-ai-agents"
  }'
```

Response:
```json
{
  "id": "org_abc123def456",
  "name": "My AI Agents Team",
  "slug": "my-ai-agents",
  "owner_id": "user_xyz789",
  "plan": "free",
  "created_at": "2024-01-15T10:00:00Z"
}
```

## Organization Plans

| Plan | Triggers | Rate Limit | Features |
|------|----------|------------|----------|
| `free` | 5 | 50/hour | Basic monitoring |
| `starter` | 25 | 200/hour | Email support |
| `pro` | 100 | 1000/hour | Priority support, webhooks |
| `enterprise` | Unlimited | 5000/hour | SLA, dedicated support |

## Listing Organizations

```bash
curl https://api.agentauri.ai/api/v1/organizations \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "data": [
    {
      "id": "org_abc123",
      "name": "My AI Agents Team",
      "slug": "my-ai-agents",
      "plan": "pro",
      "role": "admin"
    }
  ],
  "total": 1
}
```

## Organization Members

### Roles

| Role | Permissions |
|------|-------------|
| `admin` | Full access, manage members, billing |
| `member` | Create/edit triggers, view analytics |
| `viewer` | Read-only access |

### Adding a Member

```bash
curl -X POST "https://api.agentauri.ai/api/v1/organizations/ORG_ID/members" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "teammate@example.com",
    "role": "member"
  }'
```

### Listing Members

```bash
curl "https://api.agentauri.ai/api/v1/organizations/ORG_ID/members" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

Response:
```json
{
  "data": [
    {
      "user_id": "user_abc123",
      "email": "admin@example.com",
      "role": "admin",
      "joined_at": "2024-01-15T10:00:00Z"
    },
    {
      "user_id": "user_def456",
      "email": "teammate@example.com",
      "role": "member",
      "joined_at": "2024-01-16T14:30:00Z"
    }
  ]
}
```

### Updating a Member's Role

```bash
curl -X PUT "https://api.agentauri.ai/api/v1/organizations/ORG_ID/members/USER_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role": "admin"}'
```

### Removing a Member

```bash
curl -X DELETE "https://api.agentauri.ai/api/v1/organizations/ORG_ID/members/USER_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Personal Organizations

When you register, a personal organization is automatically created:

- **Name**: Your username
- **Slug**: `personal-{user_id}`
- **Plan**: Free tier
- **Owner**: You (only member)

Personal organizations cannot be deleted or have members added.

## Updating an Organization

```bash
curl -X PUT "https://api.agentauri.ai/api/v1/organizations/ORG_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Renamed Organization"
  }'
```

## Deleting an Organization

```bash
curl -X DELETE "https://api.agentauri.ai/api/v1/organizations/ORG_ID" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

:::caution
Deleting an organization permanently removes all triggers, API keys, and data associated with it.
:::

## Best Practices

1. **Separate environments** - Create different organizations for dev/staging/production
2. **Use roles appropriately** - Give team members minimum required permissions
3. **Monitor usage** - Track rate limit consumption across the team
4. **Audit regularly** - Review member access periodically
