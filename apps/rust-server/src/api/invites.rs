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
    auth::Auth,
    error::ApiError,
    models::{Invite, RedeemInviteRequest},
    state::AppState,
    storage::repository::InviteRepository,
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
    let repo = InviteRepository::new(&state.storage);
    let stored = repo
        .get_by_code(&params.invite_code)
        .map_err(|_| ApiError::not_found("Invite not found"))?;

    if stored.redeemed {
        return Err(ApiError::unprocessable(
            "This invite code has already been redeemed.",
        ));
    }

    Ok(Json(Invite {
        id: stored.id,
        code: stored.code,
        redeemed: stored.redeemed,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/invite/redeem",
    request_body = RedeemInviteRequest,
    tag = "Invites",
    responses((status = 200))
)]
pub async fn redeem_invite(
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<RedeemInviteRequest>,
) -> Result<(), ApiError> {
    let repo = InviteRepository::new(&state.storage);
    repo.redeem(&request.invite_id, &user.user_id)
        .map_err(|e| match e {
            crate::storage::StorageError::NotFound(_) => ApiError::not_found("Invite not found"),
            crate::storage::StorageError::AlreadyExists(_) => {
                ApiError::unprocessable("This invite code has already been redeemed.")
            }
            _ => ApiError::internal(format!("Failed to redeem invite: {e}")),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthenticatedUser;
    use crate::storage::repository::invites::StoredInvite;
    use axum::{
        extract::{Query, State},
        http::StatusCode,
    };
    use chrono::Utc;

    fn create_test_invite(repo: &InviteRepository, code: &str) -> StoredInvite {
        let invite = StoredInvite {
            id: uuid::Uuid::new_v4().to_string(),
            code: code.to_string(),
            redeemed: false,
            created_by_user_id: Some("admin-1".to_string()),
            redeemed_by_user_id: None,
            created_at: Utc::now(),
            redeemed_at: None,
            expires_at: None,
        };
        repo.create(&invite).expect("create invite");
        invite
    }

    fn mock_auth() -> Auth {
        Auth(AuthenticatedUser {
            user_id: "test-user-123".to_string(),
            role: crate::auth::Role::Client,
            session_id: None,
            issuer: "https://test.clerk.dev".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        })
    }

    #[tokio::test]
    async fn get_invite_success() {
        let state = AppState::default();
        let repo = InviteRepository::new(&state.storage);
        let invite = create_test_invite(&repo, "AAABBB");

        let Json(returned) = get_invite(
            State(state.clone()),
            Query(InviteQuery {
                invite_code: invite.code.clone(),
            }),
        )
        .await
        .expect("invite fetch succeeds");

        assert_eq!(returned.id, invite.id);
        assert_eq!(returned.code, invite.code);
        assert!(!returned.redeemed);
    }

    #[tokio::test]
    async fn get_invite_redeemed_fails() {
        let state = AppState::default();
        let repo = InviteRepository::new(&state.storage);
        let mut invite = create_test_invite(&repo, "REDEEMED1");

        // Mark as redeemed
        invite.redeemed = true;
        repo.update(&invite).expect("update invite");

        let result = get_invite(
            State(state.clone()),
            Query(InviteQuery {
                invite_code: invite.code.clone(),
            }),
        )
        .await;

        match result {
            Err(err) => assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY),
            Ok(_) => panic!("expected error for redeemed invite"),
        }
    }

    #[tokio::test]
    async fn redeem_invite_success() {
        let state = AppState::default();
        let repo = InviteRepository::new(&state.storage);
        let invite = create_test_invite(&repo, "REDEEM_ME");

        redeem_invite(
            mock_auth(),
            State(state.clone()),
            Json(RedeemInviteRequest {
                invite_id: invite.id.clone(),
            }),
        )
        .await
        .expect("invite redeem succeeds");

        // Verify invite is now redeemed
        let result = get_invite(
            State(state.clone()),
            Query(InviteQuery {
                invite_code: invite.code.clone(),
            }),
        )
        .await;

        match result {
            Err(err) => assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY),
            Ok(_) => panic!("expected invite to be marked redeemed"),
        }
    }
}
