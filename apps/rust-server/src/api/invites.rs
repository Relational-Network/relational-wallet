// SPDX-License-Identifier: AGPL-3.0-or-later 
// 
// Copyright (C) 2025 Relational Network 
// 
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet) 

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
