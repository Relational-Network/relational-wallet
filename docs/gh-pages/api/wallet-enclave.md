---
layout: default
title: Wallet Enclave
parent: API Documentation
nav_order: 1
---

# Wallet Enclave API

The enclave REST API runs inside an Intel SGX enclave with DCAP RA-TLS attestation.

**Base URL**: `https://localhost:8080` (or configured host)  
**OpenAPI Spec**: `/api-doc/openapi.json`  
**Swagger UI**: `/docs`

## Authentication

All protected endpoints require a Clerk JWT:

```
Authorization: Bearer <jwt_token>
```

## Health Endpoints

### GET /health
Check overall service health with component status.

**Response** (200 OK):
```json
{
  "status": "ok",
  "checks": {
    "service": "ok",
    "data_dir": "ok",
    "jwks": "ok"
  }
}
```

### GET /health/live
Liveness probe - returns 200 if process is running.

### GET /health/ready
Readiness probe - returns 200 only if all dependencies (storage, JWKS) are available.

---

## User Endpoints

### GET /v1/users/me
Get the current authenticated user's information.

**Auth**: Required  
**Response** (200 OK):
```json
{
  "user_id": "user_2abc123def",
  "role": "client",
  "session_id": "sess_xyz789"
}
```

---

## Wallet Endpoints

### POST /v1/wallets
Create a new wallet with secp256k1 keypair. The public address is derived using the standard Ethereum method (keccak256 hash of public key, last 20 bytes).

**Auth**: Required  
**Request Body**:
```json
{
  "label": "My Wallet"  // Optional
}
```

**Response** (201 Created):
```json
{
  "wallet_id": "550e8400-e29b-41d4-a716-446655440000",
  "owner_user_id": "user_2abc123def",
  "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
  "status": "active",
  "label": "My Wallet",
  "created_at": "2026-01-29T10:30:00Z"
}
```

**Notes**:
- Address is 42 characters: `0x` prefix + 40 hex characters
- Private key stored securely inside SGX enclave
- Compatible with Ethereum and Avalanche C-Chain

### GET /v1/wallets
List all wallets owned by the authenticated user.

**Auth**: Required  
**Response** (200 OK):
```json
[
  {
    "id": "wallet_abc123",
    "owner_user_id": "user_2abc123def",
    "public_address": "0x1234...abcd",
    "status": "active",
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

### GET /v1/wallets/{wallet_id}
Get a specific wallet by ID.

**Auth**: Required (owner only)  
**Response** (200 OK): Wallet object  
**Error** (403 Forbidden): Not the wallet owner

### DELETE /v1/wallets/{wallet_id}
Soft-delete a wallet (marks as deleted, does not remove data).

**Auth**: Required (owner only)  
**Response** (204 No Content)

---

## Balance Endpoints

### GET /v1/wallets/{wallet_id}/balance
Get the on-chain balance of a wallet on Avalanche C-Chain.

Returns native AVAX balance and configured ERC-20 token balances (including USDC).

**Auth**: Required (owner only)  
**Query Parameters**:
- `network`: `fuji` (default) or `mainnet`
- `tokens`: Additional token contract addresses (comma-separated)

**Response** (200 OK):
```json
{
  "wallet_id": "wallet_abc123",
  "address": "0x1234...abcd",
  "network": "fuji",
  "chain_id": 43113,
  "native_balance": {
    "symbol": "AVAX",
    "name": "Avalanche",
    "balance_raw": "1000000000000000000",
    "balance_formatted": "1.0",
    "decimals": 18,
    "contract_address": null
  },
  "token_balances": [
    {
      "symbol": "USDC",
      "name": "USD Coin",
      "balance_raw": "1000000",
      "balance_formatted": "1.0",
      "decimals": 6,
      "contract_address": "0x5425890298aed601595a70ab815c96711a31bc65"
    }
  ]
}
```

**Errors**:
- 403: Not the wallet owner
- 404: Wallet not found
- 503: Blockchain network unavailable

---

## Transaction Endpoints

### POST /v1/wallets/{wallet_id}/estimate
Estimate gas cost for a transaction before sending.

**Auth**: Required (owner only)  
**Request Body**:
```json
{
  "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
  "amount": "1.5",
  "token": "native",
  "network": "fuji"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `to` | string | Recipient address (0x + 40 hex chars) |
| `amount` | string | Amount to send (human-readable, e.g., "1.5") |
| `token` | string | `"native"` for AVAX or token contract address |
| `network` | string | `"fuji"` or `"mainnet"` (default: fuji) |

**Response** (200 OK):
```json
{
  "gas_limit": "21000",
  "max_fee_per_gas": "25000000000",
  "max_priority_fee_per_gas": "1000000000",
  "estimated_cost_wei": "525000000000000",
  "estimated_cost": "0.000525"
}
```

### POST /v1/wallets/{wallet_id}/send
Sign and broadcast a transaction from the wallet.

**Auth**: Required (owner only)  
**Request Body**:
```json
{
  "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f8fE00",
  "amount": "1.5",
  "token": "native",
  "network": "fuji",
  "gas_limit": "21000",
  "max_priority_fee_per_gas": "1000000000"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `to` | string | Recipient address (required) |
| `amount` | string | Amount to send (required) |
| `token` | string | `"native"` for AVAX or USDC contract address |
| `network` | string | Network selection (default: fuji) |
| `gas_limit` | string | Optional gas limit override |
| `max_priority_fee_per_gas` | string | Optional priority fee override |

**Response** (200 OK):
```json
{
  "tx_hash": "0xabc123...def456",
  "status": "pending",
  "explorer_url": "https://testnet.snowtrace.io/tx/0xabc123...def456"
}
```

**Errors**:
- 400: Invalid address or amount
- 403: Not the wallet owner
- 422: Insufficient balance
- 503: Blockchain network unavailable

### GET /v1/wallets/{wallet_id}/transactions
List transaction history for a wallet.

Shows both **sent** and **received** transactions. Incoming transactions are found by scanning the user's other wallets for transfers to this wallet's address.

**Auth**: Required (owner only)  
**Query Parameters**:
- `network`: `fuji` or `mainnet` (default: fuji)
- `limit`: Max results (default: 50)

**Response** (200 OK):
```json
{
  "transactions": [
    {
      "tx_hash": "0xabc123...def456",
      "status": "confirmed",
      "direction": "sent",
      "from": "0x1234...abcd",
      "to": "0x5678...efgh",
      "amount": "1.5",
      "token": "native",
      "network": "fuji",
      "block_number": 12345678,
      "explorer_url": "https://testnet.snowtrace.io/tx/0xabc123...",
      "timestamp": "2026-02-01T10:30:00Z"
    },
    {
      "tx_hash": "0xdef789...abc012",
      "status": "confirmed",
      "direction": "received",
      "from": "0x9999...0000",
      "to": "0x1234...abcd",
      "amount": "2.0",
      "token": "native",
      "network": "fuji",
      "block_number": 12345600,
      "explorer_url": "https://testnet.snowtrace.io/tx/0xdef789...",
      "timestamp": "2026-02-01T09:00:00Z"
    }
  ]
}
```

| Field | Description |
|-------|-------------|
| `direction` | `"sent"` if this wallet initiated, `"received"` if this wallet was recipient |
| `token` | `"native"` for AVAX or ERC-20 contract address |
| `explorer_url` | Link to Snowtrace block explorer |

### GET /v1/wallets/{wallet_id}/transactions/{tx_hash}
Get the status of a specific transaction. Used for polling after send.

**Auth**: Required (owner only)  
**Response** (200 OK):
```json
{
  "tx_hash": "0xabc123...def456",
  "status": "confirmed",
  "block_number": 12345678,
  "confirmations": 5,
  "gas_used": "21000",
  "timestamp": "2026-02-01T10:30:00Z"
}
```

| Status | Description |
|--------|-------------|
| `pending` | Transaction submitted, awaiting confirmation |
| `confirmed` | Transaction included in block |
| `failed` | Transaction reverted or failed |

---

### GET /v1/wallets/{wallet_id}/balance/native
Get only the native AVAX balance (faster query).

**Auth**: Required (owner only)  
**Query Parameters**:
- `network`: `fuji` (default) or `mainnet`

**Response** (200 OK):
```json
{
  "wallet_id": "wallet_abc123",
  "address": "0x1234...abcd",
  "network": "fuji",
  "balance_wei": "1000000000000000000",
  "balance": "1.0"
}
```

---

## Bookmark Endpoints

### POST /v1/bookmarks
Create a bookmark for a wallet address.

**Auth**: Required  
**Request Body**:
```json
{
  "wallet_id": "wallet_abc123",
  "name": "My Savings",
  "address": "0x5678...efgh"
}
```

### GET /v1/bookmarks
List bookmarks for the authenticated user.

**Auth**: Required

### DELETE /v1/bookmarks/{bookmark_id}
Delete a bookmark.

**Auth**: Required (owner only)

---

## Invite Endpoints

### GET /v1/invite?code={code}
Check if an invite code is valid.

### POST /v1/invite/redeem
Redeem an invite code.

**Request Body**:
```json
{
  "code": "INVITE123"
}
```

---

## Recurring Payment Endpoints

### POST /v1/recurring/payments
Create a recurring payment schedule.

### GET /v1/recurring/payments
List recurring payments for the authenticated user.

### PUT /v1/recurring/payment/{id}
Update a recurring payment.

### DELETE /v1/recurring/payment/{id}
Cancel a recurring payment.

### PUT /v1/recurring/payment/{id}/last-paid-date
Update the last paid date for a recurring payment.

### GET /v1/recurring/payments/today
List payments due today.

---

## Admin Endpoints

**Auth**: Admin role required

### GET /v1/admin/stats
Get system statistics (wallet counts, etc.).

**Response** (200 OK):
```json
{
  "total_wallets": 150,
  "active_wallets": 142,
  "suspended_wallets": 5,
  "deleted_wallets": 3,
  "total_bookmarks": 500,
  "total_invites": 50,
  "redeemed_invites": 35,
  "total_recurring_payments": 200,
  "active_recurring_payments": 180,
  "storage_health": "ok",
  "uptime_seconds": 86400
}
```

### GET /v1/admin/wallets
List all wallets across all users.

### GET /v1/admin/users
List all users with resource counts.

### GET /v1/admin/audit/events
Query audit logs with filters.

**Query Parameters**:
- `user_id`: Filter by user
- `resource_id`: Filter by resource
- `limit`: Max results (default: 100)

### GET /v1/admin/health
Get detailed health information including storage metrics.

### POST /v1/admin/wallets/{wallet_id}/suspend
Suspend a wallet (blocks all operations).

### POST /v1/admin/wallets/{wallet_id}/activate
Reactivate a suspended wallet.

---

## Error Responses

All errors follow this format:

```json
{
  "error": "Error message",
  "status": 400
}
```

| Status | Description |
|--------|-------------|
| 400 | Bad Request - Invalid input |
| 401 | Unauthorized - Missing or invalid token |
| 403 | Forbidden - Insufficient permissions or not resource owner |
| 404 | Not Found - Resource doesn't exist |
| 422 | Unprocessable Entity - Validation failed |
| 503 | Service Unavailable - Dependency unavailable |

---

## Future Endpoints (Planned)

| Endpoint | Description | Status |
|----------|-------------|--------|
| `GET /attestation` | Get DCAP attestation evidence | Planned |

## Recently Implemented

| Endpoint | Description | Version |
|----------|-------------|--------|
| `GET /v1/wallets/{id}/balance` | Query on-chain balance (AVAX + ERC-20) | v0.1.0 |
| `GET /v1/wallets/{id}/balance/native` | Query native AVAX balance only | v0.1.0 |
| `POST /v1/wallets/{id}/estimate` | Estimate gas for transaction | v0.2.0 |
| `POST /v1/wallets/{id}/send` | Sign and broadcast transaction | v0.2.0 |
| `GET /v1/wallets/{id}/transactions` | List transaction history | v0.2.0 |
| `GET /v1/wallets/{id}/transactions/{tx_hash}` | Get transaction status | v0.2.0 |