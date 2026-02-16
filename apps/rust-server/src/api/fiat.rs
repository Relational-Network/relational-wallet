// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Fiat on-ramp/off-ramp API using a Fuji-only reserve wallet flow.

use std::{env, str::FromStr, sync::Arc};
use std::sync::OnceLock;

use alloy::{
    primitives::{Address, U256},
    sol,
    sol_types::SolCall,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utoipa::{IntoParams, ToSchema};

use crate::{
    audit_log,
    auth::{AdminOnly, Auth},
    blockchain::{
        ensure_fuji_network, parse_amount, wallet_from_pem, AvaxClient, TxBuilder, AVAX_FUJI,
    },
    error::ApiError,
    providers::truelayer::{
        CreateOffRampRequest, CreateOnRampRequest, ProviderExecutionStatus, TrueLayerClient,
        TrueLayerError,
    },
    state::AppState,
    storage::{
        AuditEventType, FiatDirection, FiatRequestRepository, FiatRequestStatus,
        FiatServiceWalletMetadata, FiatServiceWalletRepository, StoredFiatRequest,
        StoredTransaction, TokenType, TransactionRepository, TxStatus, WalletRepository,
        WalletStatus,
    },
};

const DEFAULT_PROVIDER: &str = "truelayer_sandbox";
const SUPPORTED_PROVIDER_IDS: [&str; 1] = [DEFAULT_PROVIDER];
const REUR_DECIMALS: u8 = 6;
const REUR_CONTRACT_ENV: &str = "REUR_CONTRACT_ADDRESS_FUJI";
const RESERVE_BOOTSTRAP_ENABLED_ENV: &str = "FIAT_RESERVE_BOOTSTRAP_ENABLED";
const RESERVE_INITIAL_TOPUP_ENV: &str = "FIAT_RESERVE_INITIAL_TOPUP_EUR";
const FIAT_MIN_CONFIRMATIONS_ENV: &str = "FIAT_MIN_CONFIRMATIONS";
/// TrueLayer sandbox JWKS URL for webhook signature verification.
const TRUELAYER_SANDBOX_JWKS_URL: &str =
    "https://webhooks.truelayer-sandbox.com/.well-known/jwks";

/// Webhook path as registered in TrueLayer Console (must match exactly for signature verification).
const WEBHOOK_PATH: &str = "/v1/fiat/providers/truelayer/webhook";

sol! {
    #[sol(rpc)]
    interface RelationalEuroMinter {
        function mint(address to, uint256 amount) external;
    }
}

/// Request body for creating fiat on-ramp/off-ramp requests.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFiatRequest {
    /// Wallet to credit/debit for this fiat request.
    pub wallet_id: String,
    /// Amount in EUR decimal string (e.g. "25.50").
    pub amount_eur: String,
    /// Optional provider name (`truelayer_sandbox` default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional free-form note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Beneficiary account holder name (required for off-ramp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beneficiary_account_holder_name: Option<String>,
    /// Beneficiary IBAN (required for off-ramp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beneficiary_iban: Option<String>,
}

/// Fiat request response returned to clients.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatRequestResponse {
    /// Request ID.
    pub request_id: String,
    /// Wallet ID tied to this request.
    pub wallet_id: String,
    /// `on_ramp` or `off_ramp`.
    pub direction: FiatDirection,
    /// Amount in EUR.
    pub amount_eur: String,
    /// Provider identifier.
    pub provider: String,
    /// Current status.
    pub status: FiatRequestStatus,
    /// Settlement network.
    pub chain_network: String,
    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional service-wallet address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_wallet_address: Option<String>,
    /// Expected token amount in minor units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_amount_minor: Option<u64>,
    /// Optional provider reference/session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_reference: Option<String>,
    /// Optional provider action URL (for redirect/continue flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_action_url: Option<String>,
    /// Optional detected deposit tx hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_tx_hash: Option<String>,
    /// Optional reserve transfer tx hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_transfer_tx_hash: Option<String>,
    /// Optional provider event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_event_id: Option<String>,
    /// Optional failure reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Optional last provider sync time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_provider_sync_at: Option<String>,
    /// Optional last chain sync time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chain_sync_at: Option<String>,
    /// Creation time.
    pub created_at: String,
    /// Last update time.
    pub updated_at: String,
}

/// List response for fiat requests.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatRequestListResponse {
    /// Requests visible to the authenticated user.
    pub requests: Vec<FiatRequestResponse>,
    /// Total count.
    pub total: usize,
}

/// Provider summary exposed by fiat API.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatProviderSummary {
    /// Stable provider ID used by API requests.
    pub provider_id: String,
    /// Human-friendly provider name for UI display.
    pub display_name: String,
    /// Indicates this provider is sandbox-only in current environment.
    pub sandbox: bool,
    /// Whether backend is configured and ready for this provider.
    pub enabled: bool,
    /// Whether the provider can process on-ramp requests.
    pub supports_on_ramp: bool,
    /// Whether the provider can process off-ramp requests.
    pub supports_off_ramp: bool,
}

/// Response for provider discovery.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatProviderListResponse {
    /// Default provider ID if client does not pass one.
    pub default_provider: String,
    /// Providers currently enabled by backend.
    pub providers: Vec<FiatProviderSummary>,
}

/// Query params for listing fiat requests.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct FiatRequestListQuery {
    /// Optional wallet filter.
    pub wallet_id: Option<String>,
}

/// Reserve-wallet status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatServiceWalletStatusResponse {
    /// Stable service-wallet id.
    pub wallet_id: String,
    /// Public address.
    pub public_address: String,
    /// Whether wallet was present/bootstrapped.
    pub bootstrapped: bool,
    /// Fuji-only network value.
    pub chain_network: String,
    /// Configured rEUR contract.
    pub reur_contract_address: String,
    /// Native AVAX balance (formatted).
    pub avax_balance: String,
    /// rEUR balance (formatted).
    pub reur_balance: String,
    /// rEUR balance in raw minor units.
    pub reur_balance_raw: String,
}

/// Reserve top-up request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReserveTopUpRequest {
    /// Optional amount in EUR decimal format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_eur: Option<String>,
}

/// Reserve transfer request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReserveTransferRequest {
    /// Destination EVM address.
    pub to: String,
    /// Amount in EUR decimal format.
    pub amount_eur: String,
}

/// Reserve transaction response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReserveTransactionResponse {
    /// Submitted transaction hash.
    pub tx_hash: String,
    /// Explorer URL for tx.
    pub explorer_url: String,
    /// Amount used for operation.
    pub amount_eur: String,
    /// Amount in token minor units.
    pub amount_minor: String,
}

/// Manual sync response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FiatSyncResponse {
    /// Synchronized request.
    pub request: FiatRequestResponse,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[allow(dead_code)] // Fields deserialized from TrueLayer webhook JSON; used via serde + logging
pub struct TrueLayerWebhookPayload {
    #[serde(default, rename = "type")]
    event_type: Option<String>,
    #[serde(default)]
    event_id: Option<String>,
    #[serde(default)]
    event_version: Option<u32>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    payment_id: Option<String>,
    #[serde(default)]
    payout_id: Option<String>,
    #[serde(default)]
    reference: Option<String>,
    /// Reason for payout failure (from `payout_failed` webhook).
    #[serde(default)]
    failure_reason: Option<String>,
    /// Timestamp when payout was executed (from `payout_executed` webhook).
    #[serde(default)]
    executed_at: Option<String>,
    /// Timestamp when payout failed (from `payout_failed` webhook).
    #[serde(default)]
    failed_at: Option<String>,
    /// Payment scheme used (e.g. `faster_payments_service`, `sepa_credit_transfer`).
    #[serde(default)]
    scheme_id: Option<String>,
    /// Merchant account the payout was made from.
    #[serde(default)]
    merchant_account_id: Option<String>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

fn parse_amount_to_minor(amount: &str) -> Result<(String, u64), ApiError> {
    let trimmed = amount.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "amount_eur must be a valid positive number",
        ));
    }

    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() > 2 {
        return Err(ApiError::bad_request(
            "amount_eur must be a valid positive number",
        ));
    }

    let whole_part = parts[0];
    if whole_part.is_empty() || !whole_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::bad_request(
            "amount_eur must be a valid positive number",
        ));
    }

    let whole = whole_part
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request("amount_eur is too large"))?;

    let fraction_part = if parts.len() == 2 { parts[1] } else { "" };
    if !fraction_part.chars().all(|c| c.is_ascii_digit()) || fraction_part.len() > 2 {
        return Err(ApiError::bad_request(
            "amount_eur must have at most 2 decimal places",
        ));
    }

    let fraction = if fraction_part.is_empty() {
        0
    } else if fraction_part.len() == 1 {
        fraction_part
            .parse::<u64>()
            .map_err(|_| ApiError::bad_request("amount_eur must be a valid positive number"))?
            * 10
    } else {
        fraction_part
            .parse::<u64>()
            .map_err(|_| ApiError::bad_request("amount_eur must be a valid positive number"))?
    };

    let minor = whole
        .checked_mul(100)
        .and_then(|base| base.checked_add(fraction))
        .ok_or_else(|| ApiError::bad_request("amount_eur is too large"))?;

    if minor == 0 {
        return Err(ApiError::bad_request(
            "amount_eur must be a valid positive number",
        ));
    }

    let normalized = format!("{whole}.{fraction:02}");
    Ok((normalized, minor))
}

fn parse_amount_to_token_minor_u256(amount: &str) -> Result<U256, ApiError> {
    parse_amount(amount, REUR_DECIMALS)
        .map_err(|e| ApiError::bad_request(format!("invalid amount_eur for token settlement: {e}")))
}

fn u256_to_u64(value: U256) -> Result<u64, ApiError> {
    value
        .to_string()
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request("amount_eur is too large for settlement"))
}

fn provider_summaries() -> Vec<FiatProviderSummary> {
    let enabled = TrueLayerClient::is_configured();
    let supports_on_ramp = enabled;
    let supports_off_ramp = enabled;
    vec![FiatProviderSummary {
        provider_id: DEFAULT_PROVIDER.to_string(),
        display_name: "TrueLayer Sandbox".to_string(),
        sandbox: true,
        enabled,
        supports_on_ramp,
        supports_off_ramp,
    }]
}

fn resolve_provider_id(raw_provider: Option<String>) -> Result<String, ApiError> {
    let provider = raw_provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PROVIDER)
        .to_ascii_lowercase();

    if SUPPORTED_PROVIDER_IDS.contains(&provider.as_str()) {
        Ok(provider)
    } else {
        let supported = SUPPORTED_PROVIDER_IDS.join(", ");
        Err(ApiError::bad_request(format!(
            "Unsupported provider `{provider}`. Supported providers: {supported}"
        )))
    }
}

fn ensure_provider_enabled(provider: &str, _direction: FiatDirection) -> Result<(), ApiError> {
    if provider != DEFAULT_PROVIDER {
        return Err(ApiError::bad_request("Unsupported provider"));
    }

    if !TrueLayerClient::is_configured() {
        return Err(ApiError::service_unavailable(
            "TrueLayer sandbox is not configured. Set TRUELAYER_* environment variables.",
        ));
    }

    Ok(())
}

fn normalize_offramp_account_holder_name(raw: Option<String>) -> Result<String, ApiError> {
    let name = raw
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::bad_request("beneficiary_account_holder_name is required for off-ramp")
        })?
        .to_string();

    if name.len() > 140 {
        return Err(ApiError::bad_request(
            "beneficiary_account_holder_name must be at most 140 characters",
        ));
    }

    Ok(name)
}

fn normalize_offramp_iban(raw: Option<String>) -> Result<String, ApiError> {
    let compact = raw
        .as_deref()
        .map(|value| {
            value
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect::<String>()
        })
        .unwrap_or_default()
        .to_ascii_uppercase();

    if compact.is_empty() {
        return Err(ApiError::bad_request(
            "beneficiary_iban is required for off-ramp",
        ));
    }
    if compact.len() < 15 || compact.len() > 34 {
        return Err(ApiError::bad_request(
            "beneficiary_iban must be between 15 and 34 characters",
        ));
    }
    if !compact.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request(
            "beneficiary_iban must contain only letters and numbers",
        ));
    }

    Ok(compact)
}

fn map_provider_error(error: TrueLayerError) -> ApiError {
    match error {
        TrueLayerError::MissingConfig(message) => ApiError::service_unavailable(format!(
            "TrueLayer sandbox configuration error: {message}"
        )),
        TrueLayerError::Signing(message) => {
            ApiError::service_unavailable(format!("TrueLayer signing failed: {message}"))
        }
        TrueLayerError::Auth(message) => {
            ApiError::service_unavailable(format!("TrueLayer auth failed: {message}"))
        }
        TrueLayerError::Request(message) | TrueLayerError::InvalidResponse(message) => {
            ApiError::service_unavailable(format!("TrueLayer request failed: {message}"))
        }
    }
}

fn map_onramp_provider_status(status: ProviderExecutionStatus) -> FiatRequestStatus {
    match status {
        ProviderExecutionStatus::Completed => FiatRequestStatus::SettlementPending,
        ProviderExecutionStatus::Failed => FiatRequestStatus::Failed,
        ProviderExecutionStatus::Pending => FiatRequestStatus::AwaitingProvider,
    }
}

fn map_offramp_provider_status(status: ProviderExecutionStatus) -> FiatRequestStatus {
    match status {
        ProviderExecutionStatus::Completed => FiatRequestStatus::Completed,
        ProviderExecutionStatus::Failed => FiatRequestStatus::Failed,
        ProviderExecutionStatus::Pending => FiatRequestStatus::ProviderPending,
    }
}

fn map_webhook_status(raw: &str) -> ProviderExecutionStatus {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.contains("fail")
        || normalized.contains("reject")
        || normalized.contains("cancel")
        || normalized.contains("error")
    {
        ProviderExecutionStatus::Failed
    } else if normalized.contains("complete")
        || normalized.contains("execut")
        || normalized.contains("settl")
        || normalized.contains("success")
        || normalized.contains("authoris")
    {
        ProviderExecutionStatus::Completed
    } else {
        ProviderExecutionStatus::Pending
    }
}

fn resolve_reur_contract_address() -> Result<String, ApiError> {
    let value = env::var(REUR_CONTRACT_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            ApiError::service_unavailable(format!(
                "{REUR_CONTRACT_ENV} must be configured for fiat settlement"
            ))
        })?;

    if !value.starts_with("0x") || value.len() != 42 || !value[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::service_unavailable(format!(
            "{REUR_CONTRACT_ENV} is not a valid EVM address"
        )));
    }

    Ok(value)
}

fn is_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn reserve_bootstrap_enabled() -> bool {
    env::var(RESERVE_BOOTSTRAP_ENABLED_ENV)
        .ok()
        .map(|v| is_truthy(&v))
        .unwrap_or(true)
}

fn reserve_initial_topup_amount() -> String {
    env::var(RESERVE_INITIAL_TOPUP_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "1000000.00".to_string())
}

fn fiat_min_confirmations() -> u64 {
    env::var(FIAT_MIN_CONFIRMATIONS_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .max(1)
}

fn to_response(record: &StoredFiatRequest) -> FiatRequestResponse {
    FiatRequestResponse {
        request_id: record.request_id.clone(),
        wallet_id: record.wallet_id.clone(),
        direction: record.direction,
        amount_eur: record.amount_eur.clone(),
        provider: record.provider.clone(),
        status: record.status,
        chain_network: record.chain_network.clone(),
        note: record.note.clone(),
        service_wallet_address: record.service_wallet_address.clone(),
        expected_amount_minor: record.expected_amount_minor,
        provider_reference: record.provider_reference.clone(),
        provider_action_url: record.provider_action_url.clone(),
        deposit_tx_hash: record.deposit_tx_hash.clone(),
        reserve_transfer_tx_hash: record.reserve_transfer_tx_hash.clone(),
        provider_event_id: record.provider_event_id.clone(),
        failure_reason: record.failure_reason.clone(),
        last_provider_sync_at: record.last_provider_sync_at.map(|ts| ts.to_rfc3339()),
        last_chain_sync_at: record.last_chain_sync_at.map(|ts| ts.to_rfc3339()),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

/// Cached JWKS bytes fetched from TrueLayer.
static CACHED_JWKS: OnceLock<Vec<u8>> = OnceLock::new();

/// Fetch JWKS JSON from TrueLayer's well-known endpoint, caching the result.
async fn fetch_jwks_cached() -> Result<Vec<u8>, ApiError> {
    if let Some(cached) = CACHED_JWKS.get() {
        return Ok(cached.clone());
    }

    info!(url = %TRUELAYER_SANDBOX_JWKS_URL, "Fetching TrueLayer webhook JWKS");
    let response = reqwest::Client::new()
        .get(TRUELAYER_SANDBOX_JWKS_URL)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch TrueLayer JWKS: {e}")))?;

    let jwks_bytes = response
        .bytes()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to read JWKS response: {e}")))?
        .to_vec();

    let _ = CACHED_JWKS.set(jwks_bytes.clone());
    Ok(jwks_bytes)
}

/// Verify webhook Tl-Signature using TrueLayer's JWKS public keys.
async fn verify_webhook_signature(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), ApiError> {
    let tl_signature = headers
        .get("tl-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::forbidden("Missing Tl-Signature header"))?;

    // Extract and validate JKU from signature header
    let jws_header = truelayer_signing::extract_jws_header(tl_signature)
        .map_err(|e| ApiError::forbidden(format!("Invalid Tl-Signature header: {e}")))?;

    if let Some(ref jku) = jws_header.jku {
        if jku.as_ref() != TRUELAYER_SANDBOX_JWKS_URL {
            return Err(ApiError::forbidden(format!(
                "Untrusted JKU: {jku}"
            )));
        }
    }

    // Fetch JWKS (cached)
    let jwks = fetch_jwks_cached().await?;

    // Collect headers for verification (exclude Tl-Signature itself)
    let header_pairs: Vec<(&str, &[u8])> = headers
        .iter()
        .filter(|(name, _)| name.as_str() != "tl-signature")
        .map(|(name, value)| (name.as_str(), value.as_bytes()))
        .collect();

    truelayer_signing::verify_with_jwks(&jwks)
        .method(truelayer_signing::Method::Post)
        .path(WEBHOOK_PATH)
        .headers(header_pairs)
        .body(body)
        .build_verifier()
        .verify(tl_signature)
        .map_err(|e| ApiError::forbidden(format!("Webhook signature verification failed: {e}")))?;

    Ok(())
}

fn extract_provider_reference(payload: &TrueLayerWebhookPayload) -> Option<String> {
    payload
        .payment_id
        .clone()
        .or_else(|| payload.payout_id.clone())
        .or_else(|| payload.reference.clone())
        .or_else(|| {
            payload
                .data
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
}

fn extract_webhook_status(payload: &TrueLayerWebhookPayload) -> Option<ProviderExecutionStatus> {
    // Try explicit status field first
    payload
        .status
        .as_deref()
        .map(map_webhook_status)
        // Then try event type (e.g. "payout_executed", "payment_failed")
        .or_else(|| {
            payload
                .event_type
                .as_deref()
                .map(map_webhook_status)
        })
        // Then try data.status
        .or_else(|| {
            payload
                .data
                .as_ref()
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str())
                .map(map_webhook_status)
        })
}

fn ensure_service_wallet(storage: &Arc<crate::storage::EncryptedStorage>) -> Result<FiatServiceWalletMetadata, ApiError> {
    let repo = FiatServiceWalletRepository::new(storage);
    if repo.exists() {
        return repo
            .get()
            .map_err(|e| ApiError::internal(format!("Failed to load service wallet: {e}")));
    }

    if !reserve_bootstrap_enabled() {
        return Err(ApiError::service_unavailable(
            "Fiat reserve service wallet is missing and auto-bootstrap is disabled.",
        ));
    }

    repo.bootstrap()
        .map_err(|e| ApiError::internal(format!("Failed to bootstrap service wallet: {e}")))
}

async fn send_reserve_transfer(
    storage: &Arc<crate::storage::EncryptedStorage>,
    to: &str,
    amount_eur: &str,
) -> Result<crate::blockchain::transactions::SendResult, ApiError> {
    let contract = resolve_reur_contract_address()?;
    let service_repo = FiatServiceWalletRepository::new(storage);
    let service_wallet = ensure_service_wallet(storage)?;

    let private_key_pem = service_repo
        .read_private_key()
        .map_err(|e| ApiError::internal(format!("Failed to read service wallet key: {e}")))?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(format!("Failed to load service wallet signer: {e}")))?;

    let amount_minor = parse_amount_to_token_minor_u256(amount_eur)?;
    let tx_builder = TxBuilder::new(AVAX_FUJI, eth_wallet)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to connect to chain: {e}")))?;

    tx_builder
        .send_token(to, &contract, amount_minor, None, None)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Reserve transfer failed: {e}")))
        .map(|result| {
            let _ = service_wallet;
            result
        })
}

async fn mint_to_address_from_service_wallet(
    storage: &Arc<crate::storage::EncryptedStorage>,
    to: &str,
    amount_eur: &str,
) -> Result<crate::blockchain::transactions::SendResult, ApiError> {
    let contract = resolve_reur_contract_address()?;
    let service_repo = FiatServiceWalletRepository::new(storage);

    let _service_wallet = ensure_service_wallet(storage)?;
    let private_key_pem = service_repo
        .read_private_key()
        .map_err(|e| ApiError::internal(format!("Failed to read service wallet key: {e}")))?;

    let eth_wallet = wallet_from_pem(&private_key_pem)
        .map_err(|e| ApiError::internal(format!("Failed to load service wallet signer: {e}")))?;

    let amount_minor = parse_amount_to_token_minor_u256(amount_eur)?;
    let to_addr = Address::from_str(to)
        .map_err(|e| ApiError::bad_request(format!("Invalid destination address: {e}")))?;
    let call = RelationalEuroMinter::mintCall {
        to: to_addr,
        amount: amount_minor,
    };

    let tx_builder = TxBuilder::new(AVAX_FUJI, eth_wallet)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to connect to chain: {e}")))?;

    tx_builder
        .send_contract_call(&contract, call.abi_encode(), None, None, None)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Reserve mint failed: {e}")))
}

async fn detect_confirmed_deposit(
    storage: &Arc<crate::storage::EncryptedStorage>,
    record: &StoredFiatRequest,
) -> Result<Option<String>, ApiError> {
    let reur_contract = resolve_reur_contract_address()?.to_ascii_lowercase();
    let service_wallet = record
        .service_wallet_address
        .as_deref()
        .ok_or_else(|| ApiError::internal("Missing service_wallet_address on fiat request"))?
        .to_ascii_lowercase();

    let expected_amount = parse_amount_to_token_minor_u256(&record.amount_eur)?;
    let min_confirmations = fiat_min_confirmations();

    let tx_repo = TransactionRepository::new(storage);
    let candidate_txs = tx_repo
        .list_by_wallet(&record.wallet_id)
        .map_err(|e| ApiError::internal(format!("Failed to list wallet txs: {e}")))?;

    let chain = AvaxClient::fuji()
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to connect to chain: {e}")))?;
    let current_block = chain
        .get_block_number()
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to read block number: {e}")))?;

    for tx in candidate_txs {
        let token_addr = match &tx.token {
            TokenType::Native => continue,
            TokenType::Erc20(addr) => addr,
        };

        if token_addr.to_ascii_lowercase() != reur_contract {
            continue;
        }
        if tx.to.to_ascii_lowercase() != service_wallet {
            continue;
        }

        let tx_amount = match parse_amount(&tx.amount, REUR_DECIMALS) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if tx_amount != expected_amount {
            continue;
        }

        match tx.status {
            TxStatus::Confirmed => {
                let confirmations = tx
                    .block_number
                    .map(|b| current_block.saturating_sub(b) + 1)
                    .unwrap_or(0);
                if confirmations >= min_confirmations {
                    return Ok(Some(tx.tx_hash));
                }
            }
            TxStatus::Pending => {
                let receipt = chain
                    .get_transaction_receipt_status(&tx.tx_hash)
                    .await
                    .map_err(|e| ApiError::service_unavailable(format!("Failed to read receipt: {e}")))?;

                if let Some(receipt) = receipt {
                    let _ = tx_repo.update_from_receipt(
                        &record.wallet_id,
                        &tx.tx_hash,
                        receipt.block_number,
                        receipt.gas_used,
                        receipt.success,
                    );

                    if receipt.success {
                        let confirmations = current_block.saturating_sub(receipt.block_number) + 1;
                        if confirmations >= min_confirmations {
                            return Ok(Some(tx.tx_hash));
                        }
                    }
                }
            }
            TxStatus::Failed => {}
        }
    }

    Ok(None)
}

async fn sync_onramp_request(
    storage: &Arc<crate::storage::EncryptedStorage>,
    record: &mut StoredFiatRequest,
) {
    if matches!(
        record.status,
        FiatRequestStatus::Queued | FiatRequestStatus::AwaitingProvider
    ) {
        if let Some(provider_reference) = record.provider_reference.as_deref() {
            if TrueLayerClient::is_configured() {
                match TrueLayerClient::from_env() {
                    Ok(client) => match client.fetch_onramp_status(provider_reference).await {
                        Ok(status) => {
                            record.status = map_onramp_provider_status(status);
                            record.last_provider_sync_at = Some(Utc::now());
                            record.updated_at = Utc::now();
                            if matches!(status, ProviderExecutionStatus::Failed) {
                                record.failure_reason = Some(
                                    "Provider reported failure for on-ramp request".to_string(),
                                );
                            }
                        }
                        Err(error) => {
                            warn!(
                                request_id = %record.request_id,
                                provider_reference = %provider_reference,
                                error = %error,
                                "failed to refresh on-ramp provider status"
                            );
                        }
                    },
                    Err(error) => {
                        warn!(
                            request_id = %record.request_id,
                            error = %error,
                            "failed to initialize provider client"
                        );
                    }
                }
            }
        }
    }

    if record.status == FiatRequestStatus::SettlementPending {
        if record.reserve_transfer_tx_hash.is_none() {
            let wallet_repo = WalletRepository::new(storage);
            let destination_wallet = match wallet_repo.get(&record.wallet_id) {
                Ok(wallet) => wallet,
                Err(error) => {
                    record.status = FiatRequestStatus::Failed;
                    record.failure_reason = Some(format!(
                        "Unable to load destination wallet for settlement: {error}"
                    ));
                    record.updated_at = Utc::now();
                    return;
                }
            };

            match send_reserve_transfer(storage, &destination_wallet.public_address, &record.amount_eur)
                .await
            {
                Ok(result) => {
                    record.reserve_transfer_tx_hash = Some(result.tx_hash.clone());
                    record.last_chain_sync_at = Some(Utc::now());
                    record.status = FiatRequestStatus::Completed;
                    record.failure_reason = None;
                    record.updated_at = Utc::now();

                    // Record the incoming rEUR transfer in the user's transaction history.
                    let reur_contract = resolve_reur_contract_address().unwrap_or_default();
                    let service_addr = record
                        .service_wallet_address
                        .clone()
                        .unwrap_or_default();
                    let tx_record = StoredTransaction::new_pending(
                        result.tx_hash.clone(),
                        record.wallet_id.clone(),
                        None,
                        service_addr,
                        destination_wallet.public_address.clone(),
                        record.amount_eur.clone(),
                        TokenType::Erc20(reur_contract),
                        record.chain_network.clone(),
                        result.explorer_url.clone(),
                    );
                    // The transfer already succeeded on-chain — mark confirmed.
                    let mut tx_record = tx_record;
                    tx_record.status = TxStatus::Confirmed;
                    let tx_repo = TransactionRepository::new(storage);
                    if let Err(e) = tx_repo.create(&tx_record) {
                        warn!(
                            request_id = %record.request_id,
                            tx_hash = %result.tx_hash,
                            "failed to store on-ramp settlement transaction record: {e}"
                        );
                    }
                }
                Err(error) => {
                    record.status = FiatRequestStatus::Failed;
                    record.failure_reason = Some(error.message.clone());
                    record.updated_at = Utc::now();
                }
            }
        }
    }
}

async fn sync_offramp_request(
    storage: &Arc<crate::storage::EncryptedStorage>,
    record: &mut StoredFiatRequest,
) {
    if record.status == FiatRequestStatus::AwaitingUserDeposit {
        match detect_confirmed_deposit(storage, record).await {
            Ok(Some(tx_hash)) => {
                record.deposit_tx_hash = Some(tx_hash);
                record.last_chain_sync_at = Some(Utc::now());
                record.updated_at = Utc::now();

                if !TrueLayerClient::is_configured() {
                    record.status = FiatRequestStatus::Failed;
                    record.failure_reason = Some(
                        "TrueLayer is not configured for off-ramp payout".to_string(),
                    );
                    return;
                }

                let beneficiary_account_holder_name =
                    match record.beneficiary_account_holder_name.as_deref() {
                        Some(value) => value,
                        None => {
                            record.status = FiatRequestStatus::Failed;
                            record.failure_reason = Some(
                                "Missing beneficiary account holder name".to_string(),
                            );
                            return;
                        }
                    };
                let beneficiary_iban = match record.beneficiary_iban.as_deref() {
                    Some(value) => value,
                    None => {
                        record.status = FiatRequestStatus::Failed;
                        record.failure_reason =
                            Some("Missing beneficiary IBAN".to_string());
                        return;
                    }
                };

                let amount_provider_minor = match parse_amount_to_minor(&record.amount_eur) {
                    Ok((_, minor)) => minor,
                    Err(error) => {
                        record.status = FiatRequestStatus::Failed;
                        record.failure_reason = Some(error.message.clone());
                        return;
                    }
                };

                let client = match TrueLayerClient::from_env() {
                    Ok(client) => client,
                    Err(error) => {
                        record.status = FiatRequestStatus::Failed;
                        record.failure_reason = Some(map_provider_error(error).message);
                        return;
                    }
                };

                match client
                    .create_offramp(CreateOffRampRequest {
                        request_id: &record.request_id,
                        wallet_id: &record.wallet_id,
                        user_id: &record.owner_user_id,
                        amount_in_minor: amount_provider_minor,
                        amount_eur: &record.amount_eur,
                        beneficiary_account_holder_name,
                        beneficiary_iban,
                        note: record.note.as_deref(),
                    })
                    .await
                {
                    Ok(execution) => {
                        record.provider_reference = Some(execution.provider_reference);
                        record.provider_action_url = execution.provider_action_url;
                        record.status = map_offramp_provider_status(execution.status);
                        record.last_provider_sync_at = Some(Utc::now());
                        record.updated_at = Utc::now();
                        if matches!(execution.status, ProviderExecutionStatus::Failed) {
                            record.failure_reason =
                                Some("Provider payout initialization failed".to_string());
                        } else {
                            record.failure_reason = None;
                        }
                    }
                    Err(error) => {
                        let mapped = map_provider_error(error);
                        record.status = FiatRequestStatus::Failed;
                        record.failure_reason = Some(mapped.message);
                        record.updated_at = Utc::now();
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                warn!(
                    request_id = %record.request_id,
                    error = %error.message,
                    "off-ramp deposit sync failed"
                );
            }
        }
    }

    if matches!(
        record.status,
        FiatRequestStatus::ProviderPending | FiatRequestStatus::AwaitingProvider
    ) {
        let Some(provider_reference) = record.provider_reference.as_deref() else {
            return;
        };
        if !TrueLayerClient::is_configured() {
            return;
        }

        let client = match TrueLayerClient::from_env() {
            Ok(client) => client,
            Err(error) => {
                warn!(
                    request_id = %record.request_id,
                    error = %error,
                    "failed to initialize provider client for off-ramp refresh"
                );
                return;
            }
        };

        match client.fetch_offramp_status(provider_reference).await {
            Ok(status) => {
                record.status = map_offramp_provider_status(status);
                record.last_provider_sync_at = Some(Utc::now());
                record.updated_at = Utc::now();
                if matches!(status, ProviderExecutionStatus::Failed) {
                    record.failure_reason = Some("Provider reported payout failure".to_string());
                }
            }
            Err(error) => {
                warn!(
                    request_id = %record.request_id,
                    provider_reference = %provider_reference,
                    error = %error,
                    "failed to refresh off-ramp provider status"
                );
            }
        }
    }
}

async fn sync_request_internal(
    storage: &Arc<crate::storage::EncryptedStorage>,
    record: &mut StoredFiatRequest,
) {
    if let Err(error) = ensure_fuji_network(Some(record.chain_network.as_str())) {
        record.status = FiatRequestStatus::Failed;
        record.failure_reason = Some(error);
        record.updated_at = Utc::now();
        return;
    }

    match record.direction {
        FiatDirection::OnRamp => sync_onramp_request(storage, record).await,
        FiatDirection::OffRamp => sync_offramp_request(storage, record).await,
    }
}

pub(crate) async fn sync_and_persist_request(
    storage: &Arc<crate::storage::EncryptedStorage>,
    request_id: &str,
) -> Result<StoredFiatRequest, ApiError> {
    let repo = FiatRequestRepository::new(storage);
    let mut record = repo
        .get(request_id)
        .map_err(|_| ApiError::not_found("Fiat request not found"))?;

    sync_request_internal(storage, &mut record).await;

    // Re-read from storage to avoid overwriting webhook-driven terminal status.
    // While we were calling the provider API (~200ms), the webhook handler may
    // have already set this record to Failed/Completed. Honour that.
    if let Ok(current) = repo.get(request_id) {
        if matches!(
            current.status,
            FiatRequestStatus::Completed | FiatRequestStatus::Failed
        ) && !matches!(
            record.status,
            FiatRequestStatus::Completed | FiatRequestStatus::Failed
        ) {
            info!(
                request_id = %request_id,
                stored_status = ?current.status,
                polled_status = ?record.status,
                "Skipping poller persist — webhook already set terminal status"
            );
            return Ok(current);
        }
    }

    repo.update(&record)
        .map_err(|e| ApiError::internal(format!("Failed to persist fiat request sync: {e}")))?;

    Ok(record)
}

/// Return request IDs of all fiat requests in a syncable (non-terminal) status.
pub(crate) fn list_pending_request_ids(
    storage: &Arc<crate::storage::EncryptedStorage>,
) -> Vec<String> {
    let repo = FiatRequestRepository::new(storage);
    let requests = match repo.list_all() {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "FiatPoller: failed to list fiat requests");
            return Vec::new();
        }
    };

    requests
        .into_iter()
        .filter(|r| {
            matches!(
                r.status,
                FiatRequestStatus::Queued
                    | FiatRequestStatus::AwaitingProvider
                    | FiatRequestStatus::AwaitingUserDeposit
                    | FiatRequestStatus::SettlementPending
                    | FiatRequestStatus::ProviderPending
            )
        })
        .map(|r| r.request_id)
        .collect()
}

/// List supported fiat providers for sandbox testing.
#[utoipa::path(
    get,
    path = "/v1/fiat/providers",
    tag = "Fiat",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Supported fiat providers", body = FiatProviderListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_fiat_providers(Auth(_user): Auth) -> Json<FiatProviderListResponse> {
    Json(FiatProviderListResponse {
        default_provider: DEFAULT_PROVIDER.to_string(),
        providers: provider_summaries(),
    })
}

async fn create_request(
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<CreateFiatRequest>,
    direction: FiatDirection,
) -> Result<(StatusCode, Json<FiatRequestResponse>), ApiError> {
    ensure_fuji_network(Some("fuji")).map_err(ApiError::bad_request)?;

    let CreateFiatRequest {
        wallet_id,
        amount_eur,
        provider,
        note,
        beneficiary_account_holder_name,
        beneficiary_iban,
    } = request;

    let (normalized_amount, amount_in_minor_provider) = parse_amount_to_minor(&amount_eur)?;
    let expected_amount_token_minor = u256_to_u64(parse_amount_to_token_minor_u256(&normalized_amount)?)?;
    let storage = state.storage();

    // Ensure settlement prerequisites are available.
    let _ = resolve_reur_contract_address()?;
    let service_wallet = ensure_service_wallet(storage)?;

    let wallet_repo = WalletRepository::new(storage);
    let wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found("Wallet not found"))?;

    if wallet.owner_user_id != user.user_id {
        return Err(ApiError::forbidden("You do not own this wallet"));
    }
    if wallet.status != WalletStatus::Active {
        return Err(ApiError::forbidden(
            "Wallet must be active for fiat requests",
        ));
    }

    let provider = resolve_provider_id(provider)?;
    ensure_provider_enabled(&provider, direction)?;

    let note = note.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let beneficiary = if direction == FiatDirection::OffRamp {
        Some((
            normalize_offramp_account_holder_name(beneficiary_account_holder_name)?,
            normalize_offramp_iban(beneficiary_iban)?,
        ))
    } else {
        None
    };

    let request_id = uuid::Uuid::new_v4().to_string();
    let mut record = StoredFiatRequest::new_queued(
        request_id,
        wallet_id,
        user.user_id.clone(),
        direction,
        normalized_amount.clone(),
        provider.clone(),
        note.clone(),
    );
    record.chain_network = "fuji".to_string();
    record.expected_amount_minor = Some(expected_amount_token_minor);
    record.service_wallet_address = Some(service_wallet.public_address.clone());

    if let Some((beneficiary_name, beneficiary_iban)) = beneficiary {
        record.beneficiary_account_holder_name = Some(beneficiary_name);
        record.beneficiary_iban = Some(beneficiary_iban);
    }

    if direction == FiatDirection::OnRamp {
        let client = TrueLayerClient::from_env().map_err(map_provider_error)?;
        let execution = client
            .create_onramp(CreateOnRampRequest {
                request_id: &record.request_id,
                wallet_id: &record.wallet_id,
                user_id: &record.owner_user_id,
                amount_in_minor: amount_in_minor_provider,
                amount_eur: &normalized_amount,
                note: note.as_deref(),
            })
            .await
            .map_err(map_provider_error)?;

        record.provider_reference = Some(execution.provider_reference);
        record.provider_action_url = execution.provider_action_url;
        record.status = map_onramp_provider_status(execution.status);
        record.updated_at = Utc::now();
        if matches!(execution.status, ProviderExecutionStatus::Failed) {
            record.failure_reason = Some("Provider on-ramp initialization failed".to_string());
        }
    } else {
        record.status = FiatRequestStatus::AwaitingUserDeposit;
        record.updated_at = Utc::now();
    }

    let repo = FiatRequestRepository::new(storage);
    repo.create(&record)
        .map_err(|e| ApiError::internal(format!("Failed to store fiat request: {e}")))?;

    let audit_event = match direction {
        FiatDirection::OnRamp => AuditEventType::FiatOnRampRequested,
        FiatDirection::OffRamp => AuditEventType::FiatOffRampRequested,
    };
    audit_log!(
        storage,
        audit_event,
        &user,
        "fiat_request",
        &record.request_id
    );

    Ok((StatusCode::CREATED, Json(to_response(&record))))
}

/// Create fiat on-ramp request.
#[utoipa::path(
    post,
    path = "/v1/fiat/onramp/requests",
    tag = "Fiat",
    request_body = CreateFiatRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Fiat on-ramp request created", body = FiatRequestResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Wallet not found"),
        (status = 503, description = "Provider unavailable")
    )
)]
pub async fn create_onramp_request(
    auth: Auth,
    state: State<AppState>,
    body: Json<CreateFiatRequest>,
) -> Result<(StatusCode, Json<FiatRequestResponse>), ApiError> {
    create_request(auth, state, body, FiatDirection::OnRamp).await
}

/// Create fiat off-ramp request.
#[utoipa::path(
    post,
    path = "/v1/fiat/offramp/requests",
    tag = "Fiat",
    request_body = CreateFiatRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Fiat off-ramp request created", body = FiatRequestResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Wallet not found"),
        (status = 503, description = "Provider unavailable")
    )
)]
pub async fn create_offramp_request(
    auth: Auth,
    state: State<AppState>,
    body: Json<CreateFiatRequest>,
) -> Result<(StatusCode, Json<FiatRequestResponse>), ApiError> {
    create_request(auth, state, body, FiatDirection::OffRamp).await
}

/// List fiat requests for current user.
#[utoipa::path(
    get,
    path = "/v1/fiat/requests",
    tag = "Fiat",
    params(FiatRequestListQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fiat requests listed", body = FiatRequestListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_fiat_requests(
    Auth(user): Auth,
    State(state): State<AppState>,
    Query(query): Query<FiatRequestListQuery>,
) -> Result<Json<FiatRequestListResponse>, ApiError> {
    let storage = state.storage();
    let repo = FiatRequestRepository::new(storage);

    let requests = match query.wallet_id.as_deref() {
        Some(wallet_id) => repo
            .list_by_wallet_for_owner(&user.user_id, wallet_id)
            .map_err(|e| ApiError::internal(format!("Failed to list fiat requests: {e}")))?,
        None => repo
            .list_by_owner(&user.user_id)
            .map_err(|e| ApiError::internal(format!("Failed to list fiat requests: {e}")))?,
    };

    // Serve cached status — the background FiatPoller handles provider syncing
    // every 30 s. This avoids inline TrueLayer API calls on every page view.

    let mapped: Vec<FiatRequestResponse> = requests.iter().map(to_response).collect();

    Ok(Json(FiatRequestListResponse {
        total: mapped.len(),
        requests: mapped,
    }))
}

/// Get fiat request by ID.
#[utoipa::path(
    get,
    path = "/v1/fiat/requests/{request_id}",
    tag = "Fiat",
    params(
        ("request_id" = String, Path, description = "Fiat request ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fiat request details", body = FiatRequestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_fiat_request(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(request_id): Path<String>,
) -> Result<Json<FiatRequestResponse>, ApiError> {
    let storage = state.storage();
    let repo = FiatRequestRepository::new(storage);
    let record = repo
        .get(&request_id)
        .map_err(|_| ApiError::not_found("Fiat request not found"))?;

    if record.owner_user_id != user.user_id {
        return Err(ApiError::forbidden(
            "You do not have permission to access this fiat request",
        ));
    }

    // Serve cached status — the background FiatPoller handles provider syncing.

    Ok(Json(to_response(&record)))
}

/// TrueLayer webhook callback endpoint.
///
/// Validates the `Tl-Signature` JWS header using TrueLayer's JWKS public keys
/// before processing the webhook payload.
#[utoipa::path(
    post,
    path = "/v1/fiat/providers/truelayer/webhook",
    tag = "Fiat",
    request_body = TrueLayerWebhookPayload,
    responses(
        (status = 202, description = "Webhook accepted"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden — invalid signature")
    )
)]
pub async fn truelayer_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    // Verify Tl-Signature using JWKS before touching the body
    verify_webhook_signature(&headers, &body).await?;

    // Parse the raw body into our webhook payload struct
    let payload: TrueLayerWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("Invalid webhook payload: {e}")))?;

    let provider_reference = extract_provider_reference(&payload)
        .ok_or_else(|| ApiError::bad_request("Webhook payload missing provider reference"))?;

    let storage = state.storage();
    let repo = FiatRequestRepository::new(storage);
    let mut requests = repo
        .list_all()
        .map_err(|e| ApiError::internal(format!("Failed to list fiat requests: {e}")))?;

    let Some(record) = requests
        .iter_mut()
        .find(|record| record.provider_reference.as_deref() == Some(provider_reference.as_str()))
    else {
        return Ok(StatusCode::ACCEPTED);
    };

    // ── Idempotency: skip duplicate webhooks ──
    if let Some(ref incoming_event_id) = payload.event_id {
        if record.provider_event_id.as_deref() == Some(incoming_event_id.as_str()) {
            info!(
                request_id = %record.request_id,
                event_id = %incoming_event_id,
                "Duplicate webhook event — skipping"
            );
            return Ok(StatusCode::ACCEPTED);
        }
    }

    // ── Prevent backward status transitions ──
    if let Some(new_status) = extract_webhook_status(&payload) {
        let new_mapped = if record.direction == FiatDirection::OnRamp {
            map_onramp_provider_status(new_status)
        } else {
            map_offramp_provider_status(new_status)
        };

        let is_terminal = matches!(
            record.status,
            FiatRequestStatus::Completed | FiatRequestStatus::Failed
        );

        if !is_terminal {
            record.status = new_mapped;
            if matches!(new_status, ProviderExecutionStatus::Failed) {
                // Use failure_reason from TrueLayer webhook when available,
                // fall back to generic message.
                record.failure_reason = Some(
                    payload
                        .failure_reason
                        .clone()
                        .unwrap_or_else(|| "Webhook reported provider failure".to_string()),
                );
            }
        }

        info!(
            request_id = %record.request_id,
            event_type = ?payload.event_type,
            provider_status = ?new_status,
            mapped_status = ?new_mapped,
            scheme_id = ?payload.scheme_id,
            failure_reason = ?payload.failure_reason,
            is_terminal,
            "Webhook status transition"
        );
    }

    if let Some(event_id) = payload.event_id.clone() {
        record.provider_event_id = Some(event_id);
    }

    record.last_provider_sync_at = Some(Utc::now());
    record.updated_at = Utc::now();
    // NOTE: We do NOT call sync_request_internal here — the webhook already
    // provides the definitive status from TrueLayer. Calling sync would
    // re-poll the provider API inline, duplicating work the FiatPoller
    // background task already handles.

    repo.update(record)
        .map_err(|e| ApiError::internal(format!("Failed to persist webhook update: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}

/// Get fiat reserve service-wallet status.
#[utoipa::path(
    get,
    path = "/v1/admin/fiat/service-wallet",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fiat reserve wallet status", body = FiatServiceWalletStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn get_fiat_service_wallet(
    AdminOnly(_admin): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<FiatServiceWalletStatusResponse>, ApiError> {
    let storage = state.storage();
    let service_wallet = ensure_service_wallet(storage)?;
    let contract_address = resolve_reur_contract_address()?;

    let client = AvaxClient::fuji()
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to connect to chain: {e}")))?;

    let native = client
        .get_native_balance(&service_wallet.public_address)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to read AVAX balance: {e}")))?;

    let reur = client
        .get_token_balance(&service_wallet.public_address, &contract_address)
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to read rEUR balance: {e}")))?;

    Ok(Json(FiatServiceWalletStatusResponse {
        wallet_id: service_wallet.wallet_id,
        public_address: service_wallet.public_address,
        bootstrapped: true,
        chain_network: "fuji".to_string(),
        reur_contract_address: contract_address,
        avax_balance: native.balance_formatted,
        reur_balance: reur.balance_formatted,
        reur_balance_raw: reur.balance_raw,
    }))
}

/// Idempotently bootstrap fiat reserve service wallet.
#[utoipa::path(
    post,
    path = "/v1/admin/fiat/service-wallet/bootstrap",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fiat reserve wallet bootstrapped", body = FiatServiceWalletStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn bootstrap_fiat_service_wallet(
    AdminOnly(_admin): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<FiatServiceWalletStatusResponse>, ApiError> {
    let storage = state.storage();
    let repo = FiatServiceWalletRepository::new(storage);
    let _ = repo
        .bootstrap()
        .map_err(|e| ApiError::internal(format!("Failed to bootstrap service wallet: {e}")))?;

    get_fiat_service_wallet(AdminOnly(_admin), State(state)).await
}

/// Mint rEUR into reserve wallet.
#[utoipa::path(
    post,
    path = "/v1/admin/fiat/reserve/topup",
    tag = "Admin",
    request_body = ReserveTopUpRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Reserve top-up submitted", body = ReserveTransactionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 503, description = "Service unavailable")
    )
)]
pub async fn topup_fiat_reserve(
    AdminOnly(_admin): AdminOnly,
    State(state): State<AppState>,
    Json(request): Json<ReserveTopUpRequest>,
) -> Result<Json<ReserveTransactionResponse>, ApiError> {
    let storage = state.storage();
    let service_wallet = ensure_service_wallet(storage)?;

    let amount = request
        .amount_eur
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(reserve_initial_topup_amount);

    let amount_minor = parse_amount_to_token_minor_u256(&amount)?;
    let tx = mint_to_address_from_service_wallet(storage, &service_wallet.public_address, &amount).await?;

    Ok(Json(ReserveTransactionResponse {
        tx_hash: tx.tx_hash,
        explorer_url: tx.explorer_url,
        amount_eur: amount,
        amount_minor: amount_minor.to_string(),
    }))
}

/// Transfer rEUR from reserve wallet to destination.
#[utoipa::path(
    post,
    path = "/v1/admin/fiat/reserve/transfer",
    tag = "Admin",
    request_body = ReserveTransferRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Reserve transfer submitted", body = ReserveTransactionResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn transfer_fiat_reserve(
    AdminOnly(_admin): AdminOnly,
    State(state): State<AppState>,
    Json(request): Json<ReserveTransferRequest>,
) -> Result<Json<ReserveTransactionResponse>, ApiError> {
    if !request.to.starts_with("0x")
        || request.to.len() != 42
        || !request.to[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(ApiError::bad_request("Invalid destination EVM address"));
    }

    let amount_minor = parse_amount_to_token_minor_u256(&request.amount_eur)?;
    let tx = send_reserve_transfer(state.storage(), &request.to, &request.amount_eur).await?;

    Ok(Json(ReserveTransactionResponse {
        tx_hash: tx.tx_hash,
        explorer_url: tx.explorer_url,
        amount_eur: request.amount_eur,
        amount_minor: amount_minor.to_string(),
    }))
}

/// Manual fiat request sync.
#[utoipa::path(
    post,
    path = "/v1/admin/fiat/requests/{request_id}/sync",
    tag = "Admin",
    params(
        ("request_id" = String, Path, description = "Fiat request ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fiat request synchronized", body = FiatSyncResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn sync_fiat_request_admin(
    AdminOnly(_admin): AdminOnly,
    State(state): State<AppState>,
    Path(request_id): Path<String>,
) -> Result<Json<FiatSyncResponse>, ApiError> {
    let record = sync_and_persist_request(state.storage(), &request_id).await?;
    Ok(Json(FiatSyncResponse {
        request: to_response(&record),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_provider_defaults_to_truelayer_sandbox() {
        let provider = resolve_provider_id(None).expect("default provider should resolve");
        assert_eq!(provider, "truelayer_sandbox");
    }

    #[test]
    fn resolve_provider_rejects_unknown_provider() {
        let error = resolve_provider_id(Some("unknown_provider".to_string()))
            .expect_err("unknown provider should fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn parse_amount_rejects_non_positive_values() {
        let error = parse_amount_to_minor("0").expect_err("zero amount should fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn parse_amount_converts_to_minor_units() {
        let (normalized, minor) = parse_amount_to_minor("25.5").expect("valid amount");
        assert_eq!(normalized, "25.50");
        assert_eq!(minor, 2550);
    }

    #[test]
    fn parse_amount_rejects_too_many_decimals() {
        let error = parse_amount_to_minor("1.234").expect_err("too many decimals should fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn normalize_offramp_iban_rejects_invalid_characters() {
        let error = normalize_offramp_iban(Some("GB79 CLRB 04066800*102649".to_string()))
            .expect_err("invalid IBAN should fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn normalize_offramp_iban_compacts_and_uppercases() {
        let iban = normalize_offramp_iban(Some("gb79 clrb 04066800102649".to_string()))
            .expect("valid IBAN");
        assert_eq!(iban, "GB79CLRB04066800102649");
    }

    #[test]
    fn webhook_status_mapping_is_reasonable() {
        assert_eq!(
            map_webhook_status("executed"),
            ProviderExecutionStatus::Completed
        );
        assert_eq!(map_webhook_status("failed"), ProviderExecutionStatus::Failed);
        assert_eq!(map_webhook_status("pending"), ProviderExecutionStatus::Pending);
    }
}
