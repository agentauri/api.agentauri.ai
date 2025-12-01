# OAuth 2.0 Authorization Code Flow

**Status:** 🚧 Partially Implemented (Phase 4-5)

This document describes the OAuth 2.0 authorization code flow implementation for the ERC-8004 backend. The OAuth system is partially implemented and will be completed in Phase 4-5.

## Table of Contents

- [Overview](#overview)
- [Grant Types](#grant-types)
- [Security Model](#security-model)
- [OAuth Clients](#oauth-clients)
- [Scopes & Permissions](#scopes--permissions)
- [Authorization Code Flow](#authorization-code-flow)
- [Token Management](#token-management)
- [Organization Context](#organization-context)
- [API Reference](#api-reference)
- [Examples](#examples)

## Overview

The OAuth 2.0 implementation provides secure, standards-compliant authentication for third-party applications and integrations. It supports multiple grant types and includes advanced security features like PKCE (Proof Key for Code Exchange).

### Key Features

- **RFC 6749 Compliant**: Implements OAuth 2.0 Authorization Framework
- **PKCE Support**: Enhanced security for public clients (RFC 7636)
- **Organization-Scoped**: Each OAuth client belongs to an organization
- **Secure Token Storage**: Argon2id hashing with p=4 parallelism
- **Granular Permissions**: Fine-grained scope-based access control
- **Token Rotation**: Support for refresh tokens

## Grant Types

The implementation supports three OAuth 2.0 grant types:

### 1. Authorization Code (with PKCE)

**Use Case:** Third-party applications that need to act on behalf of a user

**Flow:**
```
1. Client → Authorization Endpoint: Request authorization code
2. User → Authorization Server: Authenticate and approve
3. Authorization Server → Client: Redirect with authorization code
4. Client → Token Endpoint: Exchange code for access token (with PKCE verifier)
5. Token Endpoint → Client: Return access token + refresh token
```

**Security Features:**
- PKCE prevents authorization code interception attacks
- State parameter for CSRF protection
- Short-lived authorization codes (5 minutes)
- One-time use codes

### 2. Client Credentials

**Use Case:** Machine-to-machine authentication (server-to-server)

**Flow:**
```
1. Service → Token Endpoint: Send client_id + client_secret
2. Token Endpoint → Service: Return access token
```

**Security Features:**
- No user context (acts as the organization)
- Client secret stored as Argon2id hash
- Suitable for backend services only

### 3. Refresh Token

**Use Case:** Obtaining new access tokens without re-authentication

**Flow:**
```
1. Client → Token Endpoint: Send refresh token
2. Token Endpoint → Client: Return new access token (+ optional new refresh token)
```

**Security Features:**
- Refresh token rotation (optional)
- Refresh tokens can be revoked
- Longer expiration than access tokens

## Security Model

### Token Hashing

All sensitive tokens are hashed using **Argon2id** with OWASP-recommended parameters:

```
Algorithm: Argon2id (variant 0x13)
Memory Cost: 64 MiB (65536 KiB)
Time Cost: 3 iterations
Parallelism: 4 threads (p=4)
```

**Critical Difference from API Keys:**
OAuth tokens use `p=4` (not `p=1` like API keys) because they're verified more frequently and can benefit from parallel hashing to reduce latency.

### Token Types & Formats

| Token Type | Format | Entropy | Expiration | Hashed |
|------------|--------|---------|------------|--------|
| Client Secret | `cs_<43 chars>` | 256 bits | Never | Yes (p=4) |
| Access Token | `oauth_at_<43 chars>` | 256 bits | 1 hour | Yes (p=4) |
| Refresh Token | `oauth_rt_<43 chars>` | 256 bits | 30 days | Yes (p=4) |

**Important:** All secrets/tokens are shown **only once** at creation time. Store them securely!

### Timing Attack Mitigation

The implementation includes timing attack protection:

```rust
// Pre-computed valid Argon2 hash for constant-time verification
let dummy_hash = service.dummy_hash;
service.verify_token(token, &dummy_hash);  // Same timing as real verification
```

## OAuth Clients

### Client Types

**Confidential Clients:**
- Can securely store client secrets
- Examples: Backend services, server-side applications
- Must use client credentials for authentication

**Public Clients:**
- Cannot securely store secrets (e.g., SPAs, mobile apps)
- Must use PKCE for authorization code flow
- Client secret is optional

### Client Registration

**Endpoint:** `POST /api/v1/oauth/clients`

**Request:**
```json
{
  "client_name": "My Third-Party App",
  "redirect_uris": [
    "https://example.com/oauth/callback"
  ],
  "scopes": [
    "read:triggers",
    "write:triggers"
  ],
  "grant_types": [
    "authorization_code",
    "refresh_token"
  ],
  "is_trusted": false
}
```

**Response:**
```json
{
  "client_id": "client_550e8400-e29b-41d4-a716-446655440000",
  "client_secret": "cs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk",
  "client_name": "My Third-Party App",
  "redirect_uris": ["https://example.com/oauth/callback"],
  "scopes": ["read:triggers", "write:triggers"],
  "grant_types": ["authorization_code", "refresh_token"],
  "is_trusted": false,
  "created_at": "2025-01-30T10:00:00Z"
}
```

**⚠️ IMPORTANT:** The `client_secret` is shown **only once**. Store it securely!

### Client Management

**List Clients:** `GET /api/v1/oauth/clients`
- Returns all OAuth clients for the authenticated user's organization
- Client secrets are never included in list responses

**Get Client:** `GET /api/v1/oauth/clients/:id`
- Returns client details (secret masked)

**Delete Client:** `DELETE /api/v1/oauth/clients/:id`
- Revokes all tokens associated with the client
- Cascades to delete authorization codes

## Scopes & Permissions

OAuth scopes follow the pattern `<resource>:<action>`.

### Available Scopes

| Scope | Description |
|-------|-------------|
| `read:triggers` | View triggers and conditions |
| `write:triggers` | Create and update triggers |
| `delete:triggers` | Delete triggers |
| `read:billing` | View credit balance and transactions |
| `write:billing` | Purchase credits |
| `read:api-keys` | List API keys |
| `write:api-keys` | Create and rotate API keys |
| `delete:api-keys` | Revoke API keys |
| `read:agents` | View linked agents |
| `write:agents` | Link agents to organization |
| `delete:agents` | Unlink agents |
| `read:organizations` | View organization details |
| `write:organizations` | Update organization settings |
| `admin:all` | Full administrative access |

### Scope Validation

**Redirect URI Requirements:**
- Must use HTTPS (except `localhost` / `127.0.0.1` for development)
- Must be registered with the OAuth client
- Wildcard URIs are not allowed

**Scope Intersection:**
- Granted scopes ≤ Requested scopes ≤ Client allowed scopes
- Users can approve a subset of requested scopes

## Authorization Code Flow

### Step 1: Authorization Request

**Endpoint:** `GET /api/v1/oauth/authorize`

**Parameters:**
```
response_type=code
client_id=client_550e8400-e29b-41d4-a716-446655440000
redirect_uri=https://example.com/oauth/callback
scope=read:triggers write:triggers
state=random_state_string
code_challenge=BASE64URL(SHA256(code_verifier))
code_challenge_method=S256
```

**PKCE Parameters:**
- `code_challenge`: Base64URL-encoded SHA256 hash of code verifier
- `code_challenge_method`: Must be `S256` (SHA256)
- `code_verifier`: Random 43-128 character string (stored by client)

### Step 2: User Authorization

The user is presented with an authorization page showing:
- Application name
- Requested scopes
- Organization context (if user has multiple organizations)
- Approve/Deny buttons

### Step 3: Authorization Response

**Success:**
```
HTTP/1.1 302 Found
Location: https://example.com/oauth/callback?code=AUTH_CODE&state=random_state_string
```

**Error:**
```
HTTP/1.1 302 Found
Location: https://example.com/oauth/callback?error=access_denied&state=random_state_string
```

### Step 4: Token Exchange

**Endpoint:** `POST /api/v1/oauth/token`

**Request:**
```http
POST /api/v1/oauth/token HTTP/1.1
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code
&client_id=client_550e8400-e29b-41d4-a716-446655440000
&client_secret=cs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk
&code=AUTH_CODE
&redirect_uri=https://example.com/oauth/callback
&code_verifier=ORIGINAL_CODE_VERIFIER
```

**Response:**
```json
{
  "access_token": "oauth_at_XYZ...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "oauth_rt_ABC...",
  "scope": "read:triggers write:triggers"
}
```

## Token Management

### Using Access Tokens

Include the access token in the `Authorization` header:

```http
GET /api/v1/triggers HTTP/1.1
Authorization: Bearer oauth_at_XYZ...
```

The API Gateway extracts the token, verifies it against the hashed value in the database, and checks:
1. Token is not revoked
2. Token is not expired
3. Required scope is granted

### Refreshing Tokens

**Endpoint:** `POST /api/v1/oauth/token`

**Request:**
```http
POST /api/v1/oauth/token HTTP/1.1
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token
&client_id=client_550e8400-e29b-41d4-a716-446655440000
&client_secret=cs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk
&refresh_token=oauth_rt_ABC...
```

**Response:**
```json
{
  "access_token": "oauth_at_NEW...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "scope": "read:triggers write:triggers"
}
```

### Revoking Tokens

**Endpoint:** `POST /api/v1/oauth/revoke`

**Request:**
```http
POST /api/v1/oauth/revoke HTTP/1.1
Content-Type: application/x-www-form-urlencoded

token=oauth_at_XYZ...
&client_id=client_550e8400-e29b-41d4-a716-446655440000
&client_secret=cs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijk
```

## Organization Context

### Multi-Organization Support

Each OAuth client is associated with a single organization. When a user authorizes an application:

1. **Single Organization:** Authorization proceeds directly
2. **Multiple Organizations:** User must select which organization to grant access to

### Organization Selection Flow

**Authorization Request with Organization:**
```
GET /api/v1/oauth/authorize?
    response_type=code
    &client_id=client_xxx
    &redirect_uri=https://example.com/callback
    &scope=read:triggers
    &state=random
    &organization_id=org_123  ← Optional: Pre-select organization
```

**Authorization Page:**
```
┌─────────────────────────────────────┐
│  Authorize "My Third-Party App"     │
├─────────────────────────────────────┤
│  This app is requesting access to:  │
│  ✓ View triggers                    │
│  ✓ Create triggers                  │
│                                      │
│  Select Organization:                │
│  ○ Personal Organization             │
│  ● Acme Corp (acme-corp)            │
│  ○ Beta Inc (beta-inc)              │
│                                      │
│  [Cancel]         [Authorize]       │
└─────────────────────────────────────┘
```

### Organization-Scoped Tokens

Access tokens are scoped to a specific organization. API calls using the token can only access resources belonging to that organization.

**Token Structure (conceptual):**
```json
{
  "user_id": "user_123",
  "organization_id": "org_456",
  "client_id": "client_xxx",
  "scopes": ["read:triggers", "write:triggers"],
  "expires_at": "2025-01-30T11:00:00Z"
}
```

## API Reference

### OAuth Client Endpoints

```
POST   /api/v1/oauth/clients              Create OAuth client
GET    /api/v1/oauth/clients              List organization's clients
GET    /api/v1/oauth/clients/:id          Get client details
DELETE /api/v1/oauth/clients/:id          Delete client
PUT    /api/v1/oauth/clients/:id          Update client (rotate secret, update scopes)
```

### Authorization Endpoints

```
GET    /api/v1/oauth/authorize            Authorization request (shows consent page)
POST   /api/v1/oauth/authorize            Authorization decision (approve/deny)
POST   /api/v1/oauth/token                Token exchange/refresh
POST   /api/v1/oauth/revoke               Token revocation
GET    /api/v1/oauth/tokeninfo            Token introspection
```

### Token Management Endpoints

```
GET    /api/v1/oauth/tokens               List user's active tokens
DELETE /api/v1/oauth/tokens/:id           Revoke specific token
DELETE /api/v1/oauth/tokens               Revoke all tokens for user
```

## Examples

### Example 1: Server-Side Application (Confidential Client)

```typescript
// Step 1: Register OAuth client
const client = await fetch('https://api.8004.dev/api/v1/oauth/clients', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${userJwt}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    client_name: 'Analytics Dashboard',
    redirect_uris: ['https://dashboard.example.com/oauth/callback'],
    scopes: ['read:triggers', 'read:billing'],
    grant_types: ['authorization_code', 'refresh_token']
  })
});

const { client_id, client_secret } = await client.json();
// Store client_secret securely!

// Step 2: Generate authorization URL
const authUrl = new URL('https://api.8004.dev/api/v1/oauth/authorize');
authUrl.searchParams.set('response_type', 'code');
authUrl.searchParams.set('client_id', client_id);
authUrl.searchParams.set('redirect_uri', 'https://dashboard.example.com/oauth/callback');
authUrl.searchParams.set('scope', 'read:triggers read:billing');
authUrl.searchParams.set('state', generateRandomState());

// Redirect user to authUrl...

// Step 3: Handle callback
app.get('/oauth/callback', async (req, res) => {
  const { code, state } = req.query;

  // Verify state to prevent CSRF
  if (state !== req.session.oauth_state) {
    throw new Error('Invalid state');
  }

  // Exchange code for token
  const tokenResponse = await fetch('https://api.8004.dev/api/v1/oauth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'authorization_code',
      client_id,
      client_secret,
      code,
      redirect_uri: 'https://dashboard.example.com/oauth/callback'
    })
  });

  const { access_token, refresh_token, expires_in } = await tokenResponse.json();

  // Store tokens securely
  req.session.access_token = access_token;
  req.session.refresh_token = refresh_token;

  res.redirect('/dashboard');
});

// Step 4: Use access token
const triggers = await fetch('https://api.8004.dev/api/v1/triggers', {
  headers: { 'Authorization': `Bearer ${access_token}` }
});
```

### Example 2: Single-Page Application (Public Client with PKCE)

```javascript
// Step 1: Generate PKCE code verifier
function generateCodeVerifier() {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return base64UrlEncode(array);
}

function sha256(plain) {
  return crypto.subtle.digest('SHA-256', new TextEncoder().encode(plain));
}

async function generateCodeChallenge(verifier) {
  const hashed = await sha256(verifier);
  return base64UrlEncode(new Uint8Array(hashed));
}

// Step 2: Store code verifier in sessionStorage
const codeVerifier = generateCodeVerifier();
sessionStorage.setItem('pkce_code_verifier', codeVerifier);

// Step 3: Generate authorization URL with PKCE
const codeChallenge = await generateCodeChallenge(codeVerifier);
const authUrl = new URL('https://api.8004.dev/api/v1/oauth/authorize');
authUrl.searchParams.set('response_type', 'code');
authUrl.searchParams.set('client_id', PUBLIC_CLIENT_ID);
authUrl.searchParams.set('redirect_uri', window.location.origin + '/callback');
authUrl.searchParams.set('scope', 'read:triggers write:triggers');
authUrl.searchParams.set('state', generateRandomState());
authUrl.searchParams.set('code_challenge', codeChallenge);
authUrl.searchParams.set('code_challenge_method', 'S256');

window.location.href = authUrl;

// Step 4: Handle callback
const urlParams = new URLSearchParams(window.location.search);
const code = urlParams.get('code');
const codeVerifier = sessionStorage.getItem('pkce_code_verifier');

const tokenResponse = await fetch('https://api.8004.dev/api/v1/oauth/token', {
  method: 'POST',
  headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
  body: new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: PUBLIC_CLIENT_ID,
    code,
    redirect_uri: window.location.origin + '/callback',
    code_verifier: codeVerifier  // PKCE verification
  })
});

const { access_token } = await tokenResponse.json();
```

### Example 3: Machine-to-Machine (Client Credentials)

```python
import requests

# Step 1: Obtain access token using client credentials
token_response = requests.post(
    'https://api.8004.dev/api/v1/oauth/token',
    data={
        'grant_type': 'client_credentials',
        'client_id': 'client_xxx',
        'client_secret': 'cs_xxx',
        'scope': 'read:triggers write:triggers'
    }
)

access_token = token_response.json()['access_token']

# Step 2: Use access token for API calls
triggers = requests.get(
    'https://api.8004.dev/api/v1/triggers',
    headers={'Authorization': f'Bearer {access_token}'}
)

print(triggers.json())
```

## Security Best Practices

1. **Always use HTTPS** for redirect URIs (except localhost)
2. **Use PKCE** for public clients (SPAs, mobile apps)
3. **Implement state parameter** to prevent CSRF attacks
4. **Store client secrets securely** (environment variables, secret managers)
5. **Rotate secrets regularly** (every 90 days recommended)
6. **Use short-lived access tokens** (1 hour default)
7. **Implement refresh token rotation** for enhanced security
8. **Revoke tokens** when users log out or change passwords
9. **Monitor OAuth activity** via audit logs
10. **Request minimal scopes** (principle of least privilege)

## Troubleshooting

### Common Errors

**`invalid_client`**
- Client ID or secret is incorrect
- Client has been deleted
- Solution: Verify credentials, regenerate if necessary

**`invalid_grant`**
- Authorization code expired (5 minutes)
- Authorization code already used
- Redirect URI mismatch
- Solution: Start authorization flow again

**`invalid_scope`**
- Requested scope not allowed for client
- Solution: Update client's allowed scopes or request fewer scopes

**`access_denied`**
- User denied authorization
- User doesn't have permission for requested scopes
- Solution: User must approve or request different scopes

## Future Enhancements (Phase 4-5)

- [ ] OAuth 2.1 compliance (RFC 9449)
- [ ] Dynamic client registration (RFC 7591)
- [ ] Device authorization grant (RFC 8628)
- [ ] JWT access tokens (RFC 9068)
- [ ] Token binding
- [ ] OpenID Connect support

## References

- [RFC 6749: OAuth 2.0 Authorization Framework](https://tools.ietf.org/html/rfc6749)
- [RFC 7636: PKCE](https://tools.ietf.org/html/rfc7636)
- [RFC 6750: Bearer Token Usage](https://tools.ietf.org/html/rfc6750)
- [OAuth 2.0 Security Best Practices](https://tools.ietf.org/html/draft-ietf-oauth-security-topics)

---

**Last Updated:** January 30, 2025
**Status:** Partially Implemented (Phase 4-5)
**Contact:** For questions, see `rust-backend/crates/api-gateway/src/models/oauth.rs`
