// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Path constants and utilities for encrypted storage layout.

use std::path::{Path, PathBuf};

/// Base directory for all encrypted persistent storage.
/// This MUST be mounted as `type = "encrypted"` in the Gramine manifest.
pub const DATA_ROOT: &str = "/data";

/// Storage path utilities for the encrypted filesystem.
#[derive(Debug, Clone)]
pub struct StoragePaths {
    root: PathBuf,
}

impl Default for StoragePaths {
    fn default() -> Self {
        Self::new(DATA_ROOT)
    }
}

impl StoragePaths {
    /// Create a new StoragePaths with a custom root (useful for testing).
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Root directory for all encrypted data.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // ========== Wallet Paths ==========

    /// Directory containing all wallets.
    pub fn wallets_dir(&self) -> PathBuf {
        self.root.join("wallets")
    }

    /// Directory for a specific wallet.
    pub fn wallet_dir(&self, wallet_id: &str) -> PathBuf {
        self.wallets_dir().join(wallet_id)
    }

    /// Path to wallet metadata file.
    pub fn wallet_meta(&self, wallet_id: &str) -> PathBuf {
        self.wallet_dir(wallet_id).join("meta.json")
    }

    /// Path to wallet private key file.
    pub fn wallet_key(&self, wallet_id: &str) -> PathBuf {
        self.wallet_dir(wallet_id).join("key.pem")
    }

    /// Directory for wallet transaction history.
    pub fn wallet_txs_dir(&self, wallet_id: &str) -> PathBuf {
        self.wallet_dir(wallet_id).join("txs")
    }

    // ========== Bookmark Paths ==========

    /// Directory containing all bookmarks.
    pub fn bookmarks_dir(&self) -> PathBuf {
        self.root.join("bookmarks")
    }

    /// Path to a specific bookmark file.
    pub fn bookmark(&self, bookmark_id: &str) -> PathBuf {
        self.bookmarks_dir().join(format!("{bookmark_id}.json"))
    }

    // ========== Invite Paths ==========

    /// Directory containing all invites.
    pub fn invites_dir(&self) -> PathBuf {
        self.root.join("invites")
    }

    /// Path to a specific invite file.
    pub fn invite(&self, invite_id: &str) -> PathBuf {
        self.invites_dir().join(format!("{invite_id}.json"))
    }

    // ========== Recurring Payment Paths ==========

    /// Directory containing all recurring payments.
    pub fn recurring_dir(&self) -> PathBuf {
        self.root.join("recurring")
    }

    /// Path to a specific recurring payment file.
    pub fn recurring_payment(&self, payment_id: &str) -> PathBuf {
        self.recurring_dir().join(format!("{payment_id}.json"))
    }

    // ========== Fiat Request Paths ==========

    /// Directory containing all fiat requests.
    pub fn fiat_dir(&self) -> PathBuf {
        self.root.join("fiat")
    }

    /// Path to a specific fiat request file.
    pub fn fiat_request(&self, request_id: &str) -> PathBuf {
        self.fiat_dir().join(format!("{request_id}.json"))
    }

    // ========== Audit Log Paths ==========

    /// Directory containing audit logs.
    pub fn audit_dir(&self) -> PathBuf {
        self.root.join("audit")
    }

    /// Directory for a specific date's audit logs.
    pub fn audit_date_dir(&self, date: &str) -> PathBuf {
        self.audit_dir().join(date)
    }

    /// Path to a daily audit events file (JSONL format).
    pub fn audit_events_file(&self, date: &str) -> PathBuf {
        self.audit_date_dir(date).join("events.jsonl")
    }

    /// Path to a specific audit log entry (legacy single-file format).
    /// TODO: Use when implementing granular audit event storage
    #[allow(dead_code)]
    pub fn audit_entry(&self, timestamp: i64, event_id: &str) -> PathBuf {
        self.audit_dir()
            .join(format!("{timestamp}-{event_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_use_data_root() {
        let paths = StoragePaths::default();
        assert_eq!(paths.root(), Path::new("/data"));
    }

    #[test]
    fn custom_root_for_testing() {
        let paths = StoragePaths::new("/tmp/test-data");
        assert_eq!(paths.root(), Path::new("/tmp/test-data"));
        assert_eq!(
            paths.wallet_meta("wallet-123"),
            PathBuf::from("/tmp/test-data/wallets/wallet-123/meta.json")
        );
    }

    #[test]
    fn wallet_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.wallets_dir(), PathBuf::from("/data/wallets"));
        assert_eq!(paths.wallet_dir("w1"), PathBuf::from("/data/wallets/w1"));
        assert_eq!(
            paths.wallet_meta("w1"),
            PathBuf::from("/data/wallets/w1/meta.json")
        );
        assert_eq!(
            paths.wallet_key("w1"),
            PathBuf::from("/data/wallets/w1/key.pem")
        );
        assert_eq!(
            paths.wallet_txs_dir("w1"),
            PathBuf::from("/data/wallets/w1/txs")
        );
    }

    #[test]
    fn bookmark_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.bookmarks_dir(), PathBuf::from("/data/bookmarks"));
        assert_eq!(
            paths.bookmark("bm-123"),
            PathBuf::from("/data/bookmarks/bm-123.json")
        );
    }

    #[test]
    fn invite_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.invites_dir(), PathBuf::from("/data/invites"));
        assert_eq!(
            paths.invite("inv-456"),
            PathBuf::from("/data/invites/inv-456.json")
        );
    }

    #[test]
    fn recurring_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.recurring_dir(), PathBuf::from("/data/recurring"));
        assert_eq!(
            paths.recurring_payment("rp-789"),
            PathBuf::from("/data/recurring/rp-789.json")
        );
    }

    #[test]
    fn audit_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.audit_dir(), PathBuf::from("/data/audit"));
        assert_eq!(
            paths.audit_entry(1706400000, "evt-001"),
            PathBuf::from("/data/audit/1706400000-evt-001.json")
        );
    }

    #[test]
    fn fiat_paths_are_correct() {
        let paths = StoragePaths::default();
        assert_eq!(paths.fiat_dir(), PathBuf::from("/data/fiat"));
        assert_eq!(
            paths.fiat_request("fr-123"),
            PathBuf::from("/data/fiat/fr-123.json")
        );
    }
}
