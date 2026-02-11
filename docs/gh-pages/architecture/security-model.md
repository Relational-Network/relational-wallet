---
layout: default
title: Security Model
parent: Architecture
nav_order: 2
---

# Security Model

This document describes the security architecture of the Relational Wallet service, including key management, authentication, and data protection.

## Trusted Execution Environment (TEE)

The wallet service runs inside an **Intel SGX enclave** using the [Gramine LibOS](https://gramine.readthedocs.io/).

### SGX Protection Guarantees

1. **Memory Isolation**: Enclave memory is encrypted and protected from the host OS, hypervisor, and other processes.
2. **Code Integrity**: The enclave binary is measured (MRENCLAVE/MRSIGNER) and verified before execution.
3. **Remote Attestation**: Clients can verify they're communicating with genuine SGX hardware via DCAP attestation.

### RA-TLS (Remote Attestation TLS)

All network communication uses **RA-TLS**, which embeds SGX attestation evidence in the TLS certificate:

```
┌─────────────────┐         ┌──────────────────────┐
│   Client        │  HTTPS  │   SGX Enclave        │
│                 │────────▶│   (Gramine)          │
│  Verifies:      │         │                      │
│  - TLS cert     │         │  RA-TLS cert with:   │
│  - DCAP quote   │         │  - MRENCLAVE         │
│  - MRSIGNER     │         │  - MRSIGNER          │
└─────────────────┘         └──────────────────────┘
```

- No HTTP fallback - TLS is mandatory
- Certificates generated at runtime by `gramine-ratls`
- Clients can verify the SGX quote embedded in the certificate

## Authentication

### Clerk JWT Authentication

All protected endpoints require a valid JWT from [Clerk](https://clerk.dev):

```
Authorization: Bearer <jwt_token>
```

### JWT Verification Flow

1. Extract token from `Authorization` header
2. Fetch public keys from Clerk JWKS endpoint (cached for 5 minutes)
3. Verify signature using RS256/RS384/RS512/ES256
4. Validate claims:
   - **iss** (issuer): Must match `CLERK_ISSUER`
   - **aud** (audience): Must match `CLERK_AUDIENCE` (if configured)
   - **exp** (expiration): Must be in the future (60s clock skew tolerance)
5. Extract user ID from `sub` claim
6. Extract role from `publicMetadata.role`

### Authentication Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Production** | Full JWKS verification | When `CLERK_JWKS_URL` is set |
| **Development** | Structure validation only | Testing without Clerk |

### Role-Based Access Control

| Role | Privileges | Endpoints |
|------|------------|-----------|
| **Admin** | Full access to all data | `/v1/admin/*` |
| **Support** | Read-only metadata access | (planned) |
| **Auditor** | Read-only audit access | (planned) |
| **Client** | Own resources only | `/v1/wallets`, `/v1/bookmarks`, etc. |

Roles are hierarchical: Admin > Support > Auditor > Client

## Data Protection

### Encrypted Storage

All persistent data is stored under `/data`, which is mounted as a Gramine encrypted filesystem:

```toml
# Gramine manifest configuration
[[fs.mounts]]
type = "encrypted"
path = "/data"
key_name = "_sgx_mrsigner"
```

- **Encryption Key**: Derived from MRSIGNER (enclave identity)
- **Algorithm**: AES-GCM (Gramine default)
- **Transparency**: Application uses normal file I/O; Gramine handles encryption

### Network Access (Blockchain RPC)

The enclave needs network access to query blockchain state:

```toml
# Gramine manifest - DNS resolution for RPC calls
sgx.allowed_files = [
    "file:/etc/resolv.conf",
    "file:/etc/hosts",
    "file:/etc/nsswitch.conf",
]
```

- Allows querying Avalanche C-Chain RPC endpoints
- Required for balance queries and transaction broadcasting
- Files are read-only from host (not security-sensitive)

### Storage Layout

```
/data/
├── wallets/{id}/
│   ├── meta.json          # WalletMetadata (owner, status, address)
│   └── key.pem            # Private key (NEVER exposed via API)
├── bookmarks/{id}.json
├── invites/{id}.json
├── recurring/{id}.json
└── audit/{date}/
    └── events.jsonl       # Append-only audit log
```

### Key Security Rules

1. **Private keys are generated using secp256k1** (Ethereum/Avalanche compatible)
2. **Ethereum addresses derived via keccak256** (standard derivation)
3. **Private keys never leave the enclave unencrypted**
4. **No API endpoint exposes private keys**
5. **All file I/O goes through Gramine encrypted FS**
6. **SGX protects memory from host inspection**

## Ownership Enforcement

Every wallet is bound to a user ID:

```rust
pub struct WalletMetadata {
    pub id: String,
    pub owner_user_id: String,  // Enforced on all operations
    pub public_address: String,
    pub status: WalletStatus,
    pub created_at: DateTime<Utc>,
    // ...
}
```

- **403 Forbidden** returned if user tries to access another user's wallet
- Admin role can bypass ownership for management operations
- Ownership checks are performed at the repository layer

## Audit Logging

All sensitive operations are logged to an append-only audit trail:

```json
{
  "id": "uuid",
  "timestamp": "2024-01-15T10:30:00Z",
  "user_id": "user_123",
  "event_type": "WalletCreated",
  "resource_id": "wallet_456",
  "resource_type": "Wallet",
  "details": {"public_address": "0x..."},
  "success": true,
  "ip_address": null,
  "user_agent": null
}
```

Audit events are stored in encrypted files under `/data/audit/`.

## Request Tracing

Each HTTP request receives a unique `x-request-id` for correlation:

- Set via `X-Request-Id` header (or auto-generated UUID)
- Propagated in responses
- Included in structured log entries

Example log entry:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "target": "http_request",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "GET",
  "uri": "/v1/wallets",
  "status": 200,
  "latency_ms": 15
}
```

## Production Checklist

Before deploying to production, verify:

- [ ] `CLERK_JWKS_URL` is set (enables JWKS verification)
- [ ] `CLERK_ISSUER` is set (enables issuer validation)
- [ ] `sgx.debug = false` in Gramine manifest
- [ ] `/data` is mounted with `type = "encrypted"`
- [ ] DCAP attestation infrastructure is configured
- [ ] Enclave signing key is securely stored
- [ ] Rate limiting is in place (recommended)
