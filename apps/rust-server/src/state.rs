// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use std::sync::Arc;

use crate::auth::JwksManager;
use crate::storage::EncryptedStorage;

/// Authentication configuration.
#[derive(Clone)]
pub struct AuthConfig {
    /// JWKS manager for key fetching (None = development mode)
    pub jwks: Option<Arc<JwksManager>>,
    /// Expected issuer (Clerk instance URL)
    pub issuer: Option<String>,
    /// Expected audience (optional)
    pub audience: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwks: None,
            issuer: None,
            audience: None,
        }
    }
}

/// Application state shared across all request handlers.
///
/// ## Storage Model
///
/// The application uses encrypted persistent storage via Gramine's encrypted
/// filesystem. All data is stored under `/data`.
///
/// ## Authentication
///
/// - `auth_config` contains JWKS manager and validation settings
/// - In development mode (no CLERK_JWKS_URL), signature verification is skipped
/// - In production mode, all tokens are verified against Clerk JWKS
///
/// ## Security
///
/// - All persistent state lives under `/data` (Gramine encrypted mount)
/// - Gramine handles encryption/decryption transparently
/// - The Rust application uses normal filesystem I/O
#[derive(Clone)]
pub struct AppState {
    /// Encrypted persistent storage
    pub storage: Arc<EncryptedStorage>,
    /// Authentication configuration
    pub auth_config: AuthConfig,
}

impl AppState {
    /// Create new application state with encrypted storage.
    pub fn new(encrypted_storage: EncryptedStorage) -> Self {
        Self {
            storage: Arc::new(encrypted_storage),
            auth_config: AuthConfig::default(),
        }
    }

    /// Set authentication configuration.
    pub fn with_auth_config(mut self, auth_config: AuthConfig) -> Self {
        self.auth_config = auth_config;
        self
    }

    /// Get a reference to the encrypted storage.
    pub fn storage(&self) -> &Arc<EncryptedStorage> {
        &self.storage
    }

    /// Get the authentication configuration.
    #[allow(dead_code)]
    pub fn auth_config(&self) -> &AuthConfig {
        &self.auth_config
    }

    /// Check if production authentication is enabled.
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
            use crate::storage::StoragePaths;
            let temp_dir = std::env::temp_dir().join(format!("test-state-{}", uuid::Uuid::new_v4()));
            let paths = StoragePaths::new(&temp_dir);
            let mut storage = EncryptedStorage::new(paths);
            storage.initialize().expect("Failed to initialize test storage");
            Self::new(storage)
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

