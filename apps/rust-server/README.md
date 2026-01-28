# Rust-Server WIP

Lightweight Axum backend for bookmarks, invites, recurring payments, and wallet utilities. Uses an in-memory store for now; swap the store for a persistent backend as needed.

**Security**: This server runs inside an Intel SGX enclave with **DCAP remote attestation** and **RA-TLS** for HTTPS. TLS certificates are generated at runtime with embedded attestation evidence.

## What's included
- Bookmarks: list, create, delete.
- Invites: fetch by code, redeem.
- Recurring payments: list, create, update, delete, fetch payments due today, update last paid date.
- Wallet autofund: placeholder to autofund wallets (DEV).
- Swagger/OpenAPI: JSON at `/api-doc/openapi.json`, Swagger UI at `/docs`.
- Tests: async unit tests covering bookmark CRUD and invite fetch/redeem flows.
- **DCAP RA-TLS**: TLS certificates with SGX attestation evidence for remote verification.

## Quick start (Development)
```bash
cargo build --release
cargo test
```

> **Note**: The server requires RA-TLS credentials (`/tmp/ra-tls.crt.pem`, `/tmp/ra-tls.key.pem`) which are only generated inside the SGX enclave by `gramine-ratls`. Running `cargo run` outside SGX will fail.

## Testing
```bash
cargo test
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
3. Server starts HTTPS on port 8080

Notes:
- **No non-SGX mode** — DCAP attestation requires SGX hardware
- `sgx.debug = true` in manifest = debug mode (change to `false` for production)
- TLS is mandatory — server will not start without valid RA-TLS credentials

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
- `HOST`/`PORT`: override bind address (defaults to 0.0.0.0:8080).
- `SEED_INVITE_CODE`: seed a single invite code into the in-memory store at startup.

## Route map (all prefixed with /v1, HTTPS only)
- `GET  /v1/bookmarks?wallet_id=...`
- `POST /v1/bookmarks` *(body: wallet_id, name, address)*
- `DELETE /v1/bookmarks/{bookmark_id}`
- `GET  /v1/invite?invite_code=...`
- `POST /v1/invite/redeem` *(body: invite_id)*
- `POST /v1/wallet/autofund` *(body: wallet_id; currently a stub)*
- `GET  /v1/recurring/payments?wallet_id=...`
- `POST /v1/recurring/payment`
- `PUT  /v1/recurring/payment/{recurring_payment_id}`
- `DELETE /v1/recurring/payment/{recurring_payment_id}`
- `GET  /v1/recurring/payments/today`
- `PUT  /v1/recurring/payment/{recurring_payment_id}/last-paid-date`

## Project layout
- `src/main.rs` – HTTPS server startup with RA-TLS, state wiring, OpenAPI registration.
- `src/tls.rs` – RA-TLS certificate loading and PEM normalization for rustls.
- `src/models.rs` – request/response structs shared across APIs.
- `src/store.rs` – in-memory store; update to use external DB.
- `src/api/` – handlers grouped by domain (bookmarks, invites, recurring, wallet).

## RA-TLS Certificate Verification
Clients can verify the server's attestation by:
1. Connecting to the HTTPS endpoint
2. Extracting the X.509 certificate
3. Using Gramine's RA-TLS verification library to validate the embedded DCAP quote

## Notes on autofund
The `/v1/wallet/autofund` handler currently records the request in memory and returns `200 OK`. 
> TODO Avalanche funding implementation
