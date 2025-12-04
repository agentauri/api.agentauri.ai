# API Key Authentication

This document describes the API key authentication system (Layer 1) for api.agentauri.ai.

## Overview

API keys provide account-based authentication for developers and applications. They enable:

- Full API access to all query tiers
- All payment methods (Stripe, x402, Credits)
- Per-plan rate limiting
- Organization-scoped access control

## Key Format

### Structure

```
sk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234
└─────┘ └─────────────────────────────────────────────────┘
prefix            random bytes (32 bytes, Base64)
```

### Prefixes

| Prefix | Environment | Description |
|--------|-------------|-------------|
| `sk_live_` | Production | Real queries, real billing |
| `sk_test_` | Testing | Test queries, no billing |

### Storage

- **Client-side**: Store securely (environment variables, secrets manager)
- **Server-side**: Argon2 hash only; original key never stored
- **Prefix stored**: First 8 chars after `sk_` for identification

## Key Lifecycle

### Creation

```http
POST /api/v1/api-keys
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Production API Key",
  "environment": "live",
  "key_type": "standard",
  "permissions": ["read", "write"],
  "expires_at": "2025-12-31T23:59:59Z"  // optional
}
```

**Response** (key shown only once):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "key": "sk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234",
  "name": "Production API Key",
  "prefix": "sk_live_abc123de",
  "environment": "live",
  "key_type": "standard",
  "permissions": ["read", "write"],
  "created_at": "2025-01-15T10:00:00Z",
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**IMPORTANT**: The full key is only returned during creation. Store it securely immediately.

### Listing Keys

```http
GET /api/v1/api-keys
Authorization: Bearer <jwt_token>
```

**Response** (keys are masked):
```json
{
  "keys": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Production API Key",
      "prefix": "sk_live_abc123de",
      "environment": "live",
      "key_type": "standard",
      "permissions": ["read", "write"],
      "last_used_at": "2025-01-15T14:30:00Z",
      "created_at": "2025-01-15T10:00:00Z",
      "expires_at": "2025-12-31T23:59:59Z"
    }
  ]
}
```

### Rotation

Rotate a key to generate a new secret while maintaining the same ID and settings.

```http
POST /api/v1/api-keys/:id/rotate
Authorization: Bearer <jwt_token>
```

**Response** (new key shown only once):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "key": "sk_live_xyz789abc012def345ghi678jkl901mno234pqr567stu890",
  "rotated_at": "2025-01-20T09:00:00Z",
  "previous_key_valid_until": "2025-01-20T09:15:00Z"
}
```

**Grace Period**: Old key remains valid for 15 minutes after rotation to prevent downtime.

### Revocation

```http
DELETE /api/v1/api-keys/:id
Authorization: Bearer <jwt_token>

{
  "reason": "Security incident"  // optional
}
```

**Response**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "revoked_at": "2025-01-20T10:00:00Z",
  "revoked_by": "user@example.com",
  "reason": "Security incident"
}
```

Revoked keys are immediately invalid and cannot be restored.

## Key Types

| Type | Description | Permissions |
|------|-------------|-------------|
| `standard` | Normal API access | Configurable per-key |
| `restricted` | Limited access | Subset of standard |
| `admin` | Full organization access | All permissions |

## Permissions

| Permission | Description | Endpoints |
|------------|-------------|-----------|
| `read` | Read-only access | GET queries, list resources |
| `write` | Create/update resources | POST/PUT triggers, etc. |
| `delete` | Delete resources | DELETE triggers, revoke keys |
| `billing` | Access billing info | Credits, transactions |
| `admin` | Organization management | Members, settings |

## Database Schema

```sql
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,           -- Argon2 hash
    name TEXT NOT NULL,
    prefix TEXT NOT NULL UNIQUE,      -- 'sk_live_' or 'sk_test_' + first 8 chars
    environment TEXT NOT NULL CHECK (environment IN ('live', 'test')),
    key_type TEXT NOT NULL DEFAULT 'standard'
        CHECK (key_type IN ('standard', 'restricted', 'admin')),
    permissions JSONB NOT NULL DEFAULT '["read"]',
    rate_limit_override INTEGER,      -- NULL = use org default
    last_used_at TIMESTAMPTZ,
    last_used_ip INET,
    expires_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    revoked_by TEXT,
    revocation_reason TEXT
);

CREATE INDEX idx_api_keys_prefix ON api_keys(prefix) WHERE revoked_at IS NULL;
CREATE INDEX idx_api_keys_org ON api_keys(organization_id) WHERE revoked_at IS NULL;
```

## Validation Flow

```
1. Check authentication rate limit (per-IP + global)
   → If exceeded: 429 Too Many Requests (log to auth_failures)

2. Extract key from Authorization header
   Authorization: Bearer sk_live_abc123...

3. Parse and validate key format
   → If invalid format: 401 Unauthorized (log to auth_failures)

4. Parse prefix (first 16 chars: sk_live_abc123de)

5. Look up key by prefix in database
   → If not found: dummy_verify() + 401 Unauthorized (log to auth_failures)

6. Verify key against stored Argon2id hash (constant-time)
   → If mismatch: 401 Unauthorized (log to api_key_audit_log)

7. Check revocation status
   → If revoked: 401 Unauthorized (key revoked)

8. Check expiration
   → If expired: 401 Unauthorized (key expired)

9. Check permissions for requested operation
   → If insufficient: 403 Forbidden

10. Apply API rate limiting based on organization plan
    → If exceeded: 429 Too Many Requests

11. Update last_used_at and last_used_ip

12. Log successful auth to api_key_audit_log

13. Allow request to proceed
```

## Rate Limiting

API keys inherit rate limits from their organization's plan:

| Plan | Requests/Hour | Concurrent Tasks |
|------|---------------|------------------|
| Starter | 100 | 2 |
| Pro | 500 | 10 |
| Enterprise | 2000 | 50 |

Individual keys can have `rate_limit_override` for custom limits (lower than plan max).

## Security Hardening (Phase 3.5b)

The API key authentication system includes comprehensive security hardening:

### Timing Attack Mitigation

To prevent timing attacks during key verification, the system uses a pre-computed dummy hash:

```rust
// Pre-computed Argon2id hash for dummy verification
// This ensures constant-time verification even for non-existent keys
lazy_static! {
    static ref DUMMY_HASH: String = {
        ApiKeyService::hash_key("dummy_key_for_timing_attack_mitigation")
            .expect("Failed to compute dummy hash")
    };
}

// Even if key prefix not found, we verify against dummy hash
// This makes timing analysis ineffective
fn dummy_verify() -> bool {
    let _ = verify_key("invalid", &DUMMY_HASH);
    false
}
```

**Why this matters**: Without this mitigation, an attacker could measure response times to determine if a key prefix exists in the database (fast fail) vs. hash verification (slower).

### Authentication Rate Limiting

Dedicated rate limiting for authentication attempts using the Governor crate:

| Limit Type | Rate | Purpose |
|------------|------|---------|
| Per-IP | 20/minute | Prevent brute-force from single source |
| Global | 1000/minute | Prevent distributed attacks |

```rust
// Rate limiter configuration
AuthRateLimiter::new(
    per_ip_rate: 20,      // 20 auth attempts per minute per IP
    global_rate: 1000,    // 1000 total auth attempts per minute
)
```

Rate limit responses include `Retry-After` header with seconds until reset.

### Dual Audit Logging

Two-tier logging system for comprehensive security monitoring:

1. **`api_key_audit_log`** - Organization-scoped events
   - Key creation, rotation, revocation
   - Successful authentications
   - Failed authentications (when org is known)

2. **`auth_failures`** - Pre-organization failures
   - Invalid key format
   - Unknown key prefix
   - Rate-limited requests

```sql
-- auth_failures table for logging failures without org context
CREATE TABLE auth_failures (
    id BIGSERIAL PRIMARY KEY,
    failure_type TEXT NOT NULL,  -- 'invalid_format', 'prefix_not_found', 'rate_limited'
    key_prefix TEXT,             -- First 16 chars for pattern analysis
    ip_address INET,
    user_agent TEXT,
    endpoint TEXT,
    details JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Argon2id Parameters (OWASP Recommended)

```rust
// OWASP-recommended Argon2id parameters
const MEMORY_COST: u32 = 65536;  // 64 MiB
const TIME_COST: u32 = 3;        // 3 iterations
const PARALLELISM: u32 = 1;      // 1 thread
const OUTPUT_LENGTH: usize = 32; // 256 bits
```

## Security Best Practices

1. **Never commit keys to version control**
   - Use environment variables or secrets managers
   - Add `.env` to `.gitignore`

2. **Use test keys for development**
   - `sk_test_` keys don't affect production

3. **Rotate keys regularly**
   - Recommended: every 90 days
   - Immediately after any security concern

4. **Use minimal permissions**
   - Only grant permissions needed for the task

5. **Monitor key usage**
   - Check `last_used_at` for unused keys
   - Review access logs for anomalies
   - Monitor `auth_failures` table for attack patterns

6. **Set expiration dates**
   - Avoid permanent keys when possible

## Error Responses

| Status | Code | Description |
|--------|------|-------------|
| 401 | `INVALID_API_KEY` | Key not found or invalid |
| 401 | `KEY_REVOKED` | Key has been revoked |
| 401 | `KEY_EXPIRED` | Key has expired |
| 403 | `INSUFFICIENT_PERMISSIONS` | Key lacks required permission |
| 429 | `AUTH_RATE_LIMITED` | Authentication rate limit exceeded (20/min per IP) |
| 429 | `RATE_LIMITED` | API rate limit exceeded (per-plan) |

## Related Documentation

- [AUTHENTICATION.md](./AUTHENTICATION.md) - Authentication system overview
- [RATE_LIMITING.md](./RATE_LIMITING.md) - Rate limiting implementation
- [SECURITY_MODEL.md](./SECURITY_MODEL.md) - Security best practices

---

**Last Updated**: November 26, 2024
