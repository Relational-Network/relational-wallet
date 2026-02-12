// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Fiat on-ramp/off-ramp API with TrueLayer sandbox integration.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::{IntoParams, ToSchema};

use crate::{
    audit_log,
    auth::Auth,
    error::ApiError,
    providers::truelayer::{
        CreateOffRampRequest, CreateOnRampRequest, ProviderExecutionStatus, TrueLayerClient,
        TrueLayerError,
    },
    state::AppState,
    storage::{
        AuditEventType, FiatDirection, FiatRequestRepository, FiatRequestStatus, StoredFiatRequest,
        WalletRepository, WalletStatus,
    },
};

const DEFAULT_PROVIDER: &str = "truelayer_sandbox";
const SUPPORTED_PROVIDER_IDS: [&str; 1] = [DEFAULT_PROVIDER];

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
    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Optional provider reference/session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_reference: Option<String>,
    /// Optional provider action URL (for redirect/continue flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_action_url: Option<String>,
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

fn provider_summaries() -> Vec<FiatProviderSummary> {
    let enabled = TrueLayerClient::is_configured();
    let supports_on_ramp = enabled;
    let supports_off_ramp = enabled && TrueLayerClient::supports_offramp();
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

fn ensure_provider_enabled(provider: &str, direction: FiatDirection) -> Result<(), ApiError> {
    if provider != DEFAULT_PROVIDER {
        return Err(ApiError::bad_request("Unsupported provider"));
    }

    if !TrueLayerClient::is_configured() {
        return Err(ApiError::service_unavailable(
            "TrueLayer sandbox is not configured. Set TRUELAYER_* environment variables.",
        ));
    }

    if direction == FiatDirection::OffRamp && !TrueLayerClient::supports_offramp() {
        return Err(ApiError::service_unavailable(
            "TrueLayer off-ramp is not configured. Set TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME and TRUELAYER_OFFRAMP_IBAN.",
        ));
    }

    Ok(())
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

fn map_provider_status(status: ProviderExecutionStatus) -> FiatRequestStatus {
    match status {
        ProviderExecutionStatus::Completed => FiatRequestStatus::Completed,
        ProviderExecutionStatus::Failed => FiatRequestStatus::Failed,
        ProviderExecutionStatus::Pending => FiatRequestStatus::ProviderPending,
    }
}

fn to_response(record: &StoredFiatRequest) -> FiatRequestResponse {
    FiatRequestResponse {
        request_id: record.request_id.clone(),
        wallet_id: record.wallet_id.clone(),
        direction: record.direction,
        amount_eur: record.amount_eur.clone(),
        provider: record.provider.clone(),
        status: record.status,
        note: record.note.clone(),
        provider_reference: record.provider_reference.clone(),
        provider_action_url: record.provider_action_url.clone(),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

async fn try_refresh_provider_status(
    repo: &FiatRequestRepository<'_>,
    record: &mut StoredFiatRequest,
) {
    if record.provider != DEFAULT_PROVIDER {
        return;
    }
    if !matches!(
        record.status,
        FiatRequestStatus::Queued | FiatRequestStatus::ProviderPending
    ) {
        return;
    }
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
                "skipping fiat provider refresh due to configuration/runtime error"
            );
            return;
        }
    };

    let provider_status = match record.direction {
        FiatDirection::OnRamp => client.fetch_onramp_status(provider_reference).await,
        FiatDirection::OffRamp => client.fetch_offramp_status(provider_reference).await,
    };

    let Ok(provider_status) = provider_status else {
        warn!(
            request_id = %record.request_id,
            provider_reference = %provider_reference,
            "failed to refresh fiat status from provider"
        );
        return;
    };

    let mapped_status = map_provider_status(provider_status);
    if mapped_status != record.status {
        record.status = mapped_status;
        record.updated_at = Utc::now();
        if let Err(error) = repo.update(record) {
            warn!(
                request_id = %record.request_id,
                error = %error,
                "failed to persist refreshed fiat status"
            );
        }
    }
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
    let (normalized_amount, amount_in_minor) = parse_amount_to_minor(&request.amount_eur)?;
    let storage = state.storage();

    let wallet_repo = WalletRepository::new(&storage);
    let wallet = wallet_repo
        .get(&request.wallet_id)
        .map_err(|_| ApiError::not_found("Wallet not found"))?;

    if wallet.owner_user_id != user.user_id {
        return Err(ApiError::forbidden("You do not own this wallet"));
    }
    if wallet.status != WalletStatus::Active {
        return Err(ApiError::forbidden(
            "Wallet must be active for fiat requests",
        ));
    }

    let provider = resolve_provider_id(request.provider)?;
    ensure_provider_enabled(&provider, direction)?;

    let note = request.note.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let request_id = uuid::Uuid::new_v4().to_string();
    let mut record = StoredFiatRequest::new_queued(
        request_id,
        request.wallet_id,
        user.user_id.clone(),
        direction,
        normalized_amount.clone(),
        provider.clone(),
        note.clone(),
    );

    if provider == DEFAULT_PROVIDER {
        let client = TrueLayerClient::from_env().map_err(map_provider_error)?;
        let execution = match direction {
            FiatDirection::OnRamp => {
                client
                    .create_onramp(CreateOnRampRequest {
                        request_id: &record.request_id,
                        wallet_id: &record.wallet_id,
                        user_id: &record.owner_user_id,
                        amount_in_minor,
                        amount_eur: &normalized_amount,
                        note: note.as_deref(),
                    })
                    .await
            }
            FiatDirection::OffRamp => {
                client
                    .create_offramp(CreateOffRampRequest {
                        request_id: &record.request_id,
                        wallet_id: &record.wallet_id,
                        user_id: &record.owner_user_id,
                        amount_in_minor,
                        amount_eur: &normalized_amount,
                        note: note.as_deref(),
                    })
                    .await
            }
        }
        .map_err(map_provider_error)?;

        record.provider_reference = Some(execution.provider_reference);
        record.provider_action_url = execution.provider_action_url;
        record.status = map_provider_status(execution.status);
        record.updated_at = Utc::now();
    }

    let repo = FiatRequestRepository::new(&storage);
    repo.create(&record)
        .map_err(|e| ApiError::internal(format!("Failed to store fiat request: {e}")))?;

    let audit_event = match direction {
        FiatDirection::OnRamp => AuditEventType::FiatOnRampRequested,
        FiatDirection::OffRamp => AuditEventType::FiatOffRampRequested,
    };
    audit_log!(
        &storage,
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
    let repo = FiatRequestRepository::new(&storage);

    let mut requests = match query.wallet_id.as_deref() {
        Some(wallet_id) => repo
            .list_by_wallet_for_owner(&user.user_id, wallet_id)
            .map_err(|e| ApiError::internal(format!("Failed to list fiat requests: {e}")))?,
        None => repo
            .list_by_owner(&user.user_id)
            .map_err(|e| ApiError::internal(format!("Failed to list fiat requests: {e}")))?,
    };

    for request in &mut requests {
        try_refresh_provider_status(&repo, request).await;
    }

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
    let repo = FiatRequestRepository::new(&storage);
    let mut record = repo
        .get(&request_id)
        .map_err(|_| ApiError::not_found("Fiat request not found"))?;

    if record.owner_user_id != user.user_id {
        return Err(ApiError::forbidden(
            "You do not have permission to access this fiat request",
        ));
    }

    try_refresh_provider_status(&repo, &mut record).await;
    Ok(Json(to_response(&record)))
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
}
