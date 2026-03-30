---
layout: default
title: Installation
nav_order: 2
has_children: true
permalink: /installation/
---

# Installation
{: .fs-8 }

Set up the complete Relational Wallet stack: SGX backend, web frontend, and smart contracts.
{: .fs-5 .fw-300 }

---

## System Requirements

| Component | Requirement | Notes |
|:----------|:------------|:------|
| **CPU** | Intel CPU with SGX2 support | Required for enclave execution |
| **SGX Devices** | `/dev/sgx/enclave`, `/dev/sgx/provision` | DCAP attestation (not EPID) |
| **Gramine** | 1.8+ with `gramine-ratls-dcap` | TEE runtime and RA-TLS |
| **Rust** | 1.92+ | Pinned via `rust-toolchain.toml` |
| **Node.js** | 20+ | Frontend runtime |
| **pnpm** | Latest | Frontend package manager |
| **Foundry** | Latest | Smart contract toolchain (optional) |
| **Docker** | 24+ (optional) | For containerized SGX deployment |

## External Services

| Service | Purpose | Required? |
|:--------|:--------|:----------|
| **Clerk** | JWT authentication (sign-up, sign-in, JWKS) | Yes |
| **Avalanche RPC** | Fuji/mainnet C-Chain node | Yes (for blockchain features) |
| **TrueLayer** | Fiat on-ramp / off-ramp | Optional (for fiat features) |
| **DuckDNS** | Dynamic DNS for webhook proxy | Optional (for production proxy) |

---

## Quick Start

### Terminal 1 --- Backend (SGX Enclave)

```bash
cd apps/rust-server
cp .env.example .env        # Configure required env vars
make                        # Build SGX artifacts
make start-rust-server      # Launch inside enclave
```

Verify the backend is running:

```bash
curl -k https://localhost:8080/health
# {"status":"healthy","checks":{"service":"ok","data_dir":"ok"}}
```

### Terminal 2 --- Frontend

```bash
cd apps/wallet-web
cp .env.example .env.local  # Add Clerk keys + backend URL
pnpm install
pnpm dev                    # http://localhost:3000
```

### Terminal 3 --- Smart Contracts (optional)

```bash
cd apps/contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge test -vv
```

---

## Verification Checklist

After setup, confirm each component:

| Check | Command / URL | Expected |
|:------|:-------------|:---------|
| Backend health | `curl -k https://localhost:8080/health` | `{"status":"healthy",...}` |
| Swagger UI | `https://localhost:8080/docs` | OpenAPI documentation |
| Frontend | `http://localhost:3000` | Clerk sign-in page |
| Wallet dashboard | `http://localhost:3000/wallets` | Empty wallet list (after sign-in) |
| Contract tests | `forge test -vv` (in `apps/contracts/`) | All tests pass |

---

## Development Without SGX

If you don't have SGX hardware, you can still compile and test the backend:

```bash
cd apps/rust-server
make dev-check    # Type-check only (no SGX required)
make dev-test     # Run 155+ unit tests (no SGX required)
make dev-build    # Debug build (no SGX required)
```

The `dev` feature flag disables JWT signature verification, allowing tests to run without a Clerk instance. SGX is only required for `make` (manifest generation) and `make start-rust-server` (enclave execution).

---

## Sub-pages

- [**Rust Server**](/relational-wallet/installation/rust-server) --- SGX build flow, environment variables, Docker deployment, and enclave measurements
- [**Wallet Web**](/relational-wallet/installation/wallet-web) --- Next.js setup, Clerk configuration, proxy architecture, and route map
