// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Email index repository backed by redb for O(1) email→wallet lookups.
//!
//! This module wraps the `TxDatabase` email_lookup table to provide
//! a clean API for registering, looking up, and removing email→wallet mappings.

use std::sync::Arc;

use super::super::tx_database::{TxDatabase, TxDbResult};

/// Entry returned from the email index.
#[derive(Debug, Clone)]
pub struct EmailIndexEntry {
    pub wallet_id: String,
    #[allow(dead_code)] // Used during send-by-email resolution; wallet metadata provides the canonical source
    pub public_address: String,
}

/// Repository for email→wallet index operations (O(1) via redb).
pub struct EmailIndexRepository {
    tx_db: Arc<TxDatabase>,
}

impl EmailIndexRepository {
    /// Create a new EmailIndexRepository.
    pub fn new(tx_db: Arc<TxDatabase>) -> Self {
        Self { tx_db }
    }

    /// Register an email lookup key → wallet mapping.
    pub fn register(
        &self,
        lookup_key: &str,
        wallet_id: &str,
        public_address: &str,
    ) -> TxDbResult<()> {
        self.tx_db
            .register_email_lookup(lookup_key, wallet_id, public_address)
    }

    /// Look up a wallet by email lookup key.
    pub fn lookup(&self, lookup_key: &str) -> TxDbResult<Option<EmailIndexEntry>> {
        match self.tx_db.lookup_email(lookup_key)? {
            Some((wallet_id, public_address)) => Ok(Some(EmailIndexEntry {
                wallet_id,
                public_address,
            })),
            None => Ok(None),
        }
    }

    /// Check if an email lookup key exists.
    pub fn exists(&self, lookup_key: &str) -> TxDbResult<bool> {
        self.tx_db.email_lookup_exists(lookup_key)
    }

    /// Remove an email lookup key.
    pub fn remove(&self, lookup_key: &str) -> TxDbResult<()> {
        self.tx_db.remove_email_lookup(lookup_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (Arc<TxDatabase>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(TxDatabase::open(&dir.path().join("test.redb")).unwrap());
        (db, dir)
    }

    #[test]
    fn register_and_lookup() {
        let (db, _dir) = temp_db();
        let repo = EmailIndexRepository::new(db);

        repo.register("lookup_key_1", "wallet-1", "0xabc").unwrap();

        let entry = repo.lookup("lookup_key_1").unwrap().unwrap();
        assert_eq!(entry.wallet_id, "wallet-1");
        assert_eq!(entry.public_address, "0xabc");
    }

    #[test]
    fn lookup_not_found() {
        let (db, _dir) = temp_db();
        let repo = EmailIndexRepository::new(db);

        assert!(repo.lookup("nonexistent").unwrap().is_none());
    }

    #[test]
    fn exists_works() {
        let (db, _dir) = temp_db();
        let repo = EmailIndexRepository::new(db);

        assert!(!repo.exists("key1").unwrap());
        repo.register("key1", "w1", "0x1").unwrap();
        assert!(repo.exists("key1").unwrap());
    }

    #[test]
    fn remove_works() {
        let (db, _dir) = temp_db();
        let repo = EmailIndexRepository::new(db);

        repo.register("key_rm", "w1", "0x1").unwrap();
        assert!(repo.exists("key_rm").unwrap());

        repo.remove("key_rm").unwrap();
        assert!(!repo.exists("key_rm").unwrap());
    }
}
