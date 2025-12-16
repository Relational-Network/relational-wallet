// SPDX-License-Identifier: AGPL-3.0-or-later 
// 
// Copyright (C) 2025 Relational Network 
// 
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet) 

use axum::{extract::State, http::StatusCode, Json};

use crate::{
    error::ApiError,
    models::AutofundRequest,
    state::AppState,
};

#[utoipa::path(
    post,
    path = "/v1/wallet/autofund",
    request_body = AutofundRequest,
    tag = "Wallet",
    responses((status = 200))
)]
pub async fn autofund_wallet(
    State(state): State<AppState>,
    Json(request): Json<AutofundRequest>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.write().await;
    store.log_autofund(request);
    Ok(StatusCode::OK)
}
