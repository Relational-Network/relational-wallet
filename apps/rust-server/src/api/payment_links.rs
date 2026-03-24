// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Payment link endpoints.
//!
//! Payment links let a wallet owner generate an opaque URL token that
//! encodes their public address + optional amount/note. Recipients can
//! resolve the token without authentication to get the payment details.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};

use crate::{
    auth::Auth,
    error::ApiError,
    models::{CreatePaymentLinkRequest, CreatePaymentLinkResponse, PaymentLinkInfo},
    providers::email,
    state::AppState,
    storage::{OwnershipEnforcer, PaymentLinkData, PaymentLinkRepository, WalletRepository},
};

/// Create a payment link for a wallet.
///
/// Generates an opaque token that resolves to either the wallet's public
/// address or the owner's verified email hash/display, depending on the
/// requested recipient type.
#[utoipa::path(
    post,
    path = "/v1/wallets/{wallet_id}/payment-link",
    request_body = CreatePaymentLinkRequest,
    params(
        ("wallet_id" = String, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, body = CreatePaymentLinkResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not wallet owner"),
        (status = 404, description = "Wallet not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "payment_links"
)]
pub async fn create_payment_link(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Json(request): Json<CreatePaymentLinkRequest>,
) -> Result<Json<CreatePaymentLinkResponse>, ApiError> {
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(&storage);

    // Get and verify wallet ownership
    let wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found("Wallet not found"))?;

    wallet
        .verify_ownership(&user)
        .map_err(|_| ApiError::forbidden("You don't have permission for this wallet"))?;

    let tx_db = state
        .tx_db
        .as_ref()
        .ok_or_else(|| ApiError::internal("Transaction database not available"))?;

    let expires_hours = if request.expires_hours == 0 {
        24
    } else {
        request.expires_hours.min(720) // Cap at 30 days
    };

    let expires_at = Utc::now() + Duration::hours(expires_hours as i64);

    let (recipient_type, public_address, to_email_hash, email_display) =
        match request.recipient_type.as_str() {
            "email" => {
                let email_hash = request.to_email_hash.ok_or_else(|| {
                    ApiError::bad_request("to_email_hash is required for email payment links")
                })?;
                if !email::validate_email_hash(&email_hash) {
                    return Err(ApiError::bad_request(
                        "to_email_hash must be 64 hex characters",
                    ));
                }

                let email_display = request.email_display.ok_or_else(|| {
                    ApiError::bad_request("email_display is required for email payment links")
                })?;

                let expected_lookup_key = wallet.email_lookup_key.as_ref().ok_or_else(|| {
                    ApiError::unprocessable("Wallet is not linked to a verified email")
                })?;
                let actual_lookup_key =
                    email::hmac_lookup_key(&state.email_hmac_key, &email_hash);
                if &actual_lookup_key != expected_lookup_key {
                    return Err(ApiError::unprocessable(
                        "Email payment links must use the wallet owner's verified email",
                    ));
                }

                ("email".to_string(), None, Some(email_hash), Some(email_display))
            }
            "address" => (
                "address".to_string(),
                Some(wallet.public_address.clone()),
                None,
                None,
            ),
            _ => {
                return Err(ApiError::bad_request(
                    "recipient_type must be 'address' or 'email'",
                ))
            }
        };

    let link_data = PaymentLinkData {
        wallet_id: wallet_id.clone(),
        recipient_type,
        public_address,
        to_email_hash,
        email_display,
        amount: request.amount,
        token_type: request.token,
        note: request.note,
        expires_at,
        single_use: request.single_use,
        used: false,
    };

    let repo = PaymentLinkRepository::new(Arc::clone(tx_db));
    let token = repo
        .create(link_data)
        .map_err(|e| ApiError::internal(&format!("Failed to create payment link: {}", e)))?;

    Ok(Json(CreatePaymentLinkResponse {
        token,
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// Resolve a payment link token (no authentication required).
///
/// Returns the tagged recipient info and optional amount/note for the payment link.
#[utoipa::path(
    get,
    path = "/v1/payment-link/{token}",
    params(
        ("token" = String, Path, description = "Payment link token")
    ),
    responses(
        (status = 200, body = PaymentLinkInfo),
        (status = 404, description = "Payment link not found or expired"),
    ),
    tag = "payment_links"
)]
pub async fn resolve_payment_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<PaymentLinkInfo>, ApiError> {
    let tx_db = state
        .tx_db
        .as_ref()
        .ok_or_else(|| ApiError::internal("Transaction database not available"))?;

    let repo = PaymentLinkRepository::new(Arc::clone(tx_db));
    let data = repo
        .resolve(&token)
        .map_err(|e| ApiError::internal(&format!("Failed to resolve payment link: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Payment link not found or expired"))?;

    Ok(Json(PaymentLinkInfo {
        recipient_type: data.recipient_type,
        public_address: data.public_address,
        to_email_hash: data.to_email_hash,
        email_display: data.email_display,
        amount: data.amount,
        token_type: data.token_type,
        note: data.note,
    }))
}
