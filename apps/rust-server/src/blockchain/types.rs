// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Blockchain types and constants.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Avalanche network configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Network name for display
    pub name: &'static str,
    /// Chain ID
    pub chain_id: u64,
    /// RPC endpoint URL
    pub rpc_url: &'static str,
    /// Block explorer URL
    pub explorer_url: &'static str,
}

/// Avalanche C-Chain Mainnet configuration.
pub const AVAX_MAINNET: NetworkConfig = NetworkConfig {
    name: "Avalanche C-Chain",
    chain_id: 43114,
    rpc_url: "https://api.avax.network/ext/bc/C/rpc",
    explorer_url: "https://snowtrace.io",
};

/// Avalanche Fuji Testnet configuration.
pub const AVAX_FUJI: NetworkConfig = NetworkConfig {
    name: "Avalanche Fuji Testnet",
    chain_id: 43113,
    rpc_url: "https://api.avax-test.network/ext/bc/C/rpc",
    explorer_url: "https://testnet.snowtrace.io",
};

/// Supported network identifier for this build.
pub const NETWORK_FUJI: &str = "fuji";

/// Validate network input for Fuji-only runtime.
pub fn ensure_fuji_network(raw: Option<&str>) -> Result<(), String> {
    let value = raw.unwrap_or(NETWORK_FUJI).trim().to_ascii_lowercase();
    if value == NETWORK_FUJI {
        Ok(())
    } else {
        Err(format!(
            "Only `{NETWORK_FUJI}` network is supported in this deployment."
        ))
    }
}

/// Token balance information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenBalance {
    /// Token symbol (e.g., "AVAX", "EUROC")
    pub symbol: String,
    /// Token name
    pub name: String,
    /// Balance in smallest unit (wei for native, token decimals for ERC-20)
    pub balance_raw: String,
    /// Balance formatted with decimals
    pub balance_formatted: String,
    /// Number of decimals
    pub decimals: u8,
    /// Contract address (None for native token)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_address: Option<String>,
}

/// Wallet balance response including native and token balances.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletBalanceResponse {
    /// Wallet address
    pub address: String,
    /// Network name
    pub network: String,
    /// Chain ID
    pub chain_id: u64,
    /// Native token balance (AVAX)
    pub native_balance: TokenBalance,
    /// ERC-20 token balances
    pub token_balances: Vec<TokenBalance>,
}

/// Known ERC-20 tokens on Avalanche.
/// TODO: Configure metadata and addresses for actual euro stablecoins when available.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Erc20Token {
    pub symbol: &'static str,
    pub name: &'static str,
    pub decimals: u8,
    /// Mainnet contract address
    pub mainnet_address: Option<&'static str>,
    /// Fuji testnet contract address
    pub fuji_address: Option<&'static str>,
}

/// Euro stablecoin configuration.
/// Note: Replace with actual euro stablecoin contract addresses when deployed.
/// TODO: Deploy a test euro stablecoin on Fuji and add the address here.
#[allow(dead_code)]
pub const EUROC_TOKEN: Erc20Token = Erc20Token {
    symbol: "EUROC",
    name: "Euro Coin",
    decimals: 6,
    // Circle's EUROC on Avalanche mainnet (if available)
    mainnet_address: None, // TODO: Add actual mainnet address
    // Testnet address - deploy a test token or use a known test token
    fuji_address: None, // TODO: Add test token address
};

/// USDC for reference/testing.
pub const USDC_TOKEN: Erc20Token = Erc20Token {
    symbol: "USDC",
    name: "USD Coin",
    decimals: 6,
    // Official USDC on Avalanche C-Chain
    mainnet_address: Some("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E"),
    // Fuji testnet USDC (Circle's test token)
    fuji_address: Some("0x5425890298aed601595a70AB815c96711a31Bc65"),
};

/// Relational Euro (`rEUR`) token deployed on Fuji.
pub const REUR_TOKEN: Erc20Token = Erc20Token {
    symbol: "rEUR",
    name: "Relational Euro",
    decimals: 6,
    mainnet_address: None,
    fuji_address: Some("0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63"),
};
