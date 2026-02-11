// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Recurring payment repository for encrypted storage.
//!
//! Recurring payments define scheduled transfers from user wallets.
//! Each payment is stored as a separate JSON file under `/data/recurring/`.
//!
//! **Note**: Actual execution of recurring payments is a TODO.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Recurring payment frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaymentFrequency {
    /// Once per day
    Daily,
    /// Once per week
    Weekly,
    /// Once per month
    Monthly,
    /// Once per year
    Yearly,
}

impl From<i32> for PaymentFrequency {
    fn from(days: i32) -> Self {
        match days {
            1 => PaymentFrequency::Daily,
            7 => PaymentFrequency::Weekly,
            30 | 31 => PaymentFrequency::Monthly,
            365 | 366 => PaymentFrequency::Yearly,
            _ => PaymentFrequency::Daily, // Default fallback
        }
    }
}

impl From<PaymentFrequency> for i32 {
    fn from(freq: PaymentFrequency) -> Self {
        match freq {
            PaymentFrequency::Daily => 1,
            PaymentFrequency::Weekly => 7,
            PaymentFrequency::Monthly => 30,
            PaymentFrequency::Yearly => 365,
        }
    }
}

/// Recurring payment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    /// Payment is active and will be executed on schedule
    Active,
    /// Payment is paused (temporarily disabled)
    Paused,
    /// Payment has been cancelled
    Cancelled,
    /// Payment schedule has completed
    Completed,
}

impl Default for PaymentStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Recurring payment stored on encrypted filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct StoredRecurringPayment {
    /// Unique payment identifier (UUID)
    pub id: String,
    /// Wallet ID to pay from (must be owned by user)
    pub wallet_id: String,
    /// Owner user ID
    pub owner_user_id: String,
    /// Wallet public key (for verification)
    pub wallet_public_key: String,
    /// Recipient address
    pub recipient: String,
    /// Payment amount
    pub amount: f64,
    /// Currency code (e.g., "AVAX", "USDC")
    pub currency_code: String,
    /// Payment frequency
    pub frequency: PaymentFrequency,
    /// When payments should start (day of year, legacy format)
    pub payment_start_date: i32,
    /// When payments should end (day of year, legacy format)
    pub payment_end_date: i32,
    /// Last payment date (day of year, -1 if never paid)
    pub last_paid_date: i32,
    /// Payment status
    pub status: PaymentStatus,
    /// When this record was created
    pub created_at: DateTime<Utc>,
    /// When this record was last updated
    pub updated_at: DateTime<Utc>,
}

impl super::super::OwnedResource for StoredRecurringPayment {
    fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
}

/// Repository for recurring payment operations on encrypted storage.
pub struct RecurringRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> RecurringRepository<'a> {
    /// Create a new RecurringRepository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if a recurring payment exists.
    pub fn exists(&self, payment_id: &str) -> bool {
        self.storage
            .exists(self.storage.paths().recurring_payment(payment_id))
    }

    /// Get a recurring payment by ID.
    pub fn get(&self, payment_id: &str) -> StorageResult<StoredRecurringPayment> {
        let path = self.storage.paths().recurring_payment(payment_id);
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!(
                "Recurring payment {payment_id}"
            )));
        }
        self.storage.read_json(path)
    }

    /// Create a new recurring payment.
    pub fn create(&self, payment: &StoredRecurringPayment) -> StorageResult<()> {
        let payment_id = &payment.id;

        if self.exists(payment_id) {
            return Err(StorageError::AlreadyExists(format!(
                "Recurring payment {payment_id}"
            )));
        }

        self.storage
            .write_json(self.storage.paths().recurring_payment(payment_id), payment)
    }

    /// Update an existing recurring payment.
    pub fn update(&self, payment: &StoredRecurringPayment) -> StorageResult<()> {
        let payment_id = &payment.id;

        if !self.exists(payment_id) {
            return Err(StorageError::NotFound(format!(
                "Recurring payment {payment_id}"
            )));
        }

        self.storage
            .write_json(self.storage.paths().recurring_payment(payment_id), payment)
    }

    /// Delete a recurring payment.
    pub fn delete(&self, payment_id: &str) -> StorageResult<()> {
        if !self.exists(payment_id) {
            return Err(StorageError::NotFound(format!(
                "Recurring payment {payment_id}"
            )));
        }

        self.storage
            .delete(self.storage.paths().recurring_payment(payment_id))
    }

    /// List all recurring payments for a wallet.
    /// TODO: Recurring payment execution
    #[allow(dead_code)]
    pub fn list_by_wallet(&self, wallet_id: &str) -> StorageResult<Vec<StoredRecurringPayment>> {
        let payment_ids = self
            .storage
            .list_files(self.storage.paths().recurring_dir(), "json")?;

        let mut payments = Vec::new();
        for id in payment_ids {
            if let Ok(payment) = self.get(&id) {
                if payment.wallet_id == wallet_id && payment.status != PaymentStatus::Cancelled {
                    payments.push(payment);
                }
            }
        }

        Ok(payments)
    }

    /// List all recurring payments (admin view).
    ///
    /// Returns all payments regardless of owner. For admin use only.
    pub fn list_all(&self) -> StorageResult<Vec<StoredRecurringPayment>> {
        let payment_ids = self
            .storage
            .list_files(self.storage.paths().recurring_dir(), "json")?;

        let mut payments = Vec::new();
        for id in payment_ids {
            if let Ok(payment) = self.get(&id) {
                payments.push(payment);
            }
        }

        Ok(payments)
    }

    /// List all recurring payments owned by a user.
    pub fn list_by_owner(&self, owner_user_id: &str) -> StorageResult<Vec<StoredRecurringPayment>> {
        let payment_ids = self
            .storage
            .list_files(self.storage.paths().recurring_dir(), "json")?;

        let mut payments = Vec::new();
        for id in payment_ids {
            if let Ok(payment) = self.get(&id) {
                if payment.owner_user_id == owner_user_id
                    && payment.status != PaymentStatus::Cancelled
                {
                    payments.push(payment);
                }
            }
        }

        Ok(payments)
    }

    /// Verify payment ownership.
    pub fn verify_ownership(
        &self,
        payment_id: &str,
        owner_user_id: &str,
    ) -> StorageResult<StoredRecurringPayment> {
        let payment = self.get(payment_id)?;

        if payment.owner_user_id != owner_user_id {
            return Err(StorageError::NotFound(format!(
                "Recurring payment {payment_id} not found for user"
            )));
        }

        Ok(payment)
    }

    /// Get all active payments due today.
    ///
    /// Returns payments where:
    /// - status is Active
    /// - payment_start_date <= today
    /// - payment_end_date >= today
    /// - last_paid_date < today (or never paid)
    ///
    /// TODO: Actual payment execution is not implemented.
    #[allow(dead_code)]
    pub fn list_due_today(
        &self,
        today_day_of_year: i32,
    ) -> StorageResult<Vec<StoredRecurringPayment>> {
        let payment_ids = self
            .storage
            .list_files(self.storage.paths().recurring_dir(), "json")?;

        let mut due_payments = Vec::new();
        for id in payment_ids {
            if let Ok(payment) = self.get(&id) {
                if payment.status == PaymentStatus::Active
                    && payment.payment_start_date <= today_day_of_year
                    && payment.payment_end_date >= today_day_of_year
                    && payment.last_paid_date < today_day_of_year
                {
                    // TODO: Check frequency to determine if actually due
                    due_payments.push(payment);
                }
            }
        }

        Ok(due_payments)
    }

    /// Update the last paid date for a payment.
    pub fn update_last_paid_date(
        &self,
        payment_id: &str,
        last_paid_date: i32,
    ) -> StorageResult<StoredRecurringPayment> {
        let mut payment = self.get(payment_id)?;
        payment.last_paid_date = last_paid_date;
        payment.updated_at = Utc::now();
        self.update(&payment)?;
        Ok(payment)
    }

    /// Cancel a recurring payment.
    /// TODO: Recurring payment cancellation
    #[allow(dead_code)]
    pub fn cancel(&self, payment_id: &str) -> StorageResult<StoredRecurringPayment> {
        let mut payment = self.get(payment_id)?;
        payment.status = PaymentStatus::Cancelled;
        payment.updated_at = Utc::now();
        self.update(&payment)?;
        Ok(payment)
    }

    /// Pause a recurring payment.
    /// TODO: Recurring payment pausing
    #[allow(dead_code)]
    pub fn pause(&self, payment_id: &str) -> StorageResult<StoredRecurringPayment> {
        let mut payment = self.get(payment_id)?;
        payment.status = PaymentStatus::Paused;
        payment.updated_at = Utc::now();
        self.update(&payment)?;
        Ok(payment)
    }

    /// Resume a paused recurring payment.
    /// TODO: Recurring payment resuming
    #[allow(dead_code)]
    pub fn resume(&self, payment_id: &str) -> StorageResult<StoredRecurringPayment> {
        let mut payment = self.get(payment_id)?;
        if payment.status != PaymentStatus::Paused {
            return Err(StorageError::NotFound("Payment is not paused".to_string()));
        }
        payment.status = PaymentStatus::Active;
        payment.updated_at = Utc::now();
        self.update(&payment)?;
        Ok(payment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir =
            env::temp_dir().join(format!("test-recurring-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn test_payment(id: &str) -> StoredRecurringPayment {
        let now = Utc::now();
        StoredRecurringPayment {
            id: id.to_string(),
            wallet_id: "wallet-123".to_string(),
            owner_user_id: "user-456".to_string(),
            wallet_public_key: "0xpubkey...".to_string(),
            recipient: "0xrecipient...".to_string(),
            amount: 10.0,
            currency_code: "AVAX".to_string(),
            frequency: PaymentFrequency::Monthly,
            payment_start_date: 1,
            payment_end_date: 365,
            last_paid_date: -1,
            status: PaymentStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn create_and_get_payment() {
        let storage = test_storage();
        let repo = RecurringRepository::new(&storage);

        let payment = test_payment("rp-1");
        repo.create(&payment).unwrap();

        let loaded = repo.get("rp-1").unwrap();
        assert_eq!(loaded.id, payment.id);
        assert_eq!(loaded.amount, 10.0);
        assert_eq!(loaded.frequency, PaymentFrequency::Monthly);

        cleanup(&storage);
    }

    #[test]
    fn list_by_owner_filters_correctly() {
        let storage = test_storage();
        let repo = RecurringRepository::new(&storage);

        // Create payments for different users
        for i in 1..=3 {
            let mut p = test_payment(&format!("rp-u1-{i}"));
            p.owner_user_id = "user-1".to_string();
            repo.create(&p).unwrap();
        }

        for i in 1..=2 {
            let mut p = test_payment(&format!("rp-u2-{i}"));
            p.owner_user_id = "user-2".to_string();
            repo.create(&p).unwrap();
        }

        let user1_payments = repo.list_by_owner("user-1").unwrap();
        assert_eq!(user1_payments.len(), 3);

        let user2_payments = repo.list_by_owner("user-2").unwrap();
        assert_eq!(user2_payments.len(), 2);

        cleanup(&storage);
    }

    #[test]
    fn cancel_payment_excludes_from_list() {
        let storage = test_storage();
        let repo = RecurringRepository::new(&storage);

        let payment = test_payment("rp-cancel");
        repo.create(&payment).unwrap();

        repo.cancel("rp-cancel").unwrap();

        let payments = repo.list_by_owner(&payment.owner_user_id).unwrap();
        assert!(payments.is_empty());

        // But still exists
        let cancelled = repo.get("rp-cancel").unwrap();
        assert_eq!(cancelled.status, PaymentStatus::Cancelled);

        cleanup(&storage);
    }

    #[test]
    fn pause_and_resume_works() {
        let storage = test_storage();
        let repo = RecurringRepository::new(&storage);

        let payment = test_payment("rp-pause");
        repo.create(&payment).unwrap();

        repo.pause("rp-pause").unwrap();
        let paused = repo.get("rp-pause").unwrap();
        assert_eq!(paused.status, PaymentStatus::Paused);

        repo.resume("rp-pause").unwrap();
        let resumed = repo.get("rp-pause").unwrap();
        assert_eq!(resumed.status, PaymentStatus::Active);

        cleanup(&storage);
    }
}
