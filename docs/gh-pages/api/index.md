---
layout: default
title: API Documentation
nav_order: 3
has_children: true
permalink: /api/
---

# API Documentation

This section documents the REST APIs for interacting with the Relational Wallet.

## Endpoints Overview

| API | Base URL | Auth | Description |
|-----|----------|------|-------------|
| **Wallet Enclave** | `https://localhost:8080` | Clerk JWT | SGX backend with DCAP RA-TLS |
| **Wallet Web Proxy** | `http://localhost:3000/api/proxy` | Clerk Session | Proxies to enclave |
| **rEUR Contract (Fuji)** | `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63` | On-chain roles | Managed ERC-20 stablecoin |

## Sub-pages

- **Wallet Enclave** — Full REST API reference for the SGX backend
- **Wallet Web** — Frontend API integration patterns

## Quick Reference

### Health Check
```bash
curl -k https://localhost:8080/health
```

### Create Wallet (via proxy)
```bash
curl -X POST http://localhost:3000/api/proxy/v1/wallets \
  -H "Cookie: __session=..." 
```

### List Wallets (via proxy)
```bash
curl http://localhost:3000/api/proxy/v1/wallets \
  -H "Cookie: __session=..."
```

### Query rEUR Symbol (Fuji)
```bash
cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 "symbol()(string)" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
```

## OpenAPI Documentation

- **Swagger UI**: [https://localhost:8080/docs](https://localhost:8080/docs)
- **OpenAPI JSON**: [https://localhost:8080/api-doc/openapi.json](https://localhost:8080/api-doc/openapi.json)

Note: You'll need to accept the self-signed RA-TLS certificate in your browser to access Swagger UI.
