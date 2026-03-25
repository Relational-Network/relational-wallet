// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    audit_log,
    auth::Auth,
    error::ApiError,
    models::{Bookmark, CreateBookmarkRequest, WalletAddress},
    providers::email,
    state::AppState,
    storage::{
        AuditEventType, BookmarkRepository, OwnershipEnforcer, RecipientType, StoredBookmark,
        WalletRepository,
    },
};

#[derive(Deserialize, IntoParams)]
pub struct WalletQuery {
    pub wallet_id: WalletAddress,
}

/// List bookmarks for a wallet.
///
/// Returns all bookmarks for a wallet owned by the authenticated user.
#[utoipa::path(
    get,
    path = "/v1/bookmarks",
    params(WalletQuery),
    tag = "Bookmarks",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = [Bookmark]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your wallet"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn list_bookmarks(
    Auth(user): Auth,
    State(state): State<AppState>,
    Query(params): Query<WalletQuery>,
) -> Result<Json<Vec<Bookmark>>, ApiError> {
    let storage = state.storage();
    let wallet_id = params.wallet_id.to_string();

    // Verify wallet ownership
    let wallet_repo = WalletRepository::new(&storage);
    let wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {} not found", wallet_id)))?;

    wallet.verify_ownership(&user).map_err(|_| {
        ApiError::forbidden("You don't have permission to access this wallet's bookmarks")
    })?;

    // List bookmarks from encrypted storage
    let repo = BookmarkRepository::new(&storage);
    let bookmarks = repo
        .list_by_wallet(&wallet_id, &user.user_id)
        .map_err(|e| ApiError::internal(&format!("Failed to list bookmarks: {}", e)))?;

    // Convert to API response format
    let response: Vec<Bookmark> = bookmarks
        .into_iter()
        .map(|b| {
            let recipient_type = match b.recipient_type {
                RecipientType::Email => "email".to_string(),
                RecipientType::Address => "address".to_string(),
            };
            let address = if b.recipient_type == RecipientType::Address && !b.address.is_empty() {
                Some(WalletAddress::from(b.address))
            } else {
                None
            };
            Bookmark {
                id: b.id,
                wallet_id: WalletAddress::from(b.wallet_id),
                name: b.name,
                recipient_type,
                address,
                email_hash: b.email_hash,
                email_display: b.email_display,
            }
        })
        .collect();

    Ok(Json(response))
}

/// Create a new bookmark.
///
/// Creates a bookmark in a wallet owned by the authenticated user.
#[utoipa::path(
    post,
    path = "/v1/bookmarks",
    request_body = CreateBookmarkRequest,
    tag = "Bookmarks",
    security(("bearer_auth" = [])),
    responses(
        (status = 201, body = Bookmark),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your wallet")
    )
)]
pub async fn create_bookmark(
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<CreateBookmarkRequest>,
) -> Result<(StatusCode, Json<Bookmark>), ApiError> {
    let storage = state.storage();
    let wallet_id = request.wallet_id.to_string();

    // Validate based on recipient_type
    let (recipient_type, address_str, email_hash, email_display) = match request
        .recipient_type
        .as_str()
    {
        "email" => {
            // Validate email_hash and email_display are provided
            let hash = request
                .email_hash
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("email_hash is required for email bookmarks"))?
                .to_string();
            if !email::validate_email_hash(&hash) {
                return Err(ApiError::bad_request(
                    "email_hash must be 64 lowercase hex characters",
                ));
            }
            let display = request
                .email_display
                .as_deref()
                .ok_or_else(|| {
                    ApiError::bad_request("email_display is required for email bookmarks")
                })?
                .to_string();
            (
                RecipientType::Email,
                String::new(),
                Some(hash),
                Some(display),
            )
        }
        "address" | _ => {
            // Validate the bookmarked address is a valid Ethereum address
            let addr = request.address.as_ref().ok_or_else(|| {
                ApiError::bad_request("address is required for address bookmarks")
            })?;
            addr.validate_eth_address()
                .map_err(|e| ApiError::bad_request(&e))?;
            (RecipientType::Address, addr.to_string(), None, None)
        }
    };

    // Verify wallet ownership
    let wallet_repo = WalletRepository::new(&storage);
    let wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {} not found", wallet_id)))?;

    wallet.verify_ownership(&user).map_err(|_| {
        ApiError::forbidden("You don't have permission to add bookmarks to this wallet")
    })?;

    // Create bookmark
    let bookmark_id = uuid::Uuid::new_v4().to_string();
    let stored = StoredBookmark {
        id: bookmark_id.clone(),
        wallet_id: wallet_id.clone(),
        owner_user_id: user.user_id.clone(),
        name: request.name.clone(),
        recipient_type: recipient_type.clone(),
        address: address_str.clone(),
        email_hash: email_hash.clone(),
        email_display: email_display.clone(),
        created_at: Utc::now(),
    };

    let repo = BookmarkRepository::new(&storage);
    repo.create(&stored)
        .map_err(|e| ApiError::internal(&format!("Failed to create bookmark: {}", e)))?;

    // Audit log
    audit_log!(
        &storage,
        AuditEventType::BookmarkCreated,
        &user,
        "bookmark",
        &bookmark_id
    );

    let response = Bookmark {
        id: stored.id,
        wallet_id: request.wallet_id,
        name: stored.name,
        recipient_type: match recipient_type {
            RecipientType::Email => "email".to_string(),
            RecipientType::Address => "address".to_string(),
        },
        address: if address_str.is_empty() {
            None
        } else {
            Some(WalletAddress::from(address_str))
        },
        email_hash,
        email_display,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete a bookmark.
///
/// Deletes a bookmark owned by the authenticated user.
#[utoipa::path(
    delete,
    path = "/v1/bookmarks/{bookmark_id}",
    params(
        ("bookmark_id" = String, Path, description = "Identifier of the bookmark to delete")
    ),
    tag = "Bookmarks",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Bookmark deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your bookmark"),
        (status = 404, description = "Bookmark not found")
    )
)]
pub async fn delete_bookmark(
    Auth(user): Auth,
    Path(bookmark_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let storage = state.storage();
    let repo = BookmarkRepository::new(&storage);

    // Get bookmark and verify ownership
    let bookmark = repo
        .get(&bookmark_id)
        .map_err(|_| ApiError::not_found(&format!("Bookmark {} not found", bookmark_id)))?;

    bookmark
        .verify_ownership(&user)
        .map_err(|_| ApiError::forbidden("You don't have permission to delete this bookmark"))?;

    // Delete bookmark
    repo.delete(&bookmark_id)
        .map_err(|e| ApiError::internal(&format!("Failed to delete bookmark: {}", e)))?;

    // Audit log
    audit_log!(
        &storage,
        AuditEventType::BookmarkDeleted,
        &user,
        "bookmark",
        &bookmark_id
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthenticatedUser, Role};
    use crate::storage::{EncryptedStorage, StoragePaths, WalletMetadata, WalletStatus};
    use axum::http::StatusCode;
    use tempfile::TempDir;

    fn setup() -> (TempDir, AppState, AuthenticatedUser) {
        let temp = TempDir::new().unwrap();
        let paths = StoragePaths::new(temp.path().to_str().unwrap());
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().unwrap();

        let state = AppState::new(storage);
        let user = AuthenticatedUser {
            user_id: "test_user".to_string(),
            role: Role::Client,
            session_id: None,
            issuer: "test".to_string(),
            expires_at: 0,
        };

        (temp, state, user)
    }

    fn create_test_wallet(storage: &EncryptedStorage, user_id: &str) -> String {
        let wallet_id = uuid::Uuid::new_v4().to_string();
        let metadata = WalletMetadata {
            wallet_id: wallet_id.clone(),
            owner_user_id: user_id.to_string(),
            public_address: "0xtest".to_string(),
            created_at: Utc::now(),
            status: WalletStatus::Active,
            label: None,
            email_lookup_key: None,
        };
        let repo = WalletRepository::new(storage);
        repo.create(&metadata, b"test_key").unwrap();
        wallet_id
    }

    #[tokio::test]
    async fn create_bookmark_success() {
        let (_temp, state, user) = setup();
        let storage = state.storage();
        let wallet_id = create_test_wallet(&storage, &user.user_id);

        let request = CreateBookmarkRequest {
            wallet_id: WalletAddress::from(wallet_id.as_str()),
            name: "test_name".into(),
            recipient_type: "address".to_string(),
            address: Some(WalletAddress::from(
                "0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12",
            )),
            email_hash: None,
            email_display: None,
        };

        let (status, Json(bookmark)) =
            create_bookmark(Auth(user), State(state), Json(request.clone()))
                .await
                .expect("bookmark creation succeeds");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(bookmark.name, "test_name");
        assert!(!bookmark.id.is_empty());
    }

    #[tokio::test]
    async fn delete_bookmark_success() {
        let (_temp, state, user) = setup();
        let storage = state.storage();
        let wallet_id = create_test_wallet(&storage, &user.user_id);

        // Create a bookmark first
        let repo = BookmarkRepository::new(&storage);
        let bookmark = StoredBookmark {
            id: "bookmark_1".to_string(),
            wallet_id: wallet_id.clone(),
            owner_user_id: user.user_id.clone(),
            name: "Test".to_string(),
            recipient_type: RecipientType::Address,
            address: "0xaddr".to_string(),
            email_hash: None,
            email_display: None,
            created_at: Utc::now(),
        };
        repo.create(&bookmark).unwrap();

        let status = delete_bookmark(Auth(user), Path("bookmark_1".to_string()), State(state))
            .await
            .expect("delete succeeds");

        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn get_bookmarks_success() {
        let (_temp, state, user) = setup();
        let storage = state.storage();
        let wallet_id = create_test_wallet(&storage, &user.user_id);

        // Create a bookmark
        let repo = BookmarkRepository::new(&storage);
        let bookmark = StoredBookmark {
            id: "bookmark_2".to_string(),
            wallet_id: wallet_id.clone(),
            owner_user_id: user.user_id.clone(),
            name: "Test2".to_string(),
            recipient_type: RecipientType::Address,
            address: "0xaddr2".to_string(),
            email_hash: None,
            email_display: None,
            created_at: Utc::now(),
        };
        repo.create(&bookmark).unwrap();

        let query = WalletQuery {
            wallet_id: WalletAddress::from(wallet_id.as_str()),
        };

        let Json(bookmarks) = list_bookmarks(Auth(user), State(state), Query(query))
            .await
            .expect("list succeeds");

        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].name, "Test2");
    }
}
