// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Avalanche C-Chain client for blockchain interactions.

use std::str::FromStr;

use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    signers::local::PrivateKeySigner,
};

use super::erc20::Erc20Contract;
use super::types::*;

/// HTTP provider type for Avalanche C-Chain (with all fillers).
type HttpProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider<Ethereum>,
>;

/// Avalanche C-Chain client.
pub struct AvaxClient {
    /// Network configuration
    network: NetworkConfig,
    /// Alloy HTTP provider
    provider: HttpProvider,
}

impl AvaxClient {
    /// Create a new client for the specified network.
    pub async fn new(network: NetworkConfig) -> Result<Self, AvaxClientError> {
        let url: url::Url = network.rpc_url.parse().map_err(|e: url::ParseError| {
            AvaxClientError::InvalidRpcUrl(e.to_string())
        })?;
        
        let provider = ProviderBuilder::new()
            .connect_http(url);

        Ok(Self { 
            network, 
            provider,
        })
    }

    /// Create a client for Avalanche Fuji testnet.
    pub async fn fuji() -> Result<Self, AvaxClientError> {
        Self::new(AVAX_FUJI).await
    }

    /// Create a client for Avalanche mainnet.
    pub async fn mainnet() -> Result<Self, AvaxClientError> {
        Self::new(AVAX_MAINNET).await
    }

    /// Get the native AVAX balance for an address.
    pub async fn get_native_balance(&self, address: &str) -> Result<TokenBalance, AvaxClientError> {
        let addr = Address::from_str(address)
            .map_err(|e| AvaxClientError::InvalidAddress(e.to_string()))?;

        let balance = self.provider.get_balance(addr).await
            .map_err(|e| AvaxClientError::RpcError(e.to_string()))?;

        Ok(TokenBalance {
            symbol: "AVAX".to_string(),
            name: "Avalanche".to_string(),
            balance_raw: balance.to_string(),
            balance_formatted: format_balance(balance, 18),
            decimals: 18,
            contract_address: None,
        })
    }

    /// Get the ERC-20 token balance for an address.
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        token_address: &str,
    ) -> Result<TokenBalance, AvaxClientError> {
        let contract = Erc20Contract::new(&self.provider, token_address)?;
        contract.balance_of(wallet_address).await
    }

    /// Get all balances (native + configured tokens) for a wallet.
    pub async fn get_wallet_balances(
        &self,
        wallet_address: &str,
        token_addresses: &[&str],
    ) -> Result<WalletBalanceResponse, AvaxClientError> {
        // Get native balance
        let native_balance = self.get_native_balance(wallet_address).await?;

        // Get token balances
        let mut token_balances = Vec::new();
        for token_addr in token_addresses {
            match self.get_token_balance(wallet_address, token_addr).await {
                Ok(balance) => token_balances.push(balance),
                Err(e) => {
                    tracing::warn!(
                        "Failed to get balance for token {}: {}",
                        token_addr,
                        e
                    );
                    // Continue with other tokens
                }
            }
        }

        Ok(WalletBalanceResponse {
            address: wallet_address.to_string(),
            network: self.network.name.to_string(),
            chain_id: self.network.chain_id,
            native_balance,
            token_balances,
        })
    }

    /// Get the current block number.
    pub async fn get_block_number(&self) -> Result<u64, AvaxClientError> {
        self.provider.get_block_number().await
            .map_err(|e| AvaxClientError::RpcError(e.to_string()))
    }

    /// Get the network configuration.
    pub fn network(&self) -> &NetworkConfig {
        &self.network
    }

    /// Create a signer from a private key (hex string without 0x prefix).
    ///
    /// # Arguments
    /// * `private_key_hex` - Hex-encoded private key (64 characters, no 0x prefix)
    ///
    /// # Returns
    /// A `PrivateKeySigner` that can be used to sign transactions.
    pub fn create_signer(private_key_hex: &str) -> Result<PrivateKeySigner, AvaxClientError> {
        // Use alloy's hex decoding (from alloy-primitives)
        let key_bytes = alloy::hex::decode(private_key_hex)
            .map_err(|e| AvaxClientError::InvalidPrivateKey(e.to_string()))?;

        PrivateKeySigner::from_slice(&key_bytes)
            .map_err(|e| AvaxClientError::InvalidPrivateKey(e.to_string()))
    }

    /// Create an Ethereum wallet from a signer.
    pub fn create_wallet(signer: PrivateKeySigner) -> EthereumWallet {
        EthereumWallet::from(signer)
    }
}

/// Format a balance with the specified number of decimals.
fn format_balance(balance: U256, decimals: u8) -> String {
    if balance.is_zero() {
        return "0".to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = balance / divisor;
    let remainder = balance % divisor;

    if remainder.is_zero() {
        whole.to_string()
    } else {
        // Format with up to 6 decimal places
        let decimal_str = format!("{:0>width$}", remainder, width = decimals as usize);
        let trimmed = decimal_str.trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{}.{}", whole, &trimmed[..trimmed.len().min(6)])
        }
    }
}

/// Errors that can occur during blockchain operations.
#[derive(Debug, thiserror::Error)]
pub enum AvaxClientError {
    #[error("Invalid RPC URL: {0}")]
    InvalidRpcUrl(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Contract error: {0}")]
    ContractError(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_balance() {
        // 1 AVAX = 1e18 wei
        let one_avax = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_balance(one_avax, 18), "1");

        // 0.5 AVAX
        let half_avax = U256::from(500_000_000_000_000_000u64);
        assert_eq!(format_balance(half_avax, 18), "0.5");

        // 1.23456789 AVAX (truncated to 6 decimals)
        let complex = U256::from(1_234_567_890_000_000_000u64);
        assert_eq!(format_balance(complex, 18), "1.234567");

        // Zero
        assert_eq!(format_balance(U256::ZERO, 18), "0");

        // 1 USDC = 1e6
        let one_usdc = U256::from(1_000_000u64);
        assert_eq!(format_balance(one_usdc, 6), "1");
    }
}
