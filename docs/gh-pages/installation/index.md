---
layout: default
title: Installation
nav_order: 2
has_children: true
permalink: /installation/
---

# Installation

Relational Wallet has two runtime components in this repo:

| Component | Path | Purpose |
|-----------|------|---------|
| Rust Server (enclave backend) | `apps/rust-server` | Axum API running in Gramine SGX with RA-TLS |
| Wallet Web (frontend) | `apps/wallet-web` | Next.js + Clerk app and API proxy |

## Prerequisites

1. Intel SGX-capable host with `/dev/sgx/enclave` and `/dev/sgx/provision`
2. Gramine + `gramine-ratls-dcap`
3. Rust toolchain (see `apps/rust-server/rust-toolchain.toml`)
4. Node.js 20+ and `pnpm`
5. Clerk app credentials

## Quick Start

### Terminal 1: Rust server in SGX

```bash
cd apps/rust-server
cp .env.example .env
# fill required values in .env
make
make start-rust-server
```

### Terminal 2: Wallet web

```bash
cd apps/wallet-web
pnpm install
# create .env.local with Clerk + backend URL settings
pnpm dev
```

Open `http://localhost:3000`, sign in, then go to `/wallets`.

## Verify

1. Backend health: `curl -k https://localhost:8080/health`
2. Frontend route: `http://localhost:3000/wallets`
3. Optional admin/operator flow: `http://localhost:3000/wallets/bootstrap`

## Sub-pages

- **Rust Server** - SGX build/run flow, env vars, dev commands
- **Wallet Web** - Next.js setup, proxy behavior, Clerk config

For contract workspace setup/deployment, see the **Contracts** section (`/contracts`).
