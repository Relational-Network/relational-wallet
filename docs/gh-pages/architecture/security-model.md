---
layout: default
title: Security Model
parent: Architecture
nav_order: 2
---

# Security Model

This document summarizes security boundaries for key custody, auth, and data persistence.

## SGX Boundary

The backend runs inside Intel SGX with Gramine.

Security properties:

1. Enclave memory isolation from host/userland processes
2. Enclave measurement/signing identity checks (MRENCLAVE/MRSIGNER model)
3. RA-TLS certificate generation at runtime via `gramine-ratls`

All backend traffic is HTTPS; there is no HTTP fallback.

## Authentication

Protected routes require:

```http
Authorization: Bearer <jwt>
```

JWT handling:

1. Extract bearer token
2. Verify against Clerk JWKS in production mode
3. Validate issuer (`CLERK_ISSUER`) and optional audience (`CLERK_AUDIENCE`)
4. Apply clock skew tolerance
5. Extract user identity from `sub`
6. Extract role from `publicMetadata.role` (defaults to `client`)

## Auth Modes

| Mode | Condition | Behavior |
|------|-----------|----------|
| Production auth | `CLERK_JWKS_URL` configured | Signature + claims verification |
| Dev auth path | `dev` feature without JWKS | Decode/shape checks for local iteration |

## Role Enforcement

- Admin-only routes: `/v1/admin/*`
- Resource ownership checks for wallet-scoped user routes
- Support/auditor roles are parsed but do not currently have dedicated route families

## Storage Protection

Persistent storage root is `/data`, mounted as encrypted filesystem in Gramine.

Representative layout:

```text
/data/
  wallets/
  bookmarks/
  invites/
  recurring/
  fiat/
  system/fiat_service_wallet/
  audit/
```

Key rules:

1. Wallet private keys are generated and used inside enclave context
2. Private key files are never returned through API responses
3. Sensitive operations are audit-logged

## Network and Dependency Surface

- Avalanche Fuji RPC for balance/tx/settlement calls
- TrueLayer APIs for fiat provider flows
- Clerk JWKS endpoint for JWT verification

## Request Tracing

`x-request-id` is supported across proxy/backend hops for correlation and diagnostics.

## Production Checklist

- Set `CLERK_JWKS_URL` and `CLERK_ISSUER`
- Restrict CORS with `CORS_ALLOWED_ORIGINS`
- Use `sgx.debug = false` in production manifests
- Keep signing keys and provider secrets outside source control
- Avoid `NODE_TLS_REJECT_UNAUTHORIZED=0` in production frontend/proxy environments
