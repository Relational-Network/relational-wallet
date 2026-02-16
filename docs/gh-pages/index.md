---
layout: default
title: Home
nav_order: 1
description: Overview of the Relational Wallet documentation space.
---

# Relational Wallet Documentation

Relational Wallet is a custodial Avalanche wallet platform with SGX-backed key custody, Clerk auth, and Fuji `rEUR` settlement flows.

## Project Overview

| Area | Notes |
|------|-------|
| apps/rust-server  | SGX Axum API with wallet, transfer, admin, fiat reserve flows |
| apps/wallet-web  | Next.js 16 app shell with `/wallets`, `/pay`, `/callback`, `/wallets/bootstrap` |
| apps/contracts  | Foundry workspace with deployed Fuji `rEUR` |
| docs/sequence | Sequence diagrams |

## Key Functional Coverage

- Wallet creation/list/delete and ownership enforcement
- Push transfers, pull transfer entry (`/pay`), and mirrored transaction history
- Cash exchange UX guidance across send/receive/history
- Fiat on-ramp and off-ramp request lifecycle (`truelayer_sandbox`)
- Admin reserve wallet bootstrap/topup/transfer/manual sync (`/wallets/bootstrap` UI + `/v1/admin/fiat/*` APIs)

## Quick Start

```bash
# Backend
cd apps/rust-server
cp .env.example .env
make
make start-rust-server

# Frontend
cd apps/wallet-web
cp .env.example .env
pnpm install
pnpm dev
```

Open `http://localhost:3000`.

## Documentation Sections

- **Installation** - Rust server + wallet-web setup
- **API Documentation** - Enclave and proxy route contracts
- **Contracts** - `rEUR` contract overview, testing, and Fuji deployment runbook
- **Architecture** - Component boundaries and security model
- **Operations** - JWT testing and docs publishing
- **Legal** - Policy placeholders

## Contract Snapshot

- Network: Avalanche Fuji (`43113`)
- `rEUR`: `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`
- Deployment Tx: `0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14`
