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
- **Cryptography**: P256 keypairs for wallet addresses

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

### 3. Avalanche Settlement (Planned)

On-chain ledger for stablecoin transfers:
- Transaction signing via enclave
- Balance queries
- Payment execution

## Data Flow

### Wallet Creation

```
1. User clicks "Create Wallet" in browser
2. Browser calls /api/proxy/v1/wallets (POST)
3. Next.js proxy adds Clerk JWT, forwards to enclave
4. Enclave verifies JWT via JWKS
5. Enclave generates P256 keypair inside SGX
6. Wallet data encrypted and saved to /data
7. Public address returned to user
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
│   │   │   ├── router.rs     # API routes
│   │   │   ├── auth/         # JWT verification
│   │   │   ├── handlers/     # Request handlers
│   │   │   └── storage/      # Encrypted persistence
│   │   ├── rust-server.manifest.template
│   │   └── Makefile
│   └── wallet-web/           # Next.js frontend
│       ├── src/
│       │   ├── app/          # App Router pages
│       │   │   └── api/proxy/  # Backend proxy
│       │   ├── components/   # React components
│       │   └── lib/          # API client
│       └── package.json
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
