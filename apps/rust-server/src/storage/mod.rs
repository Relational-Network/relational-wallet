// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Encrypted Storage Module
//!
//! This module provides persistent storage using **Gramine encrypted files**.
//! All data is stored under `/data` which is mounted as an encrypted filesystem
//! in the Gramine manifest.
//!
//! ## Security Model
//!
//! - Files are encrypted on the host filesystem
//! - Files are transparently decrypted inside the enclave
//! - Encryption keys are derived by Gramine (bound to enclave identity)
//! - Files cannot be read outside the enclave
//! - Files cannot be replayed from another path
//! - Any modification outside the enclave causes read failure
//!
//! ## Storage Layout
//!
//! ```text
//! /data/
//!   wallets/{wallet_id}/
//!     meta.json       # Wallet metadata (owner, address, status)
//!     key.pem         # Private key (NEVER exposed via API)
//!     txs/            # Transaction history (TODO)
//!   bookmarks/
//!     {bookmark_id}.json
//!   invites/
//!     {invite_id}.json
//!   recurring/
//!     {payment_id}.json
//!   audit/
//!     {date}/events.jsonl  # Daily audit logs
//! ```
//!
//! ## Important Notes
//!
//! - This module uses **normal filesystem I/O**
//! - Gramine handles all encryption/decryption transparently
//! - DO NOT implement any crypto in Rust for storage
//! - DO NOT access SGX key devices directly

pub mod audit;
pub mod encrypted_fs;
pub mod ownership;
pub mod paths;
pub mod repository;

pub use audit::{AuditEvent, AuditEventType, AuditRepository};
pub use encrypted_fs::{EncryptedStorage, StorageError, StorageResult};
pub use ownership::{OwnedResource, OwnershipEnforcer};
pub use paths::StoragePaths;
pub use repository::{
    BookmarkRepository, InviteRepository, RecurringRepository, StoredBookmark, StoredTransaction,
    TokenType, TransactionRepository, TxStatus, WalletMetadata, WalletRepository, WalletResponse,
    WalletStatus,
};
