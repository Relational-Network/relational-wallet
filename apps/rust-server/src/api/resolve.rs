// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Email resolution endpoint.
//!
//! Allows authenticated users to check if a wallet exists for a given email hash,
//! without revealing the address. Used by the frontend during the "send to email"
//! flow to let the sender know the recipient has a wallet.

use axum::{extract::State, Json};

use crate::{
    auth::Auth,
    error::ApiError,
    models::{ResolveEmailRequest, ResolveEmailResponse},
    providers::email,
    state::AppState,
    storage::EmailIndexRepository,
};

/// Resolve an email hash to check if it maps to a registered wallet.
///
/// The client sends a SHA-256 hash of the normalized email. The server
/// computes the HMAC lookup key and checks the email index.
///
/// Returns `{ found: true }` if a wallet exists for that email, `{ found: false }` otherwise.
/// **Does NOT reveal the address** — the address is only resolved server-side
/// during the actual send flow.
#[utoipa::path(
    post,
    path = "/v1/resolve/email",
    request_body = ResolveEmailRequest,
    responses(
        (status = 200, body = ResolveEmailResponse),
        (status = 400, description = "Invalid email hash"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "resolve"
)]
pub async fn resolve_email(
    Auth(_user): Auth,
    State(state): State<AppState>,
    Json(body): Json<ResolveEmailRequest>,
) -> Result<Json<ResolveEmailResponse>, ApiError> {
    // Validate email_hash is 64 hex characters (SHA-256 output)
    if !email::validate_email_hash(&body.email_hash) {
        return Err(ApiError::bad_request(
            "email_hash must be exactly 64 lowercase hex characters",
        ));
    }

    let tx_db = state
        .tx_db
        .as_ref()
        .ok_or_else(|| ApiError::internal("Transaction database not available"))?;

    // Compute HMAC lookup key from the client-provided SHA-256 hash
    let lookup_key = email::hmac_lookup_key(&state.email_hmac_key, &body.email_hash);

    // Check email index
    let email_repo = EmailIndexRepository::new(tx_db.clone());
    let found = email_repo
        .exists(&lookup_key)
        .map_err(|e| ApiError::internal(&format!("Email lookup failed: {}", e)))?;

    Ok(Json(ResolveEmailResponse { found }))
}
