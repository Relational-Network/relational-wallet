---
layout: default
title: API Documentation
nav_order: 3
has_children: true
permalink: /api/
---

# API Documentation

## Endpoints Overview

| API | Base URL | Auth | Notes |
|-----|----------|------|-------|
| Rust Server | `https://localhost:8080` | Clerk JWT bearer | SGX backend (RA-TLS) |
| Wallet Web Proxy | `http://localhost:3000/api/proxy` | Clerk session | For browser-safe forwarding |
| rEUR Contract (Fuji) | `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63` | On-chain roles | Reserve settlement token |

## Quick Reference

### Health

```bash
curl -k https://localhost:8080/health
```

### List Wallets via Proxy

```bash
curl http://localhost:3000/api/proxy/v1/wallets \
  -H "Cookie: __session=..."
```

### List Fiat Providers via Proxy

```bash
curl http://localhost:3000/api/proxy/v1/fiat/providers \
  -H "Cookie: __session=..."
```

### Admin Reserve Wallet Status (direct API)

```bash
curl -k https://localhost:8080/v1/admin/fiat/service-wallet \
  -H "Authorization: Bearer $JWT"
```

### Query `rEUR` Symbol

```bash
cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 "symbol()(string)" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
```

## OpenAPI

- Swagger UI: `https://localhost:8080/docs`
- OpenAPI JSON: `https://localhost:8080/api-doc/openapi.json`

## Sub-pages

- **Rust Server** - Full backend route reference
- **Wallet Web** - Proxy behavior and frontend API integration
