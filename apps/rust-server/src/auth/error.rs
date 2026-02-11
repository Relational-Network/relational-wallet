// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Authentication errors.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Authentication error type.
///
/// These errors are used during JWT verification in production mode.
/// Some variants may not be returned directly but are kept for
/// completeness and future error mapping improvements.
#[derive(Debug)]
pub enum AuthError {
    /// No authorization header present
    MissingAuthHeader,
    /// Invalid authorization header format
    InvalidAuthHeader,
    /// Token is malformed
    MalformedToken,
    /// Token signature is invalid
    InvalidSignature,
    /// Token has expired
    TokenExpired,
    /// Token issuer is invalid
    InvalidIssuer,
    /// Token audience is invalid
    InvalidAudience,
    /// Token is not yet valid
    TokenNotYetValid,
    /// JWKS fetch failed
    JwksFetchError(String),
    /// No matching key in JWKS
    NoMatchingKey,
    /// Internal error
    InternalError(String),
    /// Insufficient permissions
    InsufficientPermissions,
}

#[derive(Serialize)]
struct AuthErrorBody {
    error: String,
    error_code: String,
}

impl AuthError {
    /// Get the error code for this error.
    pub fn error_code(&self) -> &'static str {
        match self {
            AuthError::MissingAuthHeader => "missing_auth_header",
            AuthError::InvalidAuthHeader => "invalid_auth_header",
            AuthError::MalformedToken => "malformed_token",
            AuthError::InvalidSignature => "invalid_signature",
            AuthError::TokenExpired => "token_expired",
            AuthError::InvalidIssuer => "invalid_issuer",
            AuthError::InvalidAudience => "invalid_audience",
            AuthError::TokenNotYetValid => "token_not_yet_valid",
            AuthError::JwksFetchError(_) => "jwks_fetch_error",
            AuthError::NoMatchingKey => "no_matching_key",
            AuthError::InternalError(_) => "internal_error",
            AuthError::InsufficientPermissions => "insufficient_permissions",
        }
    }

    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            AuthError::MissingAuthHeader
            | AuthError::InvalidAuthHeader
            | AuthError::MalformedToken
            | AuthError::InvalidSignature
            | AuthError::TokenExpired
            | AuthError::InvalidIssuer
            | AuthError::InvalidAudience
            | AuthError::TokenNotYetValid
            | AuthError::NoMatchingKey => StatusCode::UNAUTHORIZED,
            AuthError::InsufficientPermissions => StatusCode::FORBIDDEN,
            AuthError::JwksFetchError(_) | AuthError::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingAuthHeader => write!(f, "Authorization header is required"),
            AuthError::InvalidAuthHeader => {
                write!(f, "Invalid authorization header format (expected 'Bearer <token>')")
            }
            AuthError::MalformedToken => write!(f, "Token is malformed"),
            AuthError::InvalidSignature => write!(f, "Token signature is invalid"),
            AuthError::TokenExpired => write!(f, "Token has expired"),
            AuthError::InvalidIssuer => write!(f, "Token issuer is invalid"),
            AuthError::InvalidAudience => write!(f, "Token audience is invalid"),
            AuthError::TokenNotYetValid => write!(f, "Token is not yet valid"),
            AuthError::JwksFetchError(msg) => write!(f, "Failed to fetch JWKS: {msg}"),
            AuthError::NoMatchingKey => write!(f, "No matching key found in JWKS"),
            AuthError::InternalError(msg) => write!(f, "Internal authentication error: {msg}"),
            AuthError::InsufficientPermissions => {
                write!(f, "Insufficient permissions for this operation")
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(AuthErrorBody {
            error: self.to_string(),
            error_code: self.error_code().to_string(),
        });
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn missing_auth_returns_401() {
        let response = AuthError::MissingAuthHeader.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error_code"], "missing_auth_header");
    }

    #[tokio::test]
    async fn insufficient_permissions_returns_403() {
        let response = AuthError::InsufficientPermissions.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
