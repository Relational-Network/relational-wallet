---
layout: default
title: Rust Server
parent: API Documentation
nav_order: 1
---

# Rust Server API

Backend API running inside SGX enclave (Gramine + RA-TLS).

- Base URL: `https://localhost:8080`
- OpenAPI: `/api-doc/openapi.json`
- Swagger UI: `/docs`

All `/v1/*` routes require a Clerk JWT unless stated otherwise.

```http
Authorization: Bearer <jwt>
```

## Public Health Routes

| Method | Path | Auth |
|--------|------|------|
| GET | `/health` | No |
| GET | `/health/live` | No |
| GET | `/health/ready` | No |

## Auth/User

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/users/me` | Current authenticated user (`user_id`, `role`, optional `session_id`) |

## Wallet Lifecycle

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/wallets` | Create wallet |
| GET | `/v1/wallets` | List wallets for current user |
| GET | `/v1/wallets/{wallet_id}` | Wallet details |
| DELETE | `/v1/wallets/{wallet_id}` | Soft-delete wallet |

### Create Wallet Request

```json
{
  "label": "Personal"
}
```

### Create Wallet Response (`201`)

```json
{
  "wallet": {
    "wallet_id": "c4f3af0c-2e61-4f24-9f96-6a245ce9f692",
    "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
    "created_at": "2026-02-16T12:00:00Z",
    "status": "active",
    "label": "Personal"
  },
  "message": "Wallet created successfully"
}
```

### List Wallets Response (`200`)

```json
{
  "wallets": [
    {
      "wallet_id": "c4f3af0c-2e61-4f24-9f96-6a245ce9f692",
      "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
      "created_at": "2026-02-16T12:00:00Z",
      "status": "active",
      "label": "Personal"
    }
  ],
  "total": 1
}
```

### Delete Wallet Response (`200`)

```json
{
  "message": "Wallet deleted successfully",
  "wallet_id": "c4f3af0c-2e61-4f24-9f96-6a245ce9f692"
}
```

## Balance (Fuji-only)

Any non-`fuji` network value is rejected.

| Method | Path |
|--------|------|
| GET | `/v1/wallets/{wallet_id}/balance` |
| GET | `/v1/wallets/{wallet_id}/balance/native` |

### Full Balance Example

```json
{
  "wallet_id": "c4f3af0c-2e61-4f24-9f96-6a245ce9f692",
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
  "network": "Avalanche Fuji Testnet",
  "chain_id": 43113,
  "native_balance": {
    "symbol": "AVAX",
    "name": "Avalanche",
    "balance_raw": "1000000000000000000",
    "balance_formatted": "1.0",
    "decimals": 18
  },
  "token_balances": [
    {
      "symbol": "USDC",
      "name": "USD Coin",
      "balance_raw": "1000000",
      "balance_formatted": "1.0",
      "decimals": 6,
      "contract_address": "0x5425890298aed601595a70AB815c96711a31Bc65"
    },
    {
      "symbol": "rEUR",
      "name": "Relational Euro",
      "balance_raw": "2500000",
      "balance_formatted": "2.5",
      "decimals": 6,
      "contract_address": "0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63"
    }
  ]
}
```

## Transfers and History (Fuji-only)

| Method | Path |
|--------|------|
| POST | `/v1/wallets/{wallet_id}/estimate` |
| POST | `/v1/wallets/{wallet_id}/send` |
| GET | `/v1/wallets/{wallet_id}/transactions` |
| GET | `/v1/wallets/{wallet_id}/transactions/{tx_hash}` |

### Estimate/Send Request Fields

- `to` (EVM address)
- `amount` (decimal string)
- `token` (`native` or ERC-20 contract address)
- `network` (`fuji`)
- Optional send overrides: `gas_limit`, `max_priority_fee_per_gas`

## Bookmarks

| Method | Path | Notes |
|--------|------|-------|
| GET | `/v1/bookmarks?wallet_id={wallet_id}` | `wallet_id` query param required |
| POST | `/v1/bookmarks` | Body: `wallet_id`, `name`, `address` |
| DELETE | `/v1/bookmarks/{bookmark_id}` | Owner only |

## Invites

| Method | Path | Notes |
|--------|------|-------|
| GET | `/v1/invite?invite_code={code}` | Validate code |
| POST | `/v1/invite/redeem` | Body requires `invite_id` |

Redeem body example:

```json
{
  "invite_id": "8acafe1a-5e0a-45f1-9e59-bce289ded8dd"
}
```

## Recurring Payments

| Method | Path |
|--------|------|
| GET | `/v1/recurring/payments?wallet_id={wallet_id}` |
| POST | `/v1/recurring/payments` |
| PUT | `/v1/recurring/payment/{recurring_payment_id}` |
| DELETE | `/v1/recurring/payment/{recurring_payment_id}` |
| PUT | `/v1/recurring/payment/{recurring_payment_id}/last-paid-date` |
| GET | `/v1/recurring/payments/today` |

Create payload fields:

- `wallet_id`
- `wallet_public_key`
- `recipient`
- `amount`
- `currency_code`
- `payment_start_date` (ordinal day)
- `frequency` (days)
- `payment_end_date` (ordinal day)

## Fiat APIs

### Provider Discovery

| Method | Path |
|--------|------|
| GET | `/v1/fiat/providers` |

### User Fiat Requests

| Method | Path | Notes |
|--------|------|-------|
| POST | `/v1/fiat/onramp/requests` | Create on-ramp request |
| POST | `/v1/fiat/offramp/requests` | Create off-ramp request |
| GET | `/v1/fiat/requests` | Optional query `wallet_id` |
| GET | `/v1/fiat/requests/{request_id}` | Single request |

### Provider Webhook

| Method | Path | Notes |
|--------|------|-------|
| POST | `/v1/fiat/providers/truelayer/webhook` | Returns `503` when webhook secret not configured |

### Create Fiat Request Body

```json
{
  "wallet_id": "c4f3af0c-2e61-4f24-9f96-6a245ce9f692",
  "amount_eur": "25.00",
  "provider": "truelayer_sandbox",
  "note": "test"
}
```

For off-ramp, these are required:

- `beneficiary_account_holder_name`
- `beneficiary_iban`

### Fiat Status Values

- `queued`
- `awaiting_provider`
- `awaiting_user_deposit`
- `settlement_pending`
- `provider_pending`
- `completed`
- `failed`

## Admin APIs

| Method | Path |
|--------|------|
| GET | `/v1/admin/stats` |
| GET | `/v1/admin/wallets` |
| GET | `/v1/admin/users` |
| GET | `/v1/admin/audit/events` |
| GET | `/v1/admin/health` |
| POST | `/v1/admin/wallets/{wallet_id}/suspend` |
| POST | `/v1/admin/wallets/{wallet_id}/activate` |

### Admin Fiat / Reserve

| Method | Path |
|--------|------|
| GET | `/v1/admin/fiat/service-wallet` |
| POST | `/v1/admin/fiat/service-wallet/bootstrap` |
| POST | `/v1/admin/fiat/reserve/topup` |
| POST | `/v1/admin/fiat/reserve/transfer` |
| POST | `/v1/admin/fiat/requests/{request_id}/sync` |

## Common Error Statuses

| Status | Meaning |
|--------|---------|
| 400 | Invalid request |
| 401 | Missing/invalid auth |
| 403 | Forbidden / ownership / role |
| 404 | Resource not found |
| 422 | Validation/business rule violation |
| 503 | Dependency unavailable (provider, chain, webhook disabled) |
