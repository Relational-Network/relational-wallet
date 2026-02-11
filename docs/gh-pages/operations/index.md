---
layout: default
title: Operations
nav_order: 6
has_children: true
permalink: /operations/
---

# Operations

Operational guides for deploying and maintaining the Relational Wallet.

## Development Workflow

### Starting the System

```bash
# Terminal 1: Backend (SGX)
cd apps/rust-server
make                    # Build for SGX
gramine-sgx rust-server # Start at https://localhost:8080

# Terminal 2: Frontend
cd apps/wallet-web
pnpm dev               # Start at http://localhost:3000
```

### Verifying Health

```bash
# Backend health check
curl -k https://localhost:8080/health

# Response:
{
  "status": "ok",
  "checks": {
    "service": "ok",
    "data_dir": "ok",
    "jwks": "ok"
  }
}
```

## Environment Configuration

### Backend (rust-server)

| Variable | Required | Description |
|----------|----------|-------------|
| `CLERK_JWKS_URL` | Yes (prod) | Clerk JWKS endpoint |
| `CLERK_ISSUER` | Yes (prod) | Clerk issuer URL |
| `CLERK_AUDIENCE` | No | Expected JWT audience |
| `DATA_DIR` | No | Data directory (default: `/data`) |
| `LOG_FORMAT` | No | `json` or `pretty` |

### Frontend (wallet-web)

| Variable | Required | Description |
|----------|----------|-------------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Yes | Clerk frontend key |
| `CLERK_SECRET_KEY` | Yes | Clerk backend secret |
| `WALLET_API_BASE_URL` | Yes | Backend URL |

## Sub-pages

- **JWT Testing** — How to obtain and use JWTs for testing API endpoints
- **Publishing** — GitHub Pages deployment workflow 
