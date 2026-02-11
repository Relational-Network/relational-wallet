---
layout: default
title: Installation
nav_order: 2
has_children: true
permalink: /installation/
---

# Installation

The Relational Wallet consists of two main components:

## Components

| Component | Technology | Purpose |
|-----------|------------|---------|
| **rust-server** | Axum + Gramine SGX | Backend running in SGX enclave with DCAP RA-TLS |
| **wallet-web** | Next.js 16 + Clerk | Frontend with authentication and API proxy |

## Quick Start

### Prerequisites

1. Intel SGX hardware with `/dev/sgx/enclave` and `/dev/sgx/provision`
2. [Gramine](https://gramine.readthedocs.io/) installed
3. Node.js 18+ and pnpm
4. [Clerk](https://clerk.dev) account

### Start the System

```bash
# Terminal 1: Start the enclave backend
cd apps/rust-server
make                    # Build for SGX
gramine-sgx rust-server # Start server (https://localhost:8080)

# Terminal 2: Start the frontend
cd apps/wallet-web
pnpm install
cp .env.local.example .env.local  # Configure Clerk keys
pnpm dev                # Start dev server (http://localhost:3000)
```

### Verify Setup

1. Open [http://localhost:3000](http://localhost:3000)
2. Sign in with Clerk
3. Navigate to "Create Wallet"
4. Verify wallet appears in your list

## Sub-pages

Each sub-page provides detailed setup instructions:

- **Rust Server** — SGX build, Docker deployment, environment variables
- **Wallet Web** — Next.js setup, Clerk configuration, proxy architecture
