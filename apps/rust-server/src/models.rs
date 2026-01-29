// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # API Data Models
//!
//! This module defines the request and response data structures used by
//! the REST API. All types derive `Serialize`, `Deserialize`, and `ToSchema`
//! for automatic JSON handling and OpenAPI documentation.
//!
//! ## Wallet Address Type
//!
//! The [`WalletAddress`] newtype wraps Ethereum-style addresses (0x-prefixed,
//! 40 hex characters). It provides type safety and clear semantics.
//!
//! ## Model Categories
//!
//! - **Bookmarks**: Saved wallet addresses for quick access
//! - **Invites**: Invitation codes for new users
//! - **Recurring Payments**: Scheduled payment configurations

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// =============================================================================
// Wallet Address Type
// =============================================================================

/// Ethereum-compatible wallet address wrapper.
///
/// Provides type safety for wallet addresses throughout the API.
/// Format: `0x` followed by 40 hexadecimal characters (20 bytes).
///
/// # Example
///
/// ```rust,ignore
/// let addr = WalletAddress::from("0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalletAddress(pub String);

impl std::fmt::Display for WalletAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WalletAddress {
    fn from(value: String) -> Self {
        WalletAddress(value)
    }
}

impl From<&str> for WalletAddress {
    fn from(value: &str) -> Self {
        WalletAddress(value.to_string())
    }
}

impl From<WalletAddress> for String {
    fn from(value: WalletAddress) -> Self {
        value.0
    }
}

// =============================================================================
// Bookmark Models
// =============================================================================

/// A saved wallet address bookmark.
///
/// Bookmarks allow users to save frequently-used addresses with friendly names
/// for quick access when sending transactions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Bookmark {
    /// Unique identifier for this bookmark.
    pub id: String,
    /// The wallet this bookmark belongs to.
    pub wallet_id: WalletAddress,
    /// User-friendly name for the bookmarked address.
    pub name: String,
    /// The bookmarked wallet address.
    pub address: WalletAddress,
}

/// Request to create a new bookmark.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBookmarkRequest {
    /// The wallet to add the bookmark to (must be owned by the user).
    pub wallet_id: WalletAddress,
    /// User-friendly name for the bookmark.
    pub name: String,
    /// The wallet address to bookmark.
    pub address: WalletAddress,
}

// =============================================================================
// Invite Models
// =============================================================================

/// An invitation code for new users.
///
/// Invites can be used to control access to the wallet service. Each invite
/// code can only be redeemed once.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Invite {
    /// Unique identifier for this invite.
    pub id: String,
    /// The invite code (shared with the invitee).
    pub code: String,
    /// Whether this invite has been used.
    pub redeemed: bool,
}

/// Request to redeem an invite code.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedeemInviteRequest {
    /// The invite ID to redeem.
    pub invite_id: String,
}

// =============================================================================
// Recurring Payment Models
// =============================================================================

/// A scheduled recurring payment configuration.
///
/// Recurring payments allow automatic transfers on a schedule. The actual
/// execution logic is handled by a separate service.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct RecurringPayment {
    /// Unique identifier for this payment schedule.
    pub id: String,
    /// The source wallet for payments.
    pub wallet_id: WalletAddress,
    /// Public key of the wallet (for verification).
    pub wallet_public_key: String,
    /// The recipient address for payments.
    pub recipient: WalletAddress,
    /// Payment amount.
    pub amount: f64,
    /// Currency code (e.g., "AVAX", "USDC").
    pub currency_code: String,
    /// Start date (Unix timestamp in days).
    pub payment_start_date: i32,
    /// Frequency in days between payments.
    pub frequency: i32,
    /// End date (Unix timestamp in days, 0 = no end).
    pub payment_end_date: i32,
    /// Last payment date (Unix timestamp in days).
    pub last_paid_date: i32,
}

/// Request to create a recurring payment schedule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRecurringPaymentRequest {
    /// The source wallet (must be owned by the user).
    pub wallet_id: WalletAddress,
    /// Public key for the wallet.
    pub wallet_public_key: String,
    /// Recipient address for payments.
    pub recipient: WalletAddress,
    /// Payment amount.
    pub amount: f64,
    /// Currency code (e.g., "AVAX", "USDC").
    pub currency_code: String,
    /// Start date (Unix timestamp in days).
    pub payment_start_date: i32,
    /// Frequency in days between payments.
    pub frequency: i32,
    /// End date (Unix timestamp in days, 0 = no end).
    pub payment_end_date: i32,
}

/// Request to update an existing recurring payment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRecurringPaymentRequest {
    /// ID of the recurring payment to update.
    pub recurring_payment_id: String,
    /// Updated source wallet.
    pub wallet_id: WalletAddress,
    /// Updated public key.
    pub wallet_public_key: String,
    /// Updated recipient address.
    pub recipient: WalletAddress,
    /// Updated payment amount.
    pub amount: f64,
    /// Updated currency code.
    pub currency_code: String,
    /// Updated start date.
    pub payment_start_date: i32,
    /// Updated frequency.
    pub frequency: i32,
    /// Updated end date.
    pub payment_end_date: i32,
}

/// Request to update the last paid date for a recurring payment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLastPaidDateRequest {
    /// ID of the recurring payment.
    pub recurring_payment_id: String,
    /// New last paid date (Unix timestamp in days).
    pub last_paid_date: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_address_from_and_into_string() {
        let from_str: WalletAddress = "abc".into();
        assert_eq!(from_str.0, "abc");

        let from_string: WalletAddress = String::from("def").into();
        assert_eq!(from_string.0, "def");

        let to_string: String = WalletAddress("ghi".into()).into();
        assert_eq!(to_string, "ghi");
    }
}
