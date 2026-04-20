---
layout: default
title: Fiat
parent: API Reference
nav_order: 4
---

# Fiat API
{: .fs-7 }

EUR on-ramp and off-ramp flows powered by TrueLayer. Deposit euros to receive rEUR tokens, or burn rEUR to withdraw euros to a bank account.
{: .fs-5 .fw-300 }

---

## Overview

The fiat integration supports two flows:

| Flow | Direction | Description |
|:-----|:----------|:------------|
| **On-ramp** | EUR → rEUR | User deposits euros via bank payment; receives rEUR tokens |
| **Off-ramp** | rEUR → EUR | User burns rEUR tokens; receives euro payout to bank account |

Both flows use TrueLayer as the payment provider (currently in sandbox mode).

---

## List Providers

Get available fiat providers and their capabilities.

```http
GET /v1/fiat/providers
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "default_provider": "truelayer_sandbox",
  "providers": [
    {
      "provider_id": "truelayer_sandbox",
      "display_name": "TrueLayer (Sandbox)",
      "sandbox": true,
      "enabled": true,
      "supports_on_ramp": true,
      "supports_off_ramp": true
    }
  ]
}
```

---

## On-Ramp (Deposit EUR → Receive rEUR)

### Create On-Ramp Request

```http
POST /v1/fiat/onramp/requests
Authorization: Bearer <jwt>
Content-Type: application/json
```

#### Request Body

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `wallet_id` | string | Yes | Target wallet for rEUR deposit |
| `amount_eur` | number | Yes | Amount in EUR (e.g., `50.00`) |
| `provider` | string | No | Provider ID (default: `truelayer_sandbox`) |
| `note` | string | No | User note for the request |

```json
{
  "wallet_id": "wal_a1b2c3d4",
  "amount_eur": 50.00,
  "note": "Monthly top-up"
}
```

#### Response `201 Created`

```json
{
  "request_id": "fiat_req_123",
  "wallet_id": "wal_a1b2c3d4",
  "direction": "on_ramp",
  "amount_eur": 50.00,
  "provider": "truelayer_sandbox",
  "status": "awaiting_user_deposit",
  "provider_action_url": "https://payment.truelayer-sandbox.com/payments/pay_123",
  "created_at": "2026-03-15T10:30:00Z"
}
```

#### Example

```bash
curl -k -X POST https://localhost:8080/v1/fiat/onramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id": "wal_a1b2c3d4",
    "amount_eur": 50.00
  }'
```

### On-Ramp Lifecycle

```
1. User creates on-ramp request
   Status: "awaiting_user_deposit"
   → provider_action_url returned (TrueLayer hosted payment page)

2. User completes bank payment on TrueLayer
   Status: "provider_pending"
   → TrueLayer processes the payment

3. TrueLayer confirms payment (webhook or poll)
   Status: "settlement_pending"
   → Reserve wallet prepares rEUR transfer

4. Reserve wallet mints rEUR to user's wallet
   Status: "completed"
   → deposit_tx_hash set

   OR

   Status: "failed"
   → failure_reason set
```

---

## Off-Ramp (Burn rEUR → Receive EUR)

### Create Off-Ramp Request

```http
POST /v1/fiat/offramp/requests
Authorization: Bearer <jwt>
Content-Type: application/json
```

#### Request Body

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `wallet_id` | string | Yes | Source wallet (must hold rEUR) |
| `amount_eur` | number | Yes | Amount in EUR to withdraw |
| `provider` | string | No | Provider ID |
| `note` | string | No | User note |
| `beneficiary_account_holder_name` | string | Yes | Bank account holder name |
| `beneficiary_iban` | string | Yes | IBAN for payout |

```json
{
  "wallet_id": "wal_a1b2c3d4",
  "amount_eur": 25.00,
  "beneficiary_account_holder_name": "John Doe",
  "beneficiary_iban": "DE89370400440532013000"
}
```

#### Response `201 Created`

```json
{
  "request_id": "fiat_req_456",
  "wallet_id": "wal_a1b2c3d4",
  "direction": "off_ramp",
  "amount_eur": 25.00,
  "provider": "truelayer_sandbox",
  "status": "queued",
  "created_at": "2026-03-15T11:00:00Z"
}
```

### Off-Ramp Lifecycle

```
1. User creates off-ramp request
   Status: "queued"
   → Request queued for processing

2. System burns rEUR from user wallet
   Status: "settlement_pending"
   → reserve_transfer_tx_hash set

3. System initiates TrueLayer payout
   Status: "provider_pending"
   → TrueLayer processes bank transfer

4. Payout confirmed
   Status: "completed"

   OR

   Status: "failed"
   → failure_reason set
```

---

## List Fiat Requests

```http
GET /v1/fiat/requests
Authorization: Bearer <jwt>
```

### Query Parameters

| Parameter | Type | Required | Description |
|:----------|:-----|:---------|:------------|
| `wallet_id` | string | No | Filter by wallet |
| `active_only` | boolean | No | Only show non-terminal requests |
| `limit` | integer | No | Max results |

### Response `200 OK`

```json
{
  "requests": [
    {
      "request_id": "fiat_req_123",
      "wallet_id": "wal_a1b2c3d4",
      "direction": "on_ramp",
      "amount_eur": 50.00,
      "provider": "truelayer_sandbox",
      "status": "completed",
      "deposit_tx_hash": "0xdef789...",
      "created_at": "2026-03-15T10:30:00Z",
      "updated_at": "2026-03-15T10:35:00Z"
    }
  ],
  "total": 1
}
```

---

## Get Fiat Request Details

```http
GET /v1/fiat/requests/{request_id}
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "request_id": "fiat_req_123",
  "wallet_id": "wal_a1b2c3d4",
  "direction": "on_ramp",
  "amount_eur": 50.00,
  "provider": "truelayer_sandbox",
  "status": "completed",
  "chain_network": "fuji",
  "created_at": "2026-03-15T10:30:00Z",
  "updated_at": "2026-03-15T10:35:00Z",
  "deposit_tx_hash": "0xdef789...",
  "service_wallet_address": "0xReserveWallet...",
  "expected_amount_minor": 5000000,
  "provider_reference": "pay_123",
  "provider_event_id": "evt_456"
}
```

---

## Request Statuses

| Status | Description | Terminal |
|:-------|:------------|:---------|
| `queued` | Request created, not yet processed | No |
| `awaiting_provider` | Waiting for provider initialization | No |
| `awaiting_user_deposit` | User must complete payment on provider | No |
| `provider_pending` | Provider is processing payment/payout | No |
| `settlement_pending` | On-chain settlement in progress | No |
| `completed` | Fully settled | Yes |
| `failed` | Failed at any stage (see `failure_reason`) | Yes |

---

## TrueLayer Webhook

TrueLayer sends payment status updates via webhook. This endpoint does not require JWT authentication but validates the webhook signature if `TRUELAYER_WEBHOOK_SHARED_SECRET` is configured.

```http
POST /v1/fiat/providers/truelayer/webhook
Content-Type: application/json
```

### Response

| Code | Meaning |
|:-----|:--------|
| `202` | Webhook accepted |
| `400` | Invalid payload |
| `403` | Invalid webhook signature |

The webhook endpoint is typically exposed through the [Nginx reverse proxy](/relational-wallet/architecture/system-overview#reverse-proxy-appsproxy) with a valid Let's Encrypt certificate.
{: .note }
