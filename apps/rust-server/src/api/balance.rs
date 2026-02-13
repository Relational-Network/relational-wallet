// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Blockchain balance query endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    auth::Auth,
    blockchain::{ensure_fuji_network, AvaxClient, WalletBalanceResponse, REUR_TOKEN, USDC_TOKEN},
    error::ApiError,
    state::AppState,
    storage::{WalletRepository, WalletStatus},
};

/// Query parameters for balance request.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BalanceQuery {
    /// Network to query. Only "fuji" is supported.
    #[param(default = "fuji")]
    pub network: Option<String>,
    /// Additional token contract addresses to query (comma-separated)
    pub tokens: Option<String>,
}

/// Balance response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceResponse {
    /// Wallet ID
    pub wallet_id: String,
    /// Balance information
    #[serde(flatten)]
    pub balance: WalletBalanceResponse,
}

/// Get the balance of a wallet on the Avalanche C-Chain.
///
/// Returns native AVAX balance and any configured ERC-20 token balances.
#[utoipa::path(
    get,
    path = "/v1/wallets/{wallet_id}/balance",
    tag = "Wallets",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID"),
        BalanceQuery
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Balance retrieved successfully", body = BalanceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found"),
        (status = 503, description = "Blockchain network unavailable")
    )
)]
pub async fn get_wallet_balance(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Query(query): Query<BalanceQuery>,
) -> Result<Json<BalanceResponse>, ApiError> {
    // Get wallet from storage
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(&storage);
    let wallet = wallet_repo.get(&wallet_id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => ApiError::not_found("Wallet not found"),
        _ => ApiError::internal(&format!("Failed to access storage: {}", e)),
    })?;

    // Verify ownership
    if wallet.owner_user_id != user.user_id {
        return Err(ApiError::forbidden("You do not own this wallet"));
    }

    // Check wallet status
    if wallet.status == WalletStatus::Deleted {
        return Err(ApiError::not_found("Wallet has been deleted"));
    }

    if wallet.status == WalletStatus::Suspended {
        return Err(ApiError::forbidden("Wallet is suspended"));
    }

    ensure_fuji_network(query.network.as_deref()).map_err(ApiError::bad_request)?;
    let client = AvaxClient::fuji().await.map_err(|e| {
        ApiError::service_unavailable(&format!("Failed to connect to blockchain: {}", e))
    })?;

    // Build list of token addresses to query
    let mut token_addresses: Vec<&str> = Vec::new();

    // Add known demo tokens for Fuji.
    if let Some(addr) = USDC_TOKEN.fuji_address {
        token_addresses.push(addr);
    }
    if let Some(addr) = REUR_TOKEN.fuji_address {
        token_addresses.push(addr);
    }

    // Add any custom token addresses from query
    // Note: We can't easily add runtime strings to a Vec<&str>, so we'll handle this separately
    let custom_tokens: Vec<String> = query
        .tokens
        .as_deref()
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Query balances
    let balance = client
        .get_wallet_balances(&wallet.public_address, &token_addresses)
        .await
        .map_err(|e| ApiError::service_unavailable(&format!("Failed to query balance: {}", e)))?;

    // Query custom tokens separately
    let mut final_balance = balance;
    for token_addr in custom_tokens {
        if token_addr.starts_with("0x") {
            match client
                .get_token_balance(&wallet.public_address, &token_addr)
                .await
            {
                Ok(token_balance) => {
                    final_balance.token_balances.push(token_balance);
                }
                Err(e) => {
                    tracing::warn!("Failed to query token {}: {}", token_addr, e);
                    // Continue with other tokens
                }
            }
        }
    }

    Ok(Json(BalanceResponse {
        wallet_id: wallet.wallet_id,
        balance: final_balance,
    }))
}

/// Native token balance response (simpler version).
#[derive(Debug, Serialize, ToSchema)]
pub struct NativeBalanceResponse {
    /// Wallet ID
    pub wallet_id: String,
    /// Public address
    pub address: String,
    /// Network name
    pub network: String,
    /// AVAX balance in wei
    pub balance_wei: String,
    /// AVAX balance formatted
    pub balance: String,
}

/// Get only the native AVAX balance of a wallet (faster).
#[utoipa::path(
    get,
    path = "/v1/wallets/{wallet_id}/balance/native",
    tag = "Wallets",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID"),
        BalanceQuery
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Native balance retrieved successfully", body = NativeBalanceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found"),
        (status = 503, description = "Blockchain network unavailable")
    )
)]
pub async fn get_native_balance(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Query(query): Query<BalanceQuery>,
) -> Result<Json<NativeBalanceResponse>, ApiError> {
    // Get wallet from storage
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(&storage);
    let wallet = wallet_repo.get(&wallet_id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => ApiError::not_found("Wallet not found"),
        _ => ApiError::internal(&format!("Failed to access storage: {}", e)),
    })?;

    // Verify ownership
    if wallet.owner_user_id != user.user_id {
        return Err(ApiError::forbidden("You do not own this wallet"));
    }

    // Check wallet status
    if wallet.status == WalletStatus::Deleted {
        return Err(ApiError::not_found("Wallet has been deleted"));
    }

    ensure_fuji_network(query.network.as_deref()).map_err(ApiError::bad_request)?;
    let client = AvaxClient::fuji().await.map_err(|e| {
        ApiError::service_unavailable(&format!("Failed to connect to blockchain: {}", e))
    })?;

    // Query native balance
    let balance = client
        .get_native_balance(&wallet.public_address)
        .await
        .map_err(|e| ApiError::service_unavailable(&format!("Failed to query balance: {}", e)))?;

    Ok(Json(NativeBalanceResponse {
        wallet_id: wallet.wallet_id,
        address: wallet.public_address,
        network: client.network().name.to_string(),
        balance_wei: balance.balance_raw,
        balance: balance.balance_formatted,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_query_defaults() {
        let query = BalanceQuery {
            network: None,
            tokens: None,
        };
        assert!(query.network.is_none());
        assert!(query.tokens.is_none());
    }
}
