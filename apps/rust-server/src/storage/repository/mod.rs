// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Repository layer providing typed access to encrypted storage.
//!
//! Each repository provides CRUD operations for a specific entity type,
//! using the EncryptedStorage for all file operations.

pub mod bookmarks;
pub mod invites;
pub mod recurring;
pub mod transactions;
pub mod wallets;

pub use bookmarks::{BookmarkRepository, StoredBookmark};
pub use invites::{InviteRepository, StoredInvite};
pub use recurring::{PaymentFrequency, PaymentStatus, RecurringRepository, StoredRecurringPayment};
pub use transactions::{StoredTransaction, TokenType, TransactionRepository, TxStatus};
pub use wallets::{WalletMetadata, WalletRepository, WalletResponse, WalletStatus};
