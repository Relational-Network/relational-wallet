// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! ERC-20 token contract interactions.

use std::str::FromStr;

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    sol,
};

use super::client::AvaxClientError;
use super::types::TokenBalance;

// Define the ERC-20 interface using alloy's sol! macro
sol! {
    #[sol(rpc)]
    interface IERC20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}

/// ERC-20 contract wrapper.
pub struct Erc20Contract<P> {
    contract: IERC20::IERC20Instance<P>,
    address: Address,
}

impl<P: Provider + Clone> Erc20Contract<P> {
    /// Create a new ERC-20 contract instance.
    pub fn new(provider: &P, contract_address: &str) -> Result<Self, AvaxClientError> {
        let address = Address::from_str(contract_address)
            .map_err(|e| AvaxClientError::InvalidAddress(e.to_string()))?;

        let contract = IERC20::new(address, provider.clone());

        Ok(Self { contract, address })
    }

    /// Get the token name.
    pub async fn name(&self) -> Result<String, AvaxClientError> {
        let result = self
            .contract
            .name()
            .call()
            .await
            .map_err(|e| AvaxClientError::ContractError(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get the token symbol.
    pub async fn symbol(&self) -> Result<String, AvaxClientError> {
        let result = self
            .contract
            .symbol()
            .call()
            .await
            .map_err(|e| AvaxClientError::ContractError(e.to_string()))?;
        Ok(result.to_string())
    }

    /// Get the token decimals.
    pub async fn decimals(&self) -> Result<u8, AvaxClientError> {
        let result = self
            .contract
            .decimals()
            .call()
            .await
            .map_err(|e| AvaxClientError::ContractError(e.to_string()))?;
        Ok(result)
    }

    /// Get the balance of an address.
    pub async fn balance_of(&self, wallet_address: &str) -> Result<TokenBalance, AvaxClientError> {
        let addr = Address::from_str(wallet_address)
            .map_err(|e| AvaxClientError::InvalidAddress(e.to_string()))?;

        // Fetch token metadata and balance - use explicit types to help inference
        let name: String = self.name().await.unwrap_or_else(|_| "Unknown".to_string());
        let symbol: String = self.symbol().await.unwrap_or_else(|_| "???".to_string());
        let decimals: u8 = self.decimals().await.unwrap_or(18);

        let balance: U256 = self
            .contract
            .balanceOf(addr)
            .call()
            .await
            .map_err(|e| AvaxClientError::ContractError(e.to_string()))?;

        Ok(TokenBalance {
            symbol,
            name,
            balance_raw: balance.to_string(),
            balance_formatted: format_token_balance(balance, decimals),
            decimals,
            contract_address: Some(format!("{:?}", self.address)),
        })
    }
}

/// Format a token balance with the specified decimals.
fn format_token_balance(balance: U256, decimals: u8) -> String {
    if balance.is_zero() {
        return "0".to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = balance / divisor;
    let remainder = balance % divisor;

    if remainder.is_zero() {
        whole.to_string()
    } else {
        let decimal_str = format!("{:0>width$}", remainder, width = decimals as usize);
        let trimmed = decimal_str.trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{}.{}", whole, &trimmed[..trimmed.len().min(6)])
        }
    }
}
