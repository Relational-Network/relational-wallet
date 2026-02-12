---
layout: default
title: System Overview
parent: Architecture
nav_order: 1
---

# System Overview

The Relational Wallet is a secure wallet system designed for managing cryptographic keys and signing transactions. All sensitive operations run inside an Intel SGX enclave with DCAP remote attestation.

## Components

### 1. Wallet Enclave (rust-server)

The core backend running inside Gramine SGX:

- **Framework**: Axum async web framework
- **Security**: DCAP RA-TLS for encrypted connections with attestation
- **Storage**: Encrypted file-based persistence in `/data`
- **Authentication**: Clerk JWT verification with JWKS
- **Cryptography**: secp256k1 keypairs (Ethereum/Avalanche compatible)
- **Blockchain**: Avalanche C-Chain integration via alloy

**Key Features**:
- Health checks (liveness, readiness, component status)
- Structured logging with request correlation IDs
- Role-based access control (client/admin)
- Audit logging for all operations

### 2. Wallet Web (wallet-web)

Browser client built with Next.js 16:

- **Framework**: Next.js App Router
- **Authentication**: Clerk for user management
- **API Integration**: Server-side proxy to handle RA-TLS certificates
- **Type Safety**: OpenAPI-generated TypeScript types

**Architecture Pattern**:
```
Browser → /api/proxy/* → Next.js Server → SGX Enclave
```

The proxy pattern is necessary because browsers reject self-signed RA-TLS certificates. The Next.js server-side route can accept the certificate and forward requests.

### 3. Avalanche C-Chain Integration

On-chain ledger for AVAX and stablecoin transfers:
- **Balance queries**: Native AVAX and ERC-20 tokens (USDC)
- **Transaction signing**: Sign transactions with enclave-held keys
- **Transfer execution**: Send AVAX and USDC via Fuji testnet or mainnet
- **Gas estimation**: Dynamic RPC-based estimation with optional override

### 4. Contracts Workspace (`apps/contracts`)

Foundry-based smart contract workspace for the Euro stablecoin:

- **Contract**: `RelationalEuro (rEUR)`
- **Controls**: `mint`, `burn`, `pause/unpause`, role-based access (`AccessControl`)
- **Tests**: Forge test suite under `apps/contracts/test/`
- **Deployment**: `apps/contracts/script/DeployFuji.s.sol`

## Data Flow

### Wallet Creation

```
1. User clicks "Create Wallet" in browser
2. Browser calls /api/proxy/v1/wallets (POST)
3. Next.js proxy adds Clerk JWT, forwards to enclave
4. Enclave verifies JWT via JWKS
5. Enclave generates secp256k1 keypair inside SGX
6. Wallet data encrypted and saved to /data
7. Ethereum-compatible address returned to user
```

### Authentication Flow

```
1. User signs in via Clerk
2. Clerk issues JWT with user_id claim
3. Frontend stores JWT in session
4. API requests include Authorization: Bearer <jwt>
5. Enclave fetches JWKS from Clerk
6. Enclave verifies JWT signature with RS256
7. User identity extracted from claims
```

## Directory Structure

```
relational-wallet/
├── apps/
│   ├── rust-server/          # Enclave backend
│   │   ├── src/
│   │   │   ├── main.rs       # Entry point
│   │   │   ├── config.rs     # Configuration
│   │   │   ├── state.rs      # Application state
│   │   │   ├── api/          # HTTP handlers
│   │   │   │   ├── mod.rs    # Router setup
│   │   │   │   ├── wallets.rs
│   │   │   │   ├── balance.rs
│   │   │   │   ├── transactions.rs
│   │   │   │   └── admin.rs
│   │   │   ├── auth/         # JWT verification
│   │   │   ├── blockchain/   # Avalanche integration
│   │   │   │   ├── client.rs # RPC client
│   │   │   │   ├── erc20.rs  # Token interface
│   │   │   │   ├── signing.rs
│   │   │   │   └── transactions.rs
│   │   │   └── storage/      # Encrypted persistence
│   │   ├── rust-server.manifest.template
│   │   └── Makefile
│   └── wallet-web/           # Next.js frontend
│       ├── src/
│       │   ├── app/          # App Router pages
│       │   │   ├── wallets/  # Wallet pages
│       │   │   └── api/proxy/  # Backend proxy
│       │   ├── components/   # React components
│       │   └── lib/          # API client
│       └── package.json
│   └── contracts/            # Foundry smart contracts workspace
│       ├── src/              # Solidity contracts
│       ├── script/           # Deployment scripts
│       ├── test/             # Forge tests
│       ├── foundry.toml
│       └── README.md
├── docs/
│   ├── gh-pages/             # This documentation
│   └── architecture/         # PlantUML diagrams
└── scripts/                  # Build utilities
```

## Security Boundaries

| Boundary | Protection |
|----------|------------|
| Network → Enclave | DCAP RA-TLS encryption |
| Enclave → Storage | AES encryption |
| User → API | Clerk JWT authentication |
| JWT → Enclave | JWKS signature verification |
| Admin → System | Role-based access control |
