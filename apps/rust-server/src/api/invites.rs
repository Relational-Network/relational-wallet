// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    error::ApiError,
    models::{Invite, RedeemInviteRequest},
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct InviteQuery {
    pub invite_code: String,
}

#[utoipa::path(
    get,
    path = "/v1/invite",
    params(InviteQuery),
    tag = "Invites",
    responses((status = 200, body = Invite))
)]
pub async fn get_invite(
    State(state): State<AppState>,
    Query(params): Query<InviteQuery>,
) -> Result<Json<Invite>, ApiError> {
    let store = state.store.read().await;
    let invite = store.invite_by_code(&params.invite_code)?;
    Ok(Json(invite))
}

#[utoipa::path(
    post,
    path = "/v1/invite/redeem",
    request_body = RedeemInviteRequest,
    tag = "Invites",
    responses((status = 200))
)]
pub async fn redeem_invite(
    State(state): State<AppState>,
    Json(request): Json<RedeemInviteRequest>,
) -> Result<(), ApiError> {
    let mut store = state.store.write().await;
    store.redeem_invite(request)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::{Query, State},
        http::StatusCode,
        Json,
    };

    #[tokio::test]
    async fn get_invite_success() {
        let state = AppState::default();
        let invite = {
            let mut store = state.store.write().await;
            store.insert_invite("AAABBB", false)
        };

        let Json(returned) = get_invite(
            State(state.clone()),
            Query(InviteQuery {
                invite_code: invite.code.clone(),
            }),
        )
        .await
        .expect("invite fetch succeeds");

        assert_eq!(returned, invite);
    }

    #[tokio::test]
    async fn redeem_invite_success() {
        let state = AppState::default();
        let invite = {
            let mut store = state.store.write().await;
            store.insert_invite("AAABBB", false)
        };

        redeem_invite(
            State(state.clone()),
            Json(RedeemInviteRequest {
                invite_id: invite.id.clone(),
            }),
        )
        .await
        .expect("invite redeem succeeds");

        // A redeemed invite should now be rejected by invite_by_code.
        let result = {
            let store = state.store.read().await;
            store.invite_by_code(&invite.code)
        };

        match result {
            Err(err) => assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY),
            Ok(invite) => panic!("expected invite to be marked redeemed, got {invite:?}"),
        }
    }
}
