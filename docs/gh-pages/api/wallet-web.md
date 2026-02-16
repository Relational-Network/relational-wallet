---
layout: default
title: Wallet Web
parent: API Documentation
nav_order: 2
---

# Wallet Web API

`wallet-web` exposes a catch-all proxy route:

- `/api/proxy/[...path]`

This lets browsers call the SGX backend without directly handling self-signed RA-TLS certificates.

## Proxy Behavior

Request flow:

```text
Browser -> /api/proxy/* -> Next.js server -> https://localhost:8080/*
```

Per request, the proxy:

1. Resolves backend URL from `WALLET_API_BASE_URL` (default `https://localhost:8080`)
2. Gets Clerk token server-side
3. Prefers `getToken({ template: "default" })`, then falls back to `getToken()`
4. Adds `Authorization: Bearer <jwt>` when token exists
5. Forwards `x-request-id` header when present
6. Returns backend status/body with passthrough content type

Supported methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`.

## Safety Guard

The proxy throws in production when both are true:

- `NODE_ENV=production`
- `NODE_TLS_REJECT_UNAUTHORIZED=0`

Use trusted certificates in production.

## Main Frontend API Surfaces

Common routes that call the proxy:

- `/wallets` dashboard (`/api/proxy/v1/wallets`, `/balance`, `/transactions`, `/fiat/providers`)
- `/wallets/[wallet_id]/fiat` (`/api/proxy/v1/fiat/*`)
- `/wallets/bootstrap` (admin fiat reserve endpoints)
- `/pay` (wallet listing + send flow prefill)

## Example Calls

```bash
# Wallet list via proxy
curl http://localhost:3000/api/proxy/v1/wallets \
  -H "Cookie: __session=..."

# Fiat requests for wallet via proxy
curl "http://localhost:3000/api/proxy/v1/fiat/requests?wallet_id=<wallet_id>" \
  -H "Cookie: __session=..."

# Admin reserve topup via proxy
curl -X POST http://localhost:3000/api/proxy/v1/admin/fiat/reserve/topup \
  -H "Content-Type: application/json" \
  -H "Cookie: __session=..." \
  -d '{"amount_eur":"250.00"}'
```

## Environment Variables

```env
WALLET_API_BASE_URL=https://localhost:8080
NODE_TLS_REJECT_UNAUTHORIZED=0  # development only
```
