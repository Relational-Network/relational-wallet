// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Transaction building and broadcasting for Avalanche C-Chain.
//!
//! This module provides EIP-1559 transaction building, gas estimation,
//! and broadcasting capabilities for both native AVAX and ERC-20 transfers.

use std::str::FromStr;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};

use super::client::AvaxClientError;
use super::erc20::IERC20;
use super::types::NetworkConfig;

/// Gas estimation result.
#[derive(Debug, Clone)]
pub struct GasEstimate {
    /// Estimated gas limit
    pub gas_limit: u64,
    /// Current max fee per gas (base fee + priority fee)
    pub max_fee_per_gas: u128,
    /// Max priority fee per gas (tip)
    pub max_priority_fee_per_gas: u128,
    /// Total estimated cost in wei
    pub estimated_cost_wei: U256,
}

/// Transaction send result.
#[derive(Debug, Clone)]
pub struct SendResult {
    /// Transaction hash
    pub tx_hash: String,
    /// Explorer URL for the transaction
    pub explorer_url: String,
}

/// Transaction receipt after confirmation.
#[derive(Debug, Clone)]
pub struct TxReceipt {
    /// Transaction hash
    #[allow(dead_code)]
    pub tx_hash: String,
    /// Block number where transaction was included
    pub block_number: u64,
    /// Gas actually used
    pub gas_used: u64,
    /// Whether the transaction was successful
    pub success: bool,
}

/// Transaction builder for Avalanche C-Chain.
pub struct TxBuilder {
    network: NetworkConfig,
    provider: alloy::providers::fillers::FillProvider<
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::JoinFill<
                alloy::providers::Identity,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::GasFiller,
                    alloy::providers::fillers::JoinFill<
                        alloy::providers::fillers::BlobGasFiller,
                        alloy::providers::fillers::JoinFill<
                            alloy::providers::fillers::NonceFiller,
                            alloy::providers::fillers::ChainIdFiller,
                        >,
                    >,
                >,
            >,
            alloy::providers::fillers::WalletFiller<EthereumWallet>,
        >,
        alloy::providers::RootProvider<alloy::network::Ethereum>,
    >,
}

impl TxBuilder {
    /// Create a new transaction builder with signing capabilities.
    pub async fn new(
        network: NetworkConfig,
        wallet: EthereumWallet,
    ) -> Result<Self, AvaxClientError> {
        let url: url::Url = network
            .rpc_url
            .parse()
            .map_err(|e: url::ParseError| AvaxClientError::InvalidRpcUrl(e.to_string()))?;

        let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

        Ok(Self { network, provider })
    }

    /// Estimate gas for a native AVAX transfer.
    pub async fn estimate_native_transfer(
        &self,
        from: &str,
        to: &str,
        amount_wei: U256,
    ) -> Result<GasEstimate, AvaxClientError> {
        let from_addr = Address::from_str(from)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid from address: {}", e)))?;
        let to_addr = Address::from_str(to)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid to address: {}", e)))?;

        let tx = TransactionRequest::default()
            .from(from_addr)
            .to(to_addr)
            .value(amount_wei);

        self.estimate_gas_for_tx(tx).await
    }

    /// Estimate gas for an ERC-20 token transfer.
    pub async fn estimate_token_transfer(
        &self,
        from: &str,
        to: &str,
        token_address: &str,
        amount: U256,
    ) -> Result<GasEstimate, AvaxClientError> {
        let from_addr = Address::from_str(from)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid from address: {}", e)))?;
        let to_addr = Address::from_str(to)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid to address: {}", e)))?;
        let token_addr = Address::from_str(token_address).map_err(|e| {
            AvaxClientError::InvalidAddress(format!("Invalid token address: {}", e))
        })?;

        // Encode the transfer(to, amount) call
        let call = IERC20::transferCall {
            to: to_addr,
            amount,
        };
        let data = call.abi_encode();

        let tx = TransactionRequest::default()
            .from(from_addr)
            .to(token_addr)
            .input(data.into());

        self.estimate_gas_for_tx(tx).await
    }

    /// Internal gas estimation helper.
    async fn estimate_gas_for_tx(
        &self,
        tx: TransactionRequest,
    ) -> Result<GasEstimate, AvaxClientError> {
        // Get gas estimate
        let gas_limit = self
            .provider
            .estimate_gas(tx.clone())
            .await
            .map_err(|e| AvaxClientError::RpcError(format!("Gas estimation failed: {}", e)))?;

        // Get current gas prices
        let (max_fee_per_gas, max_priority_fee_per_gas) = self.get_gas_prices().await?;

        // Calculate estimated cost
        let estimated_cost_wei = U256::from(gas_limit) * U256::from(max_fee_per_gas);

        Ok(GasEstimate {
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            estimated_cost_wei,
        })
    }

    /// Get current gas prices from the network.
    async fn get_gas_prices(&self) -> Result<(u128, u128), AvaxClientError> {
        // Get base fee from latest block
        let block = self
            .provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .await
            .map_err(|e| AvaxClientError::RpcError(format!("Failed to get block: {}", e)))?
            .ok_or_else(|| AvaxClientError::RpcError("No latest block".to_string()))?;

        let base_fee: u128 = block
            .header
            .base_fee_per_gas
            .map(|f| f as u128)
            .unwrap_or(25_000_000_000u128); // 25 gwei default

        // Standard priority fee for Avalanche
        let priority_fee: u128 = 1_500_000_000; // 1.5 gwei

        // Max fee = 2 * base_fee + priority_fee (allows for base fee increase)
        let max_fee = base_fee.saturating_mul(2).saturating_add(priority_fee);

        Ok((max_fee, priority_fee))
    }

    /// Send a native AVAX transfer.
    ///
    /// # Arguments
    /// * `to` - Recipient address
    /// * `amount_wei` - Amount in wei
    /// * `gas_limit` - Optional gas limit override
    /// * `max_priority_fee` - Optional priority fee override
    pub async fn send_native(
        &self,
        to: &str,
        amount_wei: U256,
        gas_limit: Option<u64>,
        max_priority_fee: Option<u128>,
    ) -> Result<SendResult, AvaxClientError> {
        let to_addr = Address::from_str(to)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid to address: {}", e)))?;

        let (max_fee_per_gas, default_priority_fee) = self.get_gas_prices().await?;
        let priority_fee = max_priority_fee.unwrap_or(default_priority_fee);

        let mut tx = TransactionRequest::default()
            .to(to_addr)
            .value(amount_wei)
            .max_fee_per_gas(max_fee_per_gas)
            .max_priority_fee_per_gas(priority_fee);

        if let Some(limit) = gas_limit {
            tx = tx.gas_limit(limit);
        }

        self.send_transaction(tx).await
    }

    /// Send an ERC-20 token transfer.
    ///
    /// # Arguments
    /// * `to` - Recipient address
    /// * `token_address` - ERC-20 contract address
    /// * `amount` - Amount in token's smallest unit
    /// * `gas_limit` - Optional gas limit override
    /// * `max_priority_fee` - Optional priority fee override
    pub async fn send_token(
        &self,
        to: &str,
        token_address: &str,
        amount: U256,
        gas_limit: Option<u64>,
        max_priority_fee: Option<u128>,
    ) -> Result<SendResult, AvaxClientError> {
        let to_addr = Address::from_str(to)
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid to address: {}", e)))?;
        let token_addr = Address::from_str(token_address).map_err(|e| {
            AvaxClientError::InvalidAddress(format!("Invalid token address: {}", e))
        })?;

        // Encode the transfer(to, amount) call
        let call = IERC20::transferCall {
            to: to_addr,
            amount,
        };
        let data = call.abi_encode();

        let (max_fee_per_gas, default_priority_fee) = self.get_gas_prices().await?;
        let priority_fee = max_priority_fee.unwrap_or(default_priority_fee);

        let mut tx = TransactionRequest::default()
            .to(token_addr)
            .input(data.into())
            .max_fee_per_gas(max_fee_per_gas)
            .max_priority_fee_per_gas(priority_fee);

        if let Some(limit) = gas_limit {
            tx = tx.gas_limit(limit);
        }

        self.send_transaction(tx).await
    }

    /// Internal helper to send a transaction and return the hash.
    async fn send_transaction(
        &self,
        tx: TransactionRequest,
    ) -> Result<SendResult, AvaxClientError> {
        let pending =
            self.provider.send_transaction(tx).await.map_err(|e| {
                AvaxClientError::TransactionFailed(format!("Failed to send: {}", e))
            })?;

        let tx_hash = format!("{:?}", pending.tx_hash());
        let explorer_url = format!("{}/tx/{}", self.network.explorer_url, tx_hash);

        Ok(SendResult {
            tx_hash,
            explorer_url,
        })
    }

    /// Wait for a transaction to be confirmed and return the receipt.
    /// TODO: Transaction status polling
    #[allow(dead_code)]
    pub async fn wait_for_confirmation(&self, tx_hash: &str) -> Result<TxReceipt, AvaxClientError> {
        let hash = tx_hash
            .parse()
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid tx hash: {}", e)))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AvaxClientError::RpcError(format!("Failed to get receipt: {}", e)))?
            .ok_or_else(|| AvaxClientError::RpcError("Transaction not found".to_string()))?;

        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: receipt.block_number.unwrap_or(0),
            gas_used: receipt.gas_used as u64,
            success: receipt.status(),
        })
    }

    /// Get the transaction status by checking for a receipt.
    pub async fn get_transaction_status(
        &self,
        tx_hash: &str,
    ) -> Result<Option<TxReceipt>, AvaxClientError> {
        let hash = tx_hash
            .parse()
            .map_err(|e| AvaxClientError::InvalidAddress(format!("Invalid tx hash: {}", e)))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AvaxClientError::RpcError(format!("Failed to get receipt: {}", e)))?;

        Ok(receipt.map(|r| TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: r.block_number.unwrap_or(0),
            gas_used: r.gas_used as u64,
            success: r.status(),
        }))
    }

    /// Get the network configuration.
    /// TODO: Transaction polling
    #[allow(dead_code)]
    pub fn network(&self) -> &NetworkConfig {
        &self.network
    }
}

/// Parse a human-readable amount to wei (or token units).
///
/// # Arguments
/// * `amount` - Amount as a string (e.g., "1.5")
/// * `decimals` - Number of decimals (18 for AVAX, 6 for USDC)
///
/// # Returns
/// * `Ok(U256)` - Amount in smallest unit
/// * `Err` - If parsing fails
pub fn parse_amount(amount: &str, decimals: u8) -> Result<U256, AvaxClientError> {
    let parts: Vec<&str> = amount.split('.').collect();

    if parts.len() > 2 {
        return Err(AvaxClientError::InvalidAddress(
            "Invalid amount format".to_string(),
        ));
    }

    let whole = parts[0]
        .parse::<u128>()
        .map_err(|_| AvaxClientError::InvalidAddress("Invalid whole number".to_string()))?;

    let decimal_part = if parts.len() == 2 {
        let dec_str = parts[1];
        if dec_str.len() > decimals as usize {
            return Err(AvaxClientError::InvalidAddress(format!(
                "Too many decimal places (max {})",
                decimals
            )));
        }
        // Pad with zeros to match decimals
        let padded = format!("{:0<width$}", dec_str, width = decimals as usize);
        padded
            .parse::<u128>()
            .map_err(|_| AvaxClientError::InvalidAddress("Invalid decimal".to_string()))?
    } else {
        0u128
    };

    let multiplier = 10u128.pow(decimals as u32);
    let total = whole
        .checked_mul(multiplier)
        .and_then(|w| w.checked_add(decimal_part))
        .ok_or_else(|| AvaxClientError::InvalidAddress("Amount overflow".to_string()))?;

    Ok(U256::from(total))
}

/// Format wei (or token units) to human-readable amount.
pub fn format_amount(amount: U256, decimals: u8) -> String {
    if amount.is_zero() {
        return "0".to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = amount / divisor;
    let remainder = amount % divisor;

    if remainder.is_zero() {
        whole.to_string()
    } else {
        let decimal_str = format!("{:0>width$}", remainder, width = decimals as usize);
        let trimmed = decimal_str.trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_amount_whole() {
        let result = parse_amount("1", 18).unwrap();
        assert_eq!(result, U256::from(1_000_000_000_000_000_000u64));
    }

    #[test]
    fn test_parse_amount_decimal() {
        let result = parse_amount("1.5", 18).unwrap();
        assert_eq!(result, U256::from(1_500_000_000_000_000_000u64));
    }

    #[test]
    fn test_parse_amount_usdc() {
        // 1.5 USDC = 1_500_000 (6 decimals)
        let result = parse_amount("1.5", 6).unwrap();
        assert_eq!(result, U256::from(1_500_000u64));
    }

    #[test]
    fn test_parse_amount_small() {
        let result = parse_amount("0.001", 18).unwrap();
        assert_eq!(result, U256::from(1_000_000_000_000_000u64));
    }

    #[test]
    fn test_format_amount() {
        let one_avax = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_amount(one_avax, 18), "1");

        let one_and_half = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(format_amount(one_and_half, 18), "1.5");
    }

    #[test]
    fn test_format_amount_usdc() {
        let one_usdc = U256::from(1_000_000u64);
        assert_eq!(format_amount(one_usdc, 6), "1");

        let one_and_half = U256::from(1_500_000u64);
        assert_eq!(format_amount(one_and_half, 6), "1.5");
    }
}
