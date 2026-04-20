---
layout: default
title: Home
nav_order: 1
description: Non-custodial Avalanche wallet secured by Intel SGX enclaves.
permalink: /
---

# Relational Wallet
{: .fs-9 }

A non-custodial Avalanche wallet where private keys live exclusively inside Intel SGX enclaves. Hardware-enforced security meets seamless Web3 UX.
{: .fs-6 .fw-300 }

[Get Started](/relational-wallet/installation){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[API Reference](/relational-wallet/api){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Why Relational Wallet?

Traditional wallet services force a choice: convenience **or** security. Relational Wallet eliminates this trade-off by executing all cryptographic operations inside a Trusted Execution Environment (TEE). Your private keys are generated, stored, and used exclusively within Intel SGX enclaves --- never exposed to the host operating system, server administrators, or network observers.

| Capability | How it works |
|:-----------|:-------------|
| **Hardware-isolated keys** | secp256k1 keys generated and sealed inside Intel SGX. Even the server operator cannot extract them. |
| **Remote attestation** | Every TLS connection includes DCAP attestation evidence, letting clients cryptographically verify the enclave is genuine. |
| **Encrypted storage** | All data at rest is sealed to the enclave identity using Gramine's encrypted filesystem. |
| **Fiat on/off-ramp** | TrueLayer integration enables EUR deposits and withdrawals with automatic rEUR minting and settlement. |
| **Role-based access** | Clerk JWT authentication with four roles (Admin, Client, Support, Auditor) and strict ownership enforcement. |
| **Open source** | AGPL-3.0 licensed. Audit the code, verify the enclave measurements, trust the math. |

---

## Architecture at a Glance

```
                                 ┌─────────────────────────────────────────┐
                                 │          Intel SGX Enclave              │
  ┌──────────┐   Clerk JWT       │  ┌─────────────────────────────────┐   │
  │  Browser  │ ───────────────► │  │  Axum REST API (Rust)           │   │
  │  Next.js  │ ◄─── RA-TLS ──  │  │  • Wallet CRUD                  │   │
  │  React 19 │                  │  │  • Transaction signing           │   │
  └──────────┘                   │  │  • Balance queries               │   │
       │                         │  │  • Fiat on/off-ramp              │   │
       │                         │  │  • Admin & audit logging         │   │
       ▼                         │  └──────────┬──────────────────────┘   │
  ┌──────────┐                   │             │                          │
  │  Clerk   │                   │  ┌──────────▼──────────────────────┐   │
  │  Auth    │                   │  │  Encrypted Storage (/data)      │   │
  └──────────┘                   │  │  Sealed to enclave identity     │   │
                                 │  └─────────────────────────────────┘   │
                                 └────────────────┬────────────────────────┘
                                                  │
                              ┌───────────────────┼───────────────────┐
                              │                   │                   │
                       ┌──────▼──────┐    ┌───────▼──────┐   ┌───────▼───────┐
                       │ Avalanche   │    │  TrueLayer   │   │  rEUR Smart   │
                       │ C-Chain RPC │    │  Fiat API    │   │  Contract     │
                       └─────────────┘    └──────────────┘   └───────────────┘
```

---

## Monorepo Overview

| Component | Path | Stack | Purpose |
|:----------|:-----|:------|:--------|
| **Rust Server** | `apps/rust-server/` | Axum, Tokio, Gramine SGX | Enclave backend --- wallet ops, signing, storage |
| **Wallet Web** | `apps/wallet-web/` | Next.js 16, React 19, Clerk | Browser UI --- dashboards, send/receive, fiat flows |
| **Smart Contracts** | `apps/contracts/` | Solidity 0.8.24, Foundry | rEUR ERC-20 token (mint, burn, pause) |
| **Reverse Proxy** | `apps/proxy/` | Nginx, Let's Encrypt | TLS termination for external webhook integrations |
| **Documentation** | `docs/gh-pages/` | Jekyll, just-the-docs | This site |

---

## Network & Contract Info

| Item | Value |
|:-----|:------|
| **Network** | Avalanche C-Chain (Fuji testnet) |
| **Chain ID** | `43113` |
| **rEUR Contract** | [`0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`](https://testnet.snowtrace.io/address/0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63) |
| **Token Standard** | ERC-20 + AccessControl + Pausable + Burnable |
| **Decimals** | 6 |
| **Deployment Tx** | [`0x8987...289f14`](https://testnet.snowtrace.io/tx/0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14) |
| **License** | [AGPL-3.0-or-later](https://github.com/Relational-Network/relational-wallet/blob/main/LICENSE) |

---

## Quick Start

```bash
# Clone the repository
git clone https://github.com/Relational-Network/relational-wallet.git
cd relational-wallet

# Backend (requires Intel SGX hardware)
cd apps/rust-server
cp .env.example .env          # Configure environment
make dev-check                # Verify compilation
make dev-test                 # Run 155+ unit tests
make                          # Build SGX artifacts
make start-rust-server        # Launch inside enclave

# Frontend (separate terminal)
cd apps/wallet-web
cp .env.example .env.local    # Configure Clerk keys
pnpm install
pnpm dev                      # http://localhost:3000

# Smart contracts
cd apps/contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge test -vv                # Run Foundry tests
```

See the [Installation Guide](/relational-wallet/installation) for detailed setup instructions.

---

## Security Model

Relational Wallet's security is built on three pillars:

**1. Hardware Isolation (Intel SGX)**
: All cryptographic operations --- key generation, transaction signing, data encryption --- execute inside an SGX enclave. The host OS, hypervisor, and server operator are excluded from the trusted computing base.

**2. Remote Attestation (DCAP RA-TLS)**
: The enclave generates TLS certificates that embed DCAP attestation evidence. Clients verify the enclave's identity (MRENCLAVE measurement) before establishing a connection, ensuring they communicate with genuine, unmodified code.

**3. Sealed Storage (Gramine Encrypted FS)**
: All persistent data --- private keys, wallet metadata, transaction history --- is encrypted with keys derived from the enclave's identity. Data is unreadable outside the enclave boundary, even with physical disk access.

[Read the full Security Model](/relational-wallet/security){: .btn .btn-outline .fs-4 }

---

## Documentation Sections

### [Installation](/relational-wallet/installation)
Prerequisites, backend setup, frontend configuration, and Docker deployment.

### [Architecture](/relational-wallet/architecture)
System design, component responsibilities, data flows, and TEE attestation deep-dive.

### [API Reference](/relational-wallet/api)
Complete REST API documentation with authentication, request/response examples, and error codes.

### [Smart Contracts](/relational-wallet/contracts)
rEUR token specification, Foundry testing, and Fuji deployment runbook.

### [Security](/relational-wallet/security)
Threat model, key management, audit logging, and the SGX trust boundary.

### [User Guides](/relational-wallet/guides)
Step-by-step guides for wallet management, sending transactions, and fiat on/off-ramp flows.

### [Operations](/relational-wallet/operations)
Production deployment, health monitoring, JWT testing, and operational runbooks.

### [Legal](/relational-wallet/legal)
Privacy policy and terms of service.
