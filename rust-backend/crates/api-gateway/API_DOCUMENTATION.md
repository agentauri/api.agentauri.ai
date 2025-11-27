# API Gateway Documentation

## Overview

The API Gateway provides a RESTful API for managing triggers, conditions, and actions in the api.8004.dev multi-chain agent notification system.

**Base URL**: `http://localhost:8080/api/v1` (development)

## Security

### Authentication

The API implements a **3-layer authentication system**:

| Layer | Method | Use Case | Rate Limits |
|-------|--------|----------|-------------|
| **Layer 0** | Anonymous | Public data, x402 payments | 10 calls/hour per IP |
| **Layer 1** | API Key | Account-based access | Per-plan (100-2000/hr) |
| **Layer 2** | JWT + Wallet Signature | Full user/agent access | Per-account limits |

#### JWT Authentication (Layer 2)

Most endpoints require a valid JWT token in the Authorization header:

**Token Configuration**:
- Algorithm: HS256 (explicitly configured)
- Lifetime: 1 hour
- Format: `Authorization: Bearer <token>`

#### API Key Authentication (Layer 1)

For programmatic access, use API keys:
- Format: `sk_live_xxx` (production) or `sk_test_xxx` (testing)
- Header: `X-API-Key: sk_live_xxxxxxxxxxxxx`
- Created via `/api/v1/api-keys` endpoint

See [Authentication Documentation](../../../docs/auth/AUTHENTICATION.md) for complete details.

**Security Features**:
- Argon2 password hashing (memory-hard, side-channel resistant)
- JWT tokens expire after 1 hour
- Explicit algorithm validation (prevents algorithm confusion attacks)
- CORS whitelist (environment-based)
- Input validation on all endpoints
- JSON payload size limit: 1MB

**Rate Limiting**:
- Layer 0: 10 requests/hour per IP
- Layer 1: Per-plan limits (Starter: 100/hr, Pro: 500/hr, Enterprise: 2000/hr)
- Layer 2: Per-account limits based on subscription

### Common Security Errors

**401 Unauthorized**: Token missing, invalid, or expired
- Solution: Login again to get a new token

**413 Payload Too Large**: Request body exceeds 1MB
- Solution: Reduce payload size or split into multiple requests

**429 Too Many Requests**: Rate limit exceeded
- Solution: Wait before retrying, implement exponential backoff

---

## Authentication

All endpoints except authentication endpoints require JWT authentication.

### Headers

```
Authorization: Bearer <jwt_token>
```

## Endpoints

### Authentication

#### Register User

Create a new user account.

**Endpoint**: `POST /api/v1/auth/register`

**Request Body**:
```json
{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secure_password_123"
}
```

**Validation**:
- `username`: 3-50 characters
- `email`: Valid email format
- `password`: 8-100 characters

**Response** (201 Created):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",  // Valid for 1 hour
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "john_doe",
    "email": "john@example.com",
    "created_at": "2024-01-15T10:30:00Z",
    "last_login_at": null,
    "is_active": true
  }
}
```

**Error Responses**:
- `400 Bad Request`: Validation failed
- `409 Conflict`: Username or email already exists

---

#### Login

Authenticate and receive JWT token.

**Endpoint**: `POST /api/v1/auth/login`

**Request Body**:
```json
{
  "username_or_email": "john_doe",
  "password": "secure_password_123"
}
```

**Response** (200 OK):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",  // Valid for 1 hour
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "john_doe",
    "email": "john@example.com",
    "created_at": "2024-01-15T10:30:00Z",
    "last_login_at": "2024-01-16T14:20:00Z",
    "is_active": true
  }
}
```

**Error Responses**:
- `400 Bad Request`: Validation failed
- `401 Unauthorized`: Invalid credentials
- `403 Forbidden`: Account disabled

---

### Token Refresh

**Status**: Coming in Phase 3

Currently, JWT tokens expire after 1 hour. Users must login again to get a new token. A token refresh mechanism will be added in Phase 3 to allow extending sessions without re-entering credentials.

---

### Triggers

#### Create Trigger

Create a new trigger within an organization. Triggers are organization-scoped resources.

**Endpoint**: `POST /api/v1/triggers`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "High Value Identity Mints",
  "description": "Alert when identity NFTs are minted",
  "organization_id": "550e8400-e29b-41d4-a716-446655440002",
  "chain_id": 1,
  "registry": "identity",
  "enabled": true,
  "is_stateful": false
}
```

**Validation**:
- `name`: 1-255 characters (required)
- `description`: Max 1000 characters (optional)
- `organization_id`: Valid organization UUID (required, user must be member)
- `chain_id`: Integer (required)
- `registry`: Must be one of: `identity`, `reputation`, `validation` (required)
- `enabled`: Boolean (default: true)
- `is_stateful`: Boolean (default: false)

**Response** (201 Created):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "organization_id": "550e8400-e29b-41d4-a716-446655440002",
    "name": "High Value Identity Mints",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T15:00:00Z"
  }
}
```

---

#### List Triggers

Get all triggers for the authenticated user with pagination.

**Endpoint**: `GET /api/v1/triggers?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `limit`: Number of results (1-100, default: 20)
- `offset`: Number of results to skip (default: 0)

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "High Value Identity Mints",
      "description": "Alert when identity NFTs are minted",
      "chain_id": 1,
      "registry": "identity",
      "enabled": true,
      "is_stateful": false,
      "created_at": "2024-01-16T15:00:00Z",
      "updated_at": "2024-01-16T15:00:00Z"
    }
  ],
  "pagination": {
    "total": 42,
    "limit": 20,
    "offset": 0,
    "has_more": true
  }
}
```

---

#### Get Trigger Details

Get a single trigger with all conditions and actions.

**Endpoint**: `GET /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "High Value Identity Mints",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T15:00:00Z",
    "conditions": [
      {
        "id": 1,
        "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
        "condition_type": "event",
        "field": "event_type",
        "operator": "equals",
        "value": "AgentMinted",
        "config": null,
        "created_at": "2024-01-16T15:05:00Z"
      }
    ],
    "actions": [
      {
        "id": 1,
        "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
        "action_type": "telegram",
        "priority": 0,
        "config": {
          "chat_id": "123456789",
          "message_template": "New agent minted: {{agent_id}}"
        },
        "created_at": "2024-01-16T15:10:00Z"
      }
    ]
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

#### Update Trigger

Update an existing trigger. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Updated Trigger Name",
  "enabled": false
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Updated Trigger Name",
    "description": "Alert when identity NFTs are minted",
    "chain_id": 1,
    "registry": "identity",
    "enabled": false,
    "is_stateful": false,
    "created_at": "2024-01-16T15:00:00Z",
    "updated_at": "2024-01-16T16:00:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

#### Delete Trigger

Delete a trigger and all associated conditions and actions (cascading delete).

**Endpoint**: `DELETE /api/v1/triggers/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger not found or doesn't belong to user

---

### Trigger Conditions

#### Create Condition

Add a new condition to a trigger.

**Endpoint**: `POST /api/v1/triggers/{trigger_id}/conditions`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "condition_type": "event",
  "field": "event_type",
  "operator": "equals",
  "value": "AgentMinted",
  "config": {
    "case_sensitive": false
  }
}
```

**Validation**:
- `condition_type`: 1-100 characters (required)
- `field`: 1-255 characters (required)
- `operator`: 1-50 characters (required)
- `value`: 1-1000 characters (required)
- `config`: JSON object (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentMinted",
    "config": {
      "case_sensitive": false
    },
    "created_at": "2024-01-16T15:05:00Z"
  }
}
```

---

#### List Conditions

Get all conditions for a trigger.

**Endpoint**: `GET /api/v1/triggers/{trigger_id}/conditions`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
      "condition_type": "event",
      "field": "event_type",
      "operator": "equals",
      "value": "AgentMinted",
      "config": null,
      "created_at": "2024-01-16T15:05:00Z"
    }
  ]
}
```

---

#### Update Condition

Update an existing condition. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{trigger_id}/conditions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "value": "AgentBurned",
  "config": {
    "case_sensitive": true
  }
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentBurned",
    "config": {
      "case_sensitive": true
    },
    "created_at": "2024-01-16T15:05:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger or condition not found

---

#### Delete Condition

Delete a condition from a trigger.

**Endpoint**: `DELETE /api/v1/triggers/{trigger_id}/conditions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger or condition not found

---

### Trigger Actions

#### Create Action

Add a new action to a trigger.

**Endpoint**: `POST /api/v1/triggers/{trigger_id}/actions`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "action_type": "telegram",
  "priority": 0,
  "config": {
    "chat_id": "123456789",
    "message_template": "New agent minted: {{agent_id}}"
  }
}
```

**Validation**:
- `action_type`: Must be one of: `telegram`, `rest`, `mcp` (required)
- `priority`: Integer (default: 0)
- `config`: JSON object (required)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "action_type": "telegram",
    "priority": 0,
    "config": {
      "chat_id": "123456789",
      "message_template": "New agent minted: {{agent_id}}"
    },
    "created_at": "2024-01-16T15:10:00Z"
  }
}
```

---

#### List Actions

Get all actions for a trigger (ordered by priority, then ID).

**Endpoint**: `GET /api/v1/triggers/{trigger_id}/actions`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
      "action_type": "telegram",
      "priority": 0,
      "config": {
        "chat_id": "123456789",
        "message_template": "New agent minted: {{agent_id}}"
      },
      "created_at": "2024-01-16T15:10:00Z"
    }
  ]
}
```

---

#### Update Action

Update an existing action. All fields are optional.

**Endpoint**: `PUT /api/v1/triggers/{trigger_id}/actions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "priority": 1,
  "config": {
    "chat_id": "987654321",
    "message_template": "Agent minted: {{agent_id}} on chain {{chain_id}}"
  }
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": 1,
    "trigger_id": "550e8400-e29b-41d4-a716-446655440001",
    "action_type": "telegram",
    "priority": 1,
    "config": {
      "chat_id": "987654321",
      "message_template": "Agent minted: {{agent_id}} on chain {{chain_id}}"
    },
    "created_at": "2024-01-16T15:10:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Trigger or action not found

---

#### Delete Action

Delete an action from a trigger.

**Endpoint**: `DELETE /api/v1/triggers/{trigger_id}/actions/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `404 Not Found`: Trigger or action not found

---

### Health Check

Check API and database health.

**Endpoint**: `GET /api/v1/health`

**No authentication required**

**Response** (200 OK):
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```

**Response** (503 Service Unavailable):
```json
{
  "status": "unhealthy",
  "database": "disconnected",
  "version": "0.1.0"
}
```

---

### Organizations

Organizations are the multi-tenant unit for grouping resources, members, and billing.

#### Create Organization

Create a new organization. The creator becomes the owner.

**Endpoint**: `POST /api/v1/organizations`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "My Company",
  "slug": "my-company",
  "description": "Optional description"
}
```

**Validation**:
- `name`: 1-100 characters (required)
- `slug`: 1-50 characters, lowercase alphanumeric and hyphens (required, unique)
- `description`: Max 500 characters (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "My Company",
    "slug": "my-company",
    "description": "Optional description",
    "is_personal": false,
    "created_at": "2024-11-27T10:00:00Z",
    "updated_at": "2024-11-27T10:00:00Z"
  }
}
```

**Error Responses**:
- `400 Bad Request`: Validation failed
- `409 Conflict`: Organization slug already exists

---

#### List Organizations

Get all organizations the authenticated user is a member of.

**Endpoint**: `GET /api/v1/organizations?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `limit`: Number of results (1-100, default: 20)
- `offset`: Number of results to skip (default: 0)

**Response** (200 OK):
```json
{
  "data": [
    {
      "organization": {
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "name": "My Company",
        "slug": "my-company",
        "description": null,
        "is_personal": false,
        "created_at": "2024-11-27T10:00:00Z",
        "updated_at": "2024-11-27T10:00:00Z"
      },
      "my_role": "owner"
    }
  ],
  "pagination": {
    "total": 2,
    "limit": 20,
    "offset": 0,
    "has_more": false
  }
}
```

---

#### Get Organization

Get details for a single organization.

**Endpoint**: `GET /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": {
    "organization": {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "name": "My Company",
      "slug": "my-company",
      "description": null,
      "is_personal": false,
      "created_at": "2024-11-27T10:00:00Z",
      "updated_at": "2024-11-27T10:00:00Z"
    },
    "my_role": "owner"
  }
}
```

**Error Responses**:
- `404 Not Found`: Organization not found or not a member

---

#### Update Organization

Update organization details. Requires admin or owner role.

**Endpoint**: `PUT /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Updated Company Name",
  "description": "New description"
}
```

**Response** (200 OK): Returns updated organization

**Error Responses**:
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Organization not found

---

#### Delete Organization

Delete an organization. Requires owner role. Personal organizations cannot be deleted.

**Endpoint**: `DELETE /api/v1/organizations/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `400 Bad Request`: Cannot delete personal organization
- `403 Forbidden`: Only owner can delete
- `404 Not Found`: Organization not found

---

#### Transfer Ownership

Transfer organization ownership to another member.

**Endpoint**: `POST /api/v1/organizations/{id}/transfer`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "new_owner_id": "550e8400-e29b-41d4-a716-446655440002"
}
```

**Response** (200 OK): Returns updated organization

**Error Responses**:
- `400 Bad Request`: Cannot transfer to yourself or personal org
- `403 Forbidden`: Only owner can transfer
- `404 Not Found`: Organization not found

---

### Organization Members

#### Add Member

Add a user to an organization. Requires admin or owner role.

**Endpoint**: `POST /api/v1/organizations/{id}/members`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440002",
  "role": "member"
}
```

**Validation**:
- `user_id`: Valid user UUID (required)
- `role`: One of `viewer`, `member`, `admin` (cannot add as `owner`)

**Response** (201 Created):
```json
{
  "data": {
    "id": 1,
    "user_id": "550e8400-e29b-41d4-a716-446655440002",
    "username": "jane_doe",
    "email": "jane@example.com",
    "role": "member",
    "created_at": "2024-11-27T10:00:00Z"
  }
}
```

**Error Responses**:
- `400 Bad Request`: Cannot add as owner
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: User not found
- `409 Conflict`: User is already a member

---

#### List Members

Get all members of an organization. Email addresses are masked for privacy unless you are an admin/owner or viewing your own email.

**Endpoint**: `GET /api/v1/organizations/{id}/members?limit=20&offset=0`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "data": [
    {
      "id": 1,
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "john_doe",
      "email": "john@example.com",
      "role": "owner",
      "created_at": "2024-11-27T10:00:00Z"
    },
    {
      "id": 2,
      "user_id": "550e8400-e29b-41d4-a716-446655440002",
      "username": "jane_doe",
      "email": "j***@e***.com",
      "role": "member",
      "created_at": "2024-11-27T11:00:00Z"
    }
  ],
  "pagination": {
    "total": 2,
    "limit": 20,
    "offset": 0,
    "has_more": false
  }
}
```

---

#### Update Member Role

Update a member's role. Requires owner role.

**Endpoint**: `PUT /api/v1/organizations/{id}/members/{user_id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "role": "admin"
}
```

**Response** (200 OK): Returns updated member

**Error Responses**:
- `400 Bad Request`: Cannot change role to owner
- `403 Forbidden`: Only owner can update roles
- `404 Not Found`: Member not found

---

#### Remove Member

Remove a member from an organization. Requires admin or owner role.

**Endpoint**: `DELETE /api/v1/organizations/{id}/members/{user_id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (204 No Content)

**Error Responses**:
- `400 Bad Request`: Cannot remove the owner
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Member not found

---

### API Keys

API keys provide programmatic access to the API (Layer 1 authentication).

#### Create API Key

Create a new API key. The full key is shown **only once** at creation time.

**Endpoint**: `POST /api/v1/api-keys?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "name": "Production API Key",
  "environment": "live",
  "key_type": "standard",
  "permissions": ["read", "write"],
  "rate_limit_override": 500,
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Validation**:
- `name`: 1-100 characters (required)
- `environment`: `live` or `test` (required)
- `key_type`: `standard`, `restricted`, or `admin` (required)
- `permissions`: Array of strings (required)
- `rate_limit_override`: Integer (optional)
- `expires_at`: ISO 8601 timestamp (optional)

**Response** (201 Created):
```json
{
  "data": {
    "id": "key_550e8400",
    "key": "sk_live_abc123xyz456789...",
    "name": "Production API Key",
    "prefix": "sk_live_abc123x",
    "environment": "live",
    "key_type": "standard",
    "permissions": ["read", "write"],
    "created_at": "2024-11-27T10:00:00Z",
    "expires_at": "2025-12-31T23:59:59Z"
  }
}
```

**Security Note**: Save the `key` value immediately - it will never be shown again.

---

#### List API Keys

List all API keys for an organization. Keys are masked (only prefix shown).

**Endpoint**: `GET /api/v1/api-keys?organization_id=xxx&limit=20&offset=0&include_revoked=false`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "items": [
    {
      "id": "key_550e8400",
      "name": "Production API Key",
      "prefix": "sk_live_abc123x",
      "environment": "live",
      "key_type": "standard",
      "permissions": ["read", "write"],
      "rate_limit_override": 500,
      "last_used_at": "2024-11-27T15:30:00Z",
      "expires_at": "2025-12-31T23:59:59Z",
      "created_at": "2024-11-27T10:00:00Z",
      "created_by": "user_123",
      "is_revoked": false,
      "revoked_at": null
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20,
  "total_pages": 1
}
```

---

#### Get API Key

Get details for a specific API key.

**Endpoint**: `GET /api/v1/api-keys/{id}`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK): Returns API key details (masked)

---

#### Revoke API Key

Revoke an API key. Revoked keys cannot be used.

**Endpoint**: `DELETE /api/v1/api-keys/{id}`

**Headers**: `Authorization: Bearer <token>`

**Request Body** (optional):
```json
{
  "reason": "Compromised key"
}
```

**Response** (200 OK): Returns revoked API key

**Error Responses**:
- `400 Bad Request`: Key is already revoked
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: API key not found

---

#### Rotate API Key

Rotate an API key (revoke old, create new in one transaction).

**Endpoint**: `POST /api/v1/api-keys/{id}/rotate`

**Headers**: `Authorization: Bearer <token>`

**Request Body** (optional):
```json
{
  "name": "New Key Name",
  "expires_at": "2026-01-01T00:00:00Z"
}
```

**Response** (200 OK):
```json
{
  "data": {
    "id": "new_key_id",
    "key": "sk_live_new_secret...",
    "prefix": "sk_live_new_sec",
    "old_key_id": "key_550e8400",
    "old_key_revoked_at": "2024-11-27T12:00:00Z"
  }
}
```

**Security Note**: Save the new `key` value immediately.

---

### Agent Linking

Link on-chain agent NFTs to organizations using wallet signature verification (Layer 2 authentication).

#### Link Agent

Link an agent NFT to an organization. Requires wallet signature proving ownership.

**Endpoint**: `POST /api/v1/agents/link`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "agent_id": 42,
  "chain_id": 11155111,
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "challenge": "Sign this message to authenticate with ERC-8004 API\n\nWallet: 0x1234...\nNonce: abc123...\nExpires: 2024-11-27T12:00:00Z",
  "signature": "0x..."
}
```

**Validation**:
- `agent_id`: Integer (required)
- `chain_id`: Integer (required)
- `organization_id`: Valid org UUID (required)
- `wallet_address`: 42 characters (0x + 40 hex chars)
- `challenge`: Non-empty string with nonce and expiration
- `signature`: 130-132 characters (EIP-191 signature)

**Security Flow**:
1. Verify EIP-191 signature matches wallet address
2. Check nonce hasn't been used (replay attack prevention)
3. Verify challenge hasn't expired
4. Verify on-chain ownership via IdentityRegistry.ownerOf()
5. Create agent link

**Response** (201 Created):
```json
{
  "id": "link_550e8400",
  "agent_id": 42,
  "chain_id": 11155111,
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
  "status": "active",
  "created_at": "2024-11-27T10:00:00Z"
}
```

**Error Responses**:
- `400 Bad Request`: Invalid signature, nonce reused, or challenge expired
- `403 Forbidden`: Wallet does not own this agent NFT
- `409 Conflict`: Agent is already linked to an organization

---

#### List Linked Agents

List all agents linked to an organization.

**Endpoint**: `GET /api/v1/agents/linked?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
[
  {
    "id": "link_550e8400",
    "agent_id": 42,
    "chain_id": 11155111,
    "organization_id": "550e8400-e29b-41d4-a716-446655440001",
    "wallet_address": "0x1234567890abcdef1234567890abcdef12345678",
    "status": "active",
    "created_at": "2024-11-27T10:00:00Z"
  }
]
```

---

#### Unlink Agent

Remove an agent link from an organization. Requires admin or owner role.

**Endpoint**: `DELETE /api/v1/agents/{agent_id}/link?chain_id=xxx&organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "message": "Agent unlinked successfully"
}
```

**Error Responses**:
- `403 Forbidden`: Agent not linked to your organization
- `404 Not Found`: Agent link not found

---

### Billing

Credit management and Stripe integration for the payment system.

#### Get Credit Balance

Get the credit balance for an organization.

**Endpoint**: `GET /api/v1/billing/credits?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "balance": 50000000,
  "balance_formatted": "50.00 USDC"
}
```

**Note**: Balance is in micro-USDC (1 USDC = 1,000,000 micro-USDC).

---

#### List Transactions

List credit transactions for an organization.

**Endpoint**: `GET /api/v1/billing/transactions?organization_id=xxx&limit=20&offset=0&transaction_type=purchase`

**Headers**: `Authorization: Bearer <token>`

**Query Parameters**:
- `organization_id`: Required
- `limit`: 1-100 (default: 20)
- `offset`: Default: 0
- `transaction_type`: Filter by type (optional): `purchase`, `usage`, `refund`, `adjustment`

**Response** (200 OK):
```json
[
  {
    "id": "tx_550e8400",
    "organization_id": "550e8400-e29b-41d4-a716-446655440001",
    "amount": 50000000,
    "transaction_type": "purchase",
    "description": "Stripe checkout purchase",
    "reference_id": "cs_test_xxx",
    "balance_after": 50000000,
    "created_at": "2024-11-27T10:00:00Z"
  }
]
```

---

#### Purchase Credits

Create a Stripe Checkout session to purchase credits.

**Endpoint**: `POST /api/v1/billing/credits/purchase`

**Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "amount": 50,
  "success_url": "https://example.com/success",
  "cancel_url": "https://example.com/cancel"
}
```

**Validation**:
- `amount`: 1-10000 (USD, whole units)
- `success_url`: Valid URL
- `cancel_url`: Valid URL

**Response** (200 OK):
```json
{
  "checkout_url": "https://checkout.stripe.com/...",
  "session_id": "cs_test_xxx"
}
```

**Note**: Redirect user to `checkout_url` to complete payment.

---

#### Get Subscription

Get subscription details for an organization.

**Endpoint**: `GET /api/v1/billing/subscription?organization_id=xxx`

**Headers**: `Authorization: Bearer <token>`

**Response** (200 OK):
```json
{
  "id": "sub_550e8400",
  "organization_id": "550e8400-e29b-41d4-a716-446655440001",
  "stripe_subscription_id": "sub_xxx",
  "stripe_customer_id": "cus_xxx",
  "plan": "pro",
  "status": "active",
  "current_period_start": "2024-11-01T00:00:00Z",
  "current_period_end": "2024-12-01T00:00:00Z",
  "created_at": "2024-11-01T00:00:00Z"
}
```

---

#### Stripe Webhook

Handle Stripe webhook events. No authentication required (uses Stripe signature verification).

**Endpoint**: `POST /api/v1/billing/webhook`

**Headers**: `Stripe-Signature: <signature>`

**Handled Events**:
- `checkout.session.completed`: Add credits to organization
- `checkout.session.async_payment_failed`: Log failure
- `customer.subscription.created`: Create subscription record
- `customer.subscription.updated`: Update subscription status
- `customer.subscription.deleted`: Cancel subscription

**Response** (200 OK):
```json
{
  "received": true
}
```

---

## Error Responses

All error responses follow this format:

```json
{
  "error": "error_code",
  "message": "Human-readable error message"
}
```

### Common HTTP Status Codes

- `200 OK`: Request succeeded
- `201 Created`: Resource created successfully
- `204 No Content`: Request succeeded with no response body
- `400 Bad Request`: Invalid request or validation error
- `401 Unauthorized`: Authentication required or invalid token
- `403 Forbidden`: Authenticated but not authorized
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate username)
- `500 Internal Server Error`: Server error

---

## Additional Security Details

### JWT Tokens

- JWT tokens expire after 1 hour (production hardened)
- Tokens are signed using HS256 algorithm (explicitly configured)
- Include user ID and username in claims
- Store JWT secret in `JWT_SECRET` environment variable
- Expiration validation enabled with 60-second clock skew tolerance

### Password Requirements

- Minimum 8 characters
- Maximum 100 characters
- Hashed using Argon2 (memory-hard, side-channel resistant)

### Authorization

- Users can only access/modify their own triggers
- All trigger-related operations verify ownership
- Conditions and actions inherit trigger ownership

### Production Hardening (Week 7)

The following security enhancements were implemented:

1. **JWT Algorithm Explicitly Configured**: Prevents algorithm confusion attacks
2. **Token Lifetime Reduced**: 7 days → 1 hour (168x reduction in compromise window)
3. **JSON Payload Size Limits**: 1MB maximum (prevents DoS via memory exhaustion)
4. **Infrastructure Rate Limiting**: nginx-based rate limiting on all endpoints
5. **CORS Whitelist**: Environment-based origin restrictions

---

## Configuration

### Environment Variables

Required:
- `DB_PASSWORD`: PostgreSQL password
- `JWT_SECRET`: Secret for signing JWT tokens (production)

Optional:
- `DB_HOST`: Database host (default: localhost)
- `DB_PORT`: Database port (default: 5432)
- `DB_NAME`: Database name (default: erc8004_backend)
- `DB_USER`: Database user (default: postgres)
- `DB_MAX_CONNECTIONS`: Max DB connections (default: 10)
- `SERVER_HOST`: Server host (default: 0.0.0.0)
- `SERVER_PORT`: Server port (default: 8080)
- `ALLOWED_ORIGINS`: Comma-separated CORS origins

### Running the Server

```bash
# Set environment variables
export DB_PASSWORD=your_password
export JWT_SECRET=your_secret_key

# Run the server
cd rust-backend
cargo run --bin api-gateway
```

Server will start on `http://0.0.0.0:8080` by default.

---

## Example Usage

### Complete Workflow

1. **Register a user**:
```bash
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "secure_pass_123"
  }'
```

2. **Login to get token**:
```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username_or_email": "alice",
    "password": "secure_pass_123"
  }'
```

3. **Create a trigger**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Identity Mints Monitor",
    "description": "Track identity NFT mints",
    "chain_id": 1,
    "registry": "identity",
    "enabled": true
  }'
```

4. **Add a condition**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers/TRIGGER_ID/conditions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "condition_type": "event",
    "field": "event_type",
    "operator": "equals",
    "value": "AgentMinted"
  }'
```

5. **Add an action**:
```bash
curl -X POST http://localhost:8080/api/v1/triggers/TRIGGER_ID/actions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action_type": "telegram",
    "priority": 0,
    "config": {
      "chat_id": "123456789",
      "message_template": "Agent {{agent_id}} minted!"
    }
  }'
```

6. **List all triggers**:
```bash
curl http://localhost:8080/api/v1/triggers?limit=10&offset=0 \
  -H "Authorization: Bearer YOUR_TOKEN"
```

7. **Get trigger details**:
```bash
curl http://localhost:8080/api/v1/triggers/TRIGGER_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```

---

## Architecture

### File Structure

```
api-gateway/src/
├── main.rs                 # Entry point and server setup
├── routes.rs               # Route configuration
├── middleware.rs           # CORS and JWT authentication
├── models/                 # Request/Response DTOs
│   ├── mod.rs
│   ├── auth.rs            # Auth DTOs
│   ├── triggers.rs        # Trigger DTOs
│   ├── conditions.rs      # Condition DTOs
│   ├── actions.rs         # Action DTOs
│   └── common.rs          # Common DTOs (errors, pagination)
├── repositories/          # Database access layer
│   ├── mod.rs
│   ├── users.rs          # User repository
│   ├── triggers.rs       # Trigger repository
│   ├── conditions.rs     # Condition repository
│   └── actions.rs        # Action repository
└── handlers/             # Request handlers
    ├── mod.rs
    ├── auth.rs          # Auth handlers
    ├── triggers.rs      # Trigger handlers
    ├── conditions.rs    # Condition handlers
    ├── actions.rs       # Action handlers
    └── health.rs        # Health check handler
```

### Design Patterns

- **Repository Pattern**: Database access abstracted into repositories
- **DTO Pattern**: Separate request/response models from database models
- **Middleware Pattern**: JWT authentication applied declaratively
- **Layered Architecture**: Clear separation between handlers, repositories, and models

---

## Testing

Health check endpoint can be used for testing:

```bash
curl http://localhost:8080/api/v1/health
```

Expected response:
```json
{
  "status": "healthy",
  "database": "connected",
  "version": "0.1.0"
}
```
