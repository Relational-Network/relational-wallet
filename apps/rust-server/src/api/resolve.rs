// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Email resolution endpoint.
//!
//! Allows authenticated users to check if a wallet exists for a given email hash,
//! without revealing the address. Used by the frontend during the "send to email"
//! flow to let the sender know the recipient has a wallet.
//!
//! When the `discovery` feature is enabled and no local wallet is found, the
//! endpoint fans out to all configured peer enclaves via the two-phase VOPRF
//! protocol (Phase 2 cross-instance discovery).

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
/// If `discovery` is enabled and no local wallet is found, queries peer
/// enclaves using the VOPRF protocol.
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

    // DEBUG-VOPRF: remove after debugging — resolve entry
    tracing::info!(
        email_hash = %body.email_hash,
        local_lookup_key = %lookup_key,
        "[VOPRF-DBG] resolve_email: entered"
    );
    // END DEBUG-VOPRF

    // Check email index locally first
    let email_repo = EmailIndexRepository::new(tx_db.clone());
    let found = email_repo
        .exists(&lookup_key)
        .map_err(|e| ApiError::internal(format!("Email lookup failed: {}", e)))?;

    // DEBUG-VOPRF: remove after debugging — local index check
    tracing::info!(
        found_local = found,
        "[VOPRF-DBG] resolve_email: local index check"
    );
    // END DEBUG-VOPRF

    if found {
        return Ok(Json(ResolveEmailResponse { found: true }));
    }

    // Phase 2: Cross-instance discovery fan-out
    {
        // Use the email SHA-256 hash as the VOPRF input — this is the raw
        // identifier that gets blinded before being sent to peers.

        // DEBUG-VOPRF: remove after debugging — fan-out trigger
        tracing::info!(
            input_to_query = %body.email_hash,
            "[VOPRF-DBG] resolve_email: invoking discovery fan-out (raw SHA-256 hex)"
        );
        // END DEBUG-VOPRF

        match state.discovery_client.query(&body.email_hash).await {
            Ok(Some(_address)) => {
                // DEBUG-VOPRF: remove after debugging
                tracing::info!("[VOPRF-DBG] resolve_email: fan-out MATCHED on a peer");
                // END DEBUG-VOPRF
                // A peer has a wallet for this email. We don't reveal the
                // address here — the actual send flow resolves it again.
                return Ok(Json(ResolveEmailResponse { found: true }));
            }
            Ok(None) => {
                // DEBUG-VOPRF: remove after debugging
                tracing::info!("[VOPRF-DBG] resolve_email: fan-out returned no match");
                // END DEBUG-VOPRF
            }
            Err(e) => {
                tracing::warn!(error = %e, "Discovery fan-out failed during resolve");
                // Fall through to return found=false rather than error
            }
        }
    }

    Ok(Json(ResolveEmailResponse { found }))
}
