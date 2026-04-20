---
layout: default
title: Backend (Rust Server)
parent: API Reference
nav_order: 7
---

# Backend API Reference (Rust Server)

The enclave backend exposes a full REST API documented via OpenAPI 3.1.

- **Base URL:** `https://localhost:8080`
- **OpenAPI JSON:** [`/api-doc/openapi.json`](https://localhost:8080/api-doc/openapi.json)
- **Swagger UI:** [`/docs`](https://localhost:8080/docs)

All `/v1/*` routes require `Authorization: Bearer <jwt>` unless stated otherwise.

---

## Quick Reference

| Category | Links |
|:---------|:------|
| Authentication | [Authentication](/relational-wallet/api/authentication) |
| Wallets & Balances | [Wallets](/relational-wallet/api/wallets) |
| Transactions | [Transactions](/relational-wallet/api/transactions) |
| Fiat On/Off-Ramp | [Fiat](/relational-wallet/api/fiat) |
| Admin | [Admin](/relational-wallet/api/admin) |
| Error Codes | [Errors](/relational-wallet/api/errors) |

---

## All Endpoints at a Glance

```
GET  /health
GET  /health/live
GET  /health/ready

GET  /v1/users/me
POST /v1/resolve/email

GET  /v1/wallets
POST /v1/wallets
GET  /v1/wallets/{wallet_id}
DEL  /v1/wallets/{wallet_id}
GET  /v1/wallets/{wallet_id}/balance
POST /v1/wallets/{wallet_id}/send
POST /v1/wallets/{wallet_id}/estimate
GET  /v1/wallets/{wallet_id}/transactions
GET  /v1/wallets/{wallet_id}/transactions/{tx_hash}
POST /v1/wallets/{wallet_id}/payment-link

GET  /v1/bookmarks
POST /v1/bookmarks
DEL  /v1/bookmarks/{bookmark_id}

GET  /v1/payment-link/{token}

GET  /v1/fiat/providers
POST /v1/fiat/onramp/requests
POST /v1/fiat/offramp/requests
GET  /v1/fiat/requests
GET  /v1/fiat/requests/{request_id}
POST /v1/fiat/providers/truelayer/webhook

GET  /v1/admin/stats
GET  /v1/admin/health
GET  /v1/admin/users
GET  /v1/admin/wallets
POST /v1/admin/wallets/{wallet_id}/suspend
POST /v1/admin/wallets/{wallet_id}/activate
GET  /v1/admin/audit/events
GET  /v1/admin/fiat/service-wallet
POST /v1/admin/fiat/requests/{request_id}/sync
```
