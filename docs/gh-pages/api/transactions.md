---
layout: default
title: Transactions
parent: API Reference
nav_order: 3
---

# Transactions API
{: .fs-7 }

Sign and broadcast transactions, estimate gas fees, and query transaction history. All signing happens inside the SGX enclave --- private keys never leave the hardware boundary.
{: .fs-5 .fw-300 }

---

## Send Transaction

Sign and broadcast a transaction from a wallet. The private key is loaded from sealed storage, the transaction is signed inside the enclave, and the signed transaction is broadcast to the Avalanche C-Chain.

```http
POST /v1/wallets/{wallet_id}/send
Authorization: Bearer <jwt>
Content-Type: application/json
```

### Request Body

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `amount` | string | Yes | Amount to send (human-readable, e.g., `"1.5"`) |
| `to` | string | Conditional | Recipient address (0x...). Required if `to_email_hash` not set. |
| `to_email_hash` | string | Conditional | SHA-256 hash of recipient email. Required if `to` not set. |
| `network` | string | Yes | Network name (e.g., `"fuji"`) |
| `token` | string | Yes | Token type (`"AVAX"` for native, `"rEUR"` for ERC-20) |
| `gas_limit` | string | No | Custom gas limit (overrides estimate) |
| `max_priority_fee_per_gas` | string | No | Custom priority fee (EIP-1559) |

### Example: Send Native AVAX

```bash
curl -k -X POST https://localhost:8080/v1/wallets/wal_a1b2c3d4/send \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "0.1",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "AVAX"
  }'
```

### Example: Send rEUR Token

```bash
curl -k -X POST https://localhost:8080/v1/wallets/wal_a1b2c3d4/send \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "10.0",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "rEUR"
  }'
```

### Example: Send to Email Recipient

```bash
curl -k -X POST https://localhost:8080/v1/wallets/wal_a1b2c3d4/send \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "5.0",
    "to_email_hash": "sha256_of_recipient_email",
    "network": "fuji",
    "token": "AVAX"
  }'
```

### Response `200 OK`

```json
{
  "tx_hash": "0xabc123def456...",
  "status": "pending",
  "explorer_url": "https://testnet.snowtrace.io/tx/0xabc123def456..."
}
```

### Errors

| Code | Reason |
|:-----|:-------|
| `400` | Invalid parameters (bad address, missing fields) |
| `403` | Wallet belongs to another user or is suspended |
| `404` | Wallet not found |
| `422` | Insufficient balance for amount + gas fees |
| `503` | RPC node unavailable |

---

## Estimate Gas

Estimate the gas cost for a transaction before sending.

```http
POST /v1/wallets/{wallet_id}/estimate
Authorization: Bearer <jwt>
Content-Type: application/json
```

### Request Body

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `amount` | string | Yes | Amount to send |
| `to` | string | Conditional | Recipient address |
| `to_email_hash` | string | Conditional | Recipient email hash |
| `network` | string | Yes | Network name |
| `token` | string | Yes | Token type |

### Example

```bash
curl -k -X POST https://localhost:8080/v1/wallets/wal_a1b2c3d4/estimate \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "0.1",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "AVAX"
  }'
```

### Response `200 OK`

```json
{
  "gas_limit": "21000",
  "max_fee_per_gas": "30000000000",
  "max_priority_fee_per_gas": "1500000000",
  "estimated_cost_wei": "630000000000000",
  "estimated_cost": "0.00063"
}
```

The estimate uses EIP-1559 gas pricing with `max_fee_per_gas` and `max_priority_fee_per_gas`.
{: .note }

---

## List Transactions

Retrieve transaction history for a wallet with cursor-based pagination.

```http
GET /v1/wallets/{wallet_id}/transactions
Authorization: Bearer <jwt>
```

### Query Parameters

| Parameter | Type | Required | Description |
|:----------|:-----|:---------|:------------|
| `network` | string | No | Filter by network |
| `limit` | integer | No | Results per page (default varies) |
| `cursor` | string | No | Pagination cursor from previous response |
| `direction` | string | No | Filter by direction (`sent`, `received`) |

### Example

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions?limit=10" \
  -H "Authorization: Bearer $JWT"
```

### Response `200 OK`

```json
{
  "transactions": [
    {
      "tx_hash": "0xabc123def456...",
      "status": "confirmed",
      "direction": "sent",
      "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
      "to": "0x1234567890abcdef1234567890abcdef12345678",
      "amount": "0.1",
      "token": "AVAX",
      "network": "fuji",
      "explorer_url": "https://testnet.snowtrace.io/tx/0xabc123...",
      "timestamp": "2026-03-15T10:35:00Z",
      "block_number": 12345678
    }
  ],
  "next_cursor": "cursor_xyz"
}
```

Use `next_cursor` in subsequent requests to paginate. When `next_cursor` is `null`, there are no more results.

---

## Get Transaction Status

Poll for the confirmation status of a specific transaction.

```http
GET /v1/wallets/{wallet_id}/transactions/{tx_hash}
Authorization: Bearer <jwt>
```

### Path Parameters

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `wallet_id` | string | Wallet identifier |
| `tx_hash` | string | Transaction hash (0x...) |

### Example

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions/0xabc123..." \
  -H "Authorization: Bearer $JWT"
```

### Response `200 OK`

```json
{
  "tx_hash": "0xabc123def456...",
  "status": "confirmed",
  "block_number": 12345678,
  "confirmations": 15,
  "gas_used": "21000",
  "timestamp": "2026-03-15T10:35:00Z"
}
```

### Transaction Statuses

| Status | Description |
|:-------|:------------|
| `pending` | Transaction broadcast but not yet included in a block |
| `confirmed` | Transaction included in a block and confirmed |
| `failed` | Transaction reverted or dropped |

---

## Transaction Lifecycle

```
POST /v1/wallets/{id}/send
  │
  ▼
Status: "pending" (tx_hash returned)
  │
  ▼ (poll GET /v1/wallets/{id}/transactions/{tx_hash})
  │
  ├── Status: "confirmed" (included in block, gas_used available)
  │
  └── Status: "failed" (reverted or dropped)
```

The frontend polls the transaction status endpoint after sending. Typical confirmation time on Fuji is 1-2 seconds.
