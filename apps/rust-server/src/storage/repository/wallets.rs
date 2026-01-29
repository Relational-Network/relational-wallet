// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Wallet repository for encrypted storage.
//!
//! ## Storage Layout
//!
//! Each wallet lives in its own directory:
//! ```text
//! /data/wallets/{wallet_id}/
//!   meta.json       # Wallet metadata
//!   key.pem         # Private key (PKCS#8 PEM format)
//!   txs/            # Transaction history (TODO)
//! ```
//!
//! ## Security
//!
//! - Private keys are stored in PEM format
//! - Gramine encrypts all files transparently
//! - Private keys are NEVER returned via API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Wallet status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WalletStatus {
    /// Wallet is active and can be used
    Active,
    /// Wallet is suspended (e.g., pending admin review)
    Suspended,
    /// Wallet is deleted (soft delete, files may be retained)
    Deleted,
}

impl Default for WalletStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Wallet metadata stored in meta.json.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletMetadata {
    /// Unique wallet identifier (UUID)
    pub wallet_id: String,
    /// Clerk user ID who owns this wallet
    pub owner_user_id: String,
    /// Public address derived from the private key
    pub public_address: String,
    /// When the wallet was created
    pub created_at: DateTime<Utc>,
    /// Current wallet status
    pub status: WalletStatus,
    /// Optional human-readable label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Response returned to API clients (never includes private key).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletResponse {
    /// Unique wallet identifier
    pub wallet_id: String,
    /// Public address
    pub public_address: String,
    /// When the wallet was created
    pub created_at: DateTime<Utc>,
    /// Current wallet status
    pub status: WalletStatus,
    /// Optional human-readable label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl From<WalletMetadata> for WalletResponse {
    fn from(meta: WalletMetadata) -> Self {
        Self {
            wallet_id: meta.wallet_id,
            public_address: meta.public_address,
            created_at: meta.created_at,
            status: meta.status,
            label: meta.label,
        }
    }
}

impl super::super::OwnedResource for WalletMetadata {
    fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
}

/// Repository for wallet operations on encrypted storage.
pub struct WalletRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> WalletRepository<'a> {
    /// Create a new WalletRepository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if a wallet exists.
    pub fn exists(&self, wallet_id: &str) -> bool {
        self.storage.exists(self.storage.paths().wallet_meta(wallet_id))
    }

    /// Get wallet metadata by ID.
    pub fn get(&self, wallet_id: &str) -> StorageResult<WalletMetadata> {
        let path = self.storage.paths().wallet_meta(wallet_id);
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!("Wallet {wallet_id}")));
        }
        self.storage.read_json(path)
    }

    /// Create a new wallet.
    ///
    /// # Arguments
    /// - `metadata`: Wallet metadata to store
    /// - `private_key_pem`: Private key in PEM format
    ///
    /// # Returns
    /// - `Ok(())` if successful
    /// - `Err(StorageError::AlreadyExists)` if wallet already exists
    pub fn create(&self, metadata: &WalletMetadata, private_key_pem: &[u8]) -> StorageResult<()> {
        let wallet_id = &metadata.wallet_id;

        if self.exists(wallet_id) {
            return Err(StorageError::AlreadyExists(format!("Wallet {wallet_id}")));
        }

        // Create wallet directory structure
        let wallet_dir = self.storage.paths().wallet_dir(wallet_id);
        self.storage.create_dir(&wallet_dir)?;
        self.storage.create_dir(self.storage.paths().wallet_txs_dir(wallet_id))?;

        // Write metadata
        self.storage
            .write_json(self.storage.paths().wallet_meta(wallet_id), metadata)?;

        // Write private key
        self.storage
            .write_raw(self.storage.paths().wallet_key(wallet_id), private_key_pem)?;

        Ok(())
    }

    /// Update wallet metadata.
    ///
    /// Only updates metadata, not the private key.
    pub fn update(&self, metadata: &WalletMetadata) -> StorageResult<()> {
        let wallet_id = &metadata.wallet_id;

        if !self.exists(wallet_id) {
            return Err(StorageError::NotFound(format!("Wallet {wallet_id}")));
        }

        self.storage
            .write_json(self.storage.paths().wallet_meta(wallet_id), metadata)
    }

    /// Delete a wallet and all its data.
    ///
    /// **Warning**: This permanently deletes the wallet including the private key.
    /// TODO: Use after retention period expires for soft-deleted wallets
    #[allow(dead_code)]
    pub fn delete(&self, wallet_id: &str) -> StorageResult<()> {
        if !self.exists(wallet_id) {
            return Err(StorageError::NotFound(format!("Wallet {wallet_id}")));
        }

        self.storage
            .delete_dir(self.storage.paths().wallet_dir(wallet_id))
    }

    /// Soft-delete a wallet (mark as deleted but retain files).
    pub fn soft_delete(&self, wallet_id: &str) -> StorageResult<()> {
        let mut metadata = self.get(wallet_id)?;
        metadata.status = WalletStatus::Deleted;
        self.update(&metadata)
    }

    /// List all wallet IDs.
    pub fn list_all_ids(&self) -> StorageResult<Vec<String>> {
        self.storage.list_dirs(self.storage.paths().wallets_dir())
    }

    /// List all wallets (admin view).
    ///
    /// Returns all wallets regardless of owner. For admin use only.
    pub fn list_all_wallets(&self) -> StorageResult<Vec<WalletMetadata>> {
        let wallet_ids = self.list_all_ids()?;
        let mut wallets = Vec::new();

        for wallet_id in &wallet_ids {
            if let Ok(meta) = self.get(wallet_id) {
                wallets.push(meta);
            }
        }

        Ok(wallets)
    }

    /// List all wallets owned by a user.
    pub fn list_by_owner(&self, owner_user_id: &str) -> StorageResult<Vec<WalletMetadata>> {
        let wallet_ids = self.list_all_ids()?;
        let mut wallets = Vec::new();

        for wallet_id in &wallet_ids {
            if let Ok(meta) = self.get(wallet_id) {
                if meta.owner_user_id == owner_user_id && meta.status != WalletStatus::Deleted {
                    wallets.push(meta);
                }
            }
        }

        Ok(wallets)
    }

    /// Read the private key for a wallet.
    ///
    /// **Internal use only** - for signing operations.
    /// NEVER expose this via API.
    /// TODO: Use when implementing transaction signing
    #[allow(dead_code)]
    pub(crate) fn read_private_key(&self, wallet_id: &str) -> StorageResult<Vec<u8>> {
        if !self.exists(wallet_id) {
            return Err(StorageError::NotFound(format!("Wallet {wallet_id}")));
        }

        self.storage
            .read_raw(self.storage.paths().wallet_key(wallet_id))
    }

    /// Verify wallet ownership.
    ///
    /// Returns the wallet metadata if the user owns it, otherwise returns Forbidden.
    /// TODO: Use when implementing ownership verification middleware
    #[allow(dead_code)]
    pub fn verify_ownership(
        &self,
        wallet_id: &str,
        user_id: &str,
    ) -> StorageResult<WalletMetadata> {
        let metadata = self.get(wallet_id)?;

        if metadata.owner_user_id != user_id {
            return Err(StorageError::NotFound(format!(
                "Wallet {wallet_id} not found for user"
            )));
        }

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-wallet-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn test_metadata() -> WalletMetadata {
        WalletMetadata {
            wallet_id: "wallet-123".to_string(),
            owner_user_id: "user-456".to_string(),
            public_address: "0x1234...abcd".to_string(),
            created_at: Utc::now(),
            status: WalletStatus::Active,
            label: Some("My Wallet".to_string()),
        }
    }

    #[test]
    fn create_and_get_wallet() {
        let storage = test_storage();
        let repo = WalletRepository::new(&storage);

        let meta = test_metadata();
        let key = b"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----";

        repo.create(&meta, key).unwrap();

        let loaded = repo.get(&meta.wallet_id).unwrap();
        assert_eq!(loaded.wallet_id, meta.wallet_id);
        assert_eq!(loaded.owner_user_id, meta.owner_user_id);
        assert_eq!(loaded.public_address, meta.public_address);

        cleanup(&storage);
    }

    #[test]
    fn create_duplicate_fails() {
        let storage = test_storage();
        let repo = WalletRepository::new(&storage);

        let meta = test_metadata();
        let key = b"test-key";

        repo.create(&meta, key).unwrap();
        let result = repo.create(&meta, key);

        assert!(matches!(result, Err(StorageError::AlreadyExists(_))));

        cleanup(&storage);
    }

    #[test]
    fn list_by_owner_filters_correctly() {
        let storage = test_storage();
        let repo = WalletRepository::new(&storage);

        // Create wallets for different users
        for i in 1..=3 {
            let mut meta = test_metadata();
            meta.wallet_id = format!("wallet-user1-{i}");
            meta.owner_user_id = "user-1".to_string();
            repo.create(&meta, b"key").unwrap();
        }

        for i in 1..=2 {
            let mut meta = test_metadata();
            meta.wallet_id = format!("wallet-user2-{i}");
            meta.owner_user_id = "user-2".to_string();
            repo.create(&meta, b"key").unwrap();
        }

        let user1_wallets = repo.list_by_owner("user-1").unwrap();
        assert_eq!(user1_wallets.len(), 3);

        let user2_wallets = repo.list_by_owner("user-2").unwrap();
        assert_eq!(user2_wallets.len(), 2);

        let user3_wallets = repo.list_by_owner("user-3").unwrap();
        assert_eq!(user3_wallets.len(), 0);

        cleanup(&storage);
    }

    #[test]
    fn verify_ownership_rejects_wrong_user() {
        let storage = test_storage();
        let repo = WalletRepository::new(&storage);

        let meta = test_metadata();
        repo.create(&meta, b"key").unwrap();

        // Correct owner
        let result = repo.verify_ownership(&meta.wallet_id, &meta.owner_user_id);
        assert!(result.is_ok());

        // Wrong owner
        let result = repo.verify_ownership(&meta.wallet_id, "wrong-user");
        assert!(matches!(result, Err(StorageError::NotFound(_))));

        cleanup(&storage);
    }

    #[test]
    fn soft_delete_marks_as_deleted() {
        let storage = test_storage();
        let repo = WalletRepository::new(&storage);

        let meta = test_metadata();
        repo.create(&meta, b"key").unwrap();

        repo.soft_delete(&meta.wallet_id).unwrap();

        let loaded = repo.get(&meta.wallet_id).unwrap();
        assert_eq!(loaded.status, WalletStatus::Deleted);

        // Soft-deleted wallets don't show in list_by_owner
        let wallets = repo.list_by_owner(&meta.owner_user_id).unwrap();
        assert!(wallets.is_empty());

        cleanup(&storage);
    }

    #[test]
    fn wallet_response_from_metadata() {
        let meta = test_metadata();
        let response: WalletResponse = meta.clone().into();

        assert_eq!(response.wallet_id, meta.wallet_id);
        assert_eq!(response.public_address, meta.public_address);
        // owner_user_id should NOT be in response
    }
}
