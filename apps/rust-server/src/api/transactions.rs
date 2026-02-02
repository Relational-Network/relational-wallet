// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Transaction endpoints for signing and sending transactions.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    auth::Auth,
    blockchain::{
        format_amount, parse_amount, wallet_from_pem, AvaxClient, TxBuilder, AVAX_FUJI,
        AVAX_MAINNET, USDC_TOKEN,
    },
    error::ApiError,
    state::AppState,
    storage::{
        AuditEvent, AuditEventType, AuditRepository, StoredTransaction, TokenType, TransactionRepository,
        TxStatus, WalletRepository, WalletStatus,
    },
};

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to estimate gas for a transaction.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EstimateGasRequest {
    /// Recipient address (0x + 40 hex chars)
    pub to: String,
    /// Amount to send in human-readable format (e.g., "1.5")
    pub amount: String,
    /// Token type: "native" for AVAX or contract address for ERC-20
    #[serde(default = "default_native")]
    pub token: String,
    /// Network: "fuji" (default) or "mainnet"
    #[serde(default = "default_fuji")]
    pub network: String,
}

fn default_native() -> String {
    "native".to_string()
}

fn default_fuji() -> String {
    "fuji".to_string()
}

/// Gas estimation response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EstimateGasResponse {
    /// Estimated gas limit
    pub gas_limit: String,
    /// Max fee per gas in wei
    pub max_fee_per_gas: String,
    /// Max priority fee per gas in wei
    pub max_priority_fee_per_gas: String,
    /// Total estimated cost in wei
    pub estimated_cost_wei: String,
    /// Total estimated cost in AVAX
    pub estimated_cost: String,
}

/// Request to send a transaction.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SendTransactionRequest {
    /// Recipient address (0x + 40 hex chars)
    pub to: String,
    /// Amount to send in human-readable format (e.g., "1.5")
    pub amount: String,
    /// Token type: "native" for AVAX or contract address for ERC-20
    #[serde(default = "default_native")]
    pub token: String,
    /// Network: "fuji" (default) or "mainnet"
    #[serde(default = "default_fuji")]
    pub network: String,
    /// Optional gas limit override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    /// Optional max priority fee per gas override (in wei)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<String>,
}

/// Transaction send response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SendTransactionResponse {
    /// Transaction hash
    pub tx_hash: String,
    /// Current status
    pub status: String,
    /// Block explorer URL
    pub explorer_url: String,
}

/// Query parameters for transaction list.
#[derive(Debug, Deserialize, IntoParams)]
pub struct TransactionListQuery {
    /// Network filter: "fuji" or "mainnet"
    pub network: Option<String>,
    /// Maximum number of results (default: 50)
    #[param(default = 50)]
    pub limit: Option<usize>,
}

/// Transaction list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionListResponse {
    /// List of transactions
    pub transactions: Vec<TransactionSummary>,
}

/// Transaction summary for list view.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionSummary {
    /// Transaction hash
    pub tx_hash: String,
    /// Status: pending, confirmed, failed
    pub status: String,
    /// Direction: "sent" or "received"
    pub direction: String,
    /// Sender address
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Amount sent
    pub amount: String,
    /// Token type
    pub token: String,
    /// Network
    pub network: String,
    /// Block number (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// Block explorer URL
    pub explorer_url: String,
    /// Timestamp
    pub timestamp: String,
}

/// Transaction status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionStatusResponse {
    /// Transaction hash
    pub tx_hash: String,
    /// Status: pending, confirmed, failed
    pub status: String,
    /// Block number (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// Number of confirmations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u64>,
    /// Gas used (if confirmed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<String>,
    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Validate an Ethereum address.
fn validate_address(address: &str) -> Result<(), ApiError> {
    if !address.starts_with("0x") {
        return Err(ApiError::bad_request("Address must start with 0x"));
    }
    if address.len() != 42 {
        return Err(ApiError::bad_request(
            "Address must be 42 characters (0x + 40 hex)",
        ));
    }
    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request("Address must contain only hex characters"));
    }
    Ok(())
}

/// Get decimals for a token.
fn get_token_decimals(token: &str) -> u8 {
    if token == "native" {
        18 // AVAX
    } else if token.eq_ignore_ascii_case(USDC_TOKEN.fuji_address.unwrap_or(""))
        || token.eq_ignore_ascii_case(USDC_TOKEN.mainnet_address.unwrap_or(""))
    {
        6 // USDC
    } else {
        18 // Default to 18 for unknown tokens
    }
}

/// Convert StoredTransaction to TransactionSummary with direction.
fn to_summary_with_direction(tx: &StoredTransaction, direction: &str) -> TransactionSummary {
    let token_str = match &tx.token {
        TokenType::Native => "native".to_string(),
        TokenType::Erc20(addr) => addr.clone(),
    };

    TransactionSummary {
        tx_hash: tx.tx_hash.clone(),
        status: match tx.status {
            TxStatus::Pending => "pending".to_string(),
            TxStatus::Confirmed => "confirmed".to_string(),
            TxStatus::Failed => "failed".to_string(),
        },
        direction: direction.to_string(),
        from: tx.from.clone(),
        to: tx.to.clone(),
        amount: tx.amount.clone(),
        token: token_str,
        network: tx.network.clone(),
        block_number: tx.block_number,
        explorer_url: tx.explorer_url.clone(),
        timestamp: tx.created_at.to_rfc3339(),
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// Estimate gas for a transaction.
///
/// Returns estimated gas limit and cost before sending.
#[utoipa::path(
    post,
    path = "/v1/wallets/{wallet_id}/estimate",
    tag = "Transactions",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID")
    ),
    request_body = EstimateGasRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Gas estimate calculated", body = EstimateGasResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found"),
        (status = 503, description = "Blockchain network unavailable")
    )
)]
pub async fn estimate_gas(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Json(request): Json<EstimateGasRequest>,
) -> Result<Json<EstimateGasResponse>, ApiError> {
    // Validate recipient address
    validate_address(&request.to)?;

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

    // Get private key and create wallet
    let private_key_pem = wallet_repo.read_private_key(&wallet_id).map_err(|e| {
        ApiError::internal(&format!("Failed to read private key: {}", e))
    })?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

    // Determine network
    let network_config = match request.network.as_str() {
        "mainnet" => AVAX_MAINNET,
        _ => AVAX_FUJI,
    };

    // Create transaction builder
    let tx_builder = TxBuilder::new(network_config, eth_wallet)
        .await
        .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

    // Parse amount
    let decimals = get_token_decimals(&request.token);
    let amount_wei = parse_amount(&request.amount, decimals)
        .map_err(|e| ApiError::bad_request(&format!("Invalid amount: {}", e)))?;

    // Estimate gas
    let estimate = if request.token == "native" {
        tx_builder
            .estimate_native_transfer(&wallet.public_address, &request.to, amount_wei)
            .await
    } else {
        tx_builder
            .estimate_token_transfer(&wallet.public_address, &request.to, &request.token, amount_wei)
            .await
    }
    .map_err(|e| ApiError::service_unavailable(&format!("Gas estimation failed: {}", e)))?;

    Ok(Json(EstimateGasResponse {
        gas_limit: estimate.gas_limit.to_string(),
        max_fee_per_gas: estimate.max_fee_per_gas.to_string(),
        max_priority_fee_per_gas: estimate.max_priority_fee_per_gas.to_string(),
        estimated_cost_wei: estimate.estimated_cost_wei.to_string(),
        estimated_cost: format_amount(estimate.estimated_cost_wei, 18),
    }))
}

/// Send a transaction from a wallet.
///
/// Signs the transaction inside the SGX enclave and broadcasts to the network.
#[utoipa::path(
    post,
    path = "/v1/wallets/{wallet_id}/send",
    tag = "Transactions",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID")
    ),
    request_body = SendTransactionRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Transaction submitted", body = SendTransactionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found"),
        (status = 422, description = "Insufficient balance"),
        (status = 503, description = "Blockchain network unavailable")
    )
)]
pub async fn send_transaction(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Json(request): Json<SendTransactionRequest>,
) -> Result<Json<SendTransactionResponse>, ApiError> {
    // Validate recipient address
    validate_address(&request.to)?;

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

    // Get private key and create wallet
    let private_key_pem = wallet_repo.read_private_key(&wallet_id).map_err(|e| {
        ApiError::internal(&format!("Failed to read private key: {}", e))
    })?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

    // Determine network
    let network_config = match request.network.as_str() {
        "mainnet" => AVAX_MAINNET,
        _ => AVAX_FUJI,
    };

    // Create transaction builder
    let tx_builder = TxBuilder::new(network_config.clone(), eth_wallet)
        .await
        .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

    // Parse amount
    let decimals = get_token_decimals(&request.token);
    let amount_wei = parse_amount(&request.amount, decimals)
        .map_err(|e| ApiError::bad_request(&format!("Invalid amount: {}", e)))?;

    // Parse optional overrides
    let gas_limit = request
        .gas_limit
        .as_ref()
        .map(|s| s.parse::<u64>())
        .transpose()
        .map_err(|_| ApiError::bad_request("Invalid gas_limit"))?;

    let max_priority_fee = request
        .max_priority_fee_per_gas
        .as_ref()
        .map(|s| s.parse::<u128>())
        .transpose()
        .map_err(|_| ApiError::bad_request("Invalid max_priority_fee_per_gas"))?;

    // Send transaction
    let result = if request.token == "native" {
        tx_builder
            .send_native(&request.to, amount_wei, gas_limit, max_priority_fee)
            .await
    } else {
        tx_builder
            .send_token(&request.to, &request.token, amount_wei, gas_limit, max_priority_fee)
            .await
    }
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("insufficient funds") {
            ApiError::unprocessable("Insufficient balance for transaction")
        } else {
            ApiError::service_unavailable(&format!("Transaction failed: {}", e))
        }
    })?;

    // Store transaction record
    let token_type = if request.token == "native" {
        TokenType::Native
    } else {
        TokenType::Erc20(request.token.clone())
    };

    let stored_tx = StoredTransaction::new_pending(
        result.tx_hash.clone(),
        wallet_id.clone(),
        wallet.public_address.clone(),
        request.to.clone(),
        request.amount.clone(),
        token_type,
        request.network.clone(),
        result.explorer_url.clone(),
    );

    let tx_repo = TransactionRepository::new(&storage);
    if let Err(e) = tx_repo.create(&stored_tx) {
        tracing::warn!("Failed to store transaction record: {}", e);
    }

    // Log audit event
    let audit_repo = AuditRepository::new(&storage);
    let event = AuditEvent::new(AuditEventType::TransactionBroadcast)
        .with_user(&user.user_id)
        .with_resource(&wallet_id, "wallet")
        .with_details(serde_json::json!({
            "tx_hash": result.tx_hash,
            "to": request.to,
            "amount": request.amount,
            "token": request.token,
            "network": request.network,
        }));
    let _ = audit_repo.log(&event);

    Ok(Json(SendTransactionResponse {
        tx_hash: result.tx_hash,
        status: "pending".to_string(),
        explorer_url: result.explorer_url,
    }))
}

/// List transactions for a wallet.
#[utoipa::path(
    get,
    path = "/v1/wallets/{wallet_id}/transactions",
    tag = "Transactions",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID"),
        TransactionListQuery
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Transaction list", body = TransactionListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn list_transactions(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Query(query): Query<TransactionListQuery>,
) -> Result<Json<TransactionListResponse>, ApiError> {
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

    let tx_repo = TransactionRepository::new(&storage);
    let mut summaries: Vec<TransactionSummary> = Vec::new();

    // Get OUTGOING transactions (sent from this wallet)
    let outgoing = tx_repo.list_by_wallet(&wallet_id).map_err(|e| {
        ApiError::internal(&format!("Failed to list transactions: {}", e))
    })?;

    for tx in &outgoing {
        summaries.push(to_summary_with_direction(tx, "sent"));
    }

    // Get INCOMING transactions (received by this wallet)
    // Search all user's wallets for transactions TO this wallet's address
    let wallet_address = wallet.public_address.to_lowercase();
    let user_wallets = wallet_repo.list_by_owner(&user.user_id).unwrap_or_default();

    for other_wallet in &user_wallets {
        if other_wallet.wallet_id == wallet_id {
            continue; // Skip current wallet (already processed as outgoing)
        }

        let other_txs = tx_repo.list_by_wallet(&other_wallet.wallet_id).unwrap_or_default();
        for tx in &other_txs {
            // Check if this transaction was sent TO the current wallet
            if tx.to.to_lowercase() == wallet_address {
                summaries.push(to_summary_with_direction(tx, "received"));
            }
        }
    }

    // Sort all by timestamp descending (newest first)
    summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Filter by network if specified
    if let Some(network) = &query.network {
        summaries.retain(|tx| tx.network == *network);
    }

    // Apply limit
    let limit = query.limit.unwrap_or(50);
    summaries.truncate(limit);

    Ok(Json(TransactionListResponse {
        transactions: summaries,
    }))
}

/// Get the status of a specific transaction.
///
/// Used for polling after send. Updates stored status from blockchain if pending.
#[utoipa::path(
    get,
    path = "/v1/wallets/{wallet_id}/transactions/{tx_hash}",
    tag = "Transactions",
    params(
        ("wallet_id" = String, Path, description = "Wallet ID"),
        ("tx_hash" = String, Path, description = "Transaction hash")
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Transaction status", body = TransactionStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet or transaction not found")
    )
)]
pub async fn get_transaction_status(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path((wallet_id, tx_hash)): Path<(String, String)>,
) -> Result<Json<TransactionStatusResponse>, ApiError> {
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

    // Get transaction from storage
    let tx_repo = TransactionRepository::new(&storage);
    let tx = tx_repo.get(&wallet_id, &tx_hash).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => ApiError::not_found("Transaction not found"),
        _ => ApiError::internal(&format!("Failed to get transaction: {}", e)),
    })?;

    // If pending, check blockchain for updates
    if tx.status == TxStatus::Pending {
        // Create a read-only client to check status
        let client = match tx.network.as_str() {
            "mainnet" => AvaxClient::mainnet().await,
            _ => AvaxClient::fuji().await,
        }
        .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

        // Get current block for confirmations
        let current_block = client.get_block_number().await.unwrap_or(0);

        // Create a signing provider to check receipt (we need the wallet for this)
        let private_key_pem = wallet_repo.read_private_key(&wallet_id).map_err(|e| {
            ApiError::internal(&format!("Failed to read private key: {}", e))
        })?;

        let eth_wallet = wallet_from_pem(&private_key_pem)
            .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

        let network_config = match tx.network.as_str() {
            "mainnet" => AVAX_MAINNET,
            _ => AVAX_FUJI,
        };

        let tx_builder = TxBuilder::new(network_config, eth_wallet)
            .await
            .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

        // Check for receipt
        if let Ok(Some(receipt)) = tx_builder.get_transaction_status(&tx_hash).await {
            // Update stored transaction
            let _ = tx_repo.update_from_receipt(
                &wallet_id,
                &tx_hash,
                receipt.block_number,
                receipt.gas_used,
                receipt.success,
            );

            let confirmations = current_block.saturating_sub(receipt.block_number);

            return Ok(Json(TransactionStatusResponse {
                tx_hash: tx.tx_hash,
                status: if receipt.success {
                    "confirmed".to_string()
                } else {
                    "failed".to_string()
                },
                block_number: Some(receipt.block_number),
                confirmations: Some(confirmations),
                gas_used: Some(receipt.gas_used.to_string()),
                timestamp: Some(tx.updated_at.to_rfc3339()),
            }));
        }
    }

    // Return stored status
    let confirmations = if tx.status == TxStatus::Confirmed {
        // Try to get current block for confirmations
        let client = match tx.network.as_str() {
            "mainnet" => AvaxClient::mainnet().await,
            _ => AvaxClient::fuji().await,
        };
        
        if let (Ok(client), Some(block)) = (client, tx.block_number) {
            client.get_block_number().await.ok().map(|current| current.saturating_sub(block))
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(TransactionStatusResponse {
        tx_hash: tx.tx_hash,
        status: match tx.status {
            TxStatus::Pending => "pending".to_string(),
            TxStatus::Confirmed => "confirmed".to_string(),
            TxStatus::Failed => "failed".to_string(),
        },
        block_number: tx.block_number,
        confirmations,
        gas_used: tx.gas_used.map(|g| g.to_string()),
        timestamp: Some(tx.updated_at.to_rfc3339()),
    }))
}
