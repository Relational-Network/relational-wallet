// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::path::Path;
use utoipa::ToSchema;

use crate::config::DATA_DIR_ENV;
use crate::state::AppState;

/// Health check response with individual component status.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    /// Overall health status ("ok" or "degraded").
    pub status: String,
    /// Individual health checks and their results.
    pub checks: HealthChecks,
}

/// Individual health check results.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    /// Whether the service process is running.
    pub service: String,
    /// Data directory availability (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    /// JWKS (authentication keys) status.
    /// Only present in production mode (CLERK_JWKS_URL configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks: Option<String>,
}

/// Simple health check response for liveness probes.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

/// Check if the data directory exists and is accessible.
fn check_data_dir() -> Option<String> {
    if let Ok(dir) = std::env::var(DATA_DIR_ENV) {
        if Path::new(&dir).exists() {
            Some("ok".to_string())
        } else {
            Some("missing".to_string())
        }
    } else {
        None
    }
}

/// Check if JWKS is available (production auth mode).
async fn check_jwks(state: &AppState) -> Option<String> {
    if let Some(ref jwks_manager) = state.auth_config.jwks {
        // Check if we have cached keys
        if jwks_manager.is_cached().await {
            Some("ok".to_string())
        } else {
            // Try to fetch keys
            match jwks_manager.refresh().await {
                Ok(_) => Some("ok".to_string()),
                Err(_) => Some("unavailable".to_string()),
            }
        }
    } else {
        // Development mode - no JWKS configured
        None
    }
}

/// Health check endpoint handler.
///
/// Returns 200 if all checks pass, 503 if any check fails.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = ReadyResponse),
        (status = 503, description = "Service is unhealthy", body = ReadyResponse)
    )
)]
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<ReadyResponse>) {
    let data_dir = check_data_dir();
    let jwks = check_jwks(&state).await;
    
    let data_ok = data_dir.as_ref().map(|s| s == "ok").unwrap_or(true);
    let jwks_ok = jwks.as_ref().map(|s| s == "ok").unwrap_or(true);
    let all_ok = data_ok && jwks_ok;

    let response = ReadyResponse {
        status: if all_ok { "ok" } else { "degraded" }.to_string(),
        checks: HealthChecks {
            service: "ok".to_string(),
            data_dir,
            jwks,
        },
    };

    let status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(response))
}

/// Liveness probe handler.
///
/// Always returns 200 if the process is running.
/// Does not check dependencies - use readiness for that.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "Health",
    responses(
        (status = 200, description = "Service is alive", body = HealthResponse)
    )
)]
pub async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Readiness probe handler.
///
/// Returns 200 only if all dependencies are available.
/// Use for Kubernetes readiness probes.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse),
        (status = 503, description = "Service is not ready", body = ReadyResponse)
    )
)]
pub async fn readiness(state: State<AppState>) -> (StatusCode, Json<ReadyResponse>) {
    health(state).await
}
