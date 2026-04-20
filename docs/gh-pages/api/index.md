---
layout: default
title: API Reference
nav_order: 3
has_children: true
permalink: /api/
---

# API Reference
{: .fs-8 }

Complete REST API documentation for the Relational Wallet enclave backend. All endpoints are auto-documented via OpenAPI 3.1 and available in the interactive Swagger UI.
{: .fs-5 .fw-300 }

---

## Base URL

| Environment | URL | TLS |
|:------------|:----|:----|
| **Direct (SGX)** | `https://localhost:8080` | RA-TLS (self-signed with attestation) |
| **Browser proxy** | `http://localhost:3000/api/proxy` | Standard TLS via Next.js |
| **Production proxy** | `https://relational-wallet.duckdns.org` | Let's Encrypt |

## Interactive Documentation

| Resource | URL |
|:---------|:----|
| **Swagger UI** | [`https://localhost:8080/docs`](https://localhost:8080/docs) |
| **OpenAPI JSON** | [`https://localhost:8080/api-doc/openapi.json`](https://localhost:8080/api-doc/openapi.json) |

---

## Authentication

All `/v1/*` endpoints require a Clerk JWT in the `Authorization` header:

```http
Authorization: Bearer eyJhbGciOiJSUzI1NiIs...
```

### Obtaining a Token

**From the frontend** (development):
1. Sign in at `http://localhost:3000`
2. Navigate to `/account`
3. Copy the displayed JWT token

**Programmatically** (Clerk Backend SDK):
```bash
# Using Clerk's API
curl https://api.clerk.dev/v1/sessions/{session_id}/tokens \
  -H "Authorization: Bearer sk_test_..."
```

### Roles

| Role | Header Value | Access |
|:-----|:-------------|:-------|
| `admin` | Set via Clerk `publicMetadata.role` | All endpoints including `/v1/admin/*` |
| `client` | Default if no role specified | Own resources only |
| `support` | Set via Clerk `publicMetadata.role` | Parsed, no dedicated endpoints yet |
| `auditor` | Set via Clerk `publicMetadata.role` | Parsed, no dedicated endpoints yet |

---

## Endpoint Overview

### Health (No Auth)

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/health` | Combined health check |
| `GET` | `/health/live` | Liveness probe |
| `GET` | `/health/ready` | Readiness probe (checks dependencies) |

### Wallets

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/wallets` | List user's wallets |
| `POST` | `/v1/wallets` | Create new wallet |
| `GET` | `/v1/wallets/{wallet_id}` | Get wallet details |
| `DELETE` | `/v1/wallets/{wallet_id}` | Soft-delete wallet |

### Balances

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/wallets/{wallet_id}/balance` | Get native + token balances |

### Transactions

| Method | Path | Description |
|:-------|:-----|:------------|
| `POST` | `/v1/wallets/{wallet_id}/send` | Sign and broadcast transaction |
| `POST` | `/v1/wallets/{wallet_id}/estimate` | Estimate gas fees |
| `GET` | `/v1/wallets/{wallet_id}/transactions` | List transaction history |
| `GET` | `/v1/wallets/{wallet_id}/transactions/{tx_hash}` | Get transaction status |

### Bookmarks

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/bookmarks` | List bookmarks for a wallet |
| `POST` | `/v1/bookmarks` | Create bookmark |
| `DELETE` | `/v1/bookmarks/{bookmark_id}` | Delete bookmark |

### Fiat (TrueLayer)

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/fiat/providers` | List fiat providers |
| `POST` | `/v1/fiat/onramp/requests` | Create on-ramp request |
| `POST` | `/v1/fiat/offramp/requests` | Create off-ramp request |
| `GET` | `/v1/fiat/requests` | List fiat requests |
| `GET` | `/v1/fiat/requests/{request_id}` | Get fiat request details |
| `POST` | `/v1/fiat/providers/truelayer/webhook` | TrueLayer webhook (no auth) |

### Payment Links

| Method | Path | Description |
|:-------|:-----|:------------|
| `POST` | `/v1/wallets/{wallet_id}/payment-link` | Create payment link |
| `GET` | `/v1/payment-link/{token}` | Resolve payment link (no auth) |

### Users

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/users/me` | Get current user info |
| `POST` | `/v1/resolve/email` | Resolve email hash to existence |

### Admin (Admin Role Required)

| Method | Path | Description |
|:-------|:-----|:------------|
| `GET` | `/v1/admin/stats` | System statistics |
| `GET` | `/v1/admin/health` | Detailed health status |
| `GET` | `/v1/admin/users` | List all users |
| `GET` | `/v1/admin/wallets` | List all wallets |
| `POST` | `/v1/admin/wallets/{wallet_id}/suspend` | Suspend wallet |
| `POST` | `/v1/admin/wallets/{wallet_id}/activate` | Reactivate wallet |
| `GET` | `/v1/admin/audit/events` | Query audit logs |
| `GET` | `/v1/admin/fiat/service-wallet` | Reserve wallet status |
| `POST` | `/v1/admin/fiat/requests/{request_id}/sync` | Manual fiat sync |

---

## Common Response Codes

| Code | Meaning |
|:-----|:--------|
| `200` | Success |
| `201` | Resource created |
| `204` | Deleted (no body) |
| `400` | Bad request (invalid parameters) |
| `401` | Not authenticated (missing/invalid JWT) |
| `403` | Forbidden (ownership or role check failed) |
| `404` | Resource not found |
| `422` | Unprocessable (e.g., insufficient balance) |
| `503` | Service unavailable (dependency down) |

---

## Sub-pages

- [**Authentication**](/relational-wallet/api/authentication) --- JWT flow, JWKS verification, dev-mode tokens
- [**Wallets**](/relational-wallet/api/wallets) --- Create, list, get, delete wallets
- [**Transactions**](/relational-wallet/api/transactions) --- Send, estimate gas, history, status
- [**Fiat**](/relational-wallet/api/fiat) --- On-ramp, off-ramp, providers, webhooks
- [**Admin**](/relational-wallet/api/admin) --- Stats, users, wallets, audit, suspension
- [**Errors**](/relational-wallet/api/errors) --- Error response format and codes
