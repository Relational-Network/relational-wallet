---
layout: default
title: Security Model
parent: Architecture
nav_order: 2
---

# Security Model
{: .fs-7 }

Defense-in-depth security across hardware isolation, authentication, authorization, encrypted storage, and audit logging.
{: .fs-5 .fw-300 }

---

## Trust Boundary

The SGX enclave defines the trust boundary. Everything outside the enclave --- the host OS, hypervisor, server operator, and network --- is untrusted.

```
┌─────────────── UNTRUSTED ───────────────────────────────────────────────┐
│                                                                         │
│  Host OS    Hypervisor    Server Admin    Network    Physical Access     │
│                                                                         │
│  ┌───────────────────── TRUSTED (SGX Enclave) ────────────────────────┐ │
│  │                                                                     │ │
│  │  • Key generation (secp256k1)                                       │ │
│  │  • Transaction signing                                              │ │
│  │  • JWT verification (JWKS)                                          │ │
│  │  • Data encryption/decryption                                       │ │
│  │  • Business logic (all API handlers)                                │ │
│  │  • Audit log writing                                                │ │
│  │  • Cross-instance VOPRF discovery (RA-TLS mutual auth)              │ │
│  │                                                                     │ │
│  │  Storage: Gramine encrypted FS (sealed to enclave identity)         │ │
│  │  Network: RA-TLS (attestation evidence in TLS certificates)         │ │
│  │                                                                     │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### What the enclave protects against

| Threat | Protection |
|:-------|:-----------|
| Malicious host OS | SGX memory encryption; host cannot read enclave memory |
| Server operator | Keys sealed to enclave identity; operator cannot extract keys |
| Physical disk access | Gramine encrypted FS; data unreadable without enclave |
| Man-in-the-middle | RA-TLS; clients verify enclave identity before connecting |
| Binary tampering | MRENCLAVE measurement; any code change produces a different hash |
| Replay attacks | DCAP attestation includes freshness evidence |

---

## Authentication

All `/v1/*` endpoints require a valid Clerk JWT in the `Authorization: Bearer <token>` header.

### JWT Verification Flow

```
Incoming request
  │
  ▼
Extract Authorization: Bearer <token>
  │
  ▼
Fetch JWKS from Clerk (cached with 2× TTL grace period)
  │
  ▼
Verify signature (RS256 / RS384 / RS512 / ES256)
  │
  ▼
Validate claims:
  • iss == CLERK_ISSUER
  • aud == CLERK_AUDIENCE (if configured)
  • exp > now - 60s (clock skew tolerance)
  │
  ▼
Extract identity:
  • user_id from "sub" claim
  • role from "publicMetadata.role" (default: "client")
  • session_id from "sid" claim
  │
  ▼
Attach AuthenticatedUser to request context
```

### Auth Modes

| Mode | Condition | Behavior |
|:-----|:----------|:---------|
| **Production** | `CLERK_JWKS_URL` configured | Full cryptographic signature + claims verification |
| **Development** | `dev` feature flag, no JWKS URL | Token decoded and shape-checked, signature not verified |

If `CLERK_AUDIENCE` is not set, audience validation is skipped with a startup warning. This is acceptable for development but should be configured in production.
{: .warning }

### JWKS Caching

The backend caches Clerk's JWKS response to avoid blocking on network requests:

- **Normal TTL**: Based on Clerk's `Cache-Control` header
- **Grace period**: 2x the normal TTL
- **During grace**: Cached keys are used while a background refresh is attempted
- **After grace expiry**: Authentication fails closed (requests rejected)

This ensures that a temporary Clerk outage does not immediately lock out all users.

---

## Authorization

### Role Hierarchy

| Role | Access Level |
|:-----|:-------------|
| **Admin** | Full access: all endpoints including `/v1/admin/*` |
| **Client** | Standard access: own wallets, transactions, bookmarks, fiat requests |
| **Support** | Parsed but no dedicated endpoints yet |
| **Auditor** | Parsed but no dedicated endpoints yet |

Roles are assigned via Clerk's `publicMetadata.role` field. If no role is specified, the user defaults to `client`.

### Ownership Enforcement

Every wallet-scoped operation verifies that the authenticated user owns the resource:

```
Request: DELETE /v1/wallets/{wallet_id}
  │
  ▼
Load wallet metadata from /data/wallets/{wallet_id}/meta.json
  │
  ▼
Check: meta.user_id == authenticated_user.user_id
  │
  ├── Match → proceed with operation
  └── No match → 403 Forbidden
```

This applies to:
- Wallet read/delete
- Balance queries
- Transaction send/history
- Bookmark CRUD
- Fiat request creation/queries

### Axum Extractors

| Extractor | Purpose |
|:----------|:--------|
| `Auth` | Requires valid JWT; extracts `AuthenticatedUser` |
| `AdminOnly` | Requires valid JWT + `admin` role |
| `OptionalAuth` | Accepts unauthenticated requests (e.g., payment link resolution) |

---

## Encrypted Storage

All persistent data lives in Gramine's encrypted filesystem mounted at `/data`.

### Encryption Key Derivation

| Environment | Key Source | Properties |
|:------------|:----------|:-----------|
| **Development** | `data/.dev_storage_key` (persistent 16-byte file) | Survives rebuilds; deterministic for dev workflow |
| **Production** | Derived from `_sgx_mrsigner` (enclave signer identity) | Sealed to enclave; changes if signing key changes |

### Security Properties

- **At-rest encryption**: All files under `/data` are encrypted by Gramine before writing to disk
- **No plaintext leakage**: Even with physical disk access, data is unreadable without the enclave
- **Key isolation**: Encryption keys never leave the enclave boundary
- **Crash consistency**: Gramine's encrypted FS provides atomic file operations

### Private Key Storage

Wallet private keys are stored as PEM files:

```
/data/wallets/{wallet_id}/key.pem   # Encrypted by Gramine's sealed FS
```

- Keys are generated inside the enclave using `k256` (secp256k1)
- Keys are used for signing but **never returned via any API endpoint**
- The only operation that accesses a private key is transaction signing (`POST /v1/wallets/{id}/send`)

---

## Audit Logging

All security-relevant operations are logged to append-only JSONL files:

```
/data/audit/{YYYY-MM-DD}/events.jsonl
```

### Logged Events

| Event Type | Trigger |
|:-----------|:--------|
| `wallet_created` | New wallet generated |
| `wallet_deleted` | Wallet soft-deleted |
| `wallet_accessed` | Wallet metadata read |
| `transaction_signed` | Transaction signed inside enclave |
| `transaction_broadcast` | Transaction sent to chain |
| `bookmark_created` | Bookmark added |
| `bookmark_deleted` | Bookmark removed |
| `auth_success` | Successful JWT verification |
| `auth_failure` | Failed JWT verification |
| `permission_denied` | Unauthorized access attempt |
| `admin_access` | Admin endpoint accessed |
| `config_changed` | Configuration modification |
| `fiat_on_ramp_requested` | Fiat deposit initiated |
| `fiat_off_ramp_requested` | Fiat withdrawal initiated |

### Audit Event Structure

```json
{
  "event_id": "evt_abc123",
  "timestamp": "2026-03-15T10:30:00Z",
  "event_type": "transaction_signed",
  "success": true,
  "user_id": "user_xyz",
  "resource_type": "wallet",
  "resource_id": "wal_456",
  "details": "Signed transfer of 1.5 AVAX",
  "ip_address": "192.168.1.1"
}
```

### Querying Audit Logs

Admins can query audit logs via the API:

```bash
curl -k -H "Authorization: Bearer $JWT" \
  "https://localhost:8080/v1/admin/audit/events?start_date=2026-03-01&event_type=transaction_signed&limit=50"
```

---

## Network Security

| Layer | Protection |
|:------|:-----------|
| **Transport** | HTTPS only; no HTTP fallback. RA-TLS mandatory at startup. |
| **CORS** | Configurable via `CORS_ALLOWED_ORIGINS` (permissive if unset). |
| **TLS certificates** | RA-TLS with DCAP attestation evidence for enclave verification. |
| **External proxy** | Nginx with Let's Encrypt for webhook ingress (rate limited). |
| **Request tracing** | `x-request-id` propagated across proxy and backend for diagnostics. |

---

## Production Checklist

- [ ] Set `CLERK_JWKS_URL` and `CLERK_ISSUER` to production values
- [ ] Set `CLERK_AUDIENCE` to restrict JWT acceptance
- [ ] Restrict CORS with `CORS_ALLOWED_ORIGINS`
- [ ] Use `sgx.debug = false` in production manifest (Docker default)
- [ ] Keep signing keys and provider secrets outside source control
- [ ] Remove `NODE_TLS_REJECT_UNAUTHORIZED=0` from frontend environment
- [ ] Configure `TRUELAYER_WEBHOOK_SHARED_SECRET` for webhook HMAC validation
- [ ] Set `LOG_FORMAT=json` for structured log ingestion
- [ ] Verify MRENCLAVE matches `measurements.toml` before deployment
- [ ] Set up monitoring on `/health/ready` endpoint
