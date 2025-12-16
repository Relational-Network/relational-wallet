# Rust-Server WIP

> TODO

## What’s included
- Bookmarks: list, create, delete.
- Invites: fetch by code, redeem.
- Recurring payments: list, create, update, delete, fetch payments due today, update last paid date.
- Wallet autofund: laceholder to autofund wallets (DEV)
- Swagger/OpenAPI: JSON at `/api-doc/openapi.json`, Swagger UI at `/docs`.

## Quick start
```bash
cd apps/rust-server
cargo run
# server listens on 127.0.0.1:8080 by default; set PORT to override
# optional: SEED_INVITE_CODE=WELCOME cargo run   # preload a single invite for testing
```

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