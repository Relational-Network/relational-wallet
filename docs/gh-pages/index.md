---
layout: default
title: Home
nav_order: 1
description: Overview of the Relational Wallet documentation space.
---

# Relational Wallet Documentation

Welcome to the documentation hub for the Relational Wallet—a secure wallet system running inside Intel SGX enclaves with DCAP remote attestation.

## Project Status

| Component | Status | Description |
|-----------|--------|-------------|
| **rust-server** | ✅ Complete | Axum REST API running in Gramine SGX with DCAP RA-TLS |
| **wallet-web** | ✅ Complete | Next.js 16 frontend with Clerk authentication |
| **Integration** | ✅ Working | Frontend proxies API calls to enclave backend |

## Key Features

- **SGX Enclave Security**: All sensitive operations run inside Intel SGX trusted execution environment
- **DCAP Attestation**: Remote attestation with DCAP RA-TLS certificates
- **Clerk Authentication**: JWT-based auth with JWKS signature verification
- **P256 Key Management**: Cryptographic keys generated and stored securely in enclave
- **Structured Logging**: Request tracing with correlation IDs

## What's Inside

- **Installation** — Environment setup guides for the trusted execution environment [`(wallet-enclave)`](/installation/wallet-enclave) and the user-facing client [`(wallet-web)`](/installation/wallet-web).
- **API Documentation** — Component-level REST references with OpenAPI specs.
- **Architecture** — System overview, security model, and sequence diagrams.
- **Operations** — CI/CD, GitHub Pages publishing guidance, and operational runbooks.
- **Legal** — Privacy Policy and Terms of Service _placeholders_.

## Quick Start

```bash
# Start the enclave backend
cd apps/rust-server
gramine-sgx rust-server

# In another terminal, start the frontend
cd apps/wallet-web
pnpm dev

# Open http://localhost:3000
```

## Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────────────┐
│  Browser/User   │────▶│  Next.js App    │────▶│   SGX Enclave (Gramine) │
│                 │     │  /api/proxy/*   │     │   Axum REST API         │
└─────────────────┘     └─────────────────┘     └─────────────────────────┘
                              │                           │
                              │ Clerk JWT                 │ DCAP RA-TLS
                              ▼                           ▼
                        ┌─────────────────┐     ┌─────────────────────────┐
                        │   Clerk JWKS    │     │   Intel DCAP Services   │
                        │   (Auth)        │     │   (Attestation)         │
                        └─────────────────┘     └─────────────────────────┘
```

## Contributing to Docs

- Keep pages in Markdown with front matter defining `title`, `nav_order`, and parent metadata.
- Diagram source files live under `../architecture/` and `../sequence/`.
- Update them with PlantUML and regenerate assets via `../render.sh`.
