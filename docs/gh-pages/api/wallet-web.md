---
layout: default
title: Frontend Proxy
parent: API Reference
nav_order: 8
---

# Frontend Proxy (Wallet Web)

The Next.js frontend exposes a catch-all server-side proxy at `/api/proxy/*` that forwards requests to the SGX backend, injecting Clerk JWTs server-side.

---

## Proxy Route

```
/api/proxy/[...path]
```

Every path under `/api/proxy/` is forwarded to the enclave backend at `WALLET_API_BASE_URL`.

**Example:** `/api/proxy/v1/wallets` → `https://localhost:8080/v1/wallets`

---

## Request Flow

```
Browser fetch('/api/proxy/v1/wallets')
  │
  ▼
Next.js server-side route handler
  1. Strip /api/proxy prefix
  2. Fetch Clerk JWT from active session
  3. Add Authorization: Bearer <jwt> header
  4. Forward to WALLET_API_BASE_URL/v1/wallets
  │
  ▼
SGX backend (RA-TLS, self-signed cert)
  │
  ▼
Response forwarded back to browser
```

---

## Environment Variables

| Variable | Description |
|:---------|:------------|
| `WALLET_API_BASE_URL` | Backend URL (server-only, never exposed to browser) |
| `NODE_TLS_REJECT_UNAUTHORIZED=0` | Accept self-signed RA-TLS certs (dev only) |

---

## Example Browser Calls

```typescript
// From frontend code via the typed API client
const wallets = await fetch('/api/proxy/v1/wallets').then(r => r.json());

// Send transaction
const result = await fetch(`/api/proxy/v1/wallets/${walletId}/send`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ amount: '0.1', to: '0x...', network: 'fuji', token: 'AVAX' })
}).then(r => r.json());
```

---

## Production Notes

- `NODE_TLS_REJECT_UNAUTHORIZED=0` is rejected when `NODE_ENV=production`
- Use the [Nginx reverse proxy](/relational-wallet/architecture/system-overview#reverse-proxy-appsproxy) with a valid Let's Encrypt certificate in production
- `WALLET_API_BASE_URL` must never be a public URL in production — keep it internal

---

For complete API documentation including request/response schemas, see the [API Reference](/relational-wallet/api) section.
