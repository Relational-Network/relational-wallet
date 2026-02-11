---
layout: default
title: Architecture
nav_order: 4
has_children: true
permalink: /architecture/
---

# Architecture

This section covers the system design, security model, and operational aspects of the Relational Wallet.

## High-Level Architecture

```
┌───────────────────────────────────────────────────────────────────────┐
│                           User's Browser                               │
│   ┌─────────────────────────────────────────────────────────────────┐ │
│   │  Clerk Auth    │    React UI    │    API Calls → /api/proxy     │ │
│   └─────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP (localhost:3000)
                                    ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         Next.js Server                                 │
│   ┌─────────────────────────────────────────────────────────────────┐ │
│   │  Clerk Session  │  API Proxy (adds JWT)  │  SSR Components       │ │
│   └─────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTPS with RA-TLS (localhost:8080)
                                    ▼
┌───────────────────────────────────────────────────────────────────────┐
│                      SGX Enclave (Gramine)                             │
│   ┌─────────────────────────────────────────────────────────────────┐ │
│   │  DCAP RA-TLS  │  JWT Verification  │  Key Management  │  Storage │ │
│   └─────────────────────────────────────────────────────────────────┘ │
│   ┌─────────────────────────────────────────────────────────────────┐ │
│   │                    Encrypted /data filesystem                    │ │
│   └─────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

## Key Concepts

| Concept | Description |
|---------|-------------|
| **SGX Enclave** | Trusted execution environment protecting keys and sensitive data |
| **DCAP RA-TLS** | Remote attestation with TLS certificates containing attestation evidence |
| **Clerk Auth** | Third-party authentication with JWT tokens verified via JWKS |
| **API Proxy** | Server-side route that bridges browser and enclave |
| **Encrypted FS** | Gramine-managed encrypted filesystem sealed to enclave identity |
| **Contracts Workspace** | Foundry-based `apps/contracts` for rEUR deployment and tests |

## Sub-pages

- **System Overview** — Components, data flow, and directory structure
- **Security Model** — TEE protection, authentication, storage encryption
- **Diagram Workflow** — How to update PlantUML diagrams

## Diagram Sources

- Structural diagrams: `docs/architecture/`
- Sequence diagrams: `docs/sequence/`
- Generated assets: `docs/seq-diagrams/`
- Shared styles: `docs/includes/`

To regenerate diagrams:

```bash
cd docs
./render.sh
```
