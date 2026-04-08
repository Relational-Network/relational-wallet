// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Application State
//!
//! This module defines the shared application state that is passed to all
//! Axum request handlers via the `State` extractor.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         AppState                                │
//! │  ┌─────────────────────────┐  ┌─────────────────────────────┐  │
//! │  │  Arc<EncryptedStorage>  │  │       AuthConfig            │  │
//! │  │  - wallets/             │  │  - JWKS Manager (optional)  │  │
//! │  │  - bookmarks/           │  │  - Issuer validation        │  │
//! │  │  - invites/             │  │  - Audience validation      │  │
//! │  │  - audit/               │  │                             │  │
//! │  └─────────────────────────┘  └─────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Thread Safety
//!
//! `AppState` is `Clone` and `Send + Sync`, allowing it to be safely shared
//! across async tasks. The `EncryptedStorage` is wrapped in `Arc` for
//! reference counting.
//!
//! ## Authentication Modes
//!
//! - **Production**: `CLERK_JWKS_URL` set → JWT signatures verified via JWKS
//! - **Development**: `CLERK_JWKS_URL` not set → JWT signatures NOT verified

use std::sync::Arc;

use crate::auth::JwksManager;
use crate::providers::clerk::ClerkClient;
use crate::storage::tx_cache::TxCache;
use crate::storage::tx_database::TxDatabase;
use crate::storage::EncryptedStorage;

use crate::discovery::{DiscoveryClient, PeerRegistry, VoprfServerWrapper, VoprfTokenStore};

// =============================================================================
// Authentication Configuration
// =============================================================================

/// Authentication configuration for JWT verification.
///
/// When `jwks` is `Some`, production-grade JWT verification is enabled.
/// When `jwks` is `None`, the service runs in development mode with
/// signature verification disabled.
#[derive(Clone)]
pub struct AuthConfig {
    /// JWKS manager for fetching Clerk public keys.
    ///
    /// - `Some`: Production mode - JWT signatures are verified
    /// - `None`: Development mode - JWT signatures are NOT verified
    pub jwks: Option<Arc<JwksManager>>,

    /// Expected JWT issuer (Clerk instance URL).
    ///
    /// Example: `https://your-app.clerk.accounts.dev`
    pub issuer: Option<String>,

    /// Expected JWT audience claim (optional).
    ///
    /// Set via `CLERK_AUDIENCE` environment variable.
    pub audience: Option<String>,
}

impl Default for AuthConfig {
    /// Create a default (development mode) auth config.
    ///
    /// **Warning**: This disables JWT signature verification!
    fn default() -> Self {
        Self {
            jwks: None,
            issuer: None,
            audience: None,
        }
    }
}

// =============================================================================
// Application State
// =============================================================================

/// Shared application state for all request handlers.
///
/// This struct is passed to handlers via Axum's `State` extractor and provides
/// access to encrypted storage and authentication configuration.
///
/// ## Example
///
/// ```rust,ignore
/// async fn my_handler(
///     State(state): State<AppState>,
/// ) -> Result<Json<Data>, ApiError> {
///     let storage = state.storage();
///     // Use storage...
/// }
/// ```
#[derive(Clone)]
pub struct AppState {
    /// Reference-counted encrypted storage instance.
    ///
    /// All persistent data (wallets, bookmarks, invites, audit logs) is
    /// stored here. The underlying filesystem is encrypted by Gramine.
    pub storage: Arc<EncryptedStorage>,

    /// Authentication configuration for JWT verification.
    pub auth_config: AuthConfig,

    /// Embedded ACID transaction database (redb).
    ///
    /// Provides indexed, paginated transaction queries instead of
    /// scanning individual JSON files.
    pub tx_db: Option<Arc<TxDatabase>>,

    /// In-process LRU cache for hot wallet transaction lookups.
    pub tx_cache: Option<Arc<TxCache>>,

    /// Clerk Backend API client for fetching user emails.
    ///
    /// `None` when `CLERK_SECRET_KEY` is not set (dev mode: email
    /// features disabled).
    pub clerk_client: Option<ClerkClient>,

    /// Node-local HMAC key for email lookup key derivation.
    ///
    /// Sealed by Gramine encrypted FS — never leaves the enclave.
    /// Loaded from `/data/system/email_hmac_key.bin` on startup.
    pub email_hmac_key: [u8; 32],

    // ── Phase 2: VOPRF Discovery ──

    /// VOPRF server key for evaluating blinded queries from peers.
    pub voprf_server: Arc<VoprfServerWrapper>,

    /// Discovery client for querying peer enclaves.
    pub discovery_client: Arc<DiscoveryClient>,

    /// Peer registry with attestation policies.
    pub peer_registry: Arc<PeerRegistry>,

    /// VOPRF token store (wraps TxDatabase table).
    pub voprf_store: Arc<VoprfTokenStore>,
}

impl AppState {
    /// Create new application state with the given encrypted storage.
    ///
    /// # Arguments
    ///
    /// * `encrypted_storage` - Initialized encrypted storage instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let storage = EncryptedStorage::with_default_paths();
    /// storage.initialize()?;
    /// let state = AppState::new(storage, voprf_server, discovery_client, peer_registry, voprf_store);
    /// ```
    pub fn new(
        encrypted_storage: EncryptedStorage,
        voprf_server: Arc<VoprfServerWrapper>,
        discovery_client: Arc<DiscoveryClient>,
        peer_registry: Arc<PeerRegistry>,
        voprf_store: Arc<VoprfTokenStore>,
    ) -> Self {
        Self {
            storage: Arc::new(encrypted_storage),
            auth_config: AuthConfig::default(),
            tx_db: None,
            tx_cache: None,
            clerk_client: None,
            email_hmac_key: [0u8; 32],
            voprf_server,
            discovery_client,
            peer_registry,
            voprf_store,
        }
    }

    /// Convenience constructor for tests: creates discovery stubs automatically.
    #[cfg(test)]
    pub fn new_test(encrypted_storage: EncryptedStorage) -> Self {
        let voprf_server = Arc::new(VoprfServerWrapper::generate());
        let peer_registry = Arc::new(PeerRegistry::empty());
        let discovery_client =
            Arc::new(DiscoveryClient::new(peer_registry.clone()));
        // We need a TxDatabase for VoprfTokenStore
        let tx_db_path = encrypted_storage.paths().root().join("tx.redb");
        let tx_db = Arc::new(
            crate::storage::TxDatabase::open(&tx_db_path)
                .expect("Failed to open test tx database"),
        );
        let voprf_store = Arc::new(VoprfTokenStore::new(tx_db));
        Self::new(encrypted_storage, voprf_server, discovery_client, peer_registry, voprf_store)
    }

    /// Configure authentication settings.
    pub fn with_auth_config(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = auth_config;
        self
    }

    /// Configure the transaction database.
    pub fn with_tx_db(mut self, tx_db: Arc<TxDatabase>) -> Self {
        self.tx_db = Some(tx_db);
        self
    }

    /// Configure the transaction cache.
    pub fn with_tx_cache(mut self, tx_cache: Arc<TxCache>) -> Self {
        self.tx_cache = Some(tx_cache);
        self
    }

    /// Configure the Clerk Backend API client.
    pub fn with_clerk_client(mut self, clerk_client: ClerkClient) -> Self {
        self.clerk_client = Some(clerk_client);
        self
    }

    /// Configure the email HMAC key.
    pub fn with_email_hmac_key(mut self, key: [u8; 32]) -> Self {
        self.email_hmac_key = key;
        self
    }

    /// Get a reference to the encrypted storage.
    ///
    /// The returned `Arc` can be cloned for use in repository constructors.
    pub fn storage(&self) -> &Arc<EncryptedStorage> {
        &self.storage
    }

    /// Get the authentication configuration.
    #[allow(dead_code)]
    pub fn auth_config(&self) -> &AuthConfig {
        &self.auth_config
    }

    /// Check if production authentication is enabled.
    ///
    /// Returns `true` if JWKS verification is configured, `false` if running
    /// in development mode.
    #[allow(dead_code)]
    pub fn is_production_auth(&self) -> bool {
        self.auth_config.jwks.is_some()
    }
}

impl Default for AppState {
    fn default() -> Self {
        // Default creates a test-friendly instance with temp storage
        #[cfg(test)]
        {
            use crate::storage::{StoragePaths, TxDatabase};
            let temp_dir =
                std::env::temp_dir().join(format!("test-state-{}", uuid::Uuid::new_v4()));
            let paths = StoragePaths::new(&temp_dir);
            let mut storage = EncryptedStorage::new(paths);
            storage
                .initialize()
                .expect("Failed to initialize test storage");
            let tx_db_path = storage.paths().root().join("tx.redb");
            let tx_db =
                Arc::new(TxDatabase::open(&tx_db_path).expect("Failed to initialize tx database"));

            // Discovery stubs for test contexts
            let voprf_server = Arc::new(VoprfServerWrapper::generate());
            let voprf_store = Arc::new(VoprfTokenStore::new(tx_db.clone()));
            let peer_registry = Arc::new(PeerRegistry::empty());
            let discovery_client =
                Arc::new(DiscoveryClient::new(peer_registry.clone()));

            Self::new(storage, voprf_server, discovery_client, peer_registry, voprf_store)
                .with_tx_db(tx_db)
        }
        #[cfg(not(test))]
        {
            panic!("AppState::default() should not be used in production - use AppState::new() with initialized storage")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_can_be_cloned() {
        let state = AppState::default();
        let _cloned = state.clone();
    }

    #[test]
    fn storage_is_accessible() {
        let state = AppState::default();
        assert!(state.storage().is_initialized());
    }
}
