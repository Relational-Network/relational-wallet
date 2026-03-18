// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Shared transaction data types used by the API and tx database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
