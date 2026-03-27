// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Repository layer providing typed access to encrypted storage.
//!
//! Each repository provides CRUD operations for a specific entity type,
//! using the EncryptedStorage for all file operations.

pub mod bookmarks;
pub mod email_index;
pub mod fiat;
pub mod payment_links;
pub mod service_wallet;
pub mod transactions;
pub mod wallets;

pub use bookmarks::{BookmarkRepository, RecipientType, StoredBookmark};
pub use email_index::EmailIndexRepository;
pub use fiat::{FiatDirection, FiatRequestRepository, FiatRequestStatus, StoredFiatRequest};
pub use payment_links::{PaymentLinkData, PaymentLinkRepository};
pub use service_wallet::{FiatServiceWalletMetadata, FiatServiceWalletRepository};
pub use transactions::{StoredTransaction, TokenType, TxStatus};
pub use wallets::{WalletMetadata, WalletRepository, WalletResponse, WalletStatus};
