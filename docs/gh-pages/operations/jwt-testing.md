---
layout: default
title: JWT Testing Guide
parent: Operations
nav_order: 2
---

# JWT Testing Guide

This guide explains how to obtain and use JWT tokens to test all API endpoints and roles in the Relational Wallet enclave.

## Prerequisites

1. A [Clerk](https://clerk.dev) account with the application configured
2. The enclave backend running (see [Installation](/installation/rust-server))
3. `curl` or a similar HTTP client

## Understanding Roles

The wallet backend supports four hierarchical roles:

| Role | Access Level | Use Case |
|------|--------------|----------|
| **Admin** | Full access to all data and endpoints | System administrators |
| **Support** | Read-only metadata access (planned) | Customer support |
| **Auditor** | Read-only audit access (planned) | Compliance |
| **Client** | Own resources only | Regular users |

Roles are hierarchical: **Admin > Support > Auditor > Client**

## Method 1: Using the Frontend (Recommended for Client Role)

The easiest way to get a JWT is through the wallet-web frontend:

1. Start the frontend: `cd apps/wallet-web && pnpm dev`
2. Sign in at `http://localhost:3000/sign-in`
3. Open browser Developer Tools → Network tab
4. Navigate to any page that makes API calls (e.g., `/wallets`)
5. Find a request to `/api/proxy/*`
6. In the Request Headers, copy the cookie value

However, for direct API testing, you'll need the raw JWT token.

## Method 2: Clerk Dashboard (Admin Portal)

Get tokens directly from Clerk's dashboard:

1. Go to [Clerk Dashboard](https://dashboard.clerk.com)
2. Select your application
3. Navigate to **Users**
4. Click on a user
5. Go to **Sessions** tab
6. Click **Create Token** or **Get Token**
7. Copy the JWT

## Method 3: Frontend SDK (Programmatic)

Add this code to a test page or component:

```typescript
// In a client component
import { useAuth } from "@clerk/nextjs";

function TokenDisplay() {
  const { getToken } = useAuth();
  
  const copyToken = async () => {
    const token = await getToken();
    console.log("JWT Token:", token);
    navigator.clipboard.writeText(token || "");
    alert("Token copied to clipboard!");
  };

  return <button onClick={copyToken}>Copy JWT Token</button>;
}
```

Or in a server component:

```typescript
// In a server component or API route
import { auth } from "@clerk/nextjs/server";

export async function GET() {
  const { getToken } = await auth();
  const token = await getToken();
  console.log("JWT Token:", token);
  // Use the token...
}
```

## Method 4: Clerk Backend SDK (Node.js)

Install the Clerk SDK and create a token programmatically:

```bash
npm install @clerk/clerk-sdk-node
```

```typescript
import { clerkClient } from "@clerk/clerk-sdk-node";

// Create a session token for testing
async function getTestToken(userId: string) {
  const token = await clerkClient.signInTokens.createSignInToken({
    userId,
    expiresInSeconds: 3600, // 1 hour
  });
  return token.token;
}
```

## Assigning Roles to Users

### Via Clerk Dashboard

1. Go to Clerk Dashboard → Users
2. Select the user
3. Click **Public metadata**
4. Add JSON:
   ```json
   {
     "role": "admin"
   }
   ```
5. Save changes

### Via Clerk API

```typescript
import { clerkClient } from "@clerk/clerk-sdk-node";

await clerkClient.users.updateUser(userId, {
  publicMetadata: {
    role: "admin", // or "support", "auditor", "client"
  },
});
```

## Testing API Endpoints

### Export Your Token

```bash
export JWT="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImlxWk..."
```

### Health Check (No Auth Required)

```bash
# Basic health
curl -k https://localhost:8080/health

# Liveness probe
curl -k https://localhost:8080/health/live

# Readiness probe
curl -k https://localhost:8080/health/ready
```

### User Endpoints

```bash
# Get current user info
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/users/me
```

### Wallet Endpoints (Client Role)

```bash
# List your wallets
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/wallets

# Create a wallet
curl -k -X POST -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"label": "Test Wallet"}' \
  https://localhost:8080/v1/wallets

# Get a specific wallet
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/wallets/{wallet_id}

# Delete a wallet (soft delete)
curl -k -X DELETE -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/wallets/{wallet_id}
```

### Bookmark Endpoints (Client Role)

```bash
# List bookmarks
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/bookmarks

# Create a bookmark
curl -k -X POST -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"wallet_id": "wallet_id", "name": "My Bookmark", "address": "0x..."}' \
  https://localhost:8080/v1/bookmarks

# Delete a bookmark
curl -k -X DELETE -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/bookmarks/{bookmark_id}
```

### Invite Endpoints

```bash
# Check if invite code is valid
curl -k https://localhost:8080/v1/invite?code=INVITE123

# Redeem an invite code
curl -k -X POST -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"code": "INVITE123"}' \
  https://localhost:8080/v1/invite/redeem
```

### Recurring Payment Endpoints (Client Role)

```bash
# List recurring payments
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/recurring/payments

# Create a recurring payment
curl -k -X POST -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id": "wallet_id",
    "recipient_address": "0x...",
    "amount_cents": 1000,
    "currency": "EUR",
    "frequency": "monthly",
    "next_payment_date": "2026-02-01"
  }' \
  https://localhost:8080/v1/recurring/payments

# Get payments due today
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/recurring/payments/today
```

### Admin Endpoints (Admin Role Required)

⚠️ **These require a user with `role: "admin"` in their public metadata**

```bash
# Get system statistics
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/stats

# List all wallets (all users)
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/wallets

# List all users
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/users

# Query audit logs
curl -k -H "Authorization: Bearer $JWT" \
  "https://localhost:8080/v1/admin/audit/events?limit=50"

# Filter audit logs by user
curl -k -H "Authorization: Bearer $JWT" \
  "https://localhost:8080/v1/admin/audit/events?user_id=user_xxx&limit=10"

# Detailed health check
curl -k -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/health

# Suspend a wallet
curl -k -X POST -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/wallets/{wallet_id}/suspend

# Activate a wallet
curl -k -X POST -H "Authorization: Bearer $JWT" \
  https://localhost:8080/v1/admin/wallets/{wallet_id}/activate
```

## Testing Role Enforcement

### Test Client Cannot Access Admin Endpoints

```bash
# Using a regular user token (should return 403)
curl -k -H "Authorization: Bearer $CLIENT_JWT" \
  https://localhost:8080/v1/admin/stats
# Expected: {"error": "Forbidden", "status": 403}
```

### Test Users Cannot Access Other Users' Wallets

```bash
# Using User A's token to access User B's wallet (should return 403)
curl -k -H "Authorization: Bearer $USER_A_JWT" \
  https://localhost:8080/v1/wallets/{user_b_wallet_id}
# Expected: {"error": "Forbidden", "status": 403}
```

## Development Mode vs Production Mode

### Development Mode (No JWKS URL)

When `CLERK_JWKS_URL` is not set, the backend:
- Accepts any well-formed JWT
- Does NOT verify signatures
- Logs warnings about disabled verification

```bash
# Check if running in dev mode
curl -k https://localhost:8080/health/ready
# Look for "jwks": "unconfigured" in response
```

### Production Mode (JWKS URL Set)

When `CLERK_JWKS_URL` is configured:
- JWT signatures are verified against Clerk JWKS
- Issuer is validated against `CLERK_ISSUER`
- Audience is validated against `CLERK_AUDIENCE` (if set)
- Invalid tokens return 401

Configure for production:

```bash
export CLERK_JWKS_URL="https://your-clerk-instance.clerk.accounts.dev/.well-known/jwks.json"
export CLERK_ISSUER="https://your-clerk-instance.clerk.accounts.dev"
```

## Troubleshooting

### 401 Unauthorized

- **Cause**: Missing or invalid token
- **Check**: Token is included in `Authorization: Bearer <token>` header
- **Check**: Token hasn't expired (Clerk tokens typically expire in 60 seconds)
- **Check**: In production mode, verify JWKS URL is correct

### 403 Forbidden

- **Cause**: Valid token, but insufficient permissions
- **Check**: User has correct role in public metadata
- **Check**: For wallet operations, verify user owns the resource

### Network Error

- **Cause**: Backend not running or not reachable
- **Check**: `curl -k https://localhost:8080/health`
- **Check**: Enclave is running: `gramine-sgx rust-server`

### Self-Signed Certificate Warning

The `-k` flag in curl skips certificate verification. This is expected for development with RA-TLS self-signed certificates.

## Interactive API Testing with Swagger

The backend includes Swagger UI for interactive testing:

1. Open `https://localhost:8080/docs` in your browser
2. Accept the self-signed certificate warning
3. Click **Authorize** button
4. Enter your JWT token
5. Test endpoints interactively

## JWT Token Structure

A Clerk JWT contains these claims:

```json
{
  "azp": "your_clerk_client_id",
  "exp": 1706454321,
  "iat": 1706454261,
  "iss": "https://your-clerk-instance.clerk.accounts.dev",
  "nbf": 1706454251,
  "sid": "sess_xxx",
  "sub": "user_xxx"
}
```

The backend extracts:
- `sub` → User ID
- `sid` → Session ID
- `publicMetadata.role` → Role (via Clerk's custom claims)

## Security Notes

- **Never commit JWTs to version control**
- **Tokens expire quickly** - get a fresh one for each test session
- **Production should always validate signatures** via JWKS
- **Use HTTPS** even in development (RA-TLS provides this)
