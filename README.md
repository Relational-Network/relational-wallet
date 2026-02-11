# Relational Wallet

**TEE-backed custodial Avalanche wallet service** running inside Intel SGX enclaves with Gramine. All private keys and sensitive data are encrypted at rest using SGX sealing.

## Architecture

This monorepo contains:

- **[Rust Server](apps/rust-server/)** — Axum REST API running inside SGX with DCAP RA-TLS
- **[Wallet Web](apps/wallet-web/)** — Next.js frontend with Clerk authentication
- **[Contracts](apps/contracts/)** — Foundry workspace for `Relational Euro (rEUR)` smart contracts

## Security Model

- **Intel SGX** — All cryptographic operations run inside a hardware enclave
- **DCAP Remote Attestation** — TLS certificates embed attestation evidence
- **Gramine Encrypted FS** — All data sealed to enclave identity at `/data`
- **Clerk JWT Auth** — Role-based access (Admin, Client, Support, Auditor)
- **Ownership Enforcement** — Users can only access their own wallets

## Features

### Core
- **Wallet Management** — Create, list, delete wallets with secp256k1 key generation (Ethereum-compatible)
- **Bookmarks** — Address book per wallet with ownership enforcement
- **Invites** — Invite codes with expiration and redemption tracking
- **Recurring Payments** — Scheduled payment configuration

### Admin & Operations (Phase 6)
- **System Statistics** — Wallet counts, invite usage, uptime metrics
- **User Management** — List all users with resource counts
- **Audit Logs** — Query security events with date range and filters
- **Wallet Suspension** — Admin can suspend/reactivate wallets

### API
- Swagger UI: `https://localhost:8080/docs`
- OpenAPI JSON: `https://localhost:8080/api-doc/openapi.json`

### Smart Contracts
- **Relational Euro (`rEUR`)** managed ERC-20 contract (mint, burn, pause, role-based access)
- **Fuji Deployment**: `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`
- **Deployment Tx**: `0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14`

## Quick Start

### Prerequisites
- Intel SGX hardware with DCAP support
- Gramine with `gramine-ratls-dcap` package
- Rust 1.92+
- Node.js 20+

### Run with SGX
```bash
cd apps/rust-server
make                    # Build for SGX
make start-rust-server  # Run inside enclave
```

### Run Frontend
```bash
cd apps/wallet-web
pnpm install
pnpm dev
```

### Run Contract Tests
```bash
cd apps/contracts
forge test -vv
```

## Documentation

- [Rust Server README](apps/rust-server/README.md) — Detailed API documentation
- [Contracts README](apps/contracts/README.md) — Contract design, testing, and deployment
- [Copilot Instructions](.github/copilot-instructions.md) — AI coding guidelines
- [API Docs](docs/gh-pages/) — GitHub Pages documentation

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPL-3.0).

You may copy, modify, and redistribute this work under the terms of the AGPL-3.0.
A full copy of the license can be found in the `LICENSE` file or at:

👉 https://www.gnu.org/licenses/agpl-3.0.html

## Development Scripts

### Header Checks
```bash
./scripts/check_headers.sh                    # Verify SPDX headers
REQUIRED_YEAR=2026 ./scripts/check_headers.sh # Require specific year
./scripts/update_header_year.sh 2025 2026     # Update year in headers
```

### Tests
```bash
cd apps/rust-server && cargo test             # Run unit tests
```

