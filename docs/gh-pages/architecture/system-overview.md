---
layout: default
title: System Overview
parent: Architecture
nav_order: 1
---

# System Overview
{: .fs-7 }

Detailed breakdown of every component in the Relational Wallet stack.
{: .fs-5 .fw-300 }

---

## Technology Stack

| Layer | Technology | Version | Purpose |
|:------|:-----------|:--------|:--------|
| **Runtime** | Intel SGX + Gramine | SGX2 / Gramine 1.8 | Hardware enclave execution |
| **Backend** | Rust + Axum + Tokio | Rust 1.92, Axum 0.8 | Async REST API |
| **Frontend** | Next.js + React | Next 16, React 19 | Browser UI with SSR |
| **Authentication** | Clerk | Latest | JWT identity + JWKS |
| **Blockchain** | Avalanche C-Chain | Alloy 1.5 | EVM transactions via RPC |
| **Cryptography** | k256 + ring | k256 0.13, ring 0.17 | secp256k1 + TLS primitives |
| **Database** | redb | 3.1 | Embedded ACID key-value store |
| **Smart Contracts** | Solidity + Foundry | Solc 0.8.24 | rEUR ERC-20 token |
| **Fiat** | TrueLayer | Sandbox | EUR on/off-ramp |
| **Proxy** | Nginx | Latest | TLS termination for webhooks |
| **CI/CD** | GitHub Actions | --- | Automated testing + builds |

---

## Rust Server (`apps/rust-server/`)

The core backend, running entirely inside an Intel SGX enclave.

### Module Architecture

```
src/
├── main.rs              # Server startup, HTTPS with RA-TLS, route composition
├── state.rs             # AppState: encrypted storage, background tasks, chain client
├── config.rs            # Environment variable parsing
├── tls.rs               # RA-TLS certificate/key loading
├── models.rs            # Request/response DTOs
├── error.rs             # Typed API error handling
├── fiat_poller.rs       # Background fiat request status polling
│
├── api/                 # Route handlers
│   ├── wallets.rs       # Create, list, get, delete wallets
│   ├── balance.rs       # Native AVAX + ERC-20 token balances
│   ├── transactions.rs  # Send, estimate gas, history, status
│   ├── bookmarks.rs     # Address book CRUD
│   ├── invites.rs       # Invite validation + redemption
│   ├── payment_links.rs # Email-based payment request links
│   ├── fiat.rs          # On-ramp/off-ramp lifecycle
│   ├── admin.rs         # Stats, users, wallets, audit, suspension
│   ├── users.rs         # GET /v1/users/me
│   └── resolve.rs       # Email hash → address resolution
│
├── auth/                # Authentication & authorization
│   ├── claims.rs        # ClerkClaims struct + user extraction
│   ├── extractor.rs     # Auth, AdminOnly, OptionalAuth extractors
│   ├── jwks.rs          # JWKS fetching + caching with TTL grace
│   ├── jwt_crypto.rs    # JWT signing/verification (RS256/384/512, ES256)
│   ├── middleware.rs     # Auth middleware + request ID propagation
│   └── roles.rs         # Role enum: Admin, Client, Support, Auditor
│
├── blockchain/          # Chain interaction
│   ├── client.rs        # Avalanche C-Chain RPC client (Alloy)
│   ├── erc20.rs         # ERC-20 balance queries
│   ├── signing.rs       # secp256k1 key generation + signing
│   ├── transactions.rs  # Tx construction, signing, broadcasting
│   └── types.rs         # BlockchainTx, GasEstimate types
│
├── storage/             # Data persistence
│   ├── encrypted_fs.rs  # File-based encrypted storage (JSON)
│   ├── audit.rs         # Audit event logging + queries
│   ├── paths.rs         # Directory structure constants
│   ├── ownership.rs     # User ownership verification
│   ├── tx_cache.rs      # In-memory LRU transaction cache
│   ├── tx_database.rs   # redb transaction storage
│   └── repository/      # Domain-specific CRUD
│       ├── wallets.rs   # Wallet metadata + PEM key storage
│       ├── bookmarks.rs # Bookmark CRUD
│       ├── invites.rs   # Invite storage + tracking
│       ├── payment_links.rs  # Email payment links
│       ├── transactions.rs   # Transaction history
│       ├── fiat.rs      # Fiat request tracking
│       ├── email_index.rs    # Email → UserId index
│       └── service_wallet.rs # Reserve wallet key storage
│
├── providers/           # External service integrations
│   ├── truelayer.rs     # TrueLayer API client (970 LOC)
│   ├── clerk.rs         # Clerk backend SDK
│   └── email.rs         # Email sending
│
└── indexer/             # Background services
    └── mod.rs           # Blockchain event indexer
```

### Key Design Decisions

| Decision | Rationale |
|:---------|:----------|
| **File-based storage** over database | Simplifies enclave deployment; no database process inside SGX. Gramine's encrypted FS provides ACID-like durability. |
| **redb** for indexed data | Pure Rust, embedded, zero-dependency database for transaction indexing. No network surface. |
| **Alloy** over ethers | Modern, maintained Ethereum library with native async. Direct EIP-1559 transaction support. |
| **Axum** over Actix | Tower middleware ecosystem, strong typing, simpler async model. |
| **No key export API** | Private keys are sealed to the enclave. Signing happens server-side; keys never leave SGX. |
| **Background poller** for fiat | TrueLayer webhooks are best-effort; the poller ensures eventual consistency. |

---

## Wallet Web (`apps/wallet-web/`)

Next.js 16 single-page application with server-side rendering and API proxying.

### Architecture

```
src/
├── app/                    # Next.js App Router pages
│   ├── layout.tsx          # Root layout (ClerkProvider, global styles)
│   ├── page.tsx            # Landing page
│   ├── api/proxy/          # Server-side API proxy to enclave
│   ├── wallets/            # Wallet CRUD, send, receive, fiat
│   ├── admin/              # Admin dashboard
│   ├── pay/                # Payment link viewer
│   ├── callback/           # TrueLayer callback handler
│   └── account/            # User account + JWT display
│
├── components/             # Reusable UI components
│   ├── WalletBalance.tsx   # Balance with auto-refresh
│   ├── WalletSwitcher.tsx  # Multi-wallet selector
│   ├── AddressQRCode.tsx   # QR code generation
│   ├── CopyAddress.tsx     # Click-to-copy address
│   └── TokenDisplay.tsx    # Token balance formatting
│
├── lib/                    # Shared utilities
│   ├── api.ts              # Typed API client (proxy-based)
│   ├── auth.ts             # Clerk auth helpers
│   └── emailHash.ts        # Email hashing for payment links
│
└── types/
    └── api.ts              # Auto-generated from OpenAPI spec
```

### Proxy Flow

```
Browser → /api/proxy/v1/wallets
  ↓
Next.js route handler (src/app/api/proxy/[...path]/route.ts)
  ↓
1. Strip /api/proxy prefix
2. Fetch Clerk JWT from session
3. Forward to WALLET_API_BASE_URL with Authorization header
4. Return response to browser
```

This architecture ensures:
- Backend URL never exposed to browser
- JWT never stored in browser-accessible storage
- TLS certificate handling happens server-side

---

## Smart Contracts (`apps/contracts/`)

Single Solidity contract deployed on Avalanche Fuji testnet.

| Property | Value |
|:---------|:------|
| Name | `RelationalEuro` |
| Symbol | `rEUR` |
| Standard | ERC-20 + AccessControl + Pausable + Burnable |
| Decimals | 6 |
| Compiler | Solc 0.8.24, optimizer 200 runs, EVM Paris |
| Framework | Foundry |

See [Smart Contracts](/relational-wallet/contracts) for full details.

---

## Reverse Proxy (`apps/proxy/`)

Nginx reverse proxy for external integrations that cannot validate RA-TLS certificates.

```
External caller (TrueLayer webhook)
  ↓
Nginx (Let's Encrypt cert on relational-wallet.duckdns.org)
  ↓
Backend (RA-TLS on localhost:8080, cert verification disabled)
```

Features:
- Let's Encrypt certificate via DuckDNS DNS-01 challenge
- Rate limiting on webhook endpoint (10 req/s per IP, burst 20)
- TrueLayer-specific header forwarding (`tl-*` headers)
- Auto-renewal via cron

---

## External Dependencies

| Service | Purpose | Failure Mode |
|:--------|:--------|:-------------|
| **Clerk JWKS** | JWT verification | Cached with 2x TTL grace period; fails closed after cache expires |
| **Avalanche RPC** | Balance, gas, broadcast | Balance queries fail; transactions cannot be sent |
| **TrueLayer** | Fiat flows | On/off-ramp unavailable; existing requests polled for completion |
| **DuckDNS** | DNS for proxy | Webhook delivery fails; background poller ensures fiat completion |
