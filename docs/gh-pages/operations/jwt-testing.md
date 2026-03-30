---
layout: default
title: JWT Testing
parent: Operations
nav_order: 1
---

# JWT Testing Guide
{: .fs-7 }

How to obtain a Clerk JWT and exercise every enclave API route with `curl`.
{: .fs-5 .fw-300 }

---

## Prerequisites

1. Running backend: `make start-rust-server` (or `make dev-build && ./target/debug/rust-server` for dev mode)
2. Clerk app configured with `CLERK_JWKS_URL` and `CLERK_ISSUER`
3. `curl`, `jq` (optional for pretty-printing)

---

## Obtaining a Token

### Method 1: From the Web UI

1. Start the frontend: `pnpm dev` (in `apps/wallet-web/`)
2. Sign in at `http://localhost:3000/sign-in`
3. Navigate to `http://localhost:3000/account`
4. Copy the JWT token displayed on the page

### Method 2: From Browser DevTools

1. Sign in at `http://localhost:3000`
2. Open DevTools → **Network** tab
3. Inspect any `/api/proxy/*` request
4. Find the `Authorization: Bearer ...` header
5. Copy the token value

### Method 3: Server-side (TypeScript)

```typescript
import { auth } from "@clerk/nextjs/server";

const { getToken } = await auth();
const token = await getToken() ?? await getToken({ template: "default" });
console.log(token);
```

### Export for curl

```bash
export JWT="eyJhbGciOiJSUzI1NiIs..."
export BASE="https://localhost:8080"
```

---

## Health (No Auth)

```bash
curl -k $BASE/health
curl -k $BASE/health/live
curl -k $BASE/health/ready
```

---

## User Info

```bash
# Verify authentication and see your role
curl -k $BASE/v1/users/me \
  -H "Authorization: Bearer $JWT" | jq .
```

---

## Wallets

```bash
# Create wallet
curl -k -X POST $BASE/v1/wallets \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"label":"Test Wallet"}' | jq .

# Capture wallet_id
export WALLET_ID="wal_..."

# List wallets
curl -k $BASE/v1/wallets \
  -H "Authorization: Bearer $JWT" | jq .

# Get specific wallet
curl -k $BASE/v1/wallets/$WALLET_ID \
  -H "Authorization: Bearer $JWT" | jq .

# Delete wallet
curl -k -X DELETE $BASE/v1/wallets/$WALLET_ID \
  -H "Authorization: Bearer $JWT"
```

---

## Balance

```bash
# Full balance (native AVAX + tokens)
curl -k "$BASE/v1/wallets/$WALLET_ID/balance?network=fuji" \
  -H "Authorization: Bearer $JWT" | jq .
```

---

## Transactions

```bash
# Gas estimate
curl -k -X POST $BASE/v1/wallets/$WALLET_ID/estimate \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fB23",
    "amount": "0.01",
    "token": "AVAX",
    "network": "fuji"
  }' | jq .

# Send (requires funded wallet)
curl -k -X POST $BASE/v1/wallets/$WALLET_ID/send \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fB23",
    "amount": "0.01",
    "token": "AVAX",
    "network": "fuji"
  }' | jq .

# Transaction history
curl -k "$BASE/v1/wallets/$WALLET_ID/transactions?network=fuji&limit=10" \
  -H "Authorization: Bearer $JWT" | jq .

# Single transaction status
curl -k "$BASE/v1/wallets/$WALLET_ID/transactions/0x..." \
  -H "Authorization: Bearer $JWT" | jq .
```

---

## Bookmarks

```bash
# List bookmarks for wallet
curl -k "$BASE/v1/bookmarks?wallet_id=$WALLET_ID" \
  -H "Authorization: Bearer $JWT" | jq .

# Create address bookmark
curl -k -X POST $BASE/v1/bookmarks \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_id\": \"$WALLET_ID\",
    \"name\": \"Alice\",
    \"recipient_type\": \"address\",
    \"address\": \"0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00\"
  }" | jq .

# Delete bookmark
curl -k -X DELETE $BASE/v1/bookmarks/bkm_... \
  -H "Authorization: Bearer $JWT"
```

---

## Fiat Flows

```bash
# List providers
curl -k $BASE/v1/fiat/providers \
  -H "Authorization: Bearer $JWT" | jq .

# Create on-ramp request
curl -k -X POST $BASE/v1/fiat/onramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_id\": \"$WALLET_ID\",
    \"amount_eur\": 25.00
  }" | jq .

# Create off-ramp request
curl -k -X POST $BASE/v1/fiat/offramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_id\": \"$WALLET_ID\",
    \"amount_eur\": 10.00,
    \"beneficiary_account_holder_name\": \"Test User\",
    \"beneficiary_iban\": \"GB79CLRB04066800102649\"
  }" | jq .

# List fiat requests
curl -k "$BASE/v1/fiat/requests?wallet_id=$WALLET_ID" \
  -H "Authorization: Bearer $JWT" | jq .

# Get specific request
curl -k $BASE/v1/fiat/requests/fiat_req_... \
  -H "Authorization: Bearer $JWT" | jq .
```

---

## Payment Links

```bash
# Create payment link
curl -k -X POST $BASE/v1/wallets/$WALLET_ID/payment-link \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "recipient_type": "address",
    "amount": "0.5",
    "token": "AVAX",
    "expires_hours": 24,
    "note": "Test payment",
    "single_use": false
  }' | jq .

# Resolve payment link (no auth)
curl -k $BASE/v1/payment-link/pay_... | jq .
```

---

## Admin Routes (Admin Role Required)

Set your Clerk user's `publicMetadata.role = "admin"` first, then obtain a fresh token.

```bash
export ADMIN_JWT="eyJ..."   # JWT with admin role

# System stats
curl -k $BASE/v1/admin/stats \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# Detailed health
curl -k $BASE/v1/admin/health \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# List all users
curl -k $BASE/v1/admin/users \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# List all wallets
curl -k $BASE/v1/admin/wallets \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# Suspend a wallet
curl -k -X POST $BASE/v1/admin/wallets/$WALLET_ID/suspend \
  -H "Authorization: Bearer $ADMIN_JWT"

# Reactivate a wallet
curl -k -X POST $BASE/v1/admin/wallets/$WALLET_ID/activate \
  -H "Authorization: Bearer $ADMIN_JWT"

# Audit logs (last 20 events)
curl -k "$BASE/v1/admin/audit/events?limit=20" \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# Audit logs for specific user
curl -k "$BASE/v1/admin/audit/events?user_id=user_2abc&limit=50" \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# Reserve wallet status
curl -k $BASE/v1/admin/fiat/service-wallet \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .

# Manual fiat sync
curl -k -X POST $BASE/v1/admin/fiat/requests/fiat_req_.../sync \
  -H "Authorization: Bearer $ADMIN_JWT" | jq .
```

---

## Troubleshooting

| Status Code | Common Cause | Fix |
|:------------|:-------------|:----|
| `401` | Missing/expired/invalid JWT | Get a fresh token from the web UI |
| `403` | Wrong owner or insufficient role | Check your role; use the correct wallet ID |
| `400` | Malformed request body | Verify JSON structure and field types |
| `404` | Resource doesn't exist or deleted | Check the ID; note deleted wallets return 404 |
| `422` | Insufficient balance | Fund wallet via faucet or on-ramp |
| `503` | RPC node or TrueLayer unavailable | Check `FUJI_RPC_URL`; verify TrueLayer credentials |
