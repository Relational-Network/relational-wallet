// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Payment link repository backed by redb for opaque token-based payment sharing.
//!
//! Payment links contain no PII — only an opaque server-generated token
//! that resolves to a wallet's public address with optional amount/note.

use std::sync::Arc;

use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::{DateTime, Utc};
use k256::elliptic_curve::rand_core::OsRng;
use serde::{Deserialize, Serialize};

use super::super::tx_database::{TxDatabase, TxDbResult};

/// Data stored for each payment link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLinkData {
    pub wallet_id: String,
    pub public_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub single_use: bool,
    pub used: bool,
}

/// Repository for payment link operations (O(1) via redb).
pub struct PaymentLinkRepository {
    tx_db: Arc<TxDatabase>,
}

impl PaymentLinkRepository {
    /// Create a new PaymentLinkRepository.
    pub fn new(tx_db: Arc<TxDatabase>) -> Self {
        Self { tx_db }
    }

    /// Generate a random 22-char base64url token and store the link data.
    ///
    /// Returns the generated token string.
    pub fn create(&self, data: PaymentLinkData) -> TxDbResult<String> {
        let token = generate_token();
        let json = serde_json::to_string(&data)?;
        self.tx_db.store_payment_link(&token, &json)?;
        Ok(token)
    }

    /// Resolve a token → link data. Checks expiry and marks used if single_use.
    ///
    /// Returns None if token not found, expired, or already used.
    pub fn resolve(&self, token: &str) -> TxDbResult<Option<PaymentLinkData>> {
        let json = match self.tx_db.get_payment_link(token)? {
            Some(j) => j,
            None => return Ok(None),
        };

        let mut data: PaymentLinkData = serde_json::from_str(&json)?;

        // Check expiry
        if Utc::now() > data.expires_at {
            // Remove expired link
            self.tx_db.remove_payment_link(token)?;
            return Ok(None);
        }

        // Check if already used (single-use)
        if data.single_use && data.used {
            return Ok(None);
        }

        // Mark as used if single-use
        if data.single_use {
            data.used = true;
            let updated_json = serde_json::to_string(&data)?;
            self.tx_db.update_payment_link(token, &updated_json)?;
        }

        Ok(Some(data))
    }

    /// Remove expired payment links. Returns count removed.
    pub fn cleanup_expired(&self) -> TxDbResult<u64> {
        let all = self.tx_db.iter_payment_links()?;
        let now = Utc::now();
        let mut removed = 0u64;

        for (token, json) in all {
            if let Ok(data) = serde_json::from_str::<PaymentLinkData>(&json) {
                if now > data.expires_at {
                    self.tx_db.remove_payment_link(&token)?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}

/// Generate a cryptographically random 22-character base64url token.
fn generate_token() -> String {
    use k256::elliptic_curve::rand_core::RngCore;
    let mut bytes = [0u8; 16]; // 16 bytes → 22 base64url chars
    OsRng.fill_bytes(&mut bytes);
    Base64UrlUnpadded::encode_string(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (Arc<TxDatabase>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(TxDatabase::open(&dir.path().join("test.redb")).unwrap());
        (db, dir)
    }

    fn sample_link_data() -> PaymentLinkData {
        PaymentLinkData {
            wallet_id: "wallet-1".to_string(),
            public_address: "0xabc123".to_string(),
            amount: Some("1.5".to_string()),
            token_type: Some("native".to_string()),
            note: Some("Lunch".to_string()),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            single_use: false,
            used: false,
        }
    }

    #[test]
    fn create_and_resolve() {
        let (db, _dir) = temp_db();
        let repo = PaymentLinkRepository::new(db);

        let data = sample_link_data();
        let token = repo.create(data.clone()).unwrap();
        assert!(!token.is_empty());

        let resolved = repo.resolve(&token).unwrap().unwrap();
        assert_eq!(resolved.wallet_id, "wallet-1");
        assert_eq!(resolved.public_address, "0xabc123");
        assert_eq!(resolved.amount, Some("1.5".to_string()));
    }

    #[test]
    fn resolve_not_found() {
        let (db, _dir) = temp_db();
        let repo = PaymentLinkRepository::new(db);

        assert!(repo.resolve("nonexistent_token").unwrap().is_none());
    }

    #[test]
    fn resolve_expired() {
        let (db, _dir) = temp_db();
        let repo = PaymentLinkRepository::new(db);

        let mut data = sample_link_data();
        data.expires_at = Utc::now() - chrono::Duration::hours(1); // Already expired

        let token = repo.create(data).unwrap();
        assert!(repo.resolve(&token).unwrap().is_none());
    }

    #[test]
    fn single_use_works() {
        let (db, _dir) = temp_db();
        let repo = PaymentLinkRepository::new(db);

        let mut data = sample_link_data();
        data.single_use = true;

        let token = repo.create(data).unwrap();

        // First resolve succeeds
        let first = repo.resolve(&token).unwrap();
        assert!(first.is_some());

        // Second resolve fails (marked used)
        let second = repo.resolve(&token).unwrap();
        assert!(second.is_none());
    }

    #[test]
    fn cleanup_expired_removes_old() {
        let (db, _dir) = temp_db();
        let repo = PaymentLinkRepository::new(db);

        // Create an expired link
        let mut data = sample_link_data();
        data.expires_at = Utc::now() - chrono::Duration::hours(1);
        let _token = repo.create(data).unwrap();

        // Create a valid link
        let valid_data = sample_link_data();
        let valid_token = repo.create(valid_data).unwrap();

        let removed = repo.cleanup_expired().unwrap();
        assert_eq!(removed, 1);

        // Valid link still works
        assert!(repo.resolve(&valid_token).unwrap().is_some());
    }

    #[test]
    fn token_format() {
        let token = generate_token();
        assert_eq!(token.len(), 22); // 16 bytes → 22 base64url chars without padding
        // All chars should be base64url safe
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
