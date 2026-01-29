// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Invite repository for encrypted storage.
//!
//! Invites are user-scoped codes for onboarding.
//! Each invite is stored as a separate JSON file under `/data/invites/`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Invite stored on encrypted filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct StoredInvite {
    /// Unique invite identifier (UUID)
    pub id: String,
    /// Invite code (user-facing)
    pub code: String,
    /// Whether the invite has been redeemed
    pub redeemed: bool,
    /// User ID who created this invite (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<String>,
    /// User ID who redeemed this invite (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeemed_by_user_id: Option<String>,
    /// When the invite was created
    pub created_at: DateTime<Utc>,
    /// When the invite was redeemed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeemed_at: Option<DateTime<Utc>>,
    /// When the invite expires (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Repository for invite operations on encrypted storage.
pub struct InviteRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> InviteRepository<'a> {
    /// Create a new InviteRepository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if an invite exists.
    pub fn exists(&self, invite_id: &str) -> bool {
        self.storage.exists(self.storage.paths().invite(invite_id))
    }

    /// Get an invite by ID.
    pub fn get(&self, invite_id: &str) -> StorageResult<StoredInvite> {
        let path = self.storage.paths().invite(invite_id);
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!("Invite {invite_id}")));
        }
        self.storage.read_json(path)
    }

    /// Get an invite by code.
    pub fn get_by_code(&self, code: &str) -> StorageResult<StoredInvite> {
        let invite_ids = self
            .storage
            .list_files(self.storage.paths().invites_dir(), "json")?;

        for id in invite_ids {
            if let Ok(invite) = self.get(&id) {
                if invite.code == code {
                    return Ok(invite);
                }
            }
        }

        Err(StorageError::NotFound(format!("Invite with code {code}")))
    }

    /// Create a new invite.
    pub fn create(&self, invite: &StoredInvite) -> StorageResult<()> {
        let invite_id = &invite.id;

        if self.exists(invite_id) {
            return Err(StorageError::AlreadyExists(format!("Invite {invite_id}")));
        }

        // Check for duplicate codes
        if self.get_by_code(&invite.code).is_ok() {
            return Err(StorageError::AlreadyExists(format!(
                "Invite with code {}",
                invite.code
            )));
        }

        self.storage
            .write_json(self.storage.paths().invite(invite_id), invite)
    }

    /// Update an existing invite.
    pub fn update(&self, invite: &StoredInvite) -> StorageResult<()> {
        let invite_id = &invite.id;

        if !self.exists(invite_id) {
            return Err(StorageError::NotFound(format!("Invite {invite_id}")));
        }

        self.storage
            .write_json(self.storage.paths().invite(invite_id), invite)
    }

    /// Delete an invite.
    pub fn delete(&self, invite_id: &str) -> StorageResult<()> {
        if !self.exists(invite_id) {
            return Err(StorageError::NotFound(format!("Invite {invite_id}")));
        }

        self.storage.delete(self.storage.paths().invite(invite_id))
    }

    /// Redeem an invite.
    ///
    /// Returns an error if the invite is already redeemed or expired.
    pub fn redeem(&self, invite_id: &str, user_id: &str) -> StorageResult<StoredInvite> {
        let mut invite = self.get(invite_id)?;

        if invite.redeemed {
            return Err(StorageError::AlreadyExists(
                "Invite already redeemed".to_string(),
            ));
        }

        // Check expiration
        if let Some(expires_at) = invite.expires_at {
            if Utc::now() > expires_at {
                return Err(StorageError::NotFound("Invite has expired".to_string()));
            }
        }

        invite.redeemed = true;
        invite.redeemed_by_user_id = Some(user_id.to_string());
        invite.redeemed_at = Some(Utc::now());

        self.update(&invite)?;
        Ok(invite)
    }

    /// List all invites created by a user.
    pub fn list_by_creator(&self, user_id: &str) -> StorageResult<Vec<StoredInvite>> {
        let invite_ids = self
            .storage
            .list_files(self.storage.paths().invites_dir(), "json")?;

        let mut invites = Vec::new();
        for id in invite_ids {
            if let Ok(invite) = self.get(&id) {
                if invite.created_by_user_id.as_deref() == Some(user_id) {
                    invites.push(invite);
                }
            }
        }

        Ok(invites)
    }

    /// List all invites (admin view).
    ///
    /// Returns all invites regardless of creator. For admin use only.
    pub fn list_all(&self) -> StorageResult<Vec<StoredInvite>> {
        let invite_ids = self
            .storage
            .list_files(self.storage.paths().invites_dir(), "json")?;

        let mut invites = Vec::new();
        for id in invite_ids {
            if let Ok(invite) = self.get(&id) {
                invites.push(invite);
            }
        }

        Ok(invites)
    }

    /// List all valid (not redeemed, not expired) invites.
    pub fn list_valid(&self) -> StorageResult<Vec<StoredInvite>> {
        let invite_ids = self
            .storage
            .list_files(self.storage.paths().invites_dir(), "json")?;

        let now = Utc::now();
        let mut invites = Vec::new();

        for id in invite_ids {
            if let Ok(invite) = self.get(&id) {
                if !invite.redeemed {
                    let not_expired = invite.expires_at.map_or(true, |exp| now < exp);
                    if not_expired {
                        invites.push(invite);
                    }
                }
            }
        }

        Ok(invites)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-invite-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn test_invite(id: &str, code: &str) -> StoredInvite {
        StoredInvite {
            id: id.to_string(),
            code: code.to_string(),
            redeemed: false,
            created_by_user_id: Some("admin-1".to_string()),
            redeemed_by_user_id: None,
            created_at: Utc::now(),
            redeemed_at: None,
            expires_at: None,
        }
    }

    #[test]
    fn create_and_get_invite() {
        let storage = test_storage();
        let repo = InviteRepository::new(&storage);

        let invite = test_invite("inv-1", "WELCOME2026");
        repo.create(&invite).unwrap();

        let loaded = repo.get("inv-1").unwrap();
        assert_eq!(loaded.id, invite.id);
        assert_eq!(loaded.code, invite.code);
        assert!(!loaded.redeemed);

        cleanup(&storage);
    }

    #[test]
    fn get_by_code_works() {
        let storage = test_storage();
        let repo = InviteRepository::new(&storage);

        let invite = test_invite("inv-code", "MYCODE123");
        repo.create(&invite).unwrap();

        let loaded = repo.get_by_code("MYCODE123").unwrap();
        assert_eq!(loaded.id, "inv-code");

        cleanup(&storage);
    }

    #[test]
    fn redeem_invite_works() {
        let storage = test_storage();
        let repo = InviteRepository::new(&storage);

        let invite = test_invite("inv-redeem", "REDEEMME");
        repo.create(&invite).unwrap();

        let redeemed = repo.redeem("inv-redeem", "user-new").unwrap();
        assert!(redeemed.redeemed);
        assert_eq!(redeemed.redeemed_by_user_id.as_deref(), Some("user-new"));
        assert!(redeemed.redeemed_at.is_some());

        // Can't redeem twice
        let result = repo.redeem("inv-redeem", "user-another");
        assert!(matches!(result, Err(StorageError::AlreadyExists(_))));

        cleanup(&storage);
    }

    #[test]
    fn duplicate_code_rejected() {
        let storage = test_storage();
        let repo = InviteRepository::new(&storage);

        let invite1 = test_invite("inv-a", "SAMECODE");
        repo.create(&invite1).unwrap();

        let invite2 = test_invite("inv-b", "SAMECODE");
        let result = repo.create(&invite2);
        assert!(matches!(result, Err(StorageError::AlreadyExists(_))));

        cleanup(&storage);
    }
}
