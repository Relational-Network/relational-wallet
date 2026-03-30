---
layout: default
title: Wallets
parent: API Reference
nav_order: 2
---

# Wallets API
{: .fs-7 }

Create, list, retrieve, and delete wallets. Each wallet has a secp256k1 key pair generated inside the SGX enclave, producing an Ethereum-compatible address on the Avalanche C-Chain.
{: .fs-5 .fw-300 }

---

## Create Wallet

Generate a new wallet with a secp256k1 key pair inside the enclave.

```http
POST /v1/wallets
Authorization: Bearer <jwt>
Content-Type: application/json
```

### Request Body

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `label` | string | No | Human-readable label for the wallet |

```json
{
  "label": "My Savings"
}
```

### Response `201 Created`

```json
{
  "wallet": {
    "wallet_id": "wal_a1b2c3d4",
    "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
    "label": "My Savings",
    "status": "active",
    "created_at": "2026-03-15T10:30:00Z"
  },
  "message": "Wallet created successfully"
}
```

### Example

```bash
curl -k -X POST https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"label": "My Savings"}'
```

### Errors

| Code | Reason |
|:-----|:-------|
| `401` | Missing or invalid JWT |
| `500` | Key generation or storage failure |

---

## List Wallets

Retrieve all wallets owned by the authenticated user.

```http
GET /v1/wallets
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "wallets": [
    {
      "wallet_id": "wal_a1b2c3d4",
      "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
      "label": "My Savings",
      "status": "active",
      "created_at": "2026-03-15T10:30:00Z"
    }
  ],
  "total": 1
}
```

### Example

```bash
curl -k https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT"
```

---

## Get Wallet

Retrieve a specific wallet by ID. Ownership is enforced.

```http
GET /v1/wallets/{wallet_id}
Authorization: Bearer <jwt>
```

### Path Parameters

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `wallet_id` | string | Wallet identifier |

### Response `200 OK`

```json
{
  "wallet_id": "wal_a1b2c3d4",
  "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
  "label": "My Savings",
  "status": "active",
  "created_at": "2026-03-15T10:30:00Z"
}
```

### Errors

| Code | Reason |
|:-----|:-------|
| `403` | Wallet belongs to another user |
| `404` | Wallet not found |

---

## Delete Wallet

Soft-delete a wallet. The wallet is marked as `deleted` but data is retained.

```http
DELETE /v1/wallets/{wallet_id}
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "message": "Wallet deleted",
  "wallet_id": "wal_a1b2c3d4"
}
```

### Errors

| Code | Reason |
|:-----|:-------|
| `403` | Wallet belongs to another user |
| `404` | Wallet not found |

### Example

```bash
curl -k -X DELETE https://localhost:8080/v1/wallets/wal_a1b2c3d4 \
  -H "Authorization: Bearer $JWT"
```

---

## Get Balance

Query the native AVAX balance and ERC-20 token balances for a wallet.

```http
GET /v1/wallets/{wallet_id}/balance
Authorization: Bearer <jwt>
```

### Query Parameters

| Parameter | Type | Required | Description |
|:----------|:-----|:---------|:------------|
| `network` | string | No | Network name (default: fuji) |
| `tokens` | string | No | Comma-separated token symbols to query |

### Response `200 OK`

```json
{
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
  "network": "fuji",
  "chain_id": 43113,
  "native_balance": {
    "symbol": "AVAX",
    "name": "Avalanche",
    "balance_raw": "1500000000000000000",
    "balance_formatted": "1.5",
    "decimals": 18,
    "contract_address": null
  },
  "token_balances": [
    {
      "symbol": "rEUR",
      "name": "Relational Euro",
      "balance_raw": "10000000",
      "balance_formatted": "10.0",
      "decimals": 6,
      "contract_address": "0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63"
    }
  ]
}
```

### Example

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/balance?network=fuji" \
  -H "Authorization: Bearer $JWT"
```

### Errors

| Code | Reason |
|:-----|:-------|
| `403` | Wallet belongs to another user |
| `404` | Wallet not found |
| `503` | RPC node unavailable |

---

## Wallet Statuses

| Status | Description | Operations Allowed |
|:-------|:------------|:-------------------|
| `active` | Normal operating state | All operations |
| `suspended` | Admin-suspended | Read-only (no sends, no fiat) |
| `deleted` | Soft-deleted by owner | None |

---

## Bookmarks

Address book entries are scoped to a wallet. See the full [API overview](/relational-wallet/api) for bookmark endpoints.

### List Bookmarks

```http
GET /v1/bookmarks?wallet_id={wallet_id}
Authorization: Bearer <jwt>
```

### Create Bookmark

```http
POST /v1/bookmarks
Authorization: Bearer <jwt>
Content-Type: application/json
```

```json
{
  "wallet_id": "wal_a1b2c3d4",
  "name": "Alice",
  "recipient_type": "address",
  "address": "0x1234567890abcdef1234567890abcdef12345678"
}
```

For email-based bookmarks:

```json
{
  "wallet_id": "wal_a1b2c3d4",
  "name": "Bob",
  "recipient_type": "email",
  "email_hash": "sha256_of_email",
  "email_display": "b***@example.com"
}
```

### Delete Bookmark

```http
DELETE /v1/bookmarks/{bookmark_id}
Authorization: Bearer <jwt>
```

**Response:** `204 No Content`

---

## Payment Links

Create shareable payment request links tied to a wallet.

### Create Payment Link

```http
POST /v1/wallets/{wallet_id}/payment-link
Authorization: Bearer <jwt>
Content-Type: application/json
```

```json
{
  "recipient_type": "address",
  "amount": "1.5",
  "expires_hours": 24,
  "note": "Dinner payment",
  "single_use": true,
  "token": "AVAX"
}
```

### Response `200 OK`

```json
{
  "token": "pay_xyz789",
  "expires_at": "2026-03-16T10:30:00Z"
}
```

### Resolve Payment Link (No Auth)

```http
GET /v1/payment-link/{token}
```

```json
{
  "recipient_type": "address",
  "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
  "amount": "1.5",
  "note": "Dinner payment",
  "token_type": "AVAX"
}
```
