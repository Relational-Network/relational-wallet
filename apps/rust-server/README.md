# Rust-Server WIP

Lightweight Axum backend for bookmarks, invites, recurring payments, and wallet utilities. Uses an in-memory store for now; swap the store for a persistent backend as needed.

## What’s included
- Bookmarks: list, create, delete.
- Invites: fetch by code, redeem.
- Recurring payments: list, create, update, delete, fetch payments due today, update last paid date.
- Wallet autofund: placeholder to autofund wallets (DEV).
- Swagger/OpenAPI: JSON at `/api-doc/openapi.json`, Swagger UI at `/docs`.
- Tests: async unit tests covering bookmark CRUD and invite fetch/redeem flows.

## Quick start
```bash
cargo build
cargo run
# server listens on 127.0.0.1:8080 by default; set PORT to override
# optional: SEED_INVITE_CODE=WELCOME cargo run   # preload a single invite for testing
```

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
- Install Gramine packages: [Installation Guide](https://gramine.readthedocs.io/en/stable/installation.html#install-gramine-packages-1)
- Create a signing key (SGX only): [Quickstart](https://gramine.readthedocs.io/en/stable/quickstart.html#prepare-a-signing-key)
	```sh
	gramine-sgx-gen-private-key
	```
- Install Rust: https://www.rust-lang.org/tools/install

### Gramine (build with Makefile, then run)
Use Gramine to validate manifest/runtime, and optionally run inside SGX.

Build:
```bash
make            # non-SGX
make SGX=1      # SGX (generates .sig and .manifest.sgx)
```

Run:
```bash
make start-wallet           # Gramine Direct (non-SGX)
make SGX=1 start-wallet     # Gramine SGX
```

Notes:
- Gramine Direct runs without SGX hardware to validate the manifest/runtime.
- Gramine SGX runs inside an Intel SGX enclave.
- If you see `sgx.debug = true`, the enclave is in debug mode (not for production).

## Configuration
- `HOST`/`PORT`: override bind address (defaults to 127.0.0.1:8080).
- `SEED_INVITE_CODE`: seed a single invite code into the in-memory store at startup.

## Route map (all prefixed with /v1)
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
- `src/models.rs` – request/response structs shared across APIs.
- `src/store.rs` – in-memory store; update to use external DB.
- `src/api/` – handlers grouped by domain (bookmarks, invites, recurring, wallet).
- `src/main.rs` – wiring, state, and OpenAPI registration.

## Notes on autofund
The `/v1/wallet/autofund` handler currently records the request in memory and returns `200 OK`. 
> TODO Avalanche funding implementation
