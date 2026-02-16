// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Embedded transaction database backed by redb (pure Rust, ACID).
//!
//! ## Table Layout
//!
//! - `transactions`: tx_hash → serialized StoredTransaction
//! - `wallet_tx_index`: composite key (address|!timestamp|tx_hash) → direction
//! - `address_wallet_map`: on-chain address → wallet_id
//! - `indexer_state`: key → value (checkpoint state)

use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::Deserialize;

use super::repository::transactions::{StoredTransaction, TxStatus};
use super::EncryptedStorage;

// =============================================================================
// Table Definitions
// =============================================================================

/// Primary table: tx_hash → serialized StoredTransaction (JSON bytes).
const TRANSACTIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("transactions");

/// Index: composite key → direction ("sent"|"received").
/// Key format: `address|!timestamp_be|tx_hash` for descending-time range scans.
const WALLET_TX_INDEX: TableDefinition<&[u8], &str> = TableDefinition::new("wallet_tx_index");

/// Map: lowercase on-chain address → wallet_id.
const ADDRESS_WALLET_MAP: TableDefinition<&str, &str> = TableDefinition::new("address_wallet_map");

/// Indexer state: key → value bytes (e.g., "last_block_fuji" → u64 big-endian).
const INDEXER_STATE: TableDefinition<&str, &[u8]> = TableDefinition::new("indexer_state");

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TxDbError {
    #[error("redb error: {0}")]
    Redb(#[from] redb::Error),

    #[error("redb database error: {0}")]
    RedbDatabase(#[from] redb::DatabaseError),

    #[error("redb transaction error: {0}")]
    RedbTransaction(#[from] redb::TransactionError),

    #[error("redb table error: {0}")]
    RedbTable(#[from] redb::TableError),

    #[error("redb storage error: {0}")]
    RedbStorage(#[from] redb::StorageError),

    #[error("redb commit error: {0}")]
    RedbCommit(#[from] redb::CommitError),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),
}

pub type TxDbResult<T> = Result<T, TxDbError>;

// =============================================================================
// Index Key Helpers
// =============================================================================

/// Build a composite key for the wallet_tx_index table.
///
/// Format: `lowercase_address | inverted_timestamp_be_bytes | tx_hash`
///
/// The inverted timestamp ensures newest-first ordering when scanning forward.
fn make_index_key(wallet_address: &str, timestamp: i64, tx_hash: &str) -> Vec<u8> {
    let addr = wallet_address.to_lowercase();
    let mut key = Vec::with_capacity(addr.len() + 1 + 8 + 1 + tx_hash.len());
    key.extend_from_slice(addr.as_bytes());
    key.push(b'|');
    // Invert timestamp for descending order (newest first)
    key.extend_from_slice(&(!timestamp as u64).to_be_bytes());
    key.push(b'|');
    key.extend_from_slice(tx_hash.as_bytes());
    key
}

/// Build a prefix key for range scanning all transactions of a wallet address.
fn make_prefix(wallet_address: &str) -> Vec<u8> {
    let addr = wallet_address.to_lowercase();
    let mut prefix = Vec::with_capacity(addr.len() + 1);
    prefix.extend_from_slice(addr.as_bytes());
    prefix.push(b'|');
    prefix
}

/// Build the upper bound for a range scan (prefix with all 0xFF bytes appended).
fn make_prefix_end(wallet_address: &str) -> Vec<u8> {
    let addr = wallet_address.to_lowercase();
    let mut end = Vec::with_capacity(addr.len() + 1 + 20);
    end.extend_from_slice(addr.as_bytes());
    end.push(b'|');
    // Append enough 0xFF bytes to be past any valid key with this prefix
    end.extend_from_slice(&[0xFF; 20]);
    end
}

// =============================================================================
// TxDatabase
// =============================================================================

/// Embedded ACID transaction database.
pub struct TxDatabase {
    db: Database,
}

impl TxDatabase {
    /// Open (or create) the database at the given path.
    pub fn open(path: &Path) -> TxDbResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = Database::create(path)?;

        // Pre-create all tables so later read transactions don't fail
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(TRANSACTIONS)?;
            let _ = write_txn.open_table(WALLET_TX_INDEX)?;
            let _ = write_txn.open_table(ADDRESS_WALLET_MAP)?;
            let _ = write_txn.open_table(INDEXER_STATE)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    // =========================================================================
    // Transaction CRUD
    // =========================================================================

    /// Insert or update a transaction and its index entries.
    ///
    /// `directions` is a list of `(wallet_address, direction)` pairs, e.g.:
    /// `[("0xabc...", "sent"), ("0xdef...", "received")]`
    pub fn upsert_transaction(
        &self,
        tx: &StoredTransaction,
        directions: &[(String, &str)],
    ) -> TxDbResult<()> {
        let json = serde_json::to_vec(tx)?;
        let timestamp = tx.created_at.timestamp();

        let write_txn = self.db.begin_write()?;
        {
            let mut tx_table = write_txn.open_table(TRANSACTIONS)?;
            tx_table.insert(tx.tx_hash.as_str(), json.as_slice())?;

            let mut idx_table = write_txn.open_table(WALLET_TX_INDEX)?;
            for (addr, direction) in directions {
                let key = make_index_key(addr, timestamp, &tx.tx_hash);
                idx_table.insert(key.as_slice(), *direction)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Look up a single transaction by hash.
    pub fn get_transaction(&self, tx_hash: &str) -> TxDbResult<Option<StoredTransaction>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TRANSACTIONS)?;
        match table.get(tx_hash)? {
            Some(value) => {
                let tx: StoredTransaction = serde_json::from_slice(value.value())?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    /// Paginated listing of transactions for a wallet address.
    ///
    /// Returns `(transactions_with_direction, next_cursor)`.
    /// Each item is `(StoredTransaction, direction_string)`.
    pub fn list_by_wallet(
        &self,
        wallet_address: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> TxDbResult<(Vec<(StoredTransaction, String)>, Option<String>)> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(WALLET_TX_INDEX)?;
        let tx_table = read_txn.open_table(TRANSACTIONS)?;

        let prefix = make_prefix(wallet_address);
        let prefix_end = make_prefix_end(wallet_address);

        // Determine scan start: either after cursor or from prefix start
        let start: Vec<u8> = if let Some(cursor_str) = cursor {
            // Cursor is base64(last_index_key) — decode it
            decode_cursor(cursor_str).unwrap_or_else(|| prefix.clone())
        } else {
            prefix.clone()
        };

        let mut results = Vec::with_capacity(limit + 1);
        let range = idx_table.range(start.as_slice()..prefix_end.as_slice())?;

        let mut skip_first = cursor.is_some();
        let mut last_key: Option<Vec<u8>> = None;

        for entry in range {
            let entry = entry?;
            let key_bytes = entry.0.value().to_vec();
            let direction = entry.1.value().to_string();

            // Skip the cursor entry itself
            if skip_first {
                skip_first = false;
                continue;
            }

            // Extract tx_hash from the composite key
            if let Some(tx_hash) = extract_tx_hash_from_key(&key_bytes) {
                if let Some(value) = tx_table.get(tx_hash.as_str())? {
                    let tx: StoredTransaction = serde_json::from_slice(value.value())?;
                    results.push((tx, direction));
                    last_key = Some(key_bytes);
                }
            }

            if results.len() >= limit {
                break;
            }
        }

        // Build next cursor if we hit the limit
        let next_cursor = if results.len() >= limit {
            last_key.map(|k| encode_cursor(&k))
        } else {
            None
        };

        Ok((results, next_cursor))
    }

    /// Update the status of a stored transaction.
    pub fn update_status(
        &self,
        tx_hash: &str,
        status: TxStatus,
        block_number: Option<u64>,
        gas_used: Option<u64>,
    ) -> TxDbResult<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TRANSACTIONS)?;

            // Read existing value and deserialize before mutating
            let existing_bytes = {
                let existing = table
                    .get(tx_hash)?
                    .ok_or_else(|| TxDbError::NotFound(format!("Transaction {tx_hash}")))?;
                existing.value().to_vec()
            };

            let mut tx: StoredTransaction = serde_json::from_slice(&existing_bytes)?;
            tx.status = status;
            tx.block_number = block_number;
            tx.gas_used = gas_used;
            tx.updated_at = chrono::Utc::now();

            let json = serde_json::to_vec(&tx)?;
            table.insert(tx_hash, json.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    // =========================================================================
    // Address ↔ Wallet mapping
    // =========================================================================

    /// Register an on-chain address as belonging to a wallet.
    pub fn register_address(&self, address: &str, wallet_id: &str) -> TxDbResult<()> {
        let addr = address.to_lowercase();
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ADDRESS_WALLET_MAP)?;
            table.insert(addr.as_str(), wallet_id)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Look up which wallet_id owns a given on-chain address.
    pub fn get_wallet_id_for_address(&self, address: &str) -> TxDbResult<Option<String>> {
        let addr = address.to_lowercase();
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ADDRESS_WALLET_MAP)?;
        match table.get(addr.as_str())? {
            Some(v) => Ok(Some(v.value().to_string())),
            None => Ok(None),
        }
    }

    // =========================================================================
    // Indexer checkpoint
    // =========================================================================

    /// Get the last indexed block number for a network.
    pub fn get_last_indexed_block(&self, network: &str) -> TxDbResult<u64> {
        let key = format!("last_block_{network}");
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(INDEXER_STATE)?;
        match table.get(key.as_str())? {
            Some(v) => {
                let bytes = v.value();
                if bytes.len() >= 8 {
                    Ok(u64::from_be_bytes(bytes[..8].try_into().unwrap()))
                } else {
                    Ok(0)
                }
            }
            None => Ok(0),
        }
    }

    /// Persist the last indexed block number for a network.
    pub fn set_last_indexed_block(&self, network: &str, block: u64) -> TxDbResult<()> {
        let key = format!("last_block_{network}");
        let bytes = block.to_be_bytes();
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(INDEXER_STATE)?;
            table.insert(key.as_str(), bytes.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    // =========================================================================
    // Migration
    // =========================================================================

    /// Check whether JSON→redb migration has already been performed.
    pub fn is_migrated(&self) -> TxDbResult<bool> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(INDEXER_STATE)?;
        Ok(table.get("migrated")?.is_some())
    }

    /// Mark the database as having completed JSON→redb migration.
    pub fn mark_migrated(&self) -> TxDbResult<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(INDEXER_STATE)?;
            table.insert("migrated", &[1u8] as &[u8])?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

// =============================================================================
// Migration from JSON files to redb
// =============================================================================

/// Migrate existing JSON-based transaction history into redb.
///
/// This is idempotent — if already migrated, it returns immediately.
pub fn migrate_from_json(
    db: &TxDatabase,
    storage: &EncryptedStorage,
) -> TxDbResult<()> {
    if db.is_migrated()? {
        tracing::info!("Transaction database already migrated, skipping");
        return Ok(());
    }

    tracing::info!("Starting JSON → redb transaction migration");

    let wallets_dir = storage.paths().wallets_dir();
    if !storage.exists(&wallets_dir) {
        tracing::info!("No wallets directory found, marking migration complete");
        db.mark_migrated()?;
        return Ok(());
    }

    let mut total_txs = 0u64;
    let mut total_wallets = 0u64;

    // List all wallet directories
    if let Ok(entries) = std::fs::read_dir(&wallets_dir) {
        for entry in entries.flatten() {
            let wallet_id = entry.file_name().to_string_lossy().to_string();
            let wallet_dir = entry.path();

            // Read wallet metadata for public address
            let meta_path = wallet_dir.join("meta.json");
            if !meta_path.exists() {
                continue;
            }

            let meta_data = match std::fs::read_to_string(&meta_path) {
                Ok(d) => d,
                Err(_) => continue,
            };

            #[derive(Deserialize)]
            struct WalletMeta {
                public_address: String,
            }

            let meta: WalletMeta = match serde_json::from_str(&meta_data) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Register address → wallet_id mapping
            db.register_address(&meta.public_address, &wallet_id)?;

            // Read all transaction JSON files
            let txs_dir = wallet_dir.join("txs");
            if !txs_dir.exists() {
                total_wallets += 1;
                continue;
            }

            if let Ok(tx_entries) = std::fs::read_dir(&txs_dir) {
                for tx_entry in tx_entries.flatten() {
                    let path = tx_entry.path();
                    if path.extension().map_or(true, |e| e != "json") {
                        continue;
                    }

                    let tx_data = match std::fs::read_to_string(&path) {
                        Ok(d) => d,
                        Err(_) => continue,
                    };

                    let tx: StoredTransaction = match serde_json::from_str(&tx_data) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!(
                                path = %path.display(),
                                error = %e,
                                "Skipping malformed transaction file"
                            );
                            continue;
                        }
                    };

                    // Determine direction
                    let direction = if tx.from.to_lowercase() == meta.public_address.to_lowercase() {
                        "sent"
                    } else {
                        "received"
                    };

                    let directions = vec![(meta.public_address.clone(), direction)];
                    if let Err(e) = db.upsert_transaction(&tx, &directions) {
                        tracing::warn!(
                            tx_hash = %tx.tx_hash,
                            error = %e,
                            "Failed to migrate transaction"
                        );
                    } else {
                        total_txs += 1;
                    }
                }
            }

            total_wallets += 1;
        }
    }

    db.mark_migrated()?;
    tracing::info!(
        wallets = total_wallets,
        transactions = total_txs,
        "JSON → redb migration complete"
    );

    Ok(())
}

// =============================================================================
// Cursor Encoding
// =============================================================================

fn encode_cursor(key: &[u8]) -> String {
    // Simple hex encoding for cursor (avoids base64 dependency)
    alloy::hex::encode(key)
}

fn decode_cursor(cursor: &str) -> Option<Vec<u8>> {
    alloy::hex::decode(cursor).ok()
}

/// Extract the tx_hash portion from a composite index key.
///
/// Key format: `address|timestamp_bytes|tx_hash`
fn extract_tx_hash_from_key(key: &[u8]) -> Option<String> {
    // Find the second '|' separator
    let mut pipe_count = 0;
    for (i, &b) in key.iter().enumerate() {
        if b == b'|' {
            pipe_count += 1;
            if pipe_count == 2 {
                return String::from_utf8(key[i + 1..].to_vec()).ok();
            }
        }
    }
    None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::repository::transactions::TokenType;
    use chrono::Utc;

    fn temp_db() -> (TxDatabase, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = TxDatabase::open(&dir.path().join("test.redb")).unwrap();
        (db, dir)
    }

    fn sample_tx(hash: &str) -> StoredTransaction {
        StoredTransaction::new_pending(
            hash.to_string(),
            "wallet-1".to_string(),
            None,
            "0x1111111111111111111111111111111111111111".to_string(),
            "0x2222222222222222222222222222222222222222".to_string(),
            "10.0".to_string(),
            TokenType::Erc20("0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63".to_string()),
            "fuji".to_string(),
            format!("https://testnet.snowtrace.io/tx/{hash}"),
        )
    }

    #[test]
    fn upsert_and_get_transaction() {
        let (db, _dir) = temp_db();
        let tx = sample_tx("0xaaa");
        let dirs = vec![("0x1111111111111111111111111111111111111111".to_string(), "sent")];
        db.upsert_transaction(&tx, &dirs).unwrap();

        let retrieved = db.get_transaction("0xaaa").unwrap().unwrap();
        assert_eq!(retrieved.tx_hash, "0xaaa");
        assert_eq!(retrieved.amount, "10.0");
    }

    #[test]
    fn list_by_wallet_with_pagination() {
        let (db, _dir) = temp_db();
        let addr = "0x1111111111111111111111111111111111111111";

        // Insert 5 transactions
        for i in 0..5 {
            let mut tx = sample_tx(&format!("0x{:04}", i));
            tx.created_at = Utc::now() - chrono::Duration::seconds(5 - i);
            let dirs = vec![(addr.to_string(), "sent")];
            db.upsert_transaction(&tx, &dirs).unwrap();
        }

        // Page 1: limit 2
        let (page1, cursor) = db.list_by_wallet(addr, None, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert!(cursor.is_some());

        // Page 2: limit 2 with cursor
        let (page2, cursor2) = db.list_by_wallet(addr, cursor.as_deref(), 2).unwrap();
        assert_eq!(page2.len(), 2);
        assert!(cursor2.is_some());

        // Page 3: remaining
        let (page3, cursor3) = db.list_by_wallet(addr, cursor2.as_deref(), 2).unwrap();
        assert_eq!(page3.len(), 1);
        assert!(cursor3.is_none());
    }

    #[test]
    fn update_status_works() {
        let (db, _dir) = temp_db();
        let tx = sample_tx("0xbbb");
        let dirs = vec![("0x1111111111111111111111111111111111111111".to_string(), "sent")];
        db.upsert_transaction(&tx, &dirs).unwrap();

        db.update_status("0xbbb", TxStatus::Confirmed, Some(12345), Some(21000)).unwrap();

        let updated = db.get_transaction("0xbbb").unwrap().unwrap();
        assert_eq!(updated.status, TxStatus::Confirmed);
        assert_eq!(updated.block_number, Some(12345));
        assert_eq!(updated.gas_used, Some(21000));
    }

    #[test]
    fn address_wallet_mapping() {
        let (db, _dir) = temp_db();
        let addr = "0xABCD1234567890ABCDEF1234567890ABCDEF1234";
        db.register_address(addr, "wallet-42").unwrap();

        let result = db.get_wallet_id_for_address(addr).unwrap();
        assert_eq!(result, Some("wallet-42".to_string()));

        // Case insensitive
        let result2 = db
            .get_wallet_id_for_address(&addr.to_lowercase())
            .unwrap();
        assert_eq!(result2, Some("wallet-42".to_string()));
    }

    #[test]
    fn indexer_checkpoint() {
        let (db, _dir) = temp_db();
        assert_eq!(db.get_last_indexed_block("fuji").unwrap(), 0);

        db.set_last_indexed_block("fuji", 99999).unwrap();
        assert_eq!(db.get_last_indexed_block("fuji").unwrap(), 99999);
    }

    #[test]
    fn migration_flag() {
        let (db, _dir) = temp_db();
        assert!(!db.is_migrated().unwrap());
        db.mark_migrated().unwrap();
        assert!(db.is_migrated().unwrap());
    }

    #[test]
    fn make_index_key_ordering() {
        // Newer timestamps should produce smaller composite keys (descending)
        let key_old = make_index_key("0xaddr", 1000, "0xtx1");
        let key_new = make_index_key("0xaddr", 2000, "0xtx2");
        assert!(key_new < key_old, "Newer timestamps should sort first");
    }
}
