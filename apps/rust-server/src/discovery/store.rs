// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! VOPRF token store backed by redb.
//!
//! Maps hex-encoded VOPRF tokens to public addresses. Used during Phase B
//! of the discovery protocol when a peer sends a finalized token for lookup.

use std::sync::Arc;

use crate::storage::tx_database::{TxDatabase, TxDbResult};

/// Repository for VOPRF token → public address lookups (O(1) via redb).
pub struct VoprfTokenStore {
    tx_db: Arc<TxDatabase>,
}

impl VoprfTokenStore {
    /// Create a new VoprfTokenStore.
    pub fn new(tx_db: Arc<TxDatabase>) -> Self {
        Self { tx_db }
    }

    /// Register a VOPRF token → public address mapping.
    pub fn register(&self, token_hex: &str, public_address: &str) -> TxDbResult<()> {
        self.tx_db.register_voprf_token(token_hex, public_address)
    }

    /// Look up a public address by VOPRF token.
    pub fn lookup(&self, token_hex: &str) -> TxDbResult<Option<String>> {
        self.tx_db.lookup_voprf_token(token_hex)
    }

    /// Remove a VOPRF token mapping (on wallet deletion).
    pub fn remove(&self, token_hex: &str) -> TxDbResult<()> {
        self.tx_db.remove_voprf_token(token_hex)
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
        let store = VoprfTokenStore::new(db);

        store
            .register("aabbccdd", "0x1234567890abcdef1234567890abcdef12345678")
            .unwrap();

        let addr = store.lookup("aabbccdd").unwrap().unwrap();
        assert_eq!(addr, "0x1234567890abcdef1234567890abcdef12345678");
    }

    #[test]
    fn lookup_not_found() {
        let (db, _dir) = temp_db();
        let store = VoprfTokenStore::new(db);
        assert!(store.lookup("nonexistent").unwrap().is_none());
    }

    #[test]
    fn remove_works() {
        let (db, _dir) = temp_db();
        let store = VoprfTokenStore::new(db);

        store.register("token_rm", "0xabc").unwrap();
        assert!(store.lookup("token_rm").unwrap().is_some());

        store.remove("token_rm").unwrap();
        assert!(store.lookup("token_rm").unwrap().is_none());
    }
}
