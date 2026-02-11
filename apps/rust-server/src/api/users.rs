// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! User endpoints.

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::{Auth, AuthenticatedUser, Role};

/// Response for GET /v1/users/me
#[derive(Debug, Serialize, ToSchema)]
pub struct UserMeResponse {
    /// User's unique ID (from Clerk)
    pub user_id: String,
    /// User's role
    pub role: Role,
    /// Session ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl From<AuthenticatedUser> for UserMeResponse {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            user_id: user.user_id,
            role: user.role,
            session_id: user.session_id,
        }
    }
}

/// Get the current authenticated user's information.
///
/// This endpoint returns the identity and roles of the currently authenticated user.
#[utoipa::path(
    get,
    path = "/v1/users/me",
    tag = "Users",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "User information", body = UserMeResponse),
        (status = 401, description = "Unauthorized - invalid or missing token"),
    )
)]
pub async fn get_current_user(Auth(user): Auth) -> Json<UserMeResponse> {
    Json(user.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_me_response_from_authenticated_user() {
        let user = AuthenticatedUser {
            user_id: "user_123".to_string(),
            role: Role::Client,
            session_id: Some("sess_abc".to_string()),
            issuer: "test".to_string(),
            expires_at: 0,
        };

        let response: UserMeResponse = user.into();
        assert_eq!(response.user_id, "user_123");
        assert_eq!(response.role, Role::Client);
        assert_eq!(response.session_id, Some("sess_abc".to_string()));
    }
}
