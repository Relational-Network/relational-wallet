// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        Json,
    };

    #[tokio::test]
    async fn create_bookmark_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("test_wallet_id");
        let request = CreateBookmarkRequest {
            wallet_id: wallet_id.clone(),
            name: "test_name".into(),
            address: WalletAddress::from("test_address"),
        };

        let (status, Json(bookmark)) = create_bookmark(State(state.clone()), Json(request.clone()))
            .await
            .expect("bookmark creation succeeds");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(bookmark.wallet_id, wallet_id);
        assert_eq!(bookmark.name, request.name);
        assert_eq!(bookmark.address, request.address);
        assert!(!bookmark.id.is_empty());

        let stored = state.store.read().await.list_bookmarks(&wallet_id);
        assert_eq!(stored, vec![bookmark]);
    }

    #[tokio::test]
    async fn delete_bookmark_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("test_wallet_id");
        let bookmark = {
            let mut store = state.store.write().await;
            store.create_bookmark(CreateBookmarkRequest {
                wallet_id: wallet_id.clone(),
                name: "test_name1".into(),
                address: WalletAddress::from("test_address1"),
            })
        };

        let status = delete_bookmark(Path(bookmark.id.clone()), State(state.clone()))
            .await
            .expect("bookmark deletion succeeds");

        assert_eq!(status, StatusCode::NO_CONTENT);

        let stored = state.store.read().await.list_bookmarks(&wallet_id);
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn get_bookmarks_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("test_wallet_id");
        let other_wallet_id = WalletAddress::from("other_wallet_id");

        let mut expected = {
            let mut store = state.store.write().await;
            let first = store.create_bookmark(CreateBookmarkRequest {
                wallet_id: wallet_id.clone(),
                name: "test_name1".into(),
                address: WalletAddress::from("test_address1"),
            });
            let second = store.create_bookmark(CreateBookmarkRequest {
                wallet_id: wallet_id.clone(),
                name: "test_name2".into(),
                address: WalletAddress::from("test_address2"),
            });
            store.create_bookmark(CreateBookmarkRequest {
                wallet_id: other_wallet_id,
                name: "should_be_filtered_out".into(),
                address: WalletAddress::from("test_address3"),
            });
            vec![first, second]
        };

        let Json(mut bookmarks) = list_bookmarks(
            State(state.clone()),
            Query(WalletQuery {
                wallet_id: wallet_id.clone(),
            }),
        )
        .await
        .expect("bookmark listing succeeds");

        // Order from the HashMap is nondeterministic, so compare sorted lists.
        expected.sort_by(|a, b| a.id.cmp(&b.id));
        bookmarks.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(bookmarks, expected);
    }
}
