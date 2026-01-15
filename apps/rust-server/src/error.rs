// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }
}

impl IntoResponse for ApiError {
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
