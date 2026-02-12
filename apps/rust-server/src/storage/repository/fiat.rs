// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Fiat on-ramp/off-ramp request repository for encrypted storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Fiat request direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiatDirection {
    /// Fiat to token request (bank deposit -> token mint).
    OnRamp,
    /// Token to fiat request (token burn/redeem -> bank payout).
    OffRamp,
}

/// Fiat request lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiatRequestStatus {
    /// Request accepted and queued for provider processing.
    Queued,
    /// Provider flow started and waiting for completion.
    ProviderPending,
    /// Request settled successfully.
    Completed,
    /// Request failed.
    Failed,
}

/// Persisted fiat request record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StoredFiatRequest {
    /// Unique request identifier.
    pub request_id: String,
    /// Wallet tied to this request.
    pub wallet_id: String,
    /// Owner user ID.
    pub owner_user_id: String,
    /// On-ramp vs off-ramp direction.
    pub direction: FiatDirection,
    /// Requested fiat amount in EUR (human-readable decimal string).
    pub amount_eur: String,
    /// Selected provider identifier (stub: `truelayer_sandbox`).
    pub provider: String,
    /// Optional user note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional provider reference/session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_reference: Option<String>,
    /// Optional URL where user can continue provider authorization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_action_url: Option<String>,
    /// Current status.
    pub status: FiatRequestStatus,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl super::super::OwnedResource for StoredFiatRequest {
    fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
}

impl StoredFiatRequest {
    /// Construct a new queued fiat request.
    pub fn new_queued(
        request_id: String,
        wallet_id: String,
        owner_user_id: String,
        direction: FiatDirection,
        amount_eur: String,
        provider: String,
        note: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            request_id,
            wallet_id,
            owner_user_id,
            direction,
            amount_eur,
            provider,
            note,
            provider_reference: None,
            provider_action_url: None,
            status: FiatRequestStatus::Queued,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Repository for fiat request storage.
pub struct FiatRequestRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> FiatRequestRepository<'a> {
    /// Create repository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if request exists.
    pub fn exists(&self, request_id: &str) -> bool {
        self.storage
            .exists(self.storage.paths().fiat_request(request_id))
    }

    /// Get request by ID.
    pub fn get(&self, request_id: &str) -> StorageResult<StoredFiatRequest> {
        let path = self.storage.paths().fiat_request(request_id);
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!("Fiat request {request_id}")));
        }
        self.storage.read_json(path)
    }

    /// Persist new request.
    pub fn create(&self, request: &StoredFiatRequest) -> StorageResult<()> {
        if self.exists(&request.request_id) {
            return Err(StorageError::AlreadyExists(format!(
                "Fiat request {}",
                request.request_id
            )));
        }
        self.storage.write_json(
            self.storage.paths().fiat_request(&request.request_id),
            request,
        )
    }

    /// Update existing request.
    pub fn update(&self, request: &StoredFiatRequest) -> StorageResult<()> {
        if !self.exists(&request.request_id) {
            return Err(StorageError::NotFound(format!(
                "Fiat request {}",
                request.request_id
            )));
        }
        self.storage.write_json(
            self.storage.paths().fiat_request(&request.request_id),
            request,
        )
    }

    /// List all requests for user.
    pub fn list_by_owner(&self, owner_user_id: &str) -> StorageResult<Vec<StoredFiatRequest>> {
        let ids = self
            .storage
            .list_files(self.storage.paths().fiat_dir(), "json")?;

        let mut requests = Vec::new();
        for id in ids {
            if let Ok(record) = self.get(&id) {
                if record.owner_user_id == owner_user_id {
                    requests.push(record);
                }
            }
        }

        requests.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(requests)
    }

    /// List all requests for a given wallet owned by user.
    pub fn list_by_wallet_for_owner(
        &self,
        owner_user_id: &str,
        wallet_id: &str,
    ) -> StorageResult<Vec<StoredFiatRequest>> {
        let all = self.list_by_owner(owner_user_id)?;
        Ok(all
            .into_iter()
            .filter(|record| record.wallet_id == wallet_id)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-fiat-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("initialize test storage");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn sample_request(id: &str) -> StoredFiatRequest {
        StoredFiatRequest::new_queued(
            id.to_string(),
            "wallet-1".to_string(),
            "user-1".to_string(),
            FiatDirection::OnRamp,
            "25.50".to_string(),
            "truelayer_sandbox".to_string(),
            Some("demo".to_string()),
        )
    }

    #[test]
    fn create_and_get_request() {
        let storage = test_storage();
        let repo = FiatRequestRepository::new(&storage);
        let req = sample_request("req-1");

        repo.create(&req).expect("create request");
        let loaded = repo.get("req-1").expect("get request");
        assert_eq!(loaded.request_id, "req-1");
        assert_eq!(loaded.provider, "truelayer_sandbox");

        cleanup(&storage);
    }

    #[test]
    fn list_by_owner_filters_records() {
        let storage = test_storage();
        let repo = FiatRequestRepository::new(&storage);

        let one = sample_request("req-1");
        let mut two = sample_request("req-2");
        two.owner_user_id = "user-2".to_string();

        repo.create(&one).expect("create first");
        repo.create(&two).expect("create second");

        let owned = repo.list_by_owner("user-1").expect("list");
        assert_eq!(owned.len(), 1);
        assert_eq!(owned[0].request_id, "req-1");

        cleanup(&storage);
    }
}
