# Relational Wallet - AI Coding Instructions

## Architecture Overview

This is a **TEE-backed non-custodial stablecoin wallet** monorepo using Intel SGX for secure key management, with Avalanche as the settlement layer.

```
relational-wallet/
├── apps/rust-server/    # Axum REST API with DCAP RA-TLS (HTTPS only)
├── apps/wallet-web/     # Frontend (placeholder)
├── docs/                # PlantUML diagrams + Jekyll GitHub Pages
└── scripts/             # Header checks, licensing utilities
```

### Core Components
- **rust-server**: Axum backend with in-memory store, runs inside SGX enclaves via Gramine with DCAP remote attestation
- **Gramine RA-TLS**: TLS certificates generated at runtime by `gramine-ratls` with embedded attestation evidence
- **HTTPS mandatory**: Server refuses to start without valid RA-TLS credentials

## Key Development Workflows

### Build & Run (SGX Required)
```bash
cd apps/rust-server
cargo build --release    # Standard build (for testing outside SGX)
make                     # Build for SGX + sign enclave
make start-rust-server   # Run inside SGX enclave with DCAP RA-TLS
```

### Testing
```bash
cargo test                           # Unit tests
cargo tarpaulin --ignore-tests       # Coverage report
cargo tarpaulin --out Html           # HTML coverage report
```

### Docker (SGX with DCAP)
```bash
make docker-build && make docker-run  # Requires SGX hardware + signing key
```

## DCAP RA-TLS Architecture

### Runtime Flow
1. `gramine-sgx rust-server` launches the enclave
2. `gramine-ratls` (manifest entrypoint) generates TLS cert/key with DCAP attestation
3. Cert written to `/tmp/ra-tls.crt.pem`, key to `/tmp/ra-tls.key.pem`
4. Rust server loads credentials via [src/tls.rs](apps/rust-server/src/tls.rs)
5. Server starts HTTPS on port 8080

### TLS Module (`src/tls.rs`)
Handles Gramine's non-standard PEM labels:
```rust
// Gramine emits: "-----BEGIN TRUSTED CERTIFICATE-----"
// Rustls expects: "-----BEGIN CERTIFICATE-----"
// The tls module normalizes before parsing
```

Key functions:
- `load_ratls_credentials()` — Load cert+key from default paths (panics if missing)
- `load_ratls_certificate()` / `load_ratls_private_key()` — Individual loaders with error handling

## Code Conventions

### File Headers (Required)
All source files MUST include SPDX and copyright headers:
```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network
```
Verify with: `./scripts/check_headers.sh`

### API Structure Pattern
Follow the existing domain-driven layout in [apps/rust-server/src/api/](apps/rust-server/src/api/):
- `mod.rs` — Router composition with `utoipa` OpenAPI annotations
- `{domain}.rs` — Handlers grouped by feature (bookmarks, invites, recurring, wallet)
- All routes versioned under `/v1/`

### Handler Pattern
```rust
#[utoipa::path(get, path = "/v1/example", tag = "Example", responses((status = 200, body = T)))]
pub async fn handler(State(state): State<AppState>, Query(q): Query<Q>) -> Result<Json<T>, ApiError> {
    let store = state.store.read().await;  // RwLock access
    // ...
}
```

### Error Handling
Use `ApiError` constructors from [src/error.rs](apps/rust-server/src/error.rs):
- `ApiError::not_found("message")` → 404
- `ApiError::bad_request("message")` → 400
- `ApiError::unprocessable("message")` → 422

## OpenAPI Documentation
- Swagger UI: `https://localhost:8080/docs` (HTTPS only)
- OpenAPI JSON: `https://localhost:8080/api-doc/openapi.json`
- Register new endpoints in `ApiDoc` struct in [src/api/mod.rs](apps/rust-server/src/api/mod.rs)

## Environment Variables
| Variable | Purpose | Default |
|----------|---------|---------|
| `HOST` | Bind address | `0.0.0.0` |
| `PORT` | Bind port | `8080` |
| `SEED_INVITE_CODE` | Pre-seed invite code | — |
| `DATA_DIR` | Data directory for health checks | — |

## Gramine/SGX Configuration

### Manifest Key Settings (`rust-server.manifest.template`)
- `libos.entrypoint = "/gramine-ratls"` — RA-TLS generates certs before app starts
- `sgx.remote_attestation = "dcap"` — DCAP attestation (no EPID)
- `/tmp` mounted as `tmpfs` — Required for RA-TLS cert/key output
- `sgx.debug = true` — Change to `false` for production

### Prerequisites
- SGX signing key: `~/.config/gramine/enclave-key.pem` (generate with `gramine-sgx-gen-private-key`)
- DCAP infrastructure: PCCS configured and accessible
- SGX devices: `/dev/sgx/enclave` and `/dev/sgx/provision`
