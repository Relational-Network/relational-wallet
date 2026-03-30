---
layout: default
title: Rust Server
parent: Installation
nav_order: 1
---

# Rust Server (Gramine SGX)
{: .fs-7 }

The enclave backend is an Axum REST API compiled to run inside Intel SGX via Gramine. It handles wallet operations, transaction signing, balance queries, fiat flows, and admin functions --- all within a hardware-isolated trust boundary.
{: .fs-5 .fw-300 }

---

## Prerequisites

| Requirement | Details |
|:------------|:--------|
| SGX hardware | Intel CPU with SGX2 + DCAP attestation support |
| SGX devices | `/dev/sgx/enclave` and `/dev/sgx/provision` must be available |
| Gramine | 1.8+ with `gramine-ratls-dcap` package |
| Rust toolchain | 1.92+ (pinned in `rust-toolchain.toml`) |
| Signing key | RSA 3072-bit with exponent 3 (for enclave signing) |

### Generate an Enclave Signing Key

If you don't have one yet:

```bash
gramine-sgx-gen-private-key
# Default location: ~/.config/gramine/enclave-key.pem
```

To use a custom key path:

```bash
SGX_SIGNING_KEY=/path/to/your/key.pem make
```

---

## Environment Configuration

```bash
cd apps/rust-server
cp .env.example .env
```

### Required Variables

| Variable | Description | Example |
|:---------|:------------|:--------|
| `CLERK_JWKS_URL` | Clerk JWKS endpoint for JWT verification | `https://your-app.clerk.accounts.dev/.well-known/jwks.json` |
| `CLERK_ISSUER` | Expected JWT issuer | `https://your-app.clerk.accounts.dev` |

### Optional Variables

| Variable | Default | Description |
|:---------|:--------|:------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Bind port |
| `DATA_DIR` | `/data` | Encrypted storage mount point |
| `LOG_FORMAT` | `pretty` | Log format (`pretty` or `json`) |
| `RUST_LOG` | `info,tower_http=debug` | Log level filter |
| `CLERK_AUDIENCE` | *(none)* | JWT audience claim (recommended for production) |
| `CLERK_SECRET_KEY` | *(none)* | Clerk backend API secret |
| `CORS_ALLOWED_ORIGINS` | *(permissive)* | Comma-separated allowed origins |

### Fiat Integration Variables (TrueLayer)

| Variable | Description |
|:---------|:------------|
| `TRUELAYER_CLIENT_ID` | TrueLayer app client ID |
| `TRUELAYER_CLIENT_SECRET` | TrueLayer app client secret |
| `TRUELAYER_SIGNING_KEY_ID` | Signing key identifier |
| `TRUELAYER_SIGNING_PRIVATE_KEY_PEM` | PEM-encoded signing key |
| `TRUELAYER_MERCHANT_ACCOUNT_ID` | Merchant account for settlements |
| `REUR_CONTRACT_ADDRESS_FUJI` | rEUR contract address (default: `0x76568...1A63`) |
| `TRUELAYER_WEBHOOK_SHARED_SECRET` | Webhook HMAC validation secret |

### Fiat Optional Variables

| Variable | Default | Description |
|:---------|:--------|:------------|
| `TRUELAYER_API_BASE_URL` | Sandbox URL | TrueLayer API base |
| `TRUELAYER_AUTH_BASE_URL` | Sandbox URL | TrueLayer auth base |
| `TRUELAYER_HOSTED_PAYMENTS_BASE_URL` | Sandbox URL | Hosted payment page base |
| `TRUELAYER_CURRENCY` | `EUR` | Settlement currency |
| `FIAT_MIN_CONFIRMATIONS` | `1` | Minimum block confirmations for settlement |

---

## Development Commands

These commands do **not** require SGX hardware:

```bash
cd apps/rust-server

# Quick compile check
make dev-check

# Run all 155+ unit tests
make dev-test

# Full debug build
make dev-build

# Code coverage
cargo tarpaulin --ignore-tests
cargo tarpaulin --out Html    # HTML report
```

Cargo aliases (defined in `.cargo/config.toml`):

```bash
cargo dev-check    # cargo check --features dev
cargo dev-build    # cargo build --features dev
cargo dev-test     # cargo test --features dev
```

The `dev` feature disables JWT signature verification, allowing tests to run without Clerk credentials.
{: .note }

---

## Build and Run in SGX

### Local SGX

```bash
cd apps/rust-server

# Build all SGX artifacts
make
# Produces: rust-server.manifest, rust-server.manifest.sgx, rust-server.sig

# Launch inside enclave
make start-rust-server
# Loads .env, starts gramine-sgx with RA-TLS

# Verify
curl -k https://localhost:8080/health
```

The `make start-rust-server` target:
1. Sources `.env` for environment variables
2. Launches `gramine-sgx rust-server` which:
   - Generates RA-TLS certificates with DCAP attestation evidence
   - Starts the Axum server on the configured `HOST:PORT`
   - Mounts encrypted filesystem at `DATA_DIR`

### Docker SGX

```bash
cd apps/rust-server

# Build production Docker image
make docker-build
# Multi-stage build: Ubuntu 20.04 → Rust compile → Gramine runtime
# Signs the enclave and extracts MRENCLAVE measurement

# Run container
make docker-run
# Mounts SGX devices, data volume, and .env

# Stop container
make docker-stop

# Inspect enclave measurements
make docker-sigstruct
```

Docker build features:
- **Deterministic builds**: Fixed `SOURCE_DATE_EPOCH`, single codegen unit, reproducibility flags
- **Pinned dependencies**: Ubuntu snapshot `20260210T000000Z`, exact Rust/Gramine versions
- **Non-root execution**: Runtime user `relational` (UID/GID 10001)
- **MRENCLAVE verification**: Built measurement compared against `measurements.toml`

---

## Enclave Measurements

The `measurements.toml` file pins the expected MRENCLAVE hash for the production Docker build. This enables clients to verify they are connecting to the correct enclave binary.

```bash
# View pinned measurements
cat measurements.toml

# Verify a local Docker build matches
make verify-mrenclave

# Extract SIGSTRUCT from a running image
make docker-sigstruct
```

The CI pipeline (`rust-server-ci.yml`) automatically verifies that the built MRENCLAVE matches the pinned value. If the measurement changes unexpectedly, the build fails.

---

## Inspect Enclave Measurements

```bash
# Show MRENCLAVE, MRSIGNER, ISV fields from the built image
make show-measurements
```

---

## API Endpoints

Once running, the backend exposes:

| URL | Description |
|:----|:------------|
| `https://localhost:8080/health` | Health check (no auth) |
| `https://localhost:8080/health/live` | Liveness probe |
| `https://localhost:8080/health/ready` | Readiness probe |
| `https://localhost:8080/docs` | Swagger UI (interactive API docs) |
| `https://localhost:8080/api-doc/openapi.json` | OpenAPI 3.1 specification |

All `/v1/*` endpoints require a valid Clerk JWT in the `Authorization: Bearer <token>` header.
{: .note }

---

## Cleanup

```bash
make clean       # Remove build artifacts
make distclean   # Clean + remove target/ directory
```
