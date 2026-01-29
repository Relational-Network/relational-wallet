// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Blockchain integration module for Avalanche C-Chain.
//!
//! This module provides functionality for:
//! - Querying native AVAX balances
//! - Querying ERC-20 token balances (for euro stablecoin)
//! - Transaction signing and broadcasting

pub mod client;
pub mod erc20;
pub mod types;

pub use client::{AvaxClient, AvaxClientError};
pub use types::*;
