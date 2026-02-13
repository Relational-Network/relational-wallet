// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

mod api;
mod auth;
mod blockchain;
mod config;
mod error;
mod models;
mod providers;
mod state;
mod storage;
mod tls;

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

    // Bootstrap enclave-managed fiat reserve wallet (idempotent) unless disabled.
    let fiat_bootstrap_enabled = env::var("FIAT_RESERVE_BOOTSTRAP_ENABLED")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(true);
    if fiat_bootstrap_enabled {
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
    } else {
        info!("FIAT_RESERVE_BOOTSTRAP_ENABLED=false, skipping reserve wallet bootstrap");
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

    let state = AppState::new(encrypted_storage).with_auth_config(auth_config);

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

    // Start HTTPS server (TLS is mandatory - no HTTP fallback)
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .expect("HTTPS server failed");
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
