// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Bookmark repository for encrypted storage.
//!
//! Bookmarks are user-defined address labels for quick access.
//! Each bookmark is stored as a separate JSON file under `/data/bookmarks/`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Bookmark stored on encrypted filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct StoredBookmark {
    /// Unique bookmark identifier (UUID)
    pub id: String,
    /// Wallet ID this bookmark belongs to
    pub wallet_id: String,
    /// Owner user ID (for direct ownership verification)
    pub owner_user_id: String,
    /// Human-readable label
    pub name: String,
    /// Target address
    pub address: String,
    /// When the bookmark was created
    pub created_at: DateTime<Utc>,
}

impl super::super::OwnedResource for StoredBookmark {
    fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
}

/// Repository for bookmark operations on encrypted storage.
pub struct BookmarkRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> BookmarkRepository<'a> {
    /// Create a new BookmarkRepository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if a bookmark exists.
    pub fn exists(&self, bookmark_id: &str) -> bool {
        self.storage
            .exists(self.storage.paths().bookmark(bookmark_id))
    }

    /// Get a bookmark by ID.
    pub fn get(&self, bookmark_id: &str) -> StorageResult<StoredBookmark> {
        let path = self.storage.paths().bookmark(bookmark_id);
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!("Bookmark {bookmark_id}")));
        }
        self.storage.read_json(path)
    }

    /// Create a new bookmark.
    pub fn create(&self, bookmark: &StoredBookmark) -> StorageResult<()> {
        let bookmark_id = &bookmark.id;

        if self.exists(bookmark_id) {
            return Err(StorageError::AlreadyExists(format!(
                "Bookmark {bookmark_id}"
            )));
        }

        self.storage
            .write_json(self.storage.paths().bookmark(bookmark_id), bookmark)
    }

    /// Update an existing bookmark.
    /// TODO: Use when implementing bookmark update endpoint
    #[allow(dead_code)]
    pub fn update(&self, bookmark: &StoredBookmark) -> StorageResult<()> {
        let bookmark_id = &bookmark.id;

        if !self.exists(bookmark_id) {
            return Err(StorageError::NotFound(format!("Bookmark {bookmark_id}")));
        }

        self.storage
            .write_json(self.storage.paths().bookmark(bookmark_id), bookmark)
    }

    /// Delete a bookmark.
    pub fn delete(&self, bookmark_id: &str) -> StorageResult<()> {
        if !self.exists(bookmark_id) {
            return Err(StorageError::NotFound(format!("Bookmark {bookmark_id}")));
        }

        self.storage
            .delete(self.storage.paths().bookmark(bookmark_id))
    }

    /// List all bookmarks (admin view).
    ///
    /// Returns all bookmarks regardless of owner. For admin use only.
    pub fn list_all(&self) -> StorageResult<Vec<StoredBookmark>> {
        let bookmark_ids = self
            .storage
            .list_files(self.storage.paths().bookmarks_dir(), "json")?;

        let mut bookmarks = Vec::new();
        for id in bookmark_ids {
            if let Ok(bookmark) = self.get(&id) {
                bookmarks.push(bookmark);
            }
        }

        Ok(bookmarks)
    }

    /// List all bookmarks for a wallet owned by a user.
    pub fn list_by_wallet(
        &self,
        wallet_id: &str,
        owner_user_id: &str,
    ) -> StorageResult<Vec<StoredBookmark>> {
        let bookmark_ids = self
            .storage
            .list_files(self.storage.paths().bookmarks_dir(), "json")?;

        let mut bookmarks = Vec::new();
        for id in bookmark_ids {
            if let Ok(bookmark) = self.get(&id) {
                if bookmark.wallet_id == wallet_id && bookmark.owner_user_id == owner_user_id {
                    bookmarks.push(bookmark);
                }
            }
        }

        Ok(bookmarks)
    }

    /// List all bookmarks owned by a user.
    /// TODO: Use when implementing user-centric bookmark views
    #[allow(dead_code)]
    pub fn list_by_owner(&self, owner_user_id: &str) -> StorageResult<Vec<StoredBookmark>> {
        let bookmark_ids = self
            .storage
            .list_files(self.storage.paths().bookmarks_dir(), "json")?;

        let mut bookmarks = Vec::new();
        for id in bookmark_ids {
            if let Ok(bookmark) = self.get(&id) {
                if bookmark.owner_user_id == owner_user_id {
                    bookmarks.push(bookmark);
                }
            }
        }

        Ok(bookmarks)
    }

    /// Verify bookmark ownership.
    /// TODO: Use when implementing ownership verification middleware
    #[allow(dead_code)]
    pub fn verify_ownership(
        &self,
        bookmark_id: &str,
        owner_user_id: &str,
    ) -> StorageResult<StoredBookmark> {
        let bookmark = self.get(bookmark_id)?;

        if bookmark.owner_user_id != owner_user_id {
            return Err(StorageError::NotFound(format!(
                "Bookmark {bookmark_id} not found for user"
            )));
        }

        Ok(bookmark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-bookmark-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn test_bookmark(id: &str) -> StoredBookmark {
        StoredBookmark {
            id: id.to_string(),
            wallet_id: "wallet-123".to_string(),
            owner_user_id: "user-456".to_string(),
            name: "Exchange".to_string(),
            address: "0xabc...def".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn create_and_get_bookmark() {
        let storage = test_storage();
        let repo = BookmarkRepository::new(&storage);

        let bookmark = test_bookmark("bm-1");
        repo.create(&bookmark).unwrap();

        let loaded = repo.get("bm-1").unwrap();
        assert_eq!(loaded.id, bookmark.id);
        assert_eq!(loaded.name, bookmark.name);
        assert_eq!(loaded.address, bookmark.address);

        cleanup(&storage);
    }

    #[test]
    fn list_by_wallet_filters_correctly() {
        let storage = test_storage();
        let repo = BookmarkRepository::new(&storage);

        // Create bookmarks for different wallets
        for i in 1..=3 {
            let mut bm = test_bookmark(&format!("bm-w1-{i}"));
            bm.wallet_id = "wallet-1".to_string();
            repo.create(&bm).unwrap();
        }

        for i in 1..=2 {
            let mut bm = test_bookmark(&format!("bm-w2-{i}"));
            bm.wallet_id = "wallet-2".to_string();
            repo.create(&bm).unwrap();
        }

        let w1_bookmarks = repo.list_by_wallet("wallet-1", "user-456").unwrap();
        assert_eq!(w1_bookmarks.len(), 3);

        let w2_bookmarks = repo.list_by_wallet("wallet-2", "user-456").unwrap();
        assert_eq!(w2_bookmarks.len(), 2);

        cleanup(&storage);
    }

    #[test]
    fn verify_ownership_rejects_wrong_user() {
        let storage = test_storage();
        let repo = BookmarkRepository::new(&storage);

        let bookmark = test_bookmark("bm-owned");
        repo.create(&bookmark).unwrap();

        // Correct owner
        let result = repo.verify_ownership("bm-owned", &bookmark.owner_user_id);
        assert!(result.is_ok());

        // Wrong owner
        let result = repo.verify_ownership("bm-owned", "wrong-user");
        assert!(matches!(result, Err(StorageError::NotFound(_))));

        cleanup(&storage);
    }
}
