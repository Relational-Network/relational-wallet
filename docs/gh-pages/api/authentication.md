---
layout: default
title: Authentication
parent: API Reference
nav_order: 1
---

# Authentication
{: .fs-7 }

All protected endpoints require a Clerk JWT. This page covers the authentication flow, token format, and development mode.
{: .fs-5 .fw-300 }

---

## Bearer Token

Include the JWT in every request to `/v1/*` endpoints:

```http
GET /v1/wallets HTTP/1.1
Host: localhost:8080
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

---

## JWT Claims

The backend extracts the following claims:

| Claim | Source | Purpose |
|:------|:-------|:--------|
| `sub` | Standard JWT | User identifier (e.g., `user_2abc123`) |
| `iss` | Standard JWT | Must match `CLERK_ISSUER` |
| `aud` | Standard JWT | Must match `CLERK_AUDIENCE` (if configured) |
| `exp` | Standard JWT | Token expiry (60s clock skew tolerance) |
| `sid` | Clerk-specific | Session identifier |
| `publicMetadata.role` | Clerk-specific | User role (`admin`, `client`, `support`, `auditor`) |

If `publicMetadata.role` is absent, the user defaults to `client`.

---

## Supported Algorithms

| Algorithm | Type | Status |
|:----------|:-----|:-------|
| RS256 | RSA PKCS#1 v1.5 + SHA-256 | Supported (Clerk default) |
| RS384 | RSA PKCS#1 v1.5 + SHA-384 | Supported |
| RS512 | RSA PKCS#1 v1.5 + SHA-512 | Supported |
| ES256 | ECDSA P-256 + SHA-256 | Supported |

---

## Obtaining a Token

### From the Web UI (Development)

1. Sign in at `http://localhost:3000`
2. Navigate to `/account`
3. Copy the displayed JWT token

### From Clerk API

```bash
curl -X POST "https://api.clerk.dev/v1/sessions/{session_id}/tokens" \
  -H "Authorization: Bearer $CLERK_SECRET_KEY" \
  -H "Content-Type: application/json"
```

### Using the Token

```bash
# Set as environment variable
export JWT="eyJhbGciOiJSUzI1NiIs..."

# Use in requests
curl -k https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT"
```

---

## User Info Endpoint

Verify your authentication and role:

```bash
curl -k https://localhost:8080/v1/users/me \
  -H "Authorization: Bearer $JWT"
```

**Response:**

```json
{
  "user_id": "user_2abc123",
  "role": "client",
  "session_id": "sess_xyz789"
}
```

---

## Role-Based Access

| Role | `/v1/wallets` | `/v1/admin/*` | Own resources only |
|:-----|:--------------|:--------------|:-------------------|
| `admin` | Yes | Yes | No (full access) |
| `client` | Yes | No (403) | Yes |
| `support` | Yes | No (403) | Yes |
| `auditor` | Yes | No (403) | Yes |

### Setting a User's Role

In the Clerk dashboard:
1. Go to **Users** > select user
2. Edit **Public Metadata**
3. Set: `{ "role": "admin" }`

Or via Clerk API:

```bash
curl -X PATCH "https://api.clerk.dev/v1/users/{user_id}" \
  -H "Authorization: Bearer $CLERK_SECRET_KEY" \
  -H "Content-Type: application/json" \
  -d '{"public_metadata": {"role": "admin"}}'
```

---

## Development Mode

When the server is compiled with the `dev` feature flag and no `CLERK_JWKS_URL` is configured:

- JWT **signature is not verified** (token is only decoded and shape-checked)
- Claims (issuer, audience, expiry) are still validated
- This allows testing without a real Clerk instance

```bash
cargo dev-test    # Tests run with dev feature enabled
```

Never use `dev` feature in production builds.
{: .warning }

---

## Error Responses

| Status | Meaning | Example |
|:-------|:--------|:--------|
| `401` | Missing or invalid token | `{"error": "unauthorized", "message": "Missing authorization header"}` |
| `401` | Expired token | `{"error": "unauthorized", "message": "Token expired"}` |
| `401` | Invalid signature | `{"error": "unauthorized", "message": "Invalid token signature"}` |
| `403` | Insufficient role | `{"error": "forbidden", "message": "Admin access required"}` |
| `403` | Ownership violation | `{"error": "forbidden", "message": "Not authorized to access this wallet"}` |
