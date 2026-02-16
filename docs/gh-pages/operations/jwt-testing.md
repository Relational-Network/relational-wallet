---
layout: default
title: JWT Testing Guide
parent: Operations
nav_order: 2
---

# JWT Testing Guide

How to obtain a Clerk JWT and test enclave API routes.

## Prerequisites

1. Running backend (`https://localhost:8080`)
2. Clerk app configured for wallet-web and rust-server
3. `curl`

## Roles

Current role parsing supports:

- `admin`
- `client`
- `support`
- `auditor`

Admin-only routes are under `/v1/admin/*`.

## Getting a Token

### From wallet-web session

1. Start frontend and sign in at `http://localhost:3000/sign-in`
2. Open DevTools -> Network
3. Inspect `/api/proxy/*` requests and reuse the authenticated session via cookie for proxy tests

### Programmatic (server-side)

```ts
import { auth } from "@clerk/nextjs/server";

const { getToken } = await auth();
const token = (await getToken({ template: "default" })) ?? (await getToken());
```

### Export token

```bash
export JWT="<your-jwt>"
```

## Core Health

```bash
curl -k https://localhost:8080/health
curl -k https://localhost:8080/health/live
curl -k https://localhost:8080/health/ready
```

## User + Wallet

```bash
# Current user
curl -k https://localhost:8080/v1/users/me \
  -H "Authorization: Bearer $JWT"

# Create wallet
curl -k -X POST https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"label":"Test Wallet"}'

# List wallets
curl -k https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT"
```

## Balance + Transfers (Fuji)

```bash
# Native balance
curl -k "https://localhost:8080/v1/wallets/<wallet_id>/balance/native?network=fuji" \
  -H "Authorization: Bearer $JWT"

# Full balance
curl -k "https://localhost:8080/v1/wallets/<wallet_id>/balance?network=fuji" \
  -H "Authorization: Bearer $JWT"

# Gas estimate
curl -k -X POST https://localhost:8080/v1/wallets/<wallet_id>/estimate \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x742d35Cc6634C0532925a3b844Bc9e7595f8fB23","amount":"0.01","token":"native","network":"fuji"}'

# Send
curl -k -X POST https://localhost:8080/v1/wallets/<wallet_id>/send \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x742d35Cc6634C0532925a3b844Bc9e7595f8fB23","amount":"0.01","token":"native","network":"fuji"}'
```

## History + Bookmarks + Invites

```bash
# Transactions
curl -k "https://localhost:8080/v1/wallets/<wallet_id>/transactions?network=fuji" \
  -H "Authorization: Bearer $JWT"

# Bookmarks list (wallet_id query required)
curl -k "https://localhost:8080/v1/bookmarks?wallet_id=<wallet_id>" \
  -H "Authorization: Bearer $JWT"

# Create bookmark
curl -k -X POST https://localhost:8080/v1/bookmarks \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"<wallet_id>","name":"Savings","address":"0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00"}'

# Invite lookup
curl -k "https://localhost:8080/v1/invite?invite_code=INVITE123"

# Invite redeem (invite_id body)
curl -k -X POST https://localhost:8080/v1/invite/redeem \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"invite_id":"<invite_id>"}'
```

## Recurring

```bash
# List recurring for a wallet
curl -k "https://localhost:8080/v1/recurring/payments?wallet_id=<wallet_id>" \
  -H "Authorization: Bearer $JWT"

# Create recurring
curl -k -X POST https://localhost:8080/v1/recurring/payments \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id":"<wallet_id>",
    "wallet_public_key":"pubkey",
    "recipient":"0x742d35Cc6634C0532925a3b844Bc9e7595f8fB23",
    "amount":10.0,
    "currency_code":"rEUR",
    "payment_start_date":739500,
    "frequency":30,
    "payment_end_date":739860
  }'
```

## Fiat Flows

```bash
# Providers
curl -k https://localhost:8080/v1/fiat/providers \
  -H "Authorization: Bearer $JWT"

# Create on-ramp request
curl -k -X POST https://localhost:8080/v1/fiat/onramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"wallet_id":"<wallet_id>","amount_eur":"25.00","provider":"truelayer_sandbox"}'

# Create off-ramp request
curl -k -X POST https://localhost:8080/v1/fiat/offramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id":"<wallet_id>",
    "amount_eur":"10.00",
    "provider":"truelayer_sandbox",
    "beneficiary_account_holder_name":"Relational Test User",
    "beneficiary_iban":"GB79CLRB04066800102649"
  }'

# List fiat requests (optionally filter by wallet_id)
curl -k "https://localhost:8080/v1/fiat/requests?wallet_id=<wallet_id>" \
  -H "Authorization: Bearer $JWT"
```

## Admin + Reserve (Admin role required)

```bash
# Stats
curl -k https://localhost:8080/v1/admin/stats \
  -H "Authorization: Bearer $JWT"

# Service wallet status
curl -k https://localhost:8080/v1/admin/fiat/service-wallet \
  -H "Authorization: Bearer $JWT"

# Bootstrap service wallet
curl -k -X POST https://localhost:8080/v1/admin/fiat/service-wallet/bootstrap \
  -H "Authorization: Bearer $JWT"

# Reserve topup
curl -k -X POST https://localhost:8080/v1/admin/fiat/reserve/topup \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"amount_eur":"250.00"}'

# Reserve transfer
curl -k -X POST https://localhost:8080/v1/admin/fiat/reserve/transfer \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00","amount_eur":"25.00"}'

# Manual request sync
curl -k -X POST https://localhost:8080/v1/admin/fiat/requests/<request_id>/sync \
  -H "Authorization: Bearer $JWT"
```

## Troubleshooting

- `401`: token missing/invalid/expired
- `403`: role or ownership restriction
- `400`: payload or query mismatch
- `503`: dependency not configured/available (provider, chain, webhook disabled)
