---
layout: default
title: Rust Server
parent: Installation
nav_order: 3
---

# Rust Server (Gramine SGX)

Backend API for Relational Wallet. Runs inside Intel SGX using Gramine and RA-TLS.

## Prerequisites

- SGX host with `/dev/sgx/enclave` and `/dev/sgx/provision`
- Gramine + `gramine-ratls-dcap`
- Enclave signing key (`$HOME/.config/gramine/enclave-key.pem`), or custom key path
- Rust toolchain from `rust-toolchain.toml`

Generate signing key once (if missing):

```bash
gramine-sgx-gen-private-key
```

## Configure Environment

```bash
cd apps/rust-server
cp .env.example .env
```

Set at least:

- `CLERK_JWKS_URL`
- `CLERK_ISSUER`
- Fiat/reserve envs as needed for on-ramp/off-ramp flows

## Development Commands

```bash
cd apps/rust-server
make dev-check
make dev-test
```

Equivalent cargo aliases:

```bash
cargo dev-check
cargo dev-build
cargo dev-test
```

## Build + Run In SGX

```bash
cd apps/rust-server
make
make start-rust-server
```

- `make` builds SGX artifacts (`rust-server.manifest`, `.manifest.sgx`, `.sig`)
- `make start-rust-server` loads `.env` and starts `gramine-sgx rust-server`

Health check:

```bash
curl -k https://localhost:8080/health
```

## Docker SGX Flow

```bash
cd apps/rust-server
make docker-build
make docker-run
make docker-stop
```

See `apps/rust-server/docker/README.md` for DCAP and host-specific setup details.
