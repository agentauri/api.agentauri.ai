# API Keys Statistics Endpoint

## Endpoint

```
GET /api/v1/organizations/{id}/api-keys/stats
```

## Authentication

Requires JWT (`Authorization: Bearer <token>`) or API Key (`X-API-Key: sk_live_xxx`).

Any organization member can view stats (viewer, member, admin, owner).

## Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string (UUID) | Yes | Organization ID |

## Response

### Success (200 OK)

```json
{
  "data": {
    "total_keys": 15,
    "active_keys": 10,
    "expired_keys": 2,
    "revoked_keys": 3,
    "unused_keys": 1,
    "keys_expiring_soon": 2,
    "calls_24h": 1523,
    "failed_auth_24h": 12,
    "rate_limited_24h": 3,
    "keys_by_environment": {
      "live": 8,
      "test": 7
    },
    "keys_by_type": {
      "standard": 12,
      "restricted": 2,
      "admin": 1
    }
  }
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `total_keys` | integer | Total number of API keys (including revoked) |
| `active_keys` | integer | Keys that are not revoked and not expired |
| `expired_keys` | integer | Keys past their expiration date (not revoked) |
| `revoked_keys` | integer | Keys that have been revoked |
| `unused_keys` | integer | Active keys that have never been used |
| `keys_expiring_soon` | integer | Active keys expiring within 7 days |
| `calls_24h` | integer | Successful API calls in the last 24 hours |
| `failed_auth_24h` | integer | Failed authentication attempts in the last 24 hours |
| `rate_limited_24h` | integer | Rate-limited requests in the last 24 hours |
| `keys_by_environment` | object | Count of active keys by environment |
| `keys_by_environment.live` | integer | Active production keys |
| `keys_by_environment.test` | integer | Active test keys |
| `keys_by_type` | object | Count of active keys by type |
| `keys_by_type.standard` | integer | Standard permission keys |
| `keys_by_type.restricted` | integer | Restricted permission keys |
| `keys_by_type.admin` | integer | Admin permission keys |

### Error Responses

| Status | Error Code | Description |
|--------|------------|-------------|
| 401 | `unauthorized` | Missing or invalid authentication |
| 403 | `forbidden` | Not a member of the organization |
| 404 | `not_found` | Organization not found |

## Example Usage

### cURL

```bash
curl -X GET "https://api.agentauri.ai/api/v1/organizations/org_123/api-keys/stats" \
  -H "Authorization: Bearer <jwt_token>"
```

### TypeScript

```typescript
interface KeysByEnvironment {
  live: number;
  test: number;
}

interface KeysByType {
  standard: number;
  restricted: number;
  admin: number;
}

interface ApiKeyStats {
  total_keys: number;
  active_keys: number;
  expired_keys: number;
  revoked_keys: number;
  unused_keys: number;
  keys_expiring_soon: number;
  calls_24h: number;
  failed_auth_24h: number;
  rate_limited_24h: number;
  keys_by_environment: KeysByEnvironment;
  keys_by_type: KeysByType;
}

async function getApiKeyStats(orgId: string): Promise<ApiKeyStats> {
  const response = await fetch(
    `https://api.agentauri.ai/api/v1/organizations/${orgId}/api-keys/stats`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  if (!response.ok) {
    throw new Error(`Failed to fetch stats: ${response.status}`);
  }

  const { data } = await response.json();
  return data;
}
```

## UI Usage Suggestions

### Dashboard Cards

Display key metrics as cards:
- **Active Keys**: `active_keys` (green)
- **Expired**: `expired_keys` (yellow warning)
- **Revoked**: `revoked_keys` (gray)
- **Usage 24h**: `calls_24h`

### Alerts

Show warnings for:
- `keys_expiring_soon > 0` - "X keys expiring in 7 days"
- `unused_keys > 0` - "X keys never used (security risk)"
- `failed_auth_24h > threshold` - "Unusual auth failures detected"
- `rate_limited_24h > 0` - "Rate limits being hit"

### Charts

- **Pie chart**: Keys by environment (live vs test)
- **Pie chart**: Keys by type (standard/restricted/admin)
- **Trend chart**: `calls_24h` over time (requires polling)

## Related Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/organizations/{id}/api-keys` | List all API keys |
| `POST /api/v1/organizations/{id}/api-keys` | Create new API key |
| `DELETE /api/v1/api-keys/{id}` | Revoke API key |
| `POST /api/v1/api-keys/{id}/rotate` | Rotate API key |
