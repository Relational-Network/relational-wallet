// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Bookmark {
    pub id: String,
    pub wallet_id: WalletAddress,
    pub name: String,
    pub address: WalletAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Invite {
    pub id: String,
    pub code: String,
    pub redeemed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct RecurringPayment {
    pub id: String,
    pub wallet_id: WalletAddress,
    pub wallet_public_key: String,
    pub recipient: WalletAddress,
    pub amount: f64,
    pub currency_code: String,
    pub payment_start_date: i32,
    pub frequency: i32,
    pub payment_end_date: i32,
    pub last_paid_date: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBookmarkRequest {
    pub wallet_id: WalletAddress,
    pub name: String,
    pub address: WalletAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedeemInviteRequest {
    pub invite_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutofundRequest {
    pub wallet_id: WalletAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRecurringPaymentRequest {
    pub wallet_id: WalletAddress,
    pub wallet_public_key: String,
    pub recipient: WalletAddress,
    pub amount: f64,
    pub currency_code: String,
    pub payment_start_date: i32,
    pub frequency: i32,
    pub payment_end_date: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRecurringPaymentRequest {
    pub recurring_payment_id: String,
    pub wallet_id: WalletAddress,
    pub wallet_public_key: String,
    pub recipient: WalletAddress,
    pub amount: f64,
    pub currency_code: String,
    pub payment_start_date: i32,
    pub frequency: i32,
    pub payment_end_date: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLastPaidDateRequest {
    pub recurring_payment_id: String,
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
