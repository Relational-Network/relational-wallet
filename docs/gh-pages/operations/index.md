---
layout: default
title: Operations
nav_order: 7
has_children: true
permalink: /operations/
---

# Operations

Runbooks for local operation, admin checks, and docs publishing.

## Standard Dev Loop

### Backend (SGX)

```bash
cd apps/rust-server
make dev-check
make dev-test
make
make start-rust-server
```

### Frontend

```bash
cd apps/wallet-web
pnpm install
pnpm dev
```

## Health Checks

```bash
curl -k https://localhost:8080/health
curl -k https://localhost:8080/health/live
curl -k https://localhost:8080/health/ready
```

## Key Runtime Env Vars

### rust-server

| Variable | Required | Purpose |
|----------|----------|---------|
| `CLERK_JWKS_URL` | Yes (prod) | JWT signature verification |
| `CLERK_ISSUER` | Yes (prod) | JWT issuer validation |
| `CLERK_AUDIENCE` | Optional | JWT audience validation |
| `CORS_ALLOWED_ORIGINS` | Recommended | Restrictive CORS in non-dev |
| `TRUELAYER_CLIENT_ID` | Fiat | TrueLayer OAuth client |
| `TRUELAYER_CLIENT_SECRET` | Fiat | TrueLayer OAuth secret |
| `TRUELAYER_SIGNING_KEY_ID` | Fiat | TrueLayer signing key id |
| `TRUELAYER_SIGNING_PRIVATE_KEY_PATH` / `_PEM` | Fiat | TrueLayer signing key |
| `TRUELAYER_MERCHANT_ACCOUNT_ID` | Fiat | TrueLayer merchant account |
| `TRUELAYER_WEBHOOK_SHARED_SECRET` | Optional | Enables webhook auth/check |
| `REUR_CONTRACT_ADDRESS_FUJI` | Fiat reserve | Fuji `rEUR` contract |
| `FIAT_RESERVE_BOOTSTRAP_ENABLED` | Optional | Reserve wallet bootstrap behavior |
| `FIAT_RESERVE_INITIAL_TOPUP_EUR` | Optional | Default reserve top-up |
| `FIAT_MIN_CONFIRMATIONS` | Optional | Off-ramp deposit confirmation threshold |

### wallet-web

| Variable | Required | Purpose |
|----------|----------|---------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Yes | Clerk client key |
| `CLERK_SECRET_KEY` | Yes | Clerk server key |
| `WALLET_API_BASE_URL` | Yes | Backend URL for proxy |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Dev-only | Allow self-signed RA-TLS certs |

## Operator Route

- `/wallets/bootstrap` provides admin-oriented reserve wallet and fiat request controls through the frontend proxy.

## Sub-pages

- **Docs Publishing** - Jekyll/GitHub Pages publishing flow
- **JWT Testing Guide** - Token handling and route validation examples
