// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # API Error Handling
//!
//! This module provides a unified error type for all API responses.
//! Errors are automatically converted to JSON responses with appropriate
//! HTTP status codes.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::error::ApiError;
//!
//! // Return a 404 error
//! return Err(ApiError::not_found("Wallet not found"));
//!
//! // Return a 403 error
//! return Err(ApiError::forbidden("Not your wallet"));
//! ```
//!
//! ## JSON Response Format
//!
//! All errors are returned as JSON with a single `error` field:
//!
//! ```json
//! { "error": "Wallet not found" }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error with HTTP status and message.
///
/// This type implements `IntoResponse`, allowing it to be returned directly
/// from Axum handlers. The error is serialized as JSON.
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status code for the response.
    pub status: StatusCode,
    /// Human-readable error message (included in JSON response).
    pub message: String,
}

/// JSON body structure for error responses.
#[derive(Serialize)]
struct ErrorBody {
    /// The error message.
    error: String,
}

impl ApiError {
    /// Create a new API error with the given status and message.
    ///
    /// # Arguments
    /// * `status` - HTTP status code
    /// * `message` - Human-readable error description
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    /// Create a 404 Not Found error.
    ///
    /// Use when a requested resource does not exist.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// Create a 400 Bad Request error.
    ///
    /// Use when the request is malformed or missing required fields.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// Create a 422 Unprocessable Entity error.
    ///
    /// Use when the request is syntactically valid but semantically incorrect.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    /// Create a 500 Internal Server Error.
    ///
    /// Use for unexpected server-side failures. Avoid exposing internal details.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// Create a 403 Forbidden error.
    ///
    /// Use when the user is authenticated but lacks permission.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    /// Create a 503 Service Unavailable error.
    ///
    /// Use when a required service (e.g., blockchain RPC) is unavailable.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }
}

impl IntoResponse for ApiError {
    /// Convert the error into an Axum HTTP response.
    ///
    /// Returns a JSON body with the error message and the appropriate
    /// HTTP status code.
    fn into_response(self) -> Response {
        let body = Json(ErrorBody {
            error: self.message,
        });
        (self.status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn constructors_set_status_and_message() {
        let nf = ApiError::not_found("missing");
        assert_eq!(nf.status, StatusCode::NOT_FOUND);
        assert_eq!(nf.message, "missing");

        let bad = ApiError::bad_request("bad");
        assert_eq!(bad.status, StatusCode::BAD_REQUEST);
        assert_eq!(bad.message, "bad");

        let unp = ApiError::unprocessable("oops");
        assert_eq!(unp.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(unp.message, "oops");
    }

    #[tokio::test]
    async fn into_response_returns_json_body() {
        let response = ApiError::bad_request("bad data").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body, r#"{"error":"bad data"}"#);
    }
}
