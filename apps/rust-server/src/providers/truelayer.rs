// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! TrueLayer sandbox integration for fiat on-ramp/off-ramp.

use std::{collections::HashMap, fs, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use truelayer_signing::{sign_with_pem, Method};
use uuid::Uuid;

const DEFAULT_API_BASE_URL: &str = "https://api.truelayer-sandbox.com";
const DEFAULT_AUTH_BASE_URL: &str = "https://auth.truelayer-sandbox.com";
const DEFAULT_HOSTED_PAYMENTS_BASE_URL: &str = "https://payment.truelayer-sandbox.com";
const DEFAULT_CURRENCY: &str = "EUR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderExecutionStatus {
    Pending,
    Completed,
    Failed,
}

pub struct CreateOnRampRequest<'a> {
    pub request_id: &'a str,
    pub wallet_id: &'a str,
    pub user_id: &'a str,
    pub amount_in_minor: u64,
    pub amount_eur: &'a str,
    pub note: Option<&'a str>,
}

pub struct CreateOffRampRequest<'a> {
    pub request_id: &'a str,
    pub wallet_id: &'a str,
    pub user_id: &'a str,
    pub amount_in_minor: u64,
    pub amount_eur: &'a str,
    pub note: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct ProviderExecutionResult {
    pub provider_reference: String,
    pub provider_action_url: Option<String>,
    pub status: ProviderExecutionStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum TrueLayerError {
    #[error("TrueLayer configuration missing: {0}")]
    MissingConfig(String),

    #[error("TrueLayer signing failed: {0}")]
    Signing(String),

    #[error("TrueLayer auth failed: {0}")]
    Auth(String),

    #[error("TrueLayer request failed: {0}")]
    Request(String),

    #[error("TrueLayer response was invalid: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone)]
pub struct TrueLayerClient {
    api_base_url: String,
    auth_base_url: String,
    hosted_payments_base_url: String,
    client_id: String,
    client_secret: String,
    signing_key_id: String,
    signing_private_key_pem: String,
    merchant_account_id: String,
    currency: String,
    payout_account_holder_name: Option<String>,
    payout_iban: Option<String>,
    http: Client,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

impl TrueLayerClient {
    pub fn is_configured() -> bool {
        required_env_present("TRUELAYER_CLIENT_ID")
            && required_env_present("TRUELAYER_CLIENT_SECRET")
            && required_env_present("TRUELAYER_SIGNING_KEY_ID")
            && (required_env_present("TRUELAYER_SIGNING_PRIVATE_KEY_PEM")
                || required_env_present("TRUELAYER_SIGNING_PRIVATE_KEY_PATH"))
            && required_env_present("TRUELAYER_MERCHANT_ACCOUNT_ID")
    }

    pub fn supports_offramp() -> bool {
        required_env_present("TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME")
            && required_env_present("TRUELAYER_OFFRAMP_IBAN")
    }

    pub fn from_env() -> Result<Self, TrueLayerError> {
        let api_base_url = env_or_default("TRUELAYER_API_BASE_URL", DEFAULT_API_BASE_URL);
        let auth_base_url = env_or_default("TRUELAYER_AUTH_BASE_URL", DEFAULT_AUTH_BASE_URL);
        let hosted_payments_base_url = env_or_default(
            "TRUELAYER_HOSTED_PAYMENTS_BASE_URL",
            DEFAULT_HOSTED_PAYMENTS_BASE_URL,
        );
        let client_id = env_required("TRUELAYER_CLIENT_ID")?;
        let client_secret = env_required("TRUELAYER_CLIENT_SECRET")?;
        let signing_key_id = env_required("TRUELAYER_SIGNING_KEY_ID")?;
        let signing_private_key_pem = load_signing_key_pem()?;
        let merchant_account_id = env_required("TRUELAYER_MERCHANT_ACCOUNT_ID")?;
        let currency = env_or_default("TRUELAYER_CURRENCY", DEFAULT_CURRENCY).to_ascii_uppercase();

        let payout_account_holder_name = env_optional("TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME");
        let payout_iban = env_optional("TRUELAYER_OFFRAMP_IBAN");

        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| TrueLayerError::Request(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            api_base_url,
            auth_base_url,
            hosted_payments_base_url,
            client_id,
            client_secret,
            signing_key_id,
            signing_private_key_pem,
            merchant_account_id,
            currency,
            payout_account_holder_name,
            payout_iban,
            http,
        })
    }

    pub async fn create_onramp(
        &self,
        request: CreateOnRampRequest<'_>,
    ) -> Result<ProviderExecutionResult, TrueLayerError> {
        let provider_user_id = normalize_user_id_as_uuid(request.user_id);

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "wallet_id".to_string(),
            Value::String(request.wallet_id.to_string()),
        );
        metadata.insert(
            "request_id".to_string(),
            Value::String(request.request_id.to_string()),
        );
        metadata.insert(
            "amount_eur".to_string(),
            Value::String(request.amount_eur.to_string()),
        );
        metadata.insert(
            "user_id".to_string(),
            Value::String(request.user_id.to_string()),
        );
        if let Some(note) = request.note {
            metadata.insert("note".to_string(), Value::String(note.to_string()));
        }

        let reference = format!("rw-onramp-{}", request.request_id);
        let payload = json!({
            "amount_in_minor": request.amount_in_minor,
            "currency": self.currency,
            "payment_method": {
                "type": "bank_transfer",
                "provider_selection": {
                    "type": "user_selected"
                },
                "beneficiary": {
                    "type": "merchant_account",
                    "merchant_account_id": self.merchant_account_id,
                    "reference": reference
                }
            },
            "user": {
                "id": provider_user_id
            },
            "metadata": metadata
        });

        let response = self
            .signed_post_json("/v3/payments", "payments", &payload, request.request_id)
            .await?;

        let payment_id = response
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payment id in response".to_string())
            })?
            .to_string();

        let status = response
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payment status in response".to_string())
            })?;

        let resource_token = response
            .get("resource_token")
            .and_then(Value::as_str)
            .map(str::to_string);

        let action_url = response
            .pointer("/authorization_flow/actions/next/url")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                response
                    .pointer("/authorization_flow/actions/next/uri")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .or_else(|| {
                resource_token.as_ref().map(|token| {
                    format!(
                        "{}/payments#payment_id={}&resource_token={}",
                        self.hosted_payments_base_url.trim_end_matches('/'),
                        payment_id,
                        token
                    )
                })
            });

        Ok(ProviderExecutionResult {
            provider_reference: payment_id,
            provider_action_url: action_url,
            status: map_payment_status(status),
        })
    }

    pub async fn create_offramp(
        &self,
        request: CreateOffRampRequest<'_>,
    ) -> Result<ProviderExecutionResult, TrueLayerError> {
        let account_holder_name = self.payout_account_holder_name.as_deref().ok_or_else(|| {
            TrueLayerError::MissingConfig(
                "TRUELAYER_OFFRAMP_ACCOUNT_HOLDER_NAME is required for off-ramp payouts"
                    .to_string(),
            )
        })?;
        let iban = self.payout_iban.as_deref().ok_or_else(|| {
            TrueLayerError::MissingConfig(
                "TRUELAYER_OFFRAMP_IBAN is required for off-ramp payouts".to_string(),
            )
        })?;

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "wallet_id".to_string(),
            Value::String(request.wallet_id.to_string()),
        );
        metadata.insert(
            "request_id".to_string(),
            Value::String(request.request_id.to_string()),
        );
        metadata.insert(
            "amount_eur".to_string(),
            Value::String(request.amount_eur.to_string()),
        );
        metadata.insert(
            "user_id".to_string(),
            Value::String(request.user_id.to_string()),
        );
        if let Some(note) = request.note {
            metadata.insert("note".to_string(), Value::String(note.to_string()));
        }

        let payload = json!({
            "amount_in_minor": request.amount_in_minor,
            "currency": self.currency,
            "beneficiary": {
                "type": "external_account",
                "account_holder_name": account_holder_name,
                "account_identifier": {
                    "type": "iban",
                    "iban": iban
                }
            },
            "merchant_account_id": self.merchant_account_id,
            "metadata": metadata
        });

        let response = self
            .signed_post_json("/v3/payouts", "payouts", &payload, request.request_id)
            .await?;

        let payout_id = response
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payout id in response".to_string())
            })?
            .to_string();

        let status = response
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payout status in response".to_string())
            })?;

        Ok(ProviderExecutionResult {
            provider_reference: payout_id,
            provider_action_url: None,
            status: map_payout_status(status),
        })
    }

    pub async fn fetch_onramp_status(
        &self,
        provider_reference: &str,
    ) -> Result<ProviderExecutionStatus, TrueLayerError> {
        let response = self
            .get_json(&format!("/v3/payments/{provider_reference}"), "payments")
            .await?;
        let status = response
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payment status in response".to_string())
            })?;
        Ok(map_payment_status(status))
    }

    pub async fn fetch_offramp_status(
        &self,
        provider_reference: &str,
    ) -> Result<ProviderExecutionStatus, TrueLayerError> {
        let response = self
            .get_json(&format!("/v3/payouts/{provider_reference}"), "payouts")
            .await?;
        let status = response
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrueLayerError::InvalidResponse("missing payout status in response".to_string())
            })?;
        Ok(map_payout_status(status))
    }

    async fn access_token(&self, scope: &str) -> Result<String, TrueLayerError> {
        let mut form = HashMap::new();
        form.insert("grant_type".to_string(), "client_credentials".to_string());
        form.insert("client_id".to_string(), self.client_id.clone());
        form.insert("client_secret".to_string(), self.client_secret.clone());
        form.insert("scope".to_string(), scope.to_string());

        let response = self
            .http
            .post(format!(
                "{}/connect/token",
                self.auth_base_url.trim_end_matches('/')
            ))
            .form(&form)
            .send()
            .await
            .map_err(|e| TrueLayerError::Auth(format!("token request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TrueLayerError::Auth(format!(
                "token request returned {status}: {body}"
            )));
        }

        let token_response: OAuthTokenResponse = response
            .json()
            .await
            .map_err(|e| TrueLayerError::Auth(format!("invalid token response: {e}")))?;

        if token_response.access_token.trim().is_empty() {
            return Err(TrueLayerError::Auth(
                "token response did not include access_token".to_string(),
            ));
        }

        Ok(token_response.access_token)
    }

    async fn get_json(&self, path: &str, scope: &str) -> Result<Value, TrueLayerError> {
        let token = self.access_token(scope).await?;
        let response = self
            .http
            .get(format!(
                "{}{}",
                self.api_base_url.trim_end_matches('/'),
                path
            ))
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| TrueLayerError::Request(format!("GET {path} failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TrueLayerError::Request(format!(
                "GET {path} returned {status}: {body}"
            )));
        }

        response
            .json()
            .await
            .map_err(|e| TrueLayerError::InvalidResponse(format!("GET {path} invalid JSON: {e}")))
    }

    async fn signed_post_json(
        &self,
        path: &str,
        scope: &str,
        payload: &Value,
        idempotency_key: &str,
    ) -> Result<Value, TrueLayerError> {
        let token = self.access_token(scope).await?;
        let body = serde_json::to_string(payload)
            .map_err(|e| TrueLayerError::InvalidResponse(format!("serialize body failed: {e}")))?;

        let signature = sign_with_pem(
            &self.signing_key_id,
            self.signing_private_key_pem.as_bytes(),
        )
        .method(Method::Post)
        .path(path)
        .header("Idempotency-Key", idempotency_key.as_bytes())
        .body(body.as_bytes())
        .build_signer()
        .sign()
        .map_err(|e| TrueLayerError::Signing(e.to_string()))?;

        let response = self
            .http
            .post(format!(
                "{}{}",
                self.api_base_url.trim_end_matches('/'),
                path
            ))
            .header("Authorization", format!("Bearer {token}"))
            .header("Idempotency-Key", idempotency_key)
            .header("Tl-Signature", signature)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| TrueLayerError::Request(format!("POST {path} failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TrueLayerError::Request(format!(
                "POST {path} returned {status}: {body}"
            )));
        }

        response
            .json()
            .await
            .map_err(|e| TrueLayerError::InvalidResponse(format!("POST {path} invalid JSON: {e}")))
    }
}

pub fn map_payment_status(raw_status: &str) -> ProviderExecutionStatus {
    let status = raw_status.trim().to_ascii_lowercase();
    match status.as_str() {
        "executed" | "settled" => ProviderExecutionStatus::Completed,
        "failed" | "cancelled" | "expired" => ProviderExecutionStatus::Failed,
        _ => ProviderExecutionStatus::Pending,
    }
}

pub fn map_payout_status(raw_status: &str) -> ProviderExecutionStatus {
    let status = raw_status.trim().to_ascii_lowercase();
    match status.as_str() {
        "executed" | "settled" | "successful" => ProviderExecutionStatus::Completed,
        "failed" | "cancelled" | "rejected" => ProviderExecutionStatus::Failed,
        _ => ProviderExecutionStatus::Pending,
    }
}

fn required_env_present(name: &str) -> bool {
    env_optional(name).is_some()
}

fn env_required(name: &str) -> Result<String, TrueLayerError> {
    env_optional(name).ok_or_else(|| TrueLayerError::MissingConfig(name.to_string()))
}

fn env_optional(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(_) => None,
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn load_signing_key_pem() -> Result<String, TrueLayerError> {
    if let Some(pem) = std::env::var("TRUELAYER_SIGNING_PRIVATE_KEY_PEM")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        return Ok(pem.replace("\\n", "\n"));
    }

    let path = env_required("TRUELAYER_SIGNING_PRIVATE_KEY_PATH")?;
    let pem = fs::read_to_string(&path)
        .map_err(|e| TrueLayerError::MissingConfig(format!("failed to read {path}: {e}")))?;
    let trimmed = pem.trim().to_string();
    if trimmed.is_empty() {
        return Err(TrueLayerError::MissingConfig(format!(
            "TRUELAYER_SIGNING_PRIVATE_KEY_PATH points to an empty file: {path}"
        )));
    }
    Ok(trimmed)
}

fn normalize_user_id_as_uuid(raw_user_id: &str) -> String {
    if let Ok(parsed) = Uuid::parse_str(raw_user_id) {
        return parsed.to_string();
    }
    Uuid::new_v5(&Uuid::NAMESPACE_URL, raw_user_id.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_status_mapping_is_stable() {
        assert_eq!(
            map_payment_status("settled"),
            ProviderExecutionStatus::Completed
        );
        assert_eq!(
            map_payment_status("FAILED"),
            ProviderExecutionStatus::Failed
        );
        assert_eq!(
            map_payment_status("authorization_required"),
            ProviderExecutionStatus::Pending
        );
    }

    #[test]
    fn payout_status_mapping_is_stable() {
        assert_eq!(
            map_payout_status("executed"),
            ProviderExecutionStatus::Completed
        );
        assert_eq!(
            map_payout_status("rejected"),
            ProviderExecutionStatus::Failed
        );
        assert_eq!(
            map_payout_status("pending"),
            ProviderExecutionStatus::Pending
        );
    }

    #[test]
    fn normalize_user_id_passes_through_valid_uuid() {
        let user_id = "3f4d6542-b8ce-4226-93d3-80d6f14d6db2";
        assert_eq!(normalize_user_id_as_uuid(user_id), user_id);
    }

    #[test]
    fn normalize_user_id_generates_stable_uuid_for_non_uuid_values() {
        let first = normalize_user_id_as_uuid("user_2zR6yG2iJ0S2gr3");
        let second = normalize_user_id_as_uuid("user_2zR6yG2iJ0S2gr3");
        assert_eq!(first, second);
        assert!(Uuid::parse_str(&first).is_ok());
    }
}
