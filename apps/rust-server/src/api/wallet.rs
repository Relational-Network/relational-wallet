// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{extract::State, http::StatusCode, Json};

use crate::{error::ApiError, models::AutofundRequest, state::AppState};

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, Json};

    #[tokio::test]
    async fn autofund_wallet_records_request() {
        let state = AppState::default();
        let request = AutofundRequest {
            wallet_id: "wallet_a".into(),
        };

        let status = autofund_wallet(State(state.clone()), Json(request.clone()))
            .await
            .expect("autofund succeeds");

        assert_eq!(status, StatusCode::OK);
        let log = &state.store.read().await.autofund_log;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].wallet_id, request.wallet_id);
    }
}
