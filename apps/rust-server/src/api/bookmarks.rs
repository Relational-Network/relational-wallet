// SPDX-License-Identifier: AGPL-3.0-or-later 
// 
// Copyright (C) 2025 Relational Network 
// 
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet) 

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    error::ApiError,
    models::{Bookmark, CreateBookmarkRequest, WalletAddress},
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct WalletQuery {
    pub wallet_id: WalletAddress,
}

#[utoipa::path(
    get,
    path = "/v1/bookmarks",
    params(WalletQuery),
    tag = "Bookmarks",
    responses((status = 200, body = [Bookmark]))
)]
pub async fn list_bookmarks(
    State(state): State<AppState>,
    Query(params): Query<WalletQuery>,
) -> Result<Json<Vec<Bookmark>>, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.list_bookmarks(&params.wallet_id)))
}

#[utoipa::path(
    post,
    path = "/v1/bookmarks",
    request_body = CreateBookmarkRequest,
    tag = "Bookmarks",
    responses((status = 201, body = Bookmark))
)]
pub async fn create_bookmark(
    State(state): State<AppState>,
    Json(request): Json<CreateBookmarkRequest>,
) -> Result<(StatusCode, Json<Bookmark>), ApiError> {
    let mut store = state.store.write().await;
    let bookmark = store.create_bookmark(request);
    Ok((StatusCode::CREATED, Json(bookmark)))
}

#[utoipa::path(
    delete,
    path = "/v1/bookmarks/{bookmark_id}",
    params(
        ("bookmark_id" = String, Path, description = "Identifier of the bookmark to delete")
    ),
    tag = "Bookmarks",
    responses((status = 204))
)]
pub async fn delete_bookmark(
    Path(bookmark_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.write().await;
    store.delete_bookmark(&bookmark_id)?;
    Ok(StatusCode::NO_CONTENT)
}
