// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

mod api;
mod config;
mod error;
mod models;
mod state;
mod store;
mod tls;

#[cfg(not(test))]
use std::{env, net::SocketAddr, sync::Arc};

#[cfg(not(test))]
use api::router;
#[cfg(not(test))]
use axum_server::tls_rustls::RustlsConfig;
#[cfg(not(test))]
use state::AppState;
#[cfg(not(test))]
use store::InMemoryStore;
#[cfg(not(test))]
use tls::load_ratls_credentials;

#[cfg(not(test))]
#[tokio::main]
async fn main() {
    // Install the ring crypto provider for rustls (must be done before any TLS operations)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Load RA-TLS credentials (panics if not available - TLS is mandatory)
    println!("Loading RA-TLS credentials...");
    let (certs, key) = load_ratls_credentials();
    println!(
        "Loaded {} certificate(s) from RA-TLS",
        certs.len()
    );

    // Build rustls server config
    let tls_config = RustlsConfig::from_config(Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("Failed to build TLS server config"),
    ));

    // Initialize application state
    let mut store = InMemoryStore::new();

    if let Ok(code) = env::var("SEED_INVITE_CODE") {
        store.insert_invite(code, false);
    }

    let state = AppState::new(store);
    let app = router(state);

    // Parse bind address
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Failed to parse bind address");

    println!("Relational Wallet server listening on https://{addr} (docs at /docs)");
    println!("Running with DCAP RA-TLS attestation");

    // Start HTTPS server (TLS is mandatory - no HTTP fallback)
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .expect("HTTPS server failed");
}
