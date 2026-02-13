# Relational Wallet - Rust Server

**Custodial Avalanche wallet service** running inside Intel SGX enclave with Gramine. All private keys and sensitive data are encrypted at rest using Gramine's sealed storage.

**Security Model**:
- Runs inside Intel SGX enclave with **DCAP remote attestation**
- **RA-TLS** for HTTPS with embedded attestation evidence
- **Clerk JWT authentication** for user identity
- **Gramine encrypted filesystem** at `/data` (sealed to enclave)
- Private keys never leave the enclave unencrypted

## What's Included

### Core Features
- **Wallet Management**: Create, list, delete wallets with secp256k1 key generation inside SGX (Ethereum/Avalanche compatible)
- **Bookmarks**: Address book per wallet with ownership enforcement
- **Invites**: Invite codes with expiration and redemption tracking
- **Recurring Payments**: Scheduled payment configuration

### Security Features
- **Clerk JWT Auth**: All endpoints require valid Bearer token
- **Role-Based Access**: Admin, Client, Support, Auditor roles
- **Ownership Enforcement**: Users can only access their own wallets
- **Encrypted Storage**: All data encrypted at rest with SGX sealing key
- **DCAP RA-TLS**: TLS certificates with attestation evidence

### API & Documentation
- Swagger UI: `https://localhost:8080/docs`
- OpenAPI JSON: `https://localhost:8080/api-doc/openapi.json`

## Quick Start (Development)

```bash
cargo build --release
cargo test
```

> **Note**: The server requires RA-TLS credentials (`/tmp/ra-tls.crt.pem`, `/tmp/ra-tls.key.pem`) which are only generated inside the SGX enclave by `gramine-ratls`. Running `cargo run` outside SGX will fail.

## Testing

### Unit Tests
```bash
cargo test
```

### Testing with JWT Authentication

The server uses [Clerk](https://clerk.com) for authentication. To test protected endpoints:

#### 1. Get a JWT Token from Clerk

**Option A: Using Clerk Dashboard (Development)**
1. Go to your Clerk Dashboard → Users → Select a user
2. Click "Sessions" → Create a session token
3. Copy the JWT token

**Option B: Using Clerk Frontend SDK**
```javascript
// In your frontend app
import { useAuth } from '@clerk/nextjs';

const { getToken } = useAuth();
const token = await getToken();
console.log(token); // Use this JWT
```

**Option C: Using Clerk Backend SDK**
```javascript
// Node.js script
import { clerkClient } from '@clerk/clerk-sdk-node';

const token = await clerkClient.signInTokens.createSignInToken({
  userId: 'user_xxx',
  expiresInSeconds: 3600
});
```

#### 2. Make Authenticated Requests

```bash
# Set your JWT token
export JWT="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# Test the /v1/users/me endpoint
curl -k -H "Authorization: Bearer $JWT" https://localhost:8080/v1/users/me

# Create a wallet
curl -k -X POST -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  https://localhost:8080/v1/wallets

# List your wallets
curl -k -H "Authorization: Bearer $JWT" https://localhost:8080/v1/wallets
```

#### 3. Required Clerk Configuration

Set these environment variables:
```bash
export CLERK_JWKS_URL="https://your-instance.clerk.accounts.dev/.well-known/jwks.json"
export CLERK_ISSUER="https://your-instance.clerk.accounts.dev"
```

#### 4. Role Assignment (Optional)

To assign roles, set `public_metadata.role` in Clerk:
```json
{
  "role": "admin"  // or "client", "support", "auditor"
}
```

## Coverage
- Local quick check: `cargo tarpaulin --ignore-tests` (install with `cargo install cargo-tarpaulin`).
- HTML report: `cargo tarpaulin --out Html && open tarpaulin-report.html`.
Run after meaningful changes or wire into CI to block merges on coverage drops.

## Run using Gramine (Intel SGX)

### Requirements (one-time)
- Install Gramine packages with DCAP support: [Installation Guide](https://gramine.readthedocs.io/en/stable/installation.html#install-gramine-packages-1)
- Install `gramine-ratls-dcap` package for RA-TLS
- Create a signing key: [Quickstart](https://gramine.readthedocs.io/en/stable/quickstart.html#prepare-a-signing-key)
```sh
gramine-sgx-gen-private-key
```
- Install Rust: https://www.rust-lang.org/tools/install
- Configure DCAP infrastructure (PCCS)

### Build and Run (SGX-only)
```bash
make                    # Build for SGX (generates .sig and .manifest.sgx)
make start-rust-server  # Run inside SGX enclave with DCAP RA-TLS
```

At startup:
1. `gramine-ratls` generates TLS cert/key with DCAP attestation evidence
2. Rust server loads the RA-TLS credentials
3. Encrypted storage is initialized at `/data`
4. Server starts HTTPS on port 8080

Notes:
- **No non-SGX mode** — DCAP attestation requires SGX hardware
- `sgx.debug` in manifest is parameterized: `true` for local dev (`make`), `false` for production (`docker-build`)
- TLS is mandatory — server will not start without valid RA-TLS credentials
- **Encrypted /data** — Sealed to enclave MRSIGNER, survives restarts

## Docker (SGX with DCAP)
Build and run the SGX-enabled container (Ubuntu 20.04):

```bash
make docker-build
make docker-run
make docker-stop
```

The container:
- Serves HTTPS on `0.0.0.0:8080` (port 8080 published)
- Uses your host SGX signing key from `$HOME/.config/gramine/enclave-key.pem`
- Generates RA-TLS certificates with DCAP attestation at startup

See [docker/README.md](docker/README.md) for DCAP configuration details.

## Configuration

### Required Environment Variables
| Variable | Purpose | Example |
|----------|---------|---------|
| `CLERK_JWKS_URL` | Clerk JWKS endpoint | `https://xxx.clerk.accounts.dev/.well-known/jwks.json` |
| `CLERK_ISSUER` | Clerk issuer URL | `https://xxx.clerk.accounts.dev` |

### Optional Environment Variables
| Variable | Purpose | Default |
|----------|---------|---------|
| `HOST` | Bind address | `0.0.0.0` |
| `PORT` | Bind port | `8080` |
| `CLERK_AUDIENCE` | Expected JWT audience | — |
| `SEED_INVITE_CODE` | Pre-seed invite code | — |
| `DATA_DIR` | Encrypted data directory | `/data` |
| `TRUELAYER_CLIENT_ID` | TrueLayer OAuth client id (sandbox/prod) | — |
| `TRUELAYER_CLIENT_SECRET` | TrueLayer OAuth client secret | — |
| `TRUELAYER_SIGNING_KEY_ID` | TrueLayer signing key id | — |
| `TRUELAYER_SIGNING_PRIVATE_KEY_PATH` | Path to TrueLayer private key PEM | — |
| `TRUELAYER_MERCHANT_ACCOUNT_ID` | TrueLayer merchant account id | — |
| `REUR_CONTRACT_ADDRESS_FUJI` | Fuji `rEUR` token contract used for settlement | `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63` |
| `FIAT_RESERVE_BOOTSTRAP_ENABLED` | Auto-bootstrap enclave reserve wallet on startup | `true` |
| `FIAT_RESERVE_INITIAL_TOPUP_EUR` | Default top-up amount when admin top-up body omits amount | `1000000.00` |
| `FIAT_MIN_CONFIRMATIONS` | Required confirmations for off-ramp deposit detection | `1` |
| `TRUELAYER_WEBHOOK_SHARED_SECRET` | Enables webhook endpoint and verifies webhook secret header | — |

TrueLayer note: OAuth client credentials must be granted the `payments` scope. On-ramp return URL is hardcoded to `http://localhost:3000/callback` and must be allow-listed in TrueLayer Console. Off-ramp beneficiary account holder + IBAN are supplied per API request from the frontend dialog. If `TRUELAYER_WEBHOOK_SHARED_SECRET` is unset, webhook ingestion is disabled (`POST /v1/fiat/providers/truelayer/webhook` returns `503`) and request syncing falls back to polling on fiat list/detail endpoints.

## Route Map (all prefixed with /v1, HTTPS only)

### Authentication
- `GET  /v1/users/me` — Get current authenticated user info

### Wallets (Protected)
- `POST /v1/wallets` — Create new wallet (generates secp256k1 keypair, derives Ethereum address)
- `GET  /v1/wallets` — List user's wallets
- `GET  /v1/wallets/{id}` — Get wallet details
- `DELETE /v1/wallets/{id}` — Soft-delete wallet

### Balance (Protected)
- `GET  /v1/wallets/{id}/balance` — Get on-chain balance (native AVAX + ERC-20 tokens)
- `GET  /v1/wallets/{id}/balance/native` — Get native AVAX balance only (faster)

Query parameters: `network=fuji` only. `mainnet` is rejected.

### Bookmarks (Protected)
- `GET  /v1/bookmarks?wallet_id=...` — List bookmarks for wallet
- `POST /v1/bookmarks` — Create bookmark *(body: wallet_id, name, address)*
- `DELETE /v1/bookmarks/{bookmark_id}` — Delete bookmark

### Invites (Protected)
- `GET  /v1/invite?invite_code=...` — Get invite by code
- `POST /v1/invite/redeem` — Redeem invite *(body: invite_id)*

### Recurring Payments (Protected)
- `GET  /v1/recurring/payments?wallet_id=...` — List payments
- `POST /v1/recurring/payment` — Create payment
- `PUT  /v1/recurring/payment/{id}` — Update payment
- `DELETE /v1/recurring/payment/{id}` — Delete payment
- `GET  /v1/recurring/payments/today` — Get payments due today
- `PUT  /v1/recurring/payment/{id}/last-paid-date` — Update last paid

### Fiat (Protected)
- `GET  /v1/fiat/providers` — List configured fiat providers and capabilities
- `POST /v1/fiat/providers/truelayer/webhook` — TrueLayer webhook callback (enabled only when webhook secret is configured)
- `POST /v1/fiat/onramp/requests` — Create on-ramp request (live TrueLayer sandbox call)
- `POST /v1/fiat/offramp/requests` — Create off-ramp request (live TrueLayer sandbox call)
- `GET  /v1/fiat/requests` — List fiat requests (wallet filter optional)
- `GET  /v1/fiat/requests/{request_id}` — Get fiat request details

### Admin (Admin Role Required)
- `GET  /v1/admin/stats` — System statistics (wallet counts, invite usage, uptime)
- `GET  /v1/admin/wallets` — List all wallets across all users
- `GET  /v1/admin/users` — List all users with resource counts
- `GET  /v1/admin/audit/events` — Query audit logs (with date range and filters)
- `GET  /v1/admin/health` — Detailed health status with storage metrics
- `POST /v1/admin/wallets/{id}/suspend` — Suspend a wallet
- `POST /v1/admin/wallets/{id}/activate` — Reactivate a suspended wallet
- `GET  /v1/admin/fiat/service-wallet` — Get enclave reserve wallet status/address
- `POST /v1/admin/fiat/service-wallet/bootstrap` — Idempotently bootstrap enclave reserve wallet
- `POST /v1/admin/fiat/reserve/topup` — Mint `rEUR` into reserve wallet
- `POST /v1/admin/fiat/reserve/transfer` — Transfer `rEUR` from reserve wallet
- `POST /v1/admin/fiat/requests/{request_id}/sync` — Force sync of a fiat request (provider/chain)

## Project Layout
```
src/
├── main.rs          # HTTPS server startup with RA-TLS
├── tls.rs           # RA-TLS certificate loading
├── state.rs         # AppState with encrypted storage
├── models.rs        # Request/response structs
├── error.rs         # API error types
├── api/             # Handlers grouped by domain
│   ├── mod.rs       # Router composition + OpenAPI
│   ├── admin.rs     # Admin-only endpoints (stats, audit, wallet mgmt)
│   ├── bookmarks.rs
│   ├── invites.rs
│   ├── recurring.rs
│   ├── users.rs     # /v1/users/me endpoint
│   └── wallets.rs   # Wallet lifecycle endpoints
├── auth/            # Clerk JWT authentication
│   ├── mod.rs       # Module exports
│   ├── claims.rs    # ClerkClaims, AuthenticatedUser
│   ├── error.rs     # AuthError with HTTP status codes
│   ├── extractor.rs # Auth, AdminOnly, OptionalAuth extractors
│   ├── jwks.rs      # JWKS fetching with caching
│   ├── middleware.rs# Auth middleware
│   └── roles.rs     # Role enum (Admin, Client, Support, Auditor)
└── storage/         # Encrypted storage (Gramine sealed)
    ├── mod.rs       # Module exports
    ├── audit.rs     # Audit event logging and querying
    ├── paths.rs     # StoragePaths for /data layout
    ├── encrypted_fs.rs # EncryptedStorage operations
    └── repository/  # Domain repositories
        ├── bookmarks.rs
        ├── invites.rs
        ├── recurring.rs
        └── wallets.rs
```

## RA-TLS Certificate Verification
Clients can verify the server's attestation by:
1. Connecting to the HTTPS endpoint
2. Extracting the X.509 certificate
3. Using Gramine's RA-TLS verification library to validate the embedded DCAP quote

## Security Architecture

### Encrypted Storage
- All data stored in `/data` directory
- Gramine encrypts using `_sgx_mrsigner` key (sealed to signer)
- Survives enclave restarts, bound to signing key
- Directory structure:
  ```
  /data/
  ├── wallets/{wallet_id}/
  │   ├── meta.json
  │   └── key.pem (encrypted)
  ├── bookmarks/{bookmark_id}.json
  ├── invites/{invite_id}.json
  ├── recurring/{payment_id}.json
  ├── fiat/{request_id}.json
  ├── system/fiat_service_wallet/
  │   ├── meta.json
  │   └── key.pem (encrypted)
  └── audit/{date}/events.jsonl
  ```

### Ownership Model
- Every wallet is bound to a `user_id` from Clerk
- All operations verify ownership before access
- Private keys accessible only through signing operations
- No export of private keys, ever
