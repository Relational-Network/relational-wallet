# Relational Wallet — AI Coding Instructions

TEE-backed non-custodial wallet service. Axum REST API runs inside Intel SGX via Gramine; Avalanche C-Chain is the settlement layer; Clerk handles auth.

## Repo Layout

```
apps/
  rust-server/    # Axum REST API, RA-TLS HTTPS only, runs in SGX
  wallet-web/     # Next.js frontend with Clerk + server-side proxy
  contracts/      # Foundry — RelationalEuro ERC-20
  proxy/          # nginx TLS terminator (public deployments)
docs/             # Jekyll site + PlantUML sequence diagrams
scripts/          # Header checks, deploy helpers
```

## Backend Module Layout (`apps/rust-server/src/`)

```
api/        # HTTP handlers: admin, balance, bookmarks, fiat, health,
            # payment_links, resolve, transactions, users, wallets
auth/       # Clerk JWT verification, roles, extractors (Auth, AdminOnly)
blockchain/ # Avalanche C-Chain client (alloy)
discovery/  # Peer registry, RA-TLS FFI, attestation policy, VOPRF
indexer/    # On-chain event indexing
providers/  # External provider adapters (fiat, etc.)
storage/    # Encrypted-FS repositories with ownership enforcement
config.rs   models.rs   state.rs   error.rs   tls.rs   main.rs
```

## Conventions

**File headers (required, checked by `scripts/check_headers.sh`):**
```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network
```

**Handlers:**
```rust
pub async fn h(Auth(user): Auth, State(s): State<AppState>) -> Result<Json<T>, ApiError>
pub async fn h(AdminOnly(_): AdminOnly, State(s): State<AppState>) -> Result<Json<T>, ApiError>
```

**Errors:** `ApiError::{not_found, bad_request, forbidden, unprocessable, internal}`.

**Dependencies — minimal, pure Rust, pinned to exact minor versions.** Do NOT add: `hex` (use `alloy::hex`), `sha3` (use `alloy::primitives::keccak256`), `lazy_static` (use `std::sync::OnceLock`), `rand` (use `k256::elliptic_curve::rand_core::OsRng`), or any C-backed crypto. Stack: `axum 0.8`, `alloy 1.5`, `k256 0.13`, `tokio 1.49`, `rustls 0.23`, `jsonwebtoken 10`. Rust 1.92+.

**Ownership:** every storage operation must verify `user_id`. Private keys never leave the enclave or appear in API responses.

**Blocking work in handlers:** wrap synchronous FFI / file I/O in `tokio::task::spawn_blocking`; never call DCAP collateral fetches on the reactor thread.

## Build & Test

```bash
# Backend
cd apps/rust-server
cargo build --features dev          # local build (allows insecure JWT decode)
cargo test --features dev           # unit tests
cargo test --test blockchain_integration -- --ignored
make                                # build+sign SGX enclave (dev manifest)
make start-rust-server              # run inside SGX
make docker-build && make docker-run  # production image (sgx.debug=false)

# Frontend
cd apps/wallet-web
pnpm install && pnpm dev            # http://localhost:3000
pnpm generate-types                 # regenerate from backend OpenAPI
```

## Environment

**Backend:** `HOST`, `PORT`, `DATA_DIR` (`/data`), `CLERK_JWKS_URL`, `CLERK_ISSUER`, `CLERK_AUDIENCE`, `CORS_ALLOWED_ORIGINS`, `LOG_FORMAT`, `RUST_LOG`. Clerk vars are required in production.

**Frontend:** `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY`, `CLERK_SECRET_KEY`, `WALLET_API_BASE_URL`. `NODE_TLS_REJECT_UNAUTHORIZED=0` only in dev (frontend has a production guard).

## SGX / Gramine

Two manifest templates — no Jinja conditionals:

| Template | Purpose |
|---|---|
| `apps/rust-server/rust-server.manifest.template` | Local `make` — `sgx.debug=true`, file-based dev key at `data/.dev_storage_key` |
| `apps/rust-server/docker/rust-server.manifest.template` | Production — `sgx.debug=false`, `_sgx_mrsigner` key derivation, static DNS at `/app/dns/` (Docker's `127.0.0.11` is unreachable from SGX) |

Both: `libos.entrypoint = gramine-ratls`, `sgx.remote_attestation = "dcap"`, `/data` mounted as encrypted FS, Clerk/CORS env passthrough via `loader.env.*`.

Prereqs: `~/.config/gramine/enclave-key.pem`, PCCS configured (Azure DCAP), `/dev/sgx/{enclave,provision}` mounted.

## Storage Layout (`/data`, Gramine encrypted)

```
wallets/{id}/{meta.json, key.pem, txs/{tx_hash}.json}
bookmarks/{id}.json   invites/{id}.json   recurring/{id}.json
fiat/...              audit/{date}/events.jsonl   tx.redb
```

## OpenAPI

Swagger UI at `/docs`; spec at `/api-doc/openapi.json`. Keep `tag = "Admin"` (capital A) consistent across all admin endpoints — OpenAPI tags are case-sensitive.

## Tracking

When you add features, append a brief note to the gitignored `week_report.md` (endpoints, bugs fixed, follow-ups).
