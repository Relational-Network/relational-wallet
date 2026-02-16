---
layout: default
title: System Overview
parent: Architecture
nav_order: 1
---

# System Overview

Relational Wallet is a SGX-backed custodial wallet system with a Next.js frontend and a Rust enclave backend.

## Components

## 1. Rust Server (`apps/rust-server`)

- Axum API with `/v1/*` wallet, transaction, bookmark, invite, recurring, fiat, and admin routes
- Runs in Gramine SGX with RA-TLS certificates
- Encrypted persistent storage under `/data`
- Clerk JWT verification (JWKS in production, dev-mode decode when `dev` feature is used)
- Fuji-focused chain integration (AVAX + ERC-20 including `rEUR`)

## 2. Wallet Web (`apps/wallet-web`)

- Next.js 16 App Router + Clerk auth UI
- Proxy route `src/app/api/proxy/[...path]/route.ts`
- Route surfaces include `/wallets`, `/wallets/[wallet_id]/fiat`, `/wallets/bootstrap`, `/pay`, `/callback`, `/account`
- Proxy adds bearer token and forwards to enclave backend

## 3. External Integrations

- Avalanche Fuji RPC for balances/transfers
- Deployed `rEUR` contract for reserve settlement
- TrueLayer sandbox for on-ramp/off-ramp provider actions and payout flow

## 4. Contracts (`apps/contracts`)

- Foundry workspace for `RelationalEuro (rEUR)`
- Role-managed mint/burn/pause model
- Deployment + tests tracked in contract workspace

## Data Flow (Example: Fiat On-Ramp)

```text
1. User creates on-ramp request in wallet-web
2. Frontend calls /api/proxy/v1/fiat/onramp/requests
3. Proxy adds Clerk JWT and forwards to enclave
4. Enclave validates user + wallet ownership
5. Enclave initializes provider request (TrueLayer sandbox)
6. Status progresses via webhook/polling and settlement sync
7. Reserve wallet transfer updates request with settlement tx hash
```

## Data Flow (Example: Standard Transfer)

```text
1. User opens /wallets and selects send flow
2. Frontend requests gas estimate via proxy
3. Enclave signs tx with wallet private key in SGX
4. Tx broadcasts to Fuji and tx hash is persisted
5. Frontend polls tx status and updates activity timeline
```

## Storage Domains

Under `/data` (encrypted mount), main domains include:

- `wallets/`
- `bookmarks/`
- `invites/`
- `recurring/`
- `fiat/`
- `system/fiat_service_wallet/`
- `audit/`
