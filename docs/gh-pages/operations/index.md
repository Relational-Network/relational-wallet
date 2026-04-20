---
layout: default
title: Operations
nav_order: 8
has_children: true
permalink: /operations/
---

# Operations
{: .fs-8 }

Runbooks for local development, production deployment, health monitoring, and JWT testing.
{: .fs-5 .fw-300 }

---

## Standard Development Loop

### Backend (SGX)

```bash
cd apps/rust-server

# Quick feedback loop (no SGX required)
make dev-check         # Fast compile check
make dev-test          # Run 155+ unit tests

# Full SGX build and run
make                   # Build SGX artifacts
make start-rust-server # Launch inside enclave

# Verify
curl -k https://localhost:8080/health
```

### Frontend

```bash
cd apps/wallet-web
pnpm install           # First time only
pnpm dev               # http://localhost:3000
```

---

## Health Checks

```bash
# Basic health
curl -k https://localhost:8080/health

# Kubernetes liveness probe
curl -k https://localhost:8080/health/live

# Kubernetes readiness probe (checks all dependencies)
curl -k https://localhost:8080/health/ready

# Detailed health (admin only)
curl -k https://localhost:8080/v1/admin/health \
  -H "Authorization: Bearer $ADMIN_JWT"
```

---

## Environment Variables Quick Reference

### Rust Server

| Variable | Required | Purpose |
|:---------|:---------|:--------|
| `CLERK_JWKS_URL` | Yes (prod) | JWT signature verification |
| `CLERK_ISSUER` | Yes (prod) | JWT issuer validation |
| `CLERK_AUDIENCE` | Recommended | JWT audience claim restriction |
| `CORS_ALLOWED_ORIGINS` | Recommended | Restrict cross-origin requests |
| `HOST` | No (default: `0.0.0.0`) | Bind address |
| `PORT` | No (default: `8080`) | Bind port |
| `LOG_FORMAT` | No (default: `pretty`) | `json` for structured logging |
| `FUJI_RPC_URL` | Yes | Avalanche C-Chain RPC endpoint |
| `TRUELAYER_CLIENT_ID` | Fiat | TrueLayer OAuth client ID |
| `TRUELAYER_CLIENT_SECRET` | Fiat | TrueLayer OAuth secret |
| `TRUELAYER_SIGNING_KEY_ID` | Fiat | TrueLayer signing key ID |
| `TRUELAYER_SIGNING_PRIVATE_KEY_PEM` | Fiat | TrueLayer signing key (PEM) |
| `TRUELAYER_MERCHANT_ACCOUNT_ID` | Fiat | TrueLayer merchant account |
| `TRUELAYER_WEBHOOK_SHARED_SECRET` | Fiat | Webhook HMAC validation |
| `REUR_CONTRACT_ADDRESS_FUJI` | Fiat | rEUR contract address on Fuji |
| `FIAT_MIN_CONFIRMATIONS` | No (default: `1`) | Min block confirmations for settlement |

### Wallet Web

| Variable | Required | Purpose |
|:---------|:---------|:--------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Yes | Clerk public key |
| `CLERK_SECRET_KEY` | Yes | Clerk server key |
| `WALLET_API_BASE_URL` | Yes | Backend URL for proxy |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Dev-only | Accept RA-TLS self-signed certs |

---

## Monitoring

Monitor these signals in production:

| Signal | How to check | Alert threshold |
|:-------|:-------------|:----------------|
| Backend health | `GET /health/ready` | Non-200 response |
| Storage usage | `GET /v1/admin/health` (as admin) | `total_files` growth rate |
| Auth failures | Audit log `event_type=auth_failure` | Spike in failure rate |
| Fiat stuck requests | `GET /v1/fiat/requests?active_only=true` | Requests older than 1 hour with non-terminal status |
| SGX availability | `/dev/sgx/enclave` device exists | Device missing = enclave cannot start |

---

## Sub-pages

- [**JWT Testing**](/relational-wallet/operations/jwt-testing) --- Obtain tokens and validate all API routes
- [**Docs Publishing**](/relational-wallet/operations/publishing) --- Build and deploy this documentation site
