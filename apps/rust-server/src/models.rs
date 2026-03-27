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

impl WalletAddress {
    /// Validate that this is a valid Ethereum address:
    /// `0x` prefix followed by exactly 40 hexadecimal characters.
    ///
    /// Call this explicitly when the value is expected to be an Ethereum address
    /// (not a wallet UUID). Returns `Ok(())` on success, `Err` with a message on failure.
    pub fn validate_eth_address(&self) -> Result<(), String> {
        let s = &self.0;
        if !s.starts_with("0x") && !s.starts_with("0X") {
            return Err(format!(
                "Invalid wallet address '{}': must start with '0x'",
                s
            ));
        }
        let hex_part = &s[2..];
        if hex_part.len() != 40 {
            return Err(format!(
                "Invalid wallet address '{}': expected 40 hex chars after '0x', got {}",
                s,
                hex_part.len()
            ));
        }
        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid wallet address '{}': contains non-hex characters",
                s
            ));
        }
        Ok(())
    }
}

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
/// for quick access when sending transactions. Supports both address-based and
/// email-based recipients.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Bookmark {
    /// Unique identifier for this bookmark.
    pub id: String,
    /// The wallet this bookmark belongs to.
    pub wallet_id: WalletAddress,
    /// User-friendly name for the bookmarked address.
    pub name: String,
    /// Recipient type: "address" or "email".
    pub recipient_type: String,
    /// The bookmarked wallet address (when recipient_type=address).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<WalletAddress>,
    /// SHA-256 hash of email (when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_hash: Option<String>,
    /// Masked email for display (when recipient_type=email, e.g. "a***e@example.com").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_display: Option<String>,
}

/// Request to create a new bookmark.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBookmarkRequest {
    /// The wallet to add the bookmark to (must be owned by the user).
    pub wallet_id: WalletAddress,
    /// User-friendly name for the bookmark.
    pub name: String,
    /// Recipient type: "address" or "email". Defaults to "address".
    #[serde(default = "default_recipient_type_address")]
    pub recipient_type: String,
    /// The wallet address to bookmark (required when recipient_type=address).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<WalletAddress>,
    /// SHA-256 hash of email (required when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_hash: Option<String>,
    /// Masked email for display (required when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_display: Option<String>,
}

fn default_recipient_type_address() -> String {
    "address".to_string()
}

// =============================================================================
// Email Resolution Models
// =============================================================================

/// Request to resolve an email hash to check if a wallet exists.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveEmailRequest {
    /// SHA-256 hash of the normalized email (64 hex characters).
    pub email_hash: String,
}

/// Response to email resolution.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveEmailResponse {
    /// Whether a wallet was found for this email.
    pub found: bool,
}

// =============================================================================
// Payment Link Models
// =============================================================================

/// Request to create a payment link.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePaymentLinkRequest {
    /// Recipient type: "address" or "email". Defaults to "address".
    #[serde(default = "default_recipient_type_address")]
    pub recipient_type: String,
    /// SHA-256 hash of the normalized email (required when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_email_hash: Option<String>,
    /// Masked email for display (required when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_display: Option<String>,
    /// Pre-filled amount (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Token type: "native" or "reur" (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Note for the recipient (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Hours until expiry (default: 24).
    #[serde(default = "default_expires_hours")]
    pub expires_hours: u64,
    /// Whether the link can only be used once (default: false).
    #[serde(default)]
    pub single_use: bool,
}

fn default_expires_hours() -> u64 {
    24
}

/// Response after creating a payment link.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePaymentLinkResponse {
    /// Opaque token for the payment link.
    pub token: String,
    /// When the link expires.
    pub expires_at: String,
}

/// Public info returned when resolving a payment link (no auth required).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentLinkInfo {
    /// Recipient type: "address" or "email".
    pub recipient_type: String,
    /// Recipient's public address (when recipient_type=address).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_address: Option<String>,
    /// Recipient email hash (when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_email_hash: Option<String>,
    /// Masked email for display (when recipient_type=email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_display: Option<String>,
    /// Pre-filled amount (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Token type (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// Note from the requester (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_address_from_and_into_string() {
        let from_str: WalletAddress = "0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12".into();
        assert_eq!(from_str.0, "0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12");

        let from_string: WalletAddress =
            String::from("0xABCDEF1234567890abcdef1234567890ABCDEF12").into();
        assert_eq!(from_string.0, "0xABCDEF1234567890abcdef1234567890ABCDEF12");

        let to_string: String =
            WalletAddress("0x0000000000000000000000000000000000000001".into()).into();
        assert_eq!(to_string, "0x0000000000000000000000000000000000000001");
    }

    #[test]
    fn wallet_address_validation_accepts_valid() {
        let addr = WalletAddress::from("0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12");
        assert!(addr.validate_eth_address().is_ok());
    }

    #[test]
    fn wallet_address_validation_rejects_no_prefix() {
        let addr = WalletAddress::from("742d35Cc6634C0532925a3b844Bc9e7595f4aB12");
        assert!(addr.validate_eth_address().is_err());
    }

    #[test]
    fn wallet_address_validation_rejects_short() {
        let addr = WalletAddress::from("0x742d35Cc");
        assert!(addr.validate_eth_address().is_err());
    }

    #[test]
    fn wallet_address_validation_rejects_non_hex() {
        let addr = WalletAddress::from("0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");
        assert!(addr.validate_eth_address().is_err());
    }
}
