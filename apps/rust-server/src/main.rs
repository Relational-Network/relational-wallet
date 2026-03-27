// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

// During test compilation, `main()` is excluded via `#[cfg(not(test))]`,
// which makes items only reachable from main appear unused. Suppress
// those false positives so `cargo test` compiles warning-free.
#[cfg_attr(test, allow(unused_imports, dead_code))]
mod api;
mod auth;
mod blockchain;
mod config;
mod error;
#[cfg_attr(test, allow(dead_code))]
mod fiat_poller;
#[cfg_attr(test, allow(dead_code))]
mod indexer;
mod models;
mod providers;
#[cfg_attr(test, allow(dead_code))]
mod state;
#[cfg_attr(test, allow(unused_imports))]
mod storage;
mod tls;

#[cfg(not(test))]
use axum_server::Handle;
#[cfg(not(test))]
use std::{env, net::SocketAddr, sync::Arc, time::Duration};

#[cfg(not(test))]
use api::router;
#[cfg(not(test))]
use auth::JwksManager;
#[cfg(not(test))]
use axum_server::tls_rustls::RustlsConfig;
#[cfg(not(test))]
use state::{AppState, AuthConfig};
#[cfg(not(test))]
use storage::EncryptedStorage;
#[cfg(not(test))]
use tls::load_ratls_credentials;
#[cfg(not(test))]
use tokio_util::sync::CancellationToken;
#[cfg(not(test))]
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
#[cfg(not(test))]
use tracing::{info, warn};
#[cfg(not(test))]
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(not(test))]
#[tokio::main]
async fn main() {
    // Initialize structured logging
    init_tracing();

    // Install the ring crypto provider for rustls (must be done before any TLS operations)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Load RA-TLS credentials (panics if not available - TLS is mandatory)
    info!("Loading RA-TLS credentials...");
    let (certs, key) = load_ratls_credentials();
    info!(cert_count = certs.len(), "Loaded RA-TLS certificates");

    // Build rustls server config
    let tls_config = RustlsConfig::from_config(Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("Failed to build TLS server config"),
    ));

    // ========== Initialize Authentication ==========
    let auth_config = initialize_auth_config().await;

    // ========== Initialize Encrypted Storage ==========
    // /data is mounted as type="encrypted" with key_name="_sgx_mrsigner" in the Gramine manifest.
    // Gramine handles all encryption/decryption transparently.
    // The Rust application uses normal filesystem I/O.
    info!("Initializing encrypted storage at /data...");
    let mut encrypted_storage = EncryptedStorage::with_default_paths();
    encrypted_storage
        .initialize()
        .expect("Failed to initialize encrypted storage - is /data mounted?");

    // Verify encrypted storage is working
    encrypted_storage
        .health_check()
        .expect("Encrypted storage health check failed");
    info!("Encrypted storage initialized and verified");

    // Bootstrap enclave-managed fiat reserve service wallet (idempotent).
    {
        let repo = storage::FiatServiceWalletRepository::new(&encrypted_storage);
        match repo.bootstrap() {
            Ok(metadata) => {
                info!(
                    wallet_id = %metadata.wallet_id,
                    public_address = %metadata.public_address,
                    "Fiat reserve service wallet ready"
                );
            }
            Err(error) => {
                warn!(error = %error, "Failed to bootstrap fiat reserve service wallet");
            }
        }
    }

    // Seed invite if configured
    if let Ok(code) = env::var("SEED_INVITE_CODE") {
        use chrono::Utc;
        use storage::repository::invites::StoredInvite;
        use storage::repository::InviteRepository;

        let repo = InviteRepository::new(&encrypted_storage);
        // Only create if it doesn't exist
        if repo.get_by_code(&code).is_err() {
            let invite = StoredInvite {
                id: uuid::Uuid::new_v4().to_string(),
                code,
                redeemed: false,
                created_by_user_id: Some("system".to_string()),
                redeemed_by_user_id: None,
                created_at: Utc::now(),
                redeemed_at: None,
                expires_at: None,
            };
            if let Err(e) = repo.create(&invite) {
                warn!(error = %e, "Failed to seed invite code");
            } else {
                info!("Seeded invite code from SEED_INVITE_CODE");
            }
        }
    }

    // ========== Initialize Transaction Database (redb) ==========
    info!("Opening transaction database...");
    let tx_db_path = encrypted_storage.paths().root().join("tx.redb");
    let tx_db = Arc::new(
        storage::TxDatabase::open(&tx_db_path).unwrap_or_else(|error| {
            panic!(
                "Failed to open transaction database at {}: {}",
                tx_db_path.display(),
                error
            )
        }),
    );
    info!(path = %tx_db_path.display(), "Transaction database opened");

    // ========== Register wallet addresses in tx_db ==========
    // Ensures the address→wallet_id, user→wallet, and email_lookup maps
    // are always consistent, even after redb recreation.
    // This is idempotent and very cheap.
    {
        use storage::repository::WalletRepository;
        let repo = WalletRepository::new(&encrypted_storage);
        let mut registered = 0u32;
        if let Ok(wallets) = repo.list_all_wallets() {
            for w in &wallets {
                // address → wallet_id
                if let Err(e) = tx_db.register_address(&w.public_address, &w.wallet_id) {
                    warn!(wallet_id = %w.wallet_id, error = %e, "Failed to register wallet address");
                } else {
                    registered += 1;
                }
                // user_id → wallet_id (only for non-deleted wallets)
                if w.status != storage::WalletStatus::Deleted {
                    if let Err(e) = tx_db.register_user_wallet(&w.owner_user_id, &w.wallet_id) {
                        warn!(wallet_id = %w.wallet_id, error = %e, "Failed to register user→wallet mapping");
                    }
                    // email_lookup_key → { wallet_id, public_address }
                    if let Some(ref lookup_key) = w.email_lookup_key {
                        if let Err(e) =
                            tx_db.register_email_lookup(lookup_key, &w.wallet_id, &w.public_address)
                        {
                            warn!(wallet_id = %w.wallet_id, error = %e, "Failed to register email lookup");
                        }
                    }
                }
            }
        }
        // Also register the fiat service wallet so the indexer can
        // detect incoming transfers to it (off-ramp deposit detection).
        {
            let svc_repo = storage::FiatServiceWalletRepository::new(&encrypted_storage);
            if let Ok(meta) = svc_repo.get() {
                if let Err(e) = tx_db.register_address(&meta.public_address, &meta.wallet_id) {
                    warn!(error = %e, "Failed to register service wallet address");
                } else {
                    registered += 1;
                }
            }
        }
        info!(count = registered, "Wallet addresses registered in tx_db");
    }

    // ========== Cleanup Expired Payment Links ==========
    {
        use storage::PaymentLinkRepository;
        let repo = PaymentLinkRepository::new(tx_db.clone());
        match repo.cleanup_expired() {
            Ok(0) => {}
            Ok(n) => info!(count = n, "Removed expired payment links"),
            Err(e) => warn!(error = %e, "Failed to cleanup expired payment links"),
        }
    }

    // ========== Load or Generate Email HMAC Key ==========
    let email_hmac_key: [u8; 32] = {
        let key_path = encrypted_storage
            .paths()
            .root()
            .join("system/email_hmac_key.bin");
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if key_path.exists() {
            let bytes = std::fs::read(&key_path).expect("Failed to read email HMAC key");
            let mut key = [0u8; 32];
            if bytes.len() != 32 {
                panic!(
                    "email_hmac_key.bin has wrong length (expected 32, got {})",
                    bytes.len()
                );
            }
            key.copy_from_slice(&bytes);
            info!("Loaded email HMAC key from {}", key_path.display());
            key
        } else {
            use k256::elliptic_curve::rand_core::{OsRng, RngCore};
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            std::fs::write(&key_path, &key).expect("Failed to write email HMAC key");
            info!("Generated and stored new email HMAC key");
            key
        }
    };

    // ========== Initialize Clerk Backend API Client ==========
    let clerk_client = env::var("CLERK_SECRET_KEY").ok().map(|secret_key| {
        info!("Clerk Backend API client initialized");
        providers::clerk::ClerkClient::new(secret_key)
    });
    if clerk_client.is_none() {
        warn!("CLERK_SECRET_KEY not set — email features disabled");
    }

    let mut state = AppState::new(encrypted_storage)
        .with_auth_config(auth_config)
        .with_tx_db(tx_db.clone())
        .with_email_hmac_key(email_hmac_key);

    if let Some(clerk) = clerk_client {
        state = state.with_clerk_client(clerk);
    }

    // Create LRU cache
    let tx_cache = Arc::new(storage::TxCache::new(1000, Duration::from_secs(300)));

    // Wire tx_db and tx_cache into state
    let state = state.with_tx_cache(tx_cache.clone());

    // ========== Spawn Event Indexer ==========
    let shutdown = CancellationToken::new();
    let token_contracts = indexer::fuji_token_contracts();
    if !token_contracts.is_empty() {
        let event_indexer = indexer::EventIndexer::new(
            tx_db.clone(),
            tx_cache.clone(),
            blockchain::avax_fuji(),
            token_contracts,
        );
        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            event_indexer.run(shutdown_clone).await;
        });
        info!("ERC-20 event indexer spawned");
    } else {
        info!("No token contracts configured — event indexer not started");
    }

    // ========== Spawn Fiat Request Poller ==========
    {
        let fiat_poller =
            fiat_poller::FiatPoller::new(state.storage().clone(), tx_db.clone(), tx_cache.clone());
        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            fiat_poller.run(shutdown_clone).await;
        });
        info!("Fiat request poller spawned");
    }

    // Build router with tracing middleware for request IDs
    let app = router(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    tracing::info_span!(
                        "http_request",
                        request_id = %request_id,
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status().as_u16(),
                            latency_ms = %latency.as_millis(),
                            "response"
                        );
                    },
                ),
        );

    // Parse bind address
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Failed to parse bind address");

    info!(
        address = %addr,
        docs = "/docs",
        "Relational Wallet server starting"
    );
    info!("Running with DCAP RA-TLS attestation");
    info!(
        path = "/data",
        "Persistent storage: Gramine encrypted filesystem"
    );

    // ========== Graceful Shutdown ==========
    // Install a Ctrl+C (SIGINT) handler that:
    //   1. Cancels background tasks (indexer, fiat poller) via the CancellationToken
    //   2. Gracefully drains in-flight HTTP connections
    //   3. Lets the Database drop normally so redb can flush & close cleanly
    let handle = Handle::new();
    let server_handle = handle.clone();
    let shutdown_token = shutdown.clone();
    tokio::spawn(async move {
        // Wait for Ctrl+C (SIGINT)
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "Failed to listen for Ctrl+C");
            return;
        }
        tracing::info!("Received Ctrl+C — initiating graceful shutdown");

        // 1. Signal background tasks to stop
        shutdown_token.cancel();

        // 2. Tell axum-server to stop accepting new connections and drain
        //    existing ones within 5 seconds
        server_handle.graceful_shutdown(Some(Duration::from_secs(5)));
    });

    // Start HTTPS server (TLS is mandatory - no HTTP fallback)
    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .expect("HTTPS server failed");

    // Server has stopped — give background tasks a moment to finish
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Drop the Arc<TxDatabase> so redb can flush and close cleanly.
    // If other Arcs are still held by tasks that haven't finished,
    // the Drop will happen when the last reference is released.
    drop(tx_db);
    info!("Shutdown complete");
}

/// Initialize the tracing subscriber with JSON output for production.
#[cfg(not(test))]
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));

    // Check if we should use JSON format (production) or pretty format (development)
    let use_json = env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false);

    if use_json {
        // JSON format for production (easier to parse by log aggregators)
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    } else {
        // Pretty format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().pretty())
            .init();
    }
}

/// Initialize authentication configuration from environment variables.
///
/// Required for production:
/// - CLERK_JWKS_URL: The JWKS endpoint (e.g., https://your-clerk.clerk.accounts.dev/.well-known/jwks.json)
/// - CLERK_ISSUER: The expected issuer (e.g., https://your-clerk.clerk.accounts.dev)
///
/// Optional:
/// - CLERK_AUDIENCE: Expected audience claim
#[cfg(not(test))]
async fn initialize_auth_config() -> AuthConfig {
    let jwks_url = env::var("CLERK_JWKS_URL").ok();
    let issuer = env::var("CLERK_ISSUER").ok();
    let audience = env::var("CLERK_AUDIENCE").ok();

    if let Some(url) = jwks_url {
        info!("Initializing JWKS authentication...");
        info!(jwks_url = %url, "JWKS endpoint configured");

        let jwks_manager = JwksManager::new(&url);

        // Pre-fetch JWKS with retry — DNS may not be ready immediately
        // in containerized environments (Docker DNS at 127.0.0.11).
        let max_retries = 5u32;
        let mut fetched = false;
        for attempt in 1..=max_retries {
            match jwks_manager.refresh().await {
                Ok(()) => {
                    info!("JWKS fetched successfully");
                    fetched = true;
                    break;
                }
                Err(e) => {
                    if attempt < max_retries {
                        let delay = Duration::from_secs(u64::from(attempt));
                        warn!(
                            attempt,
                            max_retries,
                            delay_secs = delay.as_secs(),
                            error = %e,
                            "JWKS fetch failed, retrying..."
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        warn!(
                            error = %e,
                            "JWKS fetch failed after {max_retries} attempts — \
                             JWT verification will fail until next background refresh"
                        );
                    }
                }
            }
        }
        if fetched {
            info!("JWKS pre-fetch succeeded — authentication ready");
        }

        if let Some(ref iss) = issuer {
            info!(issuer = %iss, "Issuer validation enabled");
        } else {
            panic!(
                "CLERK_ISSUER must be set when CLERK_JWKS_URL is configured. \
                    Without issuer validation, JWT verification is insecure."
            );
        }

        if let Some(ref aud) = audience {
            info!(audience = %aud, "Audience validation enabled");
        }

        info!("Production authentication ENABLED");

        AuthConfig {
            jwks: Some(Arc::new(jwks_manager)),
            issuer,
            audience,
        }
    } else {
        #[cfg(feature = "dev")]
        {
            warn!("CLERK_JWKS_URL not set - running in DEVELOPMENT MODE");
            warn!("JWT signature verification is DISABLED");
            warn!("Set CLERK_JWKS_URL for production use");
        }
        #[cfg(not(feature = "dev"))]
        {
            warn!("CLERK_JWKS_URL not set - all authenticated requests will be REJECTED");
            warn!("Set CLERK_JWKS_URL to enable JWT verification");
        }

        AuthConfig::default()
    }
}
