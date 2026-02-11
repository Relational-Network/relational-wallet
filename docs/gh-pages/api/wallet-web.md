---
layout: default
title: Wallet Web
parent: API Documentation
nav_order: 2
---

# Wallet Web API

The wallet-web frontend provides a server-side API proxy for communicating with the SGX enclave backend.

## Why a Proxy?

Browsers enforce certificate validation and reject self-signed certificates. The SGX enclave uses DCAP RA-TLS which generates self-signed certificates with embedded attestation evidence.

The solution:

```
┌─────────────┐    HTTP    ┌─────────────┐   HTTPS    ┌─────────────┐
│   Browser   │ ─────────▶ │  Next.js    │ ─────────▶ │ SGX Enclave │
│             │            │  /api/proxy │            │             │
└─────────────┘            └─────────────┘            └─────────────┘
                                 │
                                 │ NODE_TLS_REJECT_UNAUTHORIZED=0
                                 │ (development only)
```

## Proxy Endpoints

### ALL /api/proxy/*

The catch-all proxy route forwards all requests to the backend.

**Supported Methods**: GET, POST, PUT, PATCH, DELETE

**Request Path Mapping**:
- `/api/proxy/v1/wallets` → `https://localhost:8080/v1/wallets`
- `/api/proxy/health` → `https://localhost:8080/health`

**Headers Added by Proxy**:
- `Authorization: Bearer <jwt>` — Clerk session token added automatically
- `Content-Type` — Preserved from original request

### Example: Create Wallet

```bash
# Browser makes request to proxy
fetch('/api/proxy/v1/wallets', {
  method: 'POST',
  credentials: 'include'  # Include Clerk session cookie
});

# Proxy forwards to backend with JWT
POST https://localhost:8080/v1/wallets
Authorization: Bearer eyJhbG...
```

### Example: List Wallets

```bash
# Browser request
fetch('/api/proxy/v1/wallets', { credentials: 'include' });

# Proxy response
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

## TypeScript API Client

The frontend provides a typed API client that automatically uses the proxy for browser-side requests:

```typescript
import { apiClient } from "@/lib/api";

// In browser - uses /api/proxy
const wallets = await apiClient.listWallets();

// In server component - can use direct URL
const wallets = await apiClient.listWallets(token);
```

## Balance Display

The wallet detail page displays real-time AVAX and ERC-20 token balances from the Avalanche blockchain:

### Example: Get Full Wallet Balance (Native + Tokens)

```bash
# Browser request (via proxy)
fetch('/api/proxy/v1/wallets/wallet_abc123/balance?network=fuji', {
  credentials: 'include'
});

# Response
{
  "wallet_id": "wallet_abc123",
  "address": "0x1234...abcd",
  "network": "Avalanche Fuji Testnet",
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
      "balance_raw": "10000000",
      "balance_formatted": "10.0",
      "decimals": 6,
      "contract_address": "0x5425890298aed601595a70AB815c96711a31Bc65"
    }
  ]
}
```

### Example: Get Native Balance Only

```bash
# Browser request (via proxy)
fetch('/api/proxy/v1/wallets/wallet_abc123/balance/native?network=fuji', {
  credentials: 'include'
});

# Response
{
  "wallet_id": "wallet_abc123",
  "address": "0x1234...abcd",
  "network": "Avalanche Fuji Testnet",
  "chain_id": 43113,
  "balance_wei": "1000000000000000000",
  "balance_avax": "1.0"
}
```

### Balance Features

| Feature | Description |
|---------|-------------|
| Native AVAX | Displays balance from Fuji testnet |
| ERC-20 Tokens | Displays USDC balance with contract address |
| Refresh | Manual refresh button for latest balance |
| Network indicator | Shows "Fuji Testnet" badge |
| Faucet links | Quick links to get test AVAX and USDC |
| Error handling | Graceful degradation on RPC failures |

### Faucet Links

For testing on Fuji testnet:
- **AVAX**: [Avalanche Faucet](https://core.app/tools/testnet-faucet/)
- **USDC**: [Circle Faucet](https://faucet.circle.com/)

## Error Handling

The proxy preserves error responses from the backend:

| Backend Status | Proxy Response |
|----------------|----------------|
| 200-299 | Pass through |
| 401 | 401 (Clerk may redirect to sign-in) |
| 403 | 403 (Permission denied) |
| 404 | 404 (Not found) |
| 503 | 503 (Blockchain unavailable) |
### Environment Variables

```env
# .env.local
WALLET_API_BASE_URL=https://localhost:8080
NODE_TLS_REJECT_UNAUTHORIZED=0  # Accept self-signed certs in dev
```

### Production Configuration

For production, use properly signed certificates and remove the self-signed cert workaround:

```env
WALLET_API_BASE_URL=https://enclave.example.com
# Remove NODE_TLS_REJECT_UNAUTHORIZED
```
