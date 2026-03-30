---
layout: default
title: Architecture
nav_order: 4
has_children: true
permalink: /architecture/
---

# Architecture
{: .fs-8 }

A multi-layer system combining a hardware-isolated backend, authenticated frontend proxy, and on-chain smart contracts.
{: .fs-5 .fw-300 }

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Client Layer                                   │
│                                                                         │
│  ┌──────────────┐     ┌──────────────────────────────────────────────┐  │
│  │   Browser     │────►│  Next.js 16 (Wallet Web)                    │  │
│  │   React 19    │     │  • Clerk authentication (sign-in/up)        │  │
│  │               │◄────│  • Server-side proxy (/api/proxy/*)         │  │
│  └──────────────┘     │  • JWT injection on backend calls            │  │
│                        └───────────────────┬──────────────────────────┘  │
└────────────────────────────────────────────┼─────────────────────────────┘
                                             │ HTTPS + Bearer JWT
                                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Intel SGX Enclave (Trust Boundary)                    │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  Axum REST API (Rust 1.92)                                       │  │
│  │                                                                   │  │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────────┐  │  │
│  │  │  Wallets    │ │  Transfers │ │  Fiat      │ │  Admin       │  │  │
│  │  │  CRUD       │ │  Sign/Send │ │  On/Off    │ │  Stats/Audit │  │  │
│  │  └────────────┘ └────────────┘ └────────────┘ └──────────────┘  │  │
│  │                                                                   │  │
│  │  ┌────────────────────┐  ┌─────────────────────────────────────┐ │  │
│  │  │  Auth Middleware    │  │  Encrypted Storage (/data)          │ │  │
│  │  │  • JWKS validation  │  │  • Wallet keys (PEM, sealed)       │ │  │
│  │  │  • Role extraction  │  │  • Metadata, bookmarks, invites    │ │  │
│  │  │  • Ownership check  │  │  • Audit logs (JSONL)              │ │  │
│  │  └────────────────────┘  └─────────────────────────────────────┘ │  │
│  │                                                                   │  │
│  │  RA-TLS Certificates (DCAP attestation evidence embedded)        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└────────────────┬──────────────────┬──────────────────┬──────────────────┘
                 │                  │                  │
          ┌──────▼──────┐   ┌──────▼──────┐   ┌───────▼───────┐
          │ Avalanche   │   │ TrueLayer   │   │ Clerk         │
          │ C-Chain     │   │ Fiat API    │   │ JWKS/Auth     │
          │ (Fuji RPC)  │   │ (Sandbox)   │   │               │
          └──────┬──────┘   └─────────────┘   └───────────────┘
                 │
          ┌──────▼──────┐
          │ rEUR Smart  │
          │ Contract    │
          │ (ERC-20)    │
          └─────────────┘
```

---

## Core Design Principles

| Principle | Implementation |
|:----------|:---------------|
| **Hardware-first security** | All key material lives inside the SGX enclave. The host OS is untrusted. |
| **Verify, don't trust** | DCAP remote attestation lets clients verify the enclave identity before connecting. |
| **Defense in depth** | Encrypted storage + JWT auth + ownership checks + role-based access + audit logging. |
| **Minimal attack surface** | No database server, no key export APIs, no plaintext storage. Embedded `redb` for indexed data. |
| **Deterministic builds** | Docker builds use pinned timestamps, snapshots, and single codegen units for reproducible MRENCLAVE. |

---

## Component Responsibilities

| Component | Responsibilities |
|:----------|:----------------|
| **Wallet Web** | User authentication, UI rendering, server-side API proxy with JWT injection |
| **Rust Server** | Wallet lifecycle, key generation, transaction signing, balance queries, fiat flows, admin ops |
| **Gramine** | SGX enclave loading, encrypted FS mount, RA-TLS certificate generation |
| **Clerk** | User identity, JWT issuance, JWKS endpoint, role metadata |
| **Avalanche RPC** | Balance queries, gas estimation, transaction broadcasting, block confirmations |
| **TrueLayer** | Fiat payment initiation, hosted payment pages, webhook notifications, payout execution |
| **rEUR Contract** | ERC-20 token minting/burning for fiat settlement, pausable transfers, role-based access |
| **Nginx Proxy** | TLS termination with Let's Encrypt for external webhook callbacks |

---

## Data Flow Examples

### Standard Transfer

```
User clicks "Send" in wallet UI
  │
  ▼
Browser → POST /api/proxy/v1/wallets/{id}/estimate
  │         (gas estimation)
  ▼
Browser → POST /api/proxy/v1/wallets/{id}/send
  │         { amount, to, network, token }
  ▼
Next.js proxy injects Clerk JWT → forwards to enclave
  │
  ▼
Enclave validates JWT + wallet ownership
  │
  ▼
Enclave loads private key from /data/wallets/{id}/key.pem
  │
  ▼
Enclave signs EIP-1559 transaction inside SGX
  │
  ▼
Broadcasts to Avalanche C-Chain via RPC
  │
  ▼
Returns tx_hash + explorer_url to frontend
  │
  ▼
Frontend polls GET /v1/wallets/{id}/transactions/{tx_hash}
  until status = "confirmed" or "failed"
```

### Fiat On-Ramp

```
User initiates on-ramp request
  │
  ▼
POST /v1/fiat/onramp/requests
  { wallet_id, amount_eur }
  │
  ▼
Enclave creates TrueLayer payment request
  │
  ▼
Returns provider_action_url (hosted payment page)
  │
  ▼
User completes bank payment on TrueLayer
  │
  ▼
TrueLayer sends webhook → Nginx proxy → enclave
  │
  ▼
Enclave updates request status
  │
  ▼
Background poller detects "settlement_pending"
  │
  ▼
Reserve wallet mints rEUR to user's wallet address
  │
  ▼
Request marked "completed" with deposit_tx_hash
```

---

## Storage Architecture

All persistent data is stored in Gramine's encrypted filesystem at `/data`:

```
/data/
├── wallets/{wallet_id}/
│   ├── meta.json          # WalletMetadata (user_id, address, label, status, created_at)
│   └── key.pem            # SEALED secp256k1 private key
├── bookmarks/{id}.json    # Address book entries (per wallet)
├── invites/{id}.json      # Invite codes with expiration + redemption
├── recurring/{id}.json    # Recurring payment configurations
├── fiat/{id}.json         # Fiat request lifecycle records
├── transactions/{wallet_id}_{tx_hash}.json
│                          # Transaction history
├── email_index/{hash}.json
│                          # Email → UserId mapping for payment links
├── system/
│   └── fiat_service_wallet/
│       ├── meta.json      # Reserve wallet metadata
│       └── key.pem        # Reserve wallet sealed key
└── audit/{date}/
    └── events.jsonl       # Append-only audit log
```

**Encryption modes:**

| Mode | Key derivation | Use case |
|:-----|:---------------|:---------|
| Development | Persistent 16-byte key at `data/.dev_storage_key` | Local testing (survives rebuilds) |
| Production | Derived from enclave signer identity (`_sgx_mrsigner`) | Sealed to enclave, unreadable outside SGX |

---

## Sub-pages

- [**System Overview**](/relational-wallet/architecture/system-overview) --- Detailed component breakdown and technology choices
- [**Security Model**](/relational-wallet/architecture/security-model) --- Authentication, authorization, encryption, and the SGX trust boundary
- [**TEE & Attestation**](/relational-wallet/architecture/tee-attestation) --- Deep-dive into Intel SGX, Gramine, DCAP RA-TLS, and enclave measurements
- [**Diagram Workflow**](/relational-wallet/architecture/diagram-workflow) --- PlantUML authoring and rendering process
