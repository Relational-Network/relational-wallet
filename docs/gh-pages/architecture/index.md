---
layout: default
title: Architecture
nav_order: 4
has_children: true
permalink: /architecture/
---

# Architecture

Relational Wallet architecture combines a browser frontend, a Next.js proxy tier, and an SGX enclave backend.

## High-Level Flow

```text
Browser (Clerk session)
  -> Next.js app (/api/proxy/*)
  -> SGX backend (Axum + RA-TLS)
  -> Avalanche Fuji + TrueLayer sandbox
```

## Core Concepts

| Concept | Description |
|---------|-------------|
| SGX enclave runtime | Private key operations and sensitive state stay inside enclave memory/storage |
| RA-TLS | TLS channel from proxy to enclave with attestation-aware cert generation |
| Clerk auth | JWT validation in backend, role-based admin gating |
| Proxy boundary | Browser talks to Next.js proxy; proxy attaches JWT to backend requests |
| Fuji reserve settlement | `rEUR` reserve wallet supports on-ramp/off-ramp settlement flows |

## Current Product Surfaces

- `/wallets` - primary dashboard and wallet actions
- `/wallets/[wallet_id]/fiat` - fiat request workflow
- `/wallets/bootstrap` - operator/admin reserve and sync tools
- `/pay` - neutral pull-transfer payer entry
- `/callback` - provider authorization return landing

## Sub-pages

- **System Overview** - component responsibilities and data flow
- **Security Model** - auth, storage, and SGX boundary details
- **Diagram Workflow** - PlantUML authoring/rendering process
