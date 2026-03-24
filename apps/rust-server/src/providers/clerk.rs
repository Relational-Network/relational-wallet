// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Clerk Backend API client for fetching user email addresses.
//!
//! Uses the Clerk Backend API (`GET /v1/users/{user_id}`) with
//! `CLERK_SECRET_KEY` to retrieve the user's primary email address.
//! The email is normalized per the frozen spec in `providers::email`.

use super::email::{normalize_email, EmailError};

fn is_verified_email(email_address: &serde_json::Value) -> bool {
    email_address["verification"]["status"].as_str() == Some("verified")
}

fn extract_single_verified_primary_email(
    body: &serde_json::Value,
    user_id: &str,
) -> Result<String, ClerkError> {
    let primary_id = body["primary_email_address_id"]
        .as_str()
        .ok_or_else(|| ClerkError::NoPrimaryEmail(user_id.to_string()))?;

    let email_addresses = body["email_addresses"]
        .as_array()
        .ok_or_else(|| ClerkError::NoPrimaryEmail(user_id.to_string()))?;

    if email_addresses.len() != 1 {
        return Err(ClerkError::InvalidEmailConfiguration {
            user_id: user_id.to_string(),
            message: format!(
                "expected exactly 1 email address, found {}",
                email_addresses.len()
            ),
        });
    }

    let primary_email = email_addresses
        .iter()
        .find(|ea| ea["id"].as_str() == Some(primary_id))
        .ok_or_else(|| ClerkError::NoPrimaryEmail(user_id.to_string()))?;

    if !is_verified_email(primary_email) {
        return Err(ClerkError::InvalidEmailConfiguration {
            user_id: user_id.to_string(),
            message: "primary email must be verified".to_string(),
        });
    }

    let primary_email = primary_email["email_address"]
        .as_str()
        .ok_or_else(|| ClerkError::NoPrimaryEmail(user_id.to_string()))?;

    Ok(primary_email.to_string())
}

/// Errors from Clerk API operations.
#[derive(Debug, thiserror::Error)]
pub enum ClerkError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Clerk API returned status {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("No primary email found for user {0}")]
    NoPrimaryEmail(String),

    #[error("Invalid Clerk email configuration for user {user_id}: {message}")]
    InvalidEmailConfiguration { user_id: String, message: String },

    #[error("Email normalization failed: {0}")]
    EmailNormalization(#[from] EmailError),
}

/// Client for the Clerk Backend API.
#[derive(Clone)]
pub struct ClerkClient {
    http: reqwest::Client,
    secret_key: String,
}

impl ClerkClient {
    /// Create a new Clerk client with the given secret key.
    pub fn new(secret_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build Clerk HTTP client");

        Self { http, secret_key }
    }

    /// Fetch the primary email for a Clerk user ID, normalized per frozen spec.
    ///
    /// Calls `GET https://api.clerk.com/v1/users/{user_id}` and extracts
    /// the primary email address from the response.
    ///
    /// Returns the normalized email string.
    pub async fn get_user_email(&self, user_id: &str) -> Result<String, ClerkError> {
        let url = format!("https://api.clerk.com/v1/users/{user_id}");

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ClerkError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let body: serde_json::Value = response.json().await?;

        let primary_email = extract_single_verified_primary_email(&body, user_id)?;

        // Normalize per frozen spec
        let normalized = normalize_email(&primary_email)?;
        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_single_verified_primary_email() {
        let body = json!({
            "primary_email_address_id": "em_123",
            "email_addresses": [{
                "id": "em_123",
                "email_address": "alice@example.com",
                "verification": { "status": "verified" }
            }]
        });

        let email = extract_single_verified_primary_email(&body, "user_123").unwrap();
        assert_eq!(email, "alice@example.com");
    }

    #[test]
    fn rejects_multiple_email_addresses() {
        let body = json!({
            "primary_email_address_id": "em_123",
            "email_addresses": [
                {
                    "id": "em_123",
                    "email_address": "alice@example.com",
                    "verification": { "status": "verified" }
                },
                {
                    "id": "em_456",
                    "email_address": "other@example.com",
                    "verification": { "status": "verified" }
                }
            ]
        });

        let err = extract_single_verified_primary_email(&body, "user_123").unwrap_err();
        assert!(matches!(
            err,
            ClerkError::InvalidEmailConfiguration { .. }
        ));
    }

    #[test]
    fn rejects_unverified_primary_email() {
        let body = json!({
            "primary_email_address_id": "em_123",
            "email_addresses": [{
                "id": "em_123",
                "email_address": "alice@example.com",
                "verification": { "status": "unverified" }
            }]
        });

        let err = extract_single_verified_primary_email(&body, "user_123").unwrap_err();
        assert!(matches!(
            err,
            ClerkError::InvalidEmailConfiguration { .. }
        ));
    }
}
