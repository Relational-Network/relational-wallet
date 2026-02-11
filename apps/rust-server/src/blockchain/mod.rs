// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Blockchain integration module for Avalanche C-Chain.
//!
//! This module provides functionality for:
//! - Querying native AVAX balances
//! - Querying ERC-20 token balances (USDC, etc.)
//! - Transaction signing and broadcasting
//! - Gas estimation

pub mod client;
pub mod erc20;
pub mod signing;
pub mod transactions;
pub mod types;

pub use client::AvaxClient;
pub use signing::wallet_from_pem;
pub use transactions::{format_amount, parse_amount, TxBuilder};
pub use types::*;
