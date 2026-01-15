// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

mod api;
mod error;
mod models;
mod state;
mod store;

#[cfg(not(test))]
use std::{env, net::SocketAddr};

#[cfg(not(test))]
use api::router;
#[cfg(not(test))]
use state::AppState;
#[cfg(not(test))]
use store::InMemoryStore;

#[cfg(not(test))]
#[tokio::main]
async fn main() {
    let mut store = InMemoryStore::new();

    if let Ok(code) = env::var("SEED_INVITE_CODE") {
        store.insert_invite(code, false);
    }

    let state = AppState::new(store);
    let app = router(state);

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Failed to parse bind address");

    println!("Relational Wallet server listening on http://{addr} (docs at /docs)");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app.into_make_service())
        .await
        .expect("Server failed");
}
