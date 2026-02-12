// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Transaction repository for persisting transaction history.
//!
//! ## Storage Layout
//!
//! Transactions are stored per-wallet in the txs/ directory:
//! ```text
//! /data/wallets/{wallet_id}/txs/
//!   {tx_hash}.json     # Individual transaction record
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

/// Transaction status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    /// Transaction has been submitted but not yet confirmed
    Pending,
    /// Transaction has been confirmed in a block
    Confirmed,
    /// Transaction failed or was reverted
    Failed,
}

impl Default for TxStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Token type for a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Native AVAX transfer
    Native,
    /// ERC-20 token transfer (stores contract address)
    #[serde(rename = "erc20")]
    Erc20(String),
}

/// Stored transaction record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StoredTransaction {
    /// Transaction hash (0x prefixed)
    pub tx_hash: String,
    /// Wallet ID that initiated the transaction
    pub wallet_id: String,
    /// Optional counterparty wallet ID when both sides are internal wallets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_wallet_id: Option<String>,
    /// Sender address
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Amount in human-readable format
    pub amount: String,
    /// Token type (native or ERC-20)
    pub token: TokenType,
    /// Network (fuji or mainnet)
    pub network: String,
    /// Current transaction status
    pub status: TxStatus,
    /// Block number (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// Gas used (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    /// Block explorer URL
    pub explorer_url: String,
    /// When the transaction was submitted
    pub created_at: DateTime<Utc>,
    /// When the status was last updated
    pub updated_at: DateTime<Utc>,
}

impl StoredTransaction {
    /// Create a new pending transaction record.
    pub fn new_pending(
        tx_hash: String,
        wallet_id: String,
        counterparty_wallet_id: Option<String>,
        from: String,
        to: String,
        amount: String,
        token: TokenType,
        network: String,
        explorer_url: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            tx_hash,
            wallet_id,
            counterparty_wallet_id,
            from,
            to,
            amount,
            token,
            network,
            status: TxStatus::Pending,
            block_number: None,
            gas_used: None,
            explorer_url,
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark the transaction as confirmed.
    pub fn mark_confirmed(&mut self, block_number: u64, gas_used: u64) {
        self.status = TxStatus::Confirmed;
        self.block_number = Some(block_number);
        self.gas_used = Some(gas_used);
        self.updated_at = Utc::now();
    }

    /// Mark the transaction as failed.
    pub fn mark_failed(&mut self) {
        self.status = TxStatus::Failed;
        self.updated_at = Utc::now();
    }
}

/// Repository for transaction operations on encrypted storage.
pub struct TransactionRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> TransactionRepository<'a> {
    /// Create a new TransactionRepository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Get the path to a transaction file.
    fn tx_path(&self, wallet_id: &str, tx_hash: &str) -> std::path::PathBuf {
        // Normalize tx_hash (remove 0x prefix for filename)
        let hash = tx_hash.strip_prefix("0x").unwrap_or(tx_hash);
        self.storage
            .paths()
            .wallet_txs_dir(wallet_id)
            .join(format!("{}.json", hash))
    }

    /// Store a new transaction record.
    pub fn create(&self, tx: &StoredTransaction) -> StorageResult<()> {
        let path = self.tx_path(&tx.wallet_id, &tx.tx_hash);

        if self.storage.exists(&path) {
            return Err(StorageError::AlreadyExists(format!(
                "Transaction {}",
                tx.tx_hash
            )));
        }

        // Ensure txs directory exists
        let txs_dir = self.storage.paths().wallet_txs_dir(&tx.wallet_id);
        if !self.storage.exists(&txs_dir) {
            self.storage.create_dir(&txs_dir)?;
        }

        self.storage.write_json(path, tx)
    }

    /// Get a transaction by hash.
    pub fn get(&self, wallet_id: &str, tx_hash: &str) -> StorageResult<StoredTransaction> {
        let path = self.tx_path(wallet_id, tx_hash);

        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!("Transaction {}", tx_hash)));
        }

        self.storage.read_json(path)
    }

    /// Update a transaction record.
    pub fn update(&self, tx: &StoredTransaction) -> StorageResult<()> {
        let path = self.tx_path(&tx.wallet_id, &tx.tx_hash);

        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(format!(
                "Transaction {}",
                tx.tx_hash
            )));
        }

        self.storage.write_json(path, tx)
    }

    /// List all transactions for a wallet.
    pub fn list_by_wallet(&self, wallet_id: &str) -> StorageResult<Vec<StoredTransaction>> {
        let txs_dir = self.storage.paths().wallet_txs_dir(wallet_id);

        if !self.storage.exists(&txs_dir) {
            return Ok(Vec::new());
        }

        let files = self.storage.list_files(&txs_dir, "json")?;
        let mut transactions = Vec::new();

        for file in files {
            // list_files returns file stems (without extension), so add .json back
            let path = txs_dir.join(format!("{}.json", file));
            match self.storage.read_json::<StoredTransaction>(&path) {
                Ok(tx) => transactions.push(tx),
                Err(e) => {
                    tracing::warn!("Failed to read transaction {}: {}", file, e);
                }
            }
        }

        // Sort by created_at descending (newest first)
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(transactions)
    }

    /// List pending transactions for a wallet (for status polling).
    /// TODO: Batch transaction status polling
    #[allow(dead_code)]
    pub fn list_pending(&self, wallet_id: &str) -> StorageResult<Vec<StoredTransaction>> {
        let all = self.list_by_wallet(wallet_id)?;
        Ok(all
            .into_iter()
            .filter(|tx| tx.status == TxStatus::Pending)
            .collect())
    }

    /// Update transaction status from blockchain receipt.
    pub fn update_from_receipt(
        &self,
        wallet_id: &str,
        tx_hash: &str,
        block_number: u64,
        gas_used: u64,
        success: bool,
    ) -> StorageResult<StoredTransaction> {
        let mut tx = self.get(wallet_id, tx_hash)?;

        if success {
            tx.mark_confirmed(block_number, gas_used);
        } else {
            tx.mark_failed();
        }

        self.update(&tx)?;
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!("test-tx-repo-{}", uuid::Uuid::new_v4()));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize");

        // Create wallet directory structure
        let wallet_dir = storage.paths().wallet_dir("wallet-123");
        storage.create_dir(&wallet_dir).unwrap();
        storage
            .create_dir(storage.paths().wallet_txs_dir("wallet-123"))
            .unwrap();

        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    fn test_transaction() -> StoredTransaction {
        StoredTransaction::new_pending(
            "0xabc123def456".to_string(),
            "wallet-123".to_string(),
            None,
            "0x1234...".to_string(),
            "0x5678...".to_string(),
            "1.5".to_string(),
            TokenType::Native,
            "fuji".to_string(),
            "https://testnet.snowtrace.io/tx/0xabc123def456".to_string(),
        )
    }

    #[test]
    fn create_and_get_transaction() {
        let storage = test_storage();
        let repo = TransactionRepository::new(&storage);

        let tx = test_transaction();
        repo.create(&tx).unwrap();

        let retrieved = repo.get("wallet-123", &tx.tx_hash).unwrap();
        assert_eq!(retrieved.tx_hash, tx.tx_hash);
        assert_eq!(retrieved.status, TxStatus::Pending);

        cleanup(&storage);
    }

    #[test]
    fn update_transaction_status() {
        let storage = test_storage();
        let repo = TransactionRepository::new(&storage);

        let tx = test_transaction();
        repo.create(&tx).unwrap();

        let updated = repo
            .update_from_receipt("wallet-123", &tx.tx_hash, 12345, 21000, true)
            .unwrap();

        assert_eq!(updated.status, TxStatus::Confirmed);
        assert_eq!(updated.block_number, Some(12345));
        assert_eq!(updated.gas_used, Some(21000));

        cleanup(&storage);
    }

    #[test]
    fn list_transactions() {
        let storage = test_storage();
        let repo = TransactionRepository::new(&storage);

        // Create multiple transactions
        let mut tx1 = test_transaction();
        tx1.tx_hash = "0x111".to_string();
        repo.create(&tx1).unwrap();

        let mut tx2 = test_transaction();
        tx2.tx_hash = "0x222".to_string();
        repo.create(&tx2).unwrap();

        let list = repo.list_by_wallet("wallet-123").unwrap();
        assert_eq!(list.len(), 2);

        cleanup(&storage);
    }
}
