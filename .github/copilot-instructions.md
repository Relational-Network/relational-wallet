# Relational Wallet - AI Coding Instructions

## Architecture Overview

This is a **TEE-backed custodial wallet service** monorepo using Intel SGX for secure key management, with Avalanche as the settlement layer.

```
relational-wallet/
├── apps/rust-server/    # Axum REST API with DCAP RA-TLS (HTTPS only)
├── apps/wallet-web/     # Next.js 16 frontend with Clerk authentication
├── docs/                # PlantUML diagrams + Jekyll GitHub Pages
└── scripts/             # Header checks, licensing utilities
```

---

## Features

### 🔐 Security & TEE

| Feature | Description |
|---------|-------------|
| **SGX Enclave Execution** | All sensitive operations run inside Intel SGX via Gramine |
| **DCAP Remote Attestation** | RA-TLS certificates with embedded attestation evidence |
| **Encrypted Storage** | Gramine sealed filesystem at `/data` (key: `_sgx_mrsigner`) |
| **secp256k1 Key Generation** | Ethereum/Avalanche-compatible keypairs generated inside enclave |
| **Private Key Isolation** | Keys never leave enclave unencrypted |

### 🔑 Authentication & Authorization

| Feature | Description |
|---------|-------------|
| **Clerk JWT Verification** | Production JWKS verification (RS256/RS384/RS512/ES256) |
| **Issuer/Audience Validation** | Configurable via `CLERK_ISSUER` and `CLERK_AUDIENCE` |
| **Role-Based Access** | Admin, Client, Support, Auditor roles with hierarchical privileges |
| **Ownership Enforcement** | Every wallet bound to user_id, verified on all operations |
| **Clock Skew Tolerance** | 60-second leeway for JWT expiration |

### 💰 Wallet Management

| Feature | Description |
|---------|-------------|
| **Create Wallet** | `POST /v1/wallets` — Generate secp256k1 keypair with Ethereum address |
| **List Wallets** | `GET /v1/wallets` — User's wallets only |
| **Get Wallet** | `GET /v1/wallets/{id}` — Owner-only access |
| **Delete Wallet** | `DELETE /v1/wallets/{id}` — Soft delete |
| **Suspend/Activate** | Admin can suspend/activate wallets |

### ⛓️ Blockchain Integration (Avalanche C-Chain)

| Feature | Description |
|---------|-------------|
| **Native Balance** | `GET /v1/wallets/{id}/balance/native` — AVAX balance query |
| **Full Balance** | `GET /v1/wallets/{id}/balance` — AVAX + ERC-20 tokens |
| **USDC Support** | Fuji testnet USDC (`0x5425890298aed601595a70AB815c96711a31Bc65`) |
| **Network Selection** | Query parameter `?network=fuji` or `?network=mainnet` |

### 👤 Admin & Audit

| Feature | Description |
|---------|-------------|
| **System Stats** | `GET /v1/admin/stats` — Wallet counts, uptime |
| **List All Wallets** | `GET /v1/admin/wallets` — Cross-user view |
| **List Users** | `GET /v1/admin/users` — Users with resource counts |
| **Audit Logs** | `GET /v1/admin/audit/events` — Query with filters |
| **Health Check** | `GET /v1/admin/health` — Detailed health with storage metrics |

### 🌐 Frontend (wallet-web)

| Feature | Description |
|---------|-------------|
| **Clerk Authentication** | Sign-in/sign-up with Clerk |
| **API Proxy** | Server-side proxy handles RA-TLS certificates |
| **Wallet Dashboard** | List, create, view wallet details |
| **Balance Display** | Real-time AVAX + USDC balances with refresh |
| **Faucet Links** | Quick links to Avalanche and Circle testnet faucets |

### 📚 Additional Features

| Feature | Description |
|---------|-------------|
| **Bookmarks** | CRUD with ownership enforcement |
| **Invites** | Validation and redemption system |
| **Recurring Payments** | Management (execution logic pending) |
| **OpenAPI/Swagger** | Auto-generated docs at `/docs` |
| **Structured Logging** | Request IDs via tracing + tower-http |

---

## Priority Backlog

### 🔴 P0 — Critical (Production Blockers)

| Task | Description | Files |
|------|-------------|-------|
| **SGX Debug Mode Off** | Set `sgx.debug = false` for production | `rust-server.manifest.template` |
| **Enclave Signing Key** | Secure production signing key | Ops/deployment |

### 🟠 P1 — High Priority

| Task | Description | Files |
|------|-------------|-------|
| **Rate Limiting** | Limit auth failures to prevent brute force | New middleware |
| **Transaction History** | Store transaction records in `txs/` directory | `src/storage/repository/wallets.rs` |
| **Clerk Organizations** | Support organization claims for multi-tenant | `src/auth/claims.rs` |

### 🟡 P2 — Medium Priority

| Task | Description | Files |
|------|-------------|-------|
| **Pagination** | Add pagination to list endpoints | `src/api/` |
| **Admin Filtering** | Add filtering/sorting to admin endpoints | `src/api/admin.rs` |
| **Storage Metrics** | Endpoint for disk usage, file counts | `src/api/admin.rs` |
| **Storage Compaction** | Remove soft-deleted data after retention | `src/storage/` |
| **Wallet Labels** | User-friendly naming for wallets | `src/storage/repository/wallets.rs` |
| **Support Role** | Read-only metadata access endpoints | `src/api/` |
| **Auditor Role** | Read-only audit access endpoints | `src/api/` |
| **Balance Caching** | Cache balance queries to avoid RPC rate limits | `src/api/balance.rs` |
| **Wallet List Balances** | Show balance summary in wallet list | `src/api/wallets.rs` |
| **Code Cleanup** | Remove unused exports and warnings | Various |

### 🔵 P3 — Lower Priority (Future)

| Task | Description | Files |
|------|-------------|-------|
| **Transaction Signing** | Sign transactions with enclave-held keys | `src/api/`, `src/blockchain/` |
| **Transaction Broadcasting** | Submit to Avalanche network | New module |
| **Smart Contract Calls** | Interact with deployed contracts | New module |
| **Event Listening** | Monitor on-chain events | New module |
| **WebSocket Support** | Real-time balance/tx updates | New module |
| **Batch Operations** | Multiple wallets/transactions in one call | `src/api/` |
| **Prometheus Metrics** | `/metrics` endpoint for monitoring | New module |
| **OpenTelemetry** | Distributed tracing headers | Middleware |
| **Backup/Export** | Export encrypted archives | New module |
| **Multi-sig Wallets** | Multi-signature wallet support | New module |

### 📋 Documentation TODO

| Task | Description |
|------|-------------|
| **Deployment Runbook** | Step-by-step production deployment guide |
| **Upgrade & Recovery** | Version upgrade and disaster recovery notes |
| **Recurring Payments** | Document execution logic when implemented |

---

## Security Checklist (Pre-Production)

- [x] JWKS signature verification enabled
- [x] JWT issuer validation enabled
- [x] JWT audience validation (optional, configurable)
- [x] Clock skew tolerance (60 seconds)
- [ ] `sgx.debug = false` in manifest
- [ ] Rate limiting on auth endpoints
- [x] Audit logging covers all sensitive operations
- [ ] No plaintext secrets in logs (review pending)
- [x] TLS certificate validation in JWKS fetch (rustls-tls)
- [ ] Enclave signing key secured
- [ ] Encrypted storage mount verified on host

---

## Development Reference

### Frontend (wallet-web)

#### Key Components

```
apps/wallet-web/src/
├── app/
│   ├── api/proxy/[...path]/route.ts  # Backend proxy (handles RA-TLS certs)
│   ├── wallets/                       # Wallet pages (list, create, detail)
│   └── account/                       # User account page
├── components/
│   └── WalletBalance.tsx             # Balance display with refresh
├── lib/
│   ├── api.ts                        # Typed API client (WalletApiClient)
│   └── auth.ts                       # Clerk helpers
└── types/api.ts                      # OpenAPI-generated types
```

#### API Proxy Pattern

Browsers reject self-signed RA-TLS certificates. The proxy handles this:

```
Browser → /api/proxy/v1/wallets → Next.js Server → SGX Enclave
```

#### Environment Variables

| Variable | Purpose | Required |
|----------|---------|----------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Clerk frontend key | Yes |
| `CLERK_SECRET_KEY` | Clerk backend secret | Yes |
| `WALLET_API_BASE_URL` | Backend URL (server-only) | Yes |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Accept self-signed certs | Dev only |

#### Commands

```bash
cd apps/wallet-web
pnpm install           # Install dependencies
pnpm dev               # Start dev server (http://localhost:3000)
pnpm generate-types    # Regenerate types from OpenAPI
```

### Backend (rust-server)

#### Module Structure

```
src/
├── api/           # HTTP handlers (admin, balance, wallets, etc.)
├── auth/          # JWT verification, roles, extractors
├── blockchain/    # Avalanche C-Chain client (alloy)
├── storage/       # Encrypted FS repositories
├── state.rs       # AppState with encrypted storage
├── models.rs      # Request/response structs
├── error.rs       # API error types
├── tls.rs         # RA-TLS credential loading
└── main.rs        # Server startup
```

#### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `HOST` | Bind address | `0.0.0.0` |
| `PORT` | Bind port | `8080` |
| `CLERK_JWKS_URL` | Clerk JWKS endpoint | **Required for production** |
| `CLERK_ISSUER` | Clerk issuer URL | **Required for production** |
| `CLERK_AUDIENCE` | Expected JWT audience | — |
| `DATA_DIR` | Encrypted data directory | `/data` |
| `LOG_FORMAT` | Logging format (`json`/`pretty`) | `pretty` |
| `RUST_LOG` | Log level filter | `info,tower_http=debug` |

#### Build & Run

```bash
cd apps/rust-server
cargo build --release    # Standard build
make                     # Build for SGX + sign enclave
make start-rust-server   # Run inside SGX enclave
```

#### Testing

```bash
cargo test                                          # Unit tests (101 passing)
cargo test --test blockchain_integration -- --ignored  # Integration tests (10 passing)
cargo tarpaulin --ignore-tests                      # Coverage report
```

### Storage Layout

```
/data/                    # Gramine encrypted mount
├── wallets/{id}/
│   ├── metadata.json     # WalletMetadata
│   └── private_key.pem   # NEVER exposed
├── bookmarks/{id}.json
├── invites/{id}.json
├── recurring/{id}.json
└── audit/{date}/events.jsonl
```

---

## Code Conventions

### Dependency Guidelines

**Minimal dependencies for enclave security and audit scope:**

| Principle | Implementation |
|-----------|----------------|
| **Use std library** | Prefer `std::sync::OnceLock` over `lazy_static` |
| **Consolidate crates** | Use `alloy::hex` instead of separate `hex` crate |
| **Feature flags** | Use `alloy::primitives::keccak256` instead of `sha3` crate |
| **Avoid C deps** | Use `rustls` not OpenSSL, pure-Rust crypto only |
| **Pin versions** | Specify exact minor versions (e.g., `"1.5.2"` not `"1"`) |

**Consolidated dependencies (do NOT add these separately):**
- `hex` → use `alloy::hex`
- `sha3` → use `alloy::primitives::keccak256`
- `lazy_static` → use `std::sync::OnceLock`
- `rand` → use `k256::elliptic_curve::rand_core::OsRng`

**Current versions (Rust 1.92+):**
```toml
axum = "0.8.8"
alloy = "1.5.2"
k256 = "0.13.4"
tokio = "1.49.0"
serde = "1.0.228"
rustls = "0.23.26"
jsonwebtoken = "10.3.0"
```

### File Headers (Required)

```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network
```

Verify with: `./scripts/check_headers.sh`

### Handler Patterns

**Protected endpoint:**
```rust
pub async fn handler(
    Auth(user): Auth,
    State(state): State<AppState>,
) -> Result<Json<T>, ApiError> {
    // user.user_id available for ownership checks
}
```

**Admin-only endpoint:**
```rust
pub async fn admin_handler(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<T>, ApiError> {
    // Only admins reach here
}
```

### Error Handling

```rust
ApiError::not_found("message")     // 404
ApiError::bad_request("message")   // 400
ApiError::forbidden("message")     // 403
ApiError::unprocessable("message") // 422
```

---

## Testing with JWT

```bash
# Get token from Clerk or frontend
export JWT="eyJhbG..."

# Make authenticated request
curl -k -H "Authorization: Bearer $JWT" https://localhost:8080/v1/users/me
```

---

## OpenAPI Documentation

- Swagger UI: `https://localhost:8080/docs`
- OpenAPI JSON: `https://localhost:8080/api-doc/openapi.json`

---

## Gramine/SGX Configuration

Key manifest settings (`rust-server.manifest.template`):
- `libos.entrypoint = "/gramine-ratls"` — RA-TLS generates certs before app
- `sgx.remote_attestation = "dcap"` — DCAP attestation
- `/data` mounted as `encrypted` with `key_name = "_sgx_mrsigner"`
- `sgx.debug = true` — **Change to `false` for production**

Prerequisites:
- SGX signing key: `~/.config/gramine/enclave-key.pem`
- DCAP infrastructure: PCCS configured
- SGX devices: `/dev/sgx/enclave` and `/dev/sgx/provision`
