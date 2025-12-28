# Social Login (OAuth 2.0)

**Status**: Implemented (December 25, 2025 - Account Linking Added)

This document describes the social authentication system for api.agentauri.ai, enabling users to sign in with Google or GitHub accounts.

## Overview

Social login uses OAuth 2.0 Authorization Code flow with PKCE to authenticate users via third-party identity providers. This provides:

- **Simplified onboarding**: No password to remember
- **Trusted identity**: Verified email from provider
- **Account linking**: Connect multiple providers to one account

## Supported Providers

| Provider | Status | Login Endpoint | Callback Endpoint | Link Endpoint |
|----------|--------|----------------|-------------------|---------------|
| Google | Active | `/api/v1/auth/google` | `/api/v1/auth/google/callback` | `/api/v1/auth/link/google` |
| GitHub | Active | `/api/v1/auth/github` | `/api/v1/auth/github/callback` | `/api/v1/auth/link/github` |

## Authentication Flow

```
User                    Frontend                API Gateway              Provider (Google/GitHub)
  |                        |                         |                           |
  |-- Click Login -------->|                         |                           |
  |                        |-- GET /auth/google ---->|                           |
  |                        |                         |-- Generate state -------->|
  |                        |<-- 302 Redirect --------|                           |
  |                        |                         |                           |
  |<-- Redirect to Provider Auth Page -------------------------------------->|
  |                        |                         |                           |
  |-- User Grants Consent -------------------------------------------------->|
  |                        |                         |                           |
  |<-- Redirect to /callback?code=xxx&state=yyy ----------------------------|
  |                        |                         |                           |
  |                        |-- GET /callback ------->|                           |
  |                        |                         |-- Exchange code -------->|
  |                        |                         |<-- Access token ---------|
  |                        |                         |-- Get user info -------->|
  |                        |                         |<-- Profile data ---------|
  |                        |                         |                           |
  |                        |                         |-- Create/find user       |
  |                        |                         |-- Generate JWT           |
  |                        |<-- 302 Redirect + JWT --|                           |
  |<-- Set token, redirect to app ---------|        |                           |
```

## API Endpoints

### Initiate Login

```http
GET /api/v1/auth/{provider}
```

**Parameters**:
- `provider`: `google` or `github`

**Response**: 302 redirect to provider's authorization page

**Example**:
```bash
curl -v http://localhost:8080/api/v1/auth/google
# < HTTP/1.1 302 Found
# < Location: https://accounts.google.com/o/oauth2/v2/auth?...
```

### OAuth Callback

```http
GET /api/v1/auth/{provider}/callback
```

**Query Parameters**:
- `code`: Authorization code from provider
- `state`: CSRF protection token

**Success Response**: 302 redirect to frontend with JWT token

```
302 Found
Location: https://app.agentauri.ai/auth/callback?token=eyJ...&user_id=usr_xxx
```

**Error Response**: 302 redirect with error details

```
302 Found
Location: https://app.agentauri.ai/auth/callback?error=auth_failed&message=...
```

### Link Provider to Existing Account (NEW)

For authenticated users who want to add a new provider to their account:

```http
GET /api/v1/auth/link/{provider}
Authorization: Bearer <jwt_token>
```

**Parameters**:
- `provider`: `google` or `github`
- `redirect_after` (query, optional): URL to redirect after linking

**Behavior**:
1. Extracts user ID from JWT token
2. Initiates OAuth flow with linking mode
3. On callback, links provider to existing account
4. Returns success or error redirect

**Example**:
```bash
# Link Google to existing account
curl -v -H "Authorization: Bearer eyJ..." \
  "http://localhost:8080/api/v1/auth/link/google?redirect_after=/settings"
```

**Success Response**:
```
302 Found
Location: /settings?linked=google&success=true
```

**Error Cases**:
- `already_linked`: Provider already linked to another account
- `identity_exists`: This Google/GitHub account already linked to different user

### Session Management Endpoints (NEW)

#### Get Current User Profile

```http
GET /api/v1/auth/me
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "id": "usr_abc123",
  "username": "john-doe",
  "email": "john@example.com",
  "name": "John Doe",
  "avatar": "https://...",
  "wallets": [
    { "address": "0x1234...abcd", "chain_id": 1 }
  ],
  "providers": ["google", "github"],
  "organizations": [
    { "id": "org_123", "name": "My Org", "slug": "my-org", "role": "owner" }
  ],
  "created_at": "2025-12-25T00:00:00Z"
}
```

#### Generate Nonce (for SIWE)

```http
POST /api/v1/auth/nonce
```

**Response**:
```json
{
  "nonce": "abc123-uuid",
  "expires_at": "2025-12-25T01:00:00Z",
  "message": "Sign this message to authenticate with AgentAuri..."
}
```

#### Logout

```http
POST /api/v1/auth/logout
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

## User Account Behavior

### New User (Registration)

When a user signs in for the first time:

1. Email is checked against existing users
2. If no match: New user created with provider as primary auth
3. Personal organization created automatically
4. JWT token returned

**User record created**:
```json
{
  "id": "usr_abc123",
  "username": "john-doe-1234",
  "email": "john@example.com",
  "password_hash": null,
  "primary_auth_provider": "google",
  "avatar_url": "https://lh3.googleusercontent.com/..."
}
```

### Existing User (Login)

When a user with matching email exists:

1. Email matched to existing user
2. Identity linked if not already linked
3. Last login timestamp updated
4. JWT token returned

### Account Linking

Users can link multiple providers to one account:

- Sign in with Google creates account
- Later sign in with GitHub (same email) links to existing account
- Both providers can now be used to log in

**User identities table**:
```sql
user_identities (
  user_id,           -- Links to users.id
  provider,          -- 'google', 'github', 'email', 'wallet'
  provider_user_id,  -- Unique ID from provider
  email,             -- Provider-reported email
  display_name,      -- Provider display name
  avatar_url         -- Provider avatar
)
```

## Configuration

### Required Environment Variables

```bash
# Frontend URL for redirects
FRONTEND_URL=https://app.agentauri.ai

# OAuth state signing (REQUIRED in production)
# Must be at least 32 characters, high entropy
OAUTH_STATE_SECRET=your-secure-random-secret-at-least-32-chars

# Google OAuth 2.0
GOOGLE_CLIENT_ID=123456789.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=GOCSPX-xxxxxxxxxxxxx
GOOGLE_REDIRECT_URI=https://api.agentauri.ai/api/v1/auth/google/callback

# GitHub OAuth 2.0
GITHUB_CLIENT_ID=Iv1.xxxxxxxxxxxx
GITHUB_CLIENT_SECRET=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
GITHUB_REDIRECT_URI=https://api.agentauri.ai/api/v1/auth/github/callback
```

### Provider Setup

#### Google Cloud Console

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create or select a project
3. Navigate to **APIs & Services > Credentials**
4. Create **OAuth 2.0 Client ID** (Web application)
5. Add authorized redirect URIs:
   - Development: `http://localhost:8080/api/v1/auth/google/callback`
   - Production: `https://api.agentauri.ai/api/v1/auth/google/callback`
6. Copy Client ID and Client Secret

#### GitHub Developer Settings

1. Go to [GitHub Developer Settings](https://github.com/settings/developers)
2. Create **New OAuth App**
3. Set Authorization callback URL:
   - Development: `http://localhost:8080/api/v1/auth/github/callback`
   - Production: `https://api.agentauri.ai/api/v1/auth/github/callback`
4. Copy Client ID and generate Client Secret

## Security

### State Parameter (CSRF Protection)

The OAuth flow uses a signed state parameter to prevent CSRF attacks:

```
state = base64(timestamp || redirect_uri || hmac_signature)
```

- **Timestamp**: Request time (prevents replay after 10 minutes)
- **Redirect URI**: Prevents redirect manipulation
- **HMAC-SHA256**: Signed with `OAUTH_STATE_SECRET`

### Production Requirements

In production (`ENVIRONMENT=production`):

- `OAUTH_STATE_SECRET` must be set (panics if missing)
- `OAUTH_STATE_SECRET` must be at least 32 characters
- `FRONTEND_URL` must be set for redirects

### Error Handling

Errors are returned as redirect parameters (not exposed to provider):

| Error Code | Description |
|------------|-------------|
| `session_expired` | State token expired (> 10 minutes) |
| `auth_failed` | Provider rejected or code exchange failed |
| `email_required` | Provider didn't return email |
| `service_unavailable` | Database or internal error |

## Integration Examples

### Frontend (React)

```typescript
// Login button
const handleGoogleLogin = () => {
  window.location.href = `${API_URL}/api/v1/auth/google`;
};

// Callback page
useEffect(() => {
  const params = new URLSearchParams(window.location.search);
  const token = params.get('token');
  const error = params.get('error');

  if (token) {
    localStorage.setItem('jwt', token);
    navigate('/dashboard');
  } else if (error) {
    setError(params.get('message'));
  }
}, []);
```

### Backend Verification

```rust
// SocialAuthService validates:
// 1. State parameter (HMAC signature, timestamp)
// 2. Exchanges code for tokens with provider
// 3. Fetches user profile from provider
// 4. Creates/updates user and identity records
// 5. Generates JWT with standard claims
```

## Limitations

- **Email required**: Provider must return verified email
- **No refresh tokens**: Social tokens not stored (re-auth required)
- **Single sign-on only**: No delegated API access via OAuth

## Related Documentation

- [AUTHENTICATION.md](./AUTHENTICATION.md) - 3-layer authentication overview
- [API_KEYS.md](./API_KEYS.md) - API key authentication (Layer 1)
- [OAUTH.md](./OAUTH.md) - OAuth 2.0 client credentials flow (for apps)

---

**Last Updated**: December 25, 2025
