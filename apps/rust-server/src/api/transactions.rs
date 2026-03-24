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
        ensure_fuji_network, format_amount, parse_amount, wallet_from_pem, AvaxClient, TxBuilder,
        avax_fuji, REUR_TOKEN,
    },
    error::ApiError,
    state::AppState,
    storage::{
        AuditEvent, AuditEventType, AuditRepository, StoredTransaction, TokenType, TxStatus,
        WalletRepository, WalletStatus,
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
    /// Network: "fuji" only.
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
    /// Network: "fuji" only.
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
    /// Network filter (must be "fuji" if provided).
    pub network: Option<String>,
    /// Maximum number of results (default: 50)
    #[param(default = 50)]
    pub limit: Option<usize>,
    /// Cursor for pagination (returned as `next_cursor` in previous response).
    pub cursor: Option<String>,
    /// Direction filter: "sent" or "received". If omitted, returns both.
    pub direction: Option<String>,
}

/// Transaction list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionListResponse {
    /// List of transactions
    pub transactions: Vec<TransactionSummary>,
    /// Cursor for the next page (null if no more pages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
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
        return Err(ApiError::bad_request(
            "Address must contain only hex characters",
        ));
    }
    Ok(())
}

/// Get decimals for a token.
fn get_token_decimals(token: &str) -> u8 {
    if token == "native" {
        18 // AVAX
    } else if token.eq_ignore_ascii_case(REUR_TOKEN.fuji_address.unwrap_or("")) {
        6 // rEUR
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

    ensure_fuji_network(Some(request.network.as_str())).map_err(ApiError::bad_request)?;

    // Get private key and create wallet
    let private_key_pem = wallet_repo
        .read_private_key(&wallet_id)
        .map_err(|e| ApiError::internal(&format!("Failed to read private key: {}", e)))?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

    // Determine network (Fuji-only).
    let network_config = avax_fuji();

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
            .estimate_token_transfer(
                &wallet.public_address,
                &request.to,
                &request.token,
                amount_wei,
            )
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

    ensure_fuji_network(Some(request.network.as_str())).map_err(ApiError::bad_request)?;

    // Get private key and create wallet
    let private_key_pem = wallet_repo
        .read_private_key(&wallet_id)
        .map_err(|e| ApiError::internal(&format!("Failed to read private key: {}", e)))?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

    // Determine network (Fuji-only).
    let network_config = avax_fuji();

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
            .send_token(
                &request.to,
                &request.token,
                amount_wei,
                gas_limit,
                max_priority_fee,
            )
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

    // If recipient belongs to an internal wallet, mirror a transaction record
    // to that wallet so history lookups stay wallet-local and scalable.
    let recipient_wallet_id = wallet_repo.list_all_wallets().ok().and_then(|wallets| {
        wallets
            .into_iter()
            .find(|w| {
                w.status != WalletStatus::Deleted
                    && w.public_address.eq_ignore_ascii_case(&request.to)
            })
            .map(|w| w.wallet_id)
    });

    let stored_tx = StoredTransaction::new_pending(
        result.tx_hash.clone(),
        wallet_id.clone(),
        recipient_wallet_id.clone().filter(|id| id != &wallet_id),
        wallet.public_address.clone(),
        request.to.clone(),
        request.amount.clone(),
        token_type.clone(),
        request.network.clone(),
        result.explorer_url.clone(),
    );

    let tx_db = state
        .tx_db
        .as_ref()
        .expect("transaction database must be configured");
    let mut directions = vec![(wallet.public_address.clone(), "sent")];
    if let Some(ref _rcpt_id) = recipient_wallet_id {
        directions.push((request.to.clone(), "received"));
    }
    if let Err(e) = tx_db.upsert_transaction(&stored_tx, &directions) {
        tracing::warn!(error = %e, "Failed to store transaction in tx database");
    }
    if let Some(tx_cache) = &state.tx_cache {
        tx_cache.invalidate(&wallet.public_address);
        tx_cache.invalidate(&request.to);
    }

    // Mirror recipient-side transaction record for internal transfers.
    if let Some(recipient_id) = recipient_wallet_id {
        if recipient_id != wallet_id {
            let mirrored_tx = StoredTransaction::new_pending(
                result.tx_hash.clone(),
                recipient_id.clone(),
                Some(wallet_id.clone()),
                wallet.public_address.clone(),
                request.to.clone(),
                request.amount.clone(),
                token_type,
                request.network.clone(),
                result.explorer_url.clone(),
            );
            let directions = vec![(request.to.clone(), "received")];
            if let Err(e) = tx_db.upsert_transaction(&mirrored_tx, &directions) {
                tracing::warn!(error = %e, "Failed to store mirrored tx in tx database");
            }
            if let Some(tx_cache) = &state.tx_cache {
                tx_cache.invalidate(&request.to);
            }
        }
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

    // Filter by network if specified
    if let Some(network) = &query.network {
        ensure_fuji_network(Some(network.as_str())).map_err(ApiError::bad_request)?;
    }

    let limit = query.limit.unwrap_or(50).min(200);
    let wallet_address = wallet.public_address.to_lowercase();

    let tx_db = state
        .tx_db
        .as_ref()
        .expect("transaction database must be configured");
    if query.cursor.is_none() && query.direction.is_none() {
        if let Some(tx_cache) = &state.tx_cache {
            if let Some((cached, next_cursor)) = tx_cache.get_first_page(&wallet_address, limit) {
                // Skip cache if any transaction is still pending — fall
                // through to the reconciliation path that checks on-chain.
                let has_pending = cached.iter().any(|(tx, _)| tx.status == TxStatus::Pending);
                if !has_pending {
                    let summaries: Vec<TransactionSummary> = cached
                        .iter()
                        .take(limit)
                        .map(|(tx, dir)| to_summary_with_direction(tx, dir))
                        .collect();
                    return Ok(Json(TransactionListResponse {
                        transactions: summaries,
                        next_cursor,
                    }));
                }
            }
        }
    }

    let (results, next_cursor) = tx_db
        .list_by_wallet(&wallet_address, query.cursor.as_deref(), limit)
        .map_err(|e| ApiError::internal(&format!("Failed to list transactions: {}", e)))?;

    // ── Reconcile pending transactions with on-chain status ─────────
    // If there are any pending transactions in the result set, check
    // their receipt on-chain and promote them to confirmed/failed.
    // This is lightweight: typically 0-1 pending txs per page, and each
    // receipt RPC call is ~100ms.  We only create the AvaxClient if we
    // actually have pending items.
    let pending_hashes: Vec<(usize, String)> = results
        .iter()
        .enumerate()
        .filter(|(_, (tx, _))| tx.status == TxStatus::Pending)
        .map(|(i, (tx, _))| (i, tx.tx_hash.clone()))
        .collect();

    let mut updated_results = results;

    if !pending_hashes.is_empty() {
        if let Ok(client) = AvaxClient::fuji().await {
            for (idx, hash) in &pending_hashes {
                if let Ok(Some(receipt)) = client.get_transaction_receipt_status(hash).await {
                    let new_status = if receipt.success {
                        TxStatus::Confirmed
                    } else {
                        TxStatus::Failed
                    };

                    // Update redb so future reads are correct
                    let _ = tx_db.update_status(
                        hash,
                        new_status,
                        Some(receipt.block_number),
                        Some(receipt.gas_used),
                    );

                    // Update the in-memory copy for this response
                    updated_results[*idx].0.status = new_status;
                    updated_results[*idx].0.block_number = Some(receipt.block_number);
                    updated_results[*idx].0.gas_used = Some(receipt.gas_used);

                    tracing::debug!(
                        tx_hash = %hash,
                        status = ?new_status,
                        "list_transactions: reconciled pending tx"
                    );
                }
            }

            // Invalidate cache if we updated anything
            if !pending_hashes.is_empty() {
                if let Some(tx_cache) = &state.tx_cache {
                    tx_cache.invalidate(&wallet_address);
                }
            }
        }
    }

    let mut summaries: Vec<TransactionSummary> = updated_results
        .iter()
        .map(|(tx, dir)| to_summary_with_direction(tx, dir))
        .collect();

    summaries.retain(|tx| tx.network == "fuji");

    if let Some(ref direction) = query.direction {
        summaries.retain(|s| s.direction == *direction);
    }

    if query.cursor.is_none() && query.direction.is_none() {
        if let Some(tx_cache) = &state.tx_cache {
            tx_cache.put_first_page(&wallet_address, updated_results, next_cursor.clone());
        }
    }

    Ok(Json(TransactionListResponse {
        transactions: summaries,
        next_cursor,
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

    let tx_db = state
        .tx_db
        .as_ref()
        .expect("transaction database must be configured");
    let tx = tx_db
        .get_transaction(&tx_hash)
        .map_err(|e| ApiError::internal(&format!("Failed to get transaction: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Transaction not found"))?;
    if !tx.from.eq_ignore_ascii_case(&wallet.public_address)
        && !tx.to.eq_ignore_ascii_case(&wallet.public_address)
    {
        return Err(ApiError::not_found("Transaction not found"));
    }

    // If pending, check blockchain for updates
    if tx.status == TxStatus::Pending {
        if tx.network != "fuji" {
            return Err(ApiError::bad_request(
                "Only `fuji` network is supported in this deployment.",
            ));
        }
        // Create a read-only client to check status
        let client = AvaxClient::fuji()
            .await
            .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

        // Get current block for confirmations
        let current_block = client.get_block_number().await.unwrap_or(0);

        // Create a signing provider to check receipt (we need the wallet for this)
        let private_key_pem = wallet_repo
            .read_private_key(&wallet_id)
            .map_err(|e| ApiError::internal(&format!("Failed to read private key: {}", e)))?;

        let eth_wallet = wallet_from_pem(&private_key_pem)
            .map_err(|e| ApiError::internal(&format!("Failed to create signer: {}", e)))?;

        let tx_builder = TxBuilder::new(avax_fuji(), eth_wallet)
            .await
            .map_err(|e| ApiError::service_unavailable(&format!("Failed to connect: {}", e)))?;

        // Check for receipt
        if let Ok(Some(receipt)) = tx_builder.get_transaction_status(&tx_hash).await {
            let new_status = if receipt.success {
                TxStatus::Confirmed
            } else {
                TxStatus::Failed
            };
            let _ = tx_db.update_status(
                &tx_hash,
                new_status,
                Some(receipt.block_number),
                Some(receipt.gas_used),
            );
            if let Some(tx_cache) = &state.tx_cache {
                tx_cache.invalidate(&wallet.public_address);
            }

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
        if tx.network != "fuji" {
            return Err(ApiError::bad_request(
                "Only `fuji` network is supported in this deployment.",
            ));
        }
        // Try to get current block for confirmations
        let client = AvaxClient::fuji().await;

        if let (Ok(client), Some(block)) = (client, tx.block_number) {
            client
                .get_block_number()
                .await
                .ok()
                .map(|current| current.saturating_sub(block))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthenticatedUser, Role};
    use crate::storage::{WalletMetadata, WalletRepository};
    use axum::extract::{Path, Query, State};
    use chrono::Utc;

    fn mock_auth(user_id: &str) -> Auth {
        Auth(AuthenticatedUser {
            user_id: user_id.to_string(),
            role: Role::Client,
            session_id: None,
            issuer: "https://test.clerk.dev".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        })
    }

    fn wallet_meta(wallet_id: &str, owner_user_id: &str, public_address: &str) -> WalletMetadata {
        WalletMetadata {
            wallet_id: wallet_id.to_string(),
            owner_user_id: owner_user_id.to_string(),
            public_address: public_address.to_string(),
            created_at: Utc::now(),
            status: WalletStatus::Active,
            label: None,
        }
    }

    #[tokio::test]
    async fn list_transactions_shows_received_for_mirrored_cross_user_record() {
        let state = AppState::default();
        let storage = state.storage();
        let wallet_repo = WalletRepository::new(storage);
        let tx_db = state.tx_db.as_ref().unwrap();

        let sender_wallet_id = "sender-wallet-1";
        let receiver_wallet_id = "receiver-wallet-1";
        let sender_addr = "0x1111111111111111111111111111111111111111";
        let receiver_addr = "0x2222222222222222222222222222222222222222";
        let tx_hash = "0xabc111";

        wallet_repo
            .create(
                &wallet_meta(sender_wallet_id, "user-a", sender_addr),
                b"test-key-a",
            )
            .unwrap();
        wallet_repo
            .create(
                &wallet_meta(receiver_wallet_id, "user-b", receiver_addr),
                b"test-key-b",
            )
            .unwrap();
        tx_db.register_address(sender_addr, sender_wallet_id).unwrap();
        tx_db.register_address(receiver_addr, receiver_wallet_id).unwrap();

        let sender_tx = StoredTransaction::new_pending(
            tx_hash.to_string(),
            sender_wallet_id.to_string(),
            Some(receiver_wallet_id.to_string()),
            sender_addr.to_string(),
            receiver_addr.to_string(),
            "5".to_string(),
            TokenType::Native,
            "fuji".to_string(),
            "https://testnet.snowtrace.io/tx/0xabc111".to_string(),
        );
        tx_db
            .upsert_transaction(
                &sender_tx,
                &[
                    (sender_addr.to_string(), "sent"),
                    (receiver_addr.to_string(), "received"),
                ],
            )
            .unwrap();

        let response = list_transactions(
            mock_auth("user-b"),
            State(state.clone()),
            Path(receiver_wallet_id.to_string()),
            Query(TransactionListQuery {
                network: None,
                limit: None,
                cursor: None,
                direction: None,
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.transactions.len(), 1);
        assert_eq!(response.transactions[0].tx_hash, tx_hash);
        assert_eq!(response.transactions[0].direction, "received");
        assert_eq!(response.transactions[0].from.to_lowercase(), sender_addr);
        assert_eq!(response.transactions[0].to.to_lowercase(), receiver_addr);
    }

    #[tokio::test]
    async fn get_transaction_status_works_for_receiver_mirrored_record() {
        let state = AppState::default();
        let storage = state.storage();
        let wallet_repo = WalletRepository::new(storage);
        let tx_db = state.tx_db.as_ref().unwrap();

        let sender_wallet_id = "sender-wallet-2";
        let receiver_wallet_id = "receiver-wallet-2";
        let sender_addr = "0x3333333333333333333333333333333333333333";
        let receiver_addr = "0x4444444444444444444444444444444444444444";
        let tx_hash = "0xabc222";

        wallet_repo
            .create(
                &wallet_meta(sender_wallet_id, "user-a", sender_addr),
                b"test-key-a2",
            )
            .unwrap();
        wallet_repo
            .create(
                &wallet_meta(receiver_wallet_id, "user-b", receiver_addr),
                b"test-key-b2",
            )
            .unwrap();
        tx_db.register_address(sender_addr, sender_wallet_id).unwrap();
        tx_db.register_address(receiver_addr, receiver_wallet_id).unwrap();

        let mut receiver_tx = StoredTransaction::new_pending(
            tx_hash.to_string(),
            receiver_wallet_id.to_string(),
            Some(sender_wallet_id.to_string()),
            sender_addr.to_string(),
            receiver_addr.to_string(),
            "7".to_string(),
            TokenType::Native,
            "fuji".to_string(),
            "https://testnet.snowtrace.io/tx/0xabc222".to_string(),
        );
        receiver_tx.status = TxStatus::Confirmed;
        receiver_tx.block_number = None;
        receiver_tx.gas_used = None;
        tx_db
            .upsert_transaction(
                &receiver_tx,
                &[(receiver_addr.to_string(), "received")],
            )
            .unwrap();

        let response = get_transaction_status(
            mock_auth("user-b"),
            State(state.clone()),
            Path((receiver_wallet_id.to_string(), tx_hash.to_string())),
        )
        .await
        .unwrap();

        assert_eq!(response.status, "confirmed");
        assert_eq!(response.tx_hash, tx_hash);
        assert!(response.block_number.is_none());
    }
}
