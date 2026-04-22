// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Admin-only API endpoints for system management.
//!
//! These endpoints require the Admin role and provide:
//! - System statistics
//! - User and wallet overview (admin view)
//! - Audit log queries
//! - Operational tooling

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    audit_log,
    auth::AdminOnly,
    error::ApiError,
    state::AppState,
    storage::{
        AuditEvent, AuditEventType, AuditRepository, BookmarkRepository, WalletRepository,
        WalletStatus,
    },
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// System statistics response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SystemStatsResponse {
    /// Total number of wallets across all users.
    pub total_wallets: usize,
    /// Number of active wallets.
    pub active_wallets: usize,
    /// Number of suspended wallets.
    pub suspended_wallets: usize,
    /// Number of deleted wallets.
    pub deleted_wallets: usize,
    /// Total number of bookmarks.
    pub total_bookmarks: usize,
    /// Server uptime information.
    pub uptime_seconds: u64,
    /// Current timestamp.
    pub timestamp: String,
}

/// Admin wallet list item (shows all wallets regardless of owner).
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminWalletItem {
    /// Wallet unique identifier.
    pub wallet_id: String,
    /// Owner's user ID.
    pub owner_user_id: String,
    /// Public address.
    pub public_address: String,
    /// Wallet status.
    pub status: WalletStatus,
    /// When the wallet was created.
    pub created_at: String,
}

/// Response for admin wallet list.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminWalletListResponse {
    /// List of all wallets.
    pub wallets: Vec<AdminWalletItem>,
    /// Total count.
    pub total: usize,
}

/// Query parameters for audit log queries.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AuditQueryParams {
    /// Start date (YYYY-MM-DD format).
    pub start_date: Option<String>,
    /// End date (YYYY-MM-DD format).
    pub end_date: Option<String>,
    /// Filter by user ID.
    pub user_id: Option<String>,
    /// Filter by event type.
    pub event_type: Option<String>,
    /// Filter by resource type.
    pub resource_type: Option<String>,
    /// Filter by resource ID.
    pub resource_id: Option<String>,
    /// Maximum number of results (default 100).
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Response for audit log queries.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogResponse {
    /// Audit events matching the query.
    pub events: Vec<AuditEvent>,
    /// Total count (before limit/offset).
    pub total: usize,
    /// Whether there are more results.
    pub has_more: bool,
}

/// Admin user summary.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserSummary {
    /// User ID from Clerk.
    pub user_id: String,
    /// Number of wallets owned.
    pub wallet_count: usize,
    /// Number of bookmarks.
    pub bookmark_count: usize,
}

/// Response for admin user list.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserListResponse {
    /// User summaries.
    pub users: Vec<AdminUserSummary>,
    /// Total unique users.
    pub total: usize,
}

/// Detailed health check response for admins.
#[derive(Debug, Serialize, ToSchema)]
pub struct DetailedHealthResponse {
    /// Overall status.
    pub status: String,
    /// Storage health.
    pub storage: StorageHealth,
    /// Auth configuration status.
    pub auth_configured: bool,
    /// Server version.
    pub version: String,
    /// Build timestamp.
    pub build_time: String,
}

/// Storage health details.
#[derive(Debug, Serialize, ToSchema)]
pub struct StorageHealth {
    /// Data directory path.
    pub data_dir: String,
    /// Whether the data directory exists.
    pub exists: bool,
    /// Whether the data directory is writable.
    pub writable: bool,
    /// Total files in storage (approximate).
    pub total_files: usize,
}

// ============================================================================
// Server start time (for uptime calculation)
// ============================================================================
// Uses std::sync::OnceLock (stable since Rust 1.70) instead of lazy_static
// for a lighter dependency footprint in the enclave.
// ============================================================================

use std::sync::OnceLock;
use std::time::Instant;

/// Server start time, initialized on first access.
/// OnceLock provides thread-safe lazy initialization without external crates.
static SERVER_START: OnceLock<Instant> = OnceLock::new();

/// Get the server start time, initializing it on first call.
///
/// This function returns the instant when the server was first accessed,
/// which is used for uptime calculations in system stats.
fn get_server_start() -> &'static Instant {
    SERVER_START.get_or_init(Instant::now)
}

/// Initialize the server start time. Call this at startup.
///
/// This is optional since `get_server_start()` will initialize on first use,
/// but calling it explicitly at startup ensures consistent uptime reporting.
#[allow(dead_code)]
pub fn init_server_start_time() {
    // Initialize the server start time immediately
    let _ = get_server_start();
}

// ============================================================================
// Handlers
// ============================================================================

/// Get system statistics.
///
/// Returns aggregate statistics about the system including wallet counts
/// and storage metrics. Admin only.
#[utoipa::path(
    get,
    path = "/v1/admin/stats",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "System statistics", body = SystemStatsResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn get_system_stats(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<SystemStatsResponse>, ApiError> {
    let storage = state.storage();

    // Count wallets by status
    let wallet_repo = WalletRepository::new(storage);
    let all_wallets = wallet_repo.list_all_wallets().unwrap_or_default();
    let active_wallets = all_wallets
        .iter()
        .filter(|w| w.status == WalletStatus::Active)
        .count();
    let suspended_wallets = all_wallets
        .iter()
        .filter(|w| w.status == WalletStatus::Suspended)
        .count();
    let deleted_wallets = all_wallets
        .iter()
        .filter(|w| w.status == WalletStatus::Deleted)
        .count();

    // Count bookmarks
    let bookmark_repo = BookmarkRepository::new(storage);
    let total_bookmarks = bookmark_repo.list_all().unwrap_or_default().len();

    // Audit log
    audit_log!(&storage, AuditEventType::AdminAccess, &user);

    Ok(Json(SystemStatsResponse {
        total_wallets: all_wallets.len(),
        active_wallets,
        suspended_wallets,
        deleted_wallets,
        total_bookmarks,
        uptime_seconds: get_server_start().elapsed().as_secs(),
        timestamp: Utc::now().to_rfc3339(),
    }))
}

/// List all wallets (admin view).
///
/// Returns all wallets across all users. Admin only.
#[utoipa::path(
    get,
    path = "/v1/admin/wallets",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All wallets", body = AdminWalletListResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn list_all_wallets(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<AdminWalletListResponse>, ApiError> {
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(storage);

    let wallets = wallet_repo.list_all_wallets().unwrap_or_default();
    let items: Vec<AdminWalletItem> = wallets
        .into_iter()
        .map(|w| AdminWalletItem {
            wallet_id: w.wallet_id.clone(),
            owner_user_id: w.owner_user_id,
            public_address: w.public_address,
            status: w.status,
            created_at: w.created_at.to_rfc3339(),
        })
        .collect();

    let total = items.len();

    // Audit log
    audit_log!(&storage, AuditEventType::AdminAccess, &user);

    Ok(Json(AdminWalletListResponse {
        wallets: items,
        total,
    }))
}

/// List all unique users with their resource counts.
///
/// Returns a summary of all users who have wallets or bookmarks.
#[utoipa::path(
    get,
    path = "/v1/admin/users",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User summaries", body = AdminUserListResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn list_all_users(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<AdminUserListResponse>, ApiError> {
    let storage = state.storage();

    // Collect all user IDs from various sources
    let wallet_repo = WalletRepository::new(storage);
    let bookmark_repo = BookmarkRepository::new(storage);

    let wallets = wallet_repo.list_all_wallets().unwrap_or_default();
    let bookmarks = bookmark_repo.list_all().unwrap_or_default();

    // Build user map
    let mut user_map: std::collections::HashMap<String, AdminUserSummary> =
        std::collections::HashMap::new();

    for wallet in &wallets {
        let entry = user_map
            .entry(wallet.owner_user_id.clone())
            .or_insert_with(|| AdminUserSummary {
                user_id: wallet.owner_user_id.clone(),
                wallet_count: 0,
                bookmark_count: 0,
            });
        entry.wallet_count += 1;
    }

    for bookmark in &bookmarks {
        let entry = user_map
            .entry(bookmark.owner_user_id.clone())
            .or_insert_with(|| AdminUserSummary {
                user_id: bookmark.owner_user_id.clone(),
                wallet_count: 0,
                bookmark_count: 0,
            });
        entry.bookmark_count += 1;
    }

    let users: Vec<AdminUserSummary> = user_map.into_values().collect();
    let total = users.len();

    // Audit log
    audit_log!(&storage, AuditEventType::AdminAccess, &user);

    Ok(Json(AdminUserListResponse { users, total }))
}

/// Query audit logs.
///
/// Search and filter audit log entries. Supports date range, user ID,
/// event type, and resource filtering. Admin only.
#[utoipa::path(
    get,
    path = "/v1/admin/audit/events",
    tag = "Admin",
    params(AuditQueryParams),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Audit events", body = AuditLogResponse),
        (status = 400, description = "Invalid query parameters"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn query_audit_logs(
    AdminOnly(admin_user): AdminOnly,
    Query(params): Query<AuditQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<AuditLogResponse>, ApiError> {
    let storage = state.storage();
    let audit_repo = AuditRepository::new(storage);

    // Default date range: today only
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let start_date = params.start_date.as_deref().unwrap_or(&today);
    let end_date = params.end_date.as_deref().unwrap_or(&today);

    // Validate dates
    NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Invalid start_date format. Use YYYY-MM-DD."))?;
    NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request("Invalid end_date format. Use YYYY-MM-DD."))?;

    // Fetch events
    let mut events = audit_repo
        .read_events_range(start_date, end_date)
        .unwrap_or_default();

    // Apply filters
    if let Some(user_id) = &params.user_id {
        events.retain(|e| e.user_id.as_deref() == Some(user_id.as_str()));
    }

    if let Some(event_type) = &params.event_type {
        events.retain(|e| {
            let type_str = serde_json::to_string(&e.event_type)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            type_str == *event_type
        });
    }

    if let Some(resource_type) = &params.resource_type {
        events.retain(|e| e.resource_type.as_deref() == Some(resource_type.as_str()));
    }

    if let Some(resource_id) = &params.resource_id {
        events.retain(|e| e.resource_id.as_deref() == Some(resource_id.as_str()));
    }

    let total = events.len();
    let limit = params.limit.unwrap_or(100).min(1000); // Max 1000
    let offset = params.offset.unwrap_or(0);

    let has_more = offset + limit < total;
    let events: Vec<AuditEvent> = events.into_iter().skip(offset).take(limit).collect();

    // Log the admin access
    audit_log!(&storage, AuditEventType::AdminAccess, &admin_user);

    Ok(Json(AuditLogResponse {
        events,
        total,
        has_more,
    }))
}

/// Get detailed health information.
///
/// Returns comprehensive health status including storage metrics.
/// More detailed than the public health endpoint. Admin only.
#[utoipa::path(
    get,
    path = "/v1/admin/health",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Detailed health status", body = DetailedHealthResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn get_detailed_health(
    AdminOnly(_user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<DetailedHealthResponse>, ApiError> {
    let storage = state.storage();
    let data_dir = storage.paths().root().to_string_lossy().to_string();

    let exists = storage.paths().root().exists();
    let writable = if exists {
        let test_path = storage.paths().root().join(".health_check");
        std::fs::write(&test_path, "test").is_ok() && std::fs::remove_file(&test_path).is_ok()
    } else {
        false
    };

    // Count files (approximate)
    let total_files = count_files_recursive(storage.paths().root());

    // Check auth configuration
    let auth_configured = std::env::var("CLERK_JWKS_URL").is_ok();

    Ok(Json(DetailedHealthResponse {
        status: if exists && writable {
            "healthy"
        } else {
            "degraded"
        }
        .to_string(),
        storage: StorageHealth {
            data_dir,
            exists,
            writable,
            total_files,
        },
        auth_configured,
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
    }))
}

/// Suspend a wallet (admin action).
///
/// Suspends a wallet by ID. The wallet owner cannot perform operations
/// on suspended wallets until reactivated.
#[utoipa::path(
    post,
    path = "/v1/admin/wallets/{wallet_id}/suspend",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Wallet suspended"),
        (status = 404, description = "Wallet not found"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn suspend_wallet(
    AdminOnly(user): AdminOnly,
    Path(wallet_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(storage);

    let mut wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(format!("Wallet {} not found", wallet_id)))?;

    wallet.status = WalletStatus::Suspended;
    wallet_repo
        .update(&wallet)
        .map_err(|e| ApiError::internal(format!("Failed to suspend wallet: {}", e)))?;

    // Audit log
    let audit_repo = AuditRepository::new(storage);
    let event = AuditEvent::new(AuditEventType::AdminAccess)
        .with_user(&user.user_id)
        .with_resource("wallet", &wallet_id)
        .with_details(serde_json::json!({"action": "suspend"}));
    let _ = audit_repo.log(&event);

    Ok(StatusCode::OK)
}

/// Reactivate a suspended wallet (admin action).
///
/// Reactivates a previously suspended wallet.
#[utoipa::path(
    post,
    path = "/v1/admin/wallets/{wallet_id}/activate",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Wallet activated"),
        (status = 404, description = "Wallet not found"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized (admin required)")
    )
)]
pub async fn activate_wallet(
    AdminOnly(user): AdminOnly,
    Path(wallet_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let storage = state.storage();
    let wallet_repo = WalletRepository::new(storage);

    let mut wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(format!("Wallet {} not found", wallet_id)))?;

    wallet.status = WalletStatus::Active;
    wallet_repo
        .update(&wallet)
        .map_err(|e| ApiError::internal(format!("Failed to activate wallet: {}", e)))?;

    // Audit log
    let audit_repo = AuditRepository::new(storage);
    let event = AuditEvent::new(AuditEventType::AdminAccess)
        .with_user(&user.user_id)
        .with_resource("wallet", &wallet_id)
        .with_details(serde_json::json!({"action": "activate"}));
    let _ = audit_repo.log(&event);

    Ok(StatusCode::OK)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Count files recursively in a directory.
fn count_files_recursive(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                count += 1;
            } else if entry_path.is_dir() {
                count += count_files_recursive(&entry_path);
            }
        }
    }
    count
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_stats_response_serializes() {
        let stats = SystemStatsResponse {
            total_wallets: 10,
            active_wallets: 8,
            suspended_wallets: 1,
            deleted_wallets: 1,
            total_bookmarks: 25,
            uptime_seconds: 3600,
            timestamp: "2026-01-28T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_wallets"));
        assert!(json.contains("active_wallets"));
    }

    #[test]
    fn admin_wallet_item_serializes() {
        let item = AdminWalletItem {
            wallet_id: "w_123".to_string(),
            owner_user_id: "user_456".to_string(),
            public_address: "0x123...".to_string(),
            status: WalletStatus::Active,
            created_at: "2026-01-28T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("wallet_id"));
        assert!(json.contains("owner_user_id"));
    }

    #[test]
    fn audit_query_params_deserializes() {
        let params: AuditQueryParams = serde_json::from_str(
            r#"{
            "start_date": "2026-01-01",
            "end_date": "2026-01-31",
            "user_id": "user_123",
            "limit": 50
        }"#,
        )
        .unwrap();

        assert_eq!(params.start_date, Some("2026-01-01".to_string()));
        assert_eq!(params.user_id, Some("user_123".to_string()));
        assert_eq!(params.limit, Some(50));
    }

    #[test]
    fn count_files_handles_missing_dir() {
        let path = std::path::Path::new("/nonexistent/path");
        assert_eq!(count_files_recursive(path), 0);
    }
}

// ============================================================================
// Peer Management — Admin CRUD for Discovery Peer Registry
// ============================================================================

/// Response for GET /v1/admin/peers/self — this node's discovery identity.
#[derive(Debug, Serialize, ToSchema)]
pub struct SelfNodeInfoResponse {
    /// Base64-encoded VOPRF public key for this node.
    pub voprf_public_key: String,
    /// Whether RA-TLS library is available (running inside SGX).
    pub ratls_available: bool,
    /// Number of configured peers.
    pub peer_count: usize,
}

/// Request body for POST /v1/admin/peers — add a new peer.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddPeerRequest {
    /// Unique node identifier (e.g., "node-eu-1").
    pub node_id: String,
    /// HTTPS URL for the peer's API.
    pub url: String,
    /// Base64-encoded VOPRF public key.
    pub voprf_public_key: String,
    /// MRENCLAVE hex string (64 chars = 32 bytes).
    pub mrenclave: String,
    /// Optional MRSIGNER hex string (64 chars = 32 bytes).
    pub mrsigner: Option<String>,
    /// Minimum ISV SVN version (default 0).
    pub min_isv_svn: Option<u16>,
    /// ISV product ID (default 0).
    pub isv_prod_id: Option<u16>,
}

/// Response item for the peer list.
#[derive(Debug, Serialize, ToSchema)]
pub struct PeerInfoResponse {
    pub node_id: String,
    pub url: String,
    pub voprf_public_key: String,
    pub mrenclave: String,
    pub mrsigner: Option<String>,
    pub min_isv_svn: u16,
    pub isv_prod_id: u16,
}

/// GET /v1/admin/peers/self
///
/// Returns this node's VOPRF public key and discovery status.
pub async fn get_self_node_info(
    AdminOnly(_user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<SelfNodeInfoResponse>, ApiError> {
    let peer_count = state.peer_registry.peers().len();
    let ratls_available = crate::discovery::ffi::is_ratls_available();

    Ok(Json(SelfNodeInfoResponse {
        voprf_public_key: state.peer_registry.own_public_key().to_owned(),
        ratls_available,
        peer_count,
    }))
}

/// GET /v1/admin/peers
///
/// List all configured discovery peers.
pub async fn list_peers(
    AdminOnly(_user): AdminOnly,
    State(state): State<AppState>,
) -> Result<Json<Vec<PeerInfoResponse>>, ApiError> {
    let peers = state.peer_registry.list_peers();
    let response: Vec<PeerInfoResponse> = peers
        .into_iter()
        .map(|p| PeerInfoResponse {
            node_id: p.node_id,
            url: p.url,
            voprf_public_key: p.voprf_public_key,
            mrenclave: alloy::hex::encode(p.attestation_policy.mrenclave),
            mrsigner: p.attestation_policy.mrsigner.map(alloy::hex::encode),
            min_isv_svn: p.attestation_policy.min_isv_svn,
            isv_prod_id: p.attestation_policy.isv_prod_id,
        })
        .collect();

    Ok(Json(response))
}

/// POST /v1/admin/peers
///
/// Add a new discovery peer. Builds an RA-TLS client for the peer
/// (requires RA-TLS library — will fail outside SGX).
pub async fn add_peer(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
    Json(body): Json<AddPeerRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let policy = parse_attestation_policy(&body)?;

    let config = crate::discovery::PeerConfig {
        node_id: body.node_id.clone(),
        url: body.url.clone(),
        voprf_public_key: body.voprf_public_key.clone(),
        attestation_policy: policy,
    };

    state
        .peer_registry
        .add_peer(config)
        .map_err(|e| ApiError::bad_request(format!("Failed to add peer: {e}")))?;

    audit_log!(
        state.storage(),
        AuditEventType::ConfigChanged,
        &user,
        "peer",
        &body.node_id
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Peer added successfully",
            "node_id": body.node_id
        })),
    ))
}

/// DELETE /v1/admin/peers/{node_id}
///
/// Remove a discovery peer by node_id.
pub async fn remove_peer(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .peer_registry
        .remove_peer(&node_id)
        .map_err(|e| ApiError::not_found(format!("Peer not found: {e}")))?;

    audit_log!(
        state.storage(),
        AuditEventType::ConfigChanged,
        &user,
        "peer",
        &node_id
    );

    Ok(Json(serde_json::json!({
        "message": "Peer removed successfully",
        "node_id": node_id
    })))
}

/// PUT /v1/admin/peers/{node_id}
///
/// Update an existing discovery peer configuration.
pub async fn update_peer(
    AdminOnly(user): AdminOnly,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Json(body): Json<AddPeerRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.node_id != node_id {
        return Err(ApiError::bad_request(
            "node_id in path must match node_id in body",
        ));
    }

    let policy = parse_attestation_policy(&body)?;

    let config = crate::discovery::PeerConfig {
        node_id: body.node_id.clone(),
        url: body.url.clone(),
        voprf_public_key: body.voprf_public_key.clone(),
        attestation_policy: policy,
    };

    state
        .peer_registry
        .update_peer(config)
        .map_err(|e| ApiError::bad_request(format!("Failed to update peer: {e}")))?;

    audit_log!(
        state.storage(),
        AuditEventType::ConfigChanged,
        &user,
        "peer",
        &node_id
    );

    Ok(Json(serde_json::json!({
        "message": "Peer updated successfully",
        "node_id": node_id
    })))
}

/// Parse attestation policy from the AddPeerRequest.
fn parse_attestation_policy(
    body: &AddPeerRequest,
) -> Result<crate::discovery::attestation::AttestationPolicy, ApiError> {
    let mrenclave_bytes = alloy::hex::decode(&body.mrenclave)
        .map_err(|_| ApiError::bad_request("Invalid hex in mrenclave"))?;
    if mrenclave_bytes.len() != 32 {
        return Err(ApiError::bad_request(
            "mrenclave must be exactly 32 bytes (64 hex chars)",
        ));
    }
    let mut mrenclave = [0u8; 32];
    mrenclave.copy_from_slice(&mrenclave_bytes);

    let mrsigner = if let Some(ref hex) = body.mrsigner {
        let bytes = alloy::hex::decode(hex)
            .map_err(|_| ApiError::bad_request("Invalid hex in mrsigner"))?;
        if bytes.len() != 32 {
            return Err(ApiError::bad_request(
                "mrsigner must be exactly 32 bytes (64 hex chars)",
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Some(arr)
    } else {
        None
    };

    Ok(crate::discovery::attestation::AttestationPolicy {
        mrenclave,
        mrsigner,
        min_isv_svn: body.min_isv_svn.unwrap_or(0),
        isv_prod_id: body.isv_prod_id.unwrap_or(0),
    })
}

// ============================================================================
// RA-TLS Diagnostics
// ============================================================================

/// Result of a single RA-TLS verification step in a diagnostic run.
#[derive(Debug, Serialize, ToSchema)]
pub struct DiagnosticStep {
    /// Stable identifier (e.g., "library_load", "cert_read", "verify_callback").
    pub step: &'static str,
    /// Whether this step succeeded.
    pub ok: bool,
    /// Optional human-readable detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Response body for the RA-TLS test endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct RaTlsTestResponse {
    /// Overall test outcome.
    pub ok: bool,
    /// What was tested ("self" or "peer:<node_id>").
    pub target: String,
    /// Step-by-step trace.
    pub steps: Vec<DiagnosticStep>,
    /// Wrapper return code from `ra_tls_verify_callback_der`, if invoked.
    /// Negative values are mbedTLS error codes; the inner `quote3_error_t`
    /// (e.g. `0xE03A`) is logged to stderr by libra_tls_verify_dcap and
    /// can be cross-referenced from the server logs at the same timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_code: Option<i32>,
    /// Decoded explanation of the wrapper code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_code_meaning: Option<&'static str>,
    /// Measurements observed in the verified quote (only present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_measurements: Option<crate::discovery::ffi::ObservedMeasurements>,
    /// Free-form remediation hints for failure cases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remediation: Vec<&'static str>,
}

/// POST /v1/admin/peers/self/test
///
/// Run an RA-TLS dry-run verification of this enclave's own RA-TLS
/// certificate. This isolates "is the local DCAP verification stack
/// working" from "is the peer reachable / does the peer have a valid
/// quote". If self-test passes, any subsequent peer-test failure is
/// attributable to the peer's platform or network, not our verifier.
#[utoipa::path(
    post,
    path = "/v1/admin/peers/self/test",
    responses(
        (status = 200, description = "Diagnostic completed (check `ok` field)", body = RaTlsTestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not authorized (admin required)"),
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn test_self_ratls(
    AdminOnly(_user): AdminOnly,
) -> Result<Json<RaTlsTestResponse>, ApiError> {
    let mut steps: Vec<DiagnosticStep> = Vec::new();
    let target = "self".to_string();

    // Step 1: library load
    let lib_ok = crate::discovery::ffi::is_ratls_available();
    steps.push(DiagnosticStep {
        step: "library_load",
        ok: lib_ok,
        detail: if lib_ok {
            None
        } else {
            Some("libra_tls_verify_dcap.so not loadable — running outside SGX/Gramine?".into())
        },
    });
    if !lib_ok {
        return Ok(Json(RaTlsTestResponse {
            ok: false,
            target,
            steps,
            wrapper_code: None,
            wrapper_code_meaning: None,
            observed_measurements: None,
            remediation: vec![
                "Run inside an SGX-enabled container with /dev/sgx/* mounted.",
                "Verify libra_tls_verify_dcap.so is installed (gramine-ratls-dcap package).",
            ],
        }));
    }

    // Step 2: read our own RA-TLS certificate
    let cert_bytes = match std::fs::read(crate::tls::RA_TLS_CERT_PATH) {
        Ok(s) => {
            steps.push(DiagnosticStep {
                step: "cert_read",
                ok: true,
                detail: Some(format!("Read {} bytes", s.len())),
            });
            s
        }
        Err(e) => {
            steps.push(DiagnosticStep {
                step: "cert_read",
                ok: false,
                detail: Some(format!(
                    "Failed to read {}: {}",
                    crate::tls::RA_TLS_CERT_PATH,
                    e
                )),
            });
            return Ok(Json(RaTlsTestResponse {
                ok: false,
                target,
                steps,
                wrapper_code: None,
                wrapper_code_meaning: None,
                observed_measurements: None,
                remediation: vec![
                    "Check that gramine-ratls generated the cert at /tmp/ra-tls.crt.pem.",
                    "Verify libos.entrypoint is gramine-ratls in the manifest.",
                ],
            }));
        }
    };

    // Step 3: PEM (incl. Gramine TRUSTED CERTIFICATE label) → DER via tls::load_ratls_certificate
    let der = match crate::tls::load_ratls_certificate(crate::tls::RA_TLS_CERT_PATH) {
        Ok(certs) if !certs.is_empty() => {
            let der_bytes = certs[0].as_ref().to_vec();
            steps.push(DiagnosticStep {
                step: "cert_decode",
                ok: true,
                detail: Some(format!(
                    "DER length: {} bytes ({} cert{} in chain)",
                    der_bytes.len(),
                    certs.len(),
                    if certs.len() == 1 { "" } else { "s" },
                )),
            });
            der_bytes
        }
        Ok(_) => {
            steps.push(DiagnosticStep {
                step: "cert_decode",
                ok: false,
                detail: Some("No certificates parsed from PEM".into()),
            });
            return Ok(Json(RaTlsTestResponse {
                ok: false,
                target,
                steps,
                wrapper_code: None,
                wrapper_code_meaning: None,
                observed_measurements: None,
                remediation: vec![
                    "Cert file exists but contains no parseable certificates. Regenerate with gramine-ratls.",
                ],
            }));
        }
        Err(e) => {
            let head: String = String::from_utf8_lossy(&cert_bytes[..cert_bytes.len().min(80)])
                .replace('\n', "\\n");
            steps.push(DiagnosticStep {
                step: "cert_decode",
                ok: false,
                detail: Some(format!("{e} (head: {head})")),
            });
            return Ok(Json(RaTlsTestResponse {
                ok: false,
                target,
                steps,
                wrapper_code: None,
                wrapper_code_meaning: None,
                observed_measurements: None,
                remediation: vec![
                    "Inspect /tmp/ra-tls.crt.pem manually — the PEM label or base64 body may be malformed.",
                    "Expected labels: -----BEGIN TRUSTED CERTIFICATE----- or -----BEGIN CERTIFICATE-----.",
                ],
            }));
        }
    };

    // Step 4: dry-run verify (detailed — captures stderr from C library)
    let dry = match crate::discovery::ffi::verify_ratls_cert_dry_run_detailed(&der) {
        Ok(d) => d,
        Err(e) => {
            steps.push(DiagnosticStep {
                step: "verify_callback",
                ok: false,
                detail: Some(e.to_string()),
            });
            return Ok(Json(RaTlsTestResponse {
                ok: false,
                target,
                steps,
                wrapper_code: None,
                wrapper_code_meaning: None,
                observed_measurements: None,
                remediation: vec![],
            }));
        }
    };

    if dry.wrapper_code == 0 {
        steps.push(DiagnosticStep {
            step: "verify_callback",
            ok: true,
            detail: Some(
                "DCAP collateral fetch + quote signature verification + measurement callback succeeded".into(),
            ),
        });
        return Ok(Json(RaTlsTestResponse {
            ok: true,
            target,
            steps,
            wrapper_code: Some(0),
            wrapper_code_meaning: Some(crate::discovery::ffi::decode_wrapper_code(0)),
            observed_measurements: dry.observed,
            remediation: vec![],
        }));
    }

    // Failure path: surface inner quote3_error_t if we captured it from stderr.
    let mut detail = format!("DCAP quote verification failed with wrapper code {}", dry.wrapper_code);
    let mut remediation: Vec<&'static str> = Vec::new();
    if let Some(q3) = dry.quote3_error {
        let (name, hint) = crate::discovery::ffi::decode_quote3_error(q3);
        detail.push_str(&format!(
            " — inner quote3_error_t = {q3} (0x{q3:04X}) {name}"
        ));
        remediation.push(hint);
    }
    if !dry.captured_stderr.is_empty() {
        let trimmed = dry.captured_stderr.trim();
        let snippet: String = trimmed.chars().take(800).collect();
        detail.push_str(&format!("\nstderr: {snippet}"));
    }
    if remediation.is_empty() {
        remediation.extend([
            "Check the captured stderr above for an 'sgx_qv_verify_quote failed: NNNNN' line.",
            "Common causes: wrong enclave clock, collateral fetch HTTPS failure (DNS/CA), stale ~/.az-dcap-client/cache, unregistered FMSPC at Azure PCS.",
            "Try restarting aesmd inside the container and clearing ~/.az-dcap-client/cache.",
        ]);
    }
    steps.push(DiagnosticStep {
        step: "verify_callback",
        ok: false,
        detail: Some(detail),
    });
    Ok(Json(RaTlsTestResponse {
        ok: false,
        target,
        steps,
        wrapper_code: Some(dry.wrapper_code),
        wrapper_code_meaning: Some(crate::discovery::ffi::decode_wrapper_code(dry.wrapper_code)),
        observed_measurements: dry.observed,
        remediation,
    }))
}

/// POST /v1/admin/peers/{node_id}/test
///
/// Connect to a configured peer over RA-TLS and report whether the
/// handshake (which exercises our DCAP verifier against the peer's
/// quote) succeeds. No business logic is invoked — only the TLS layer.
#[utoipa::path(
    post,
    path = "/v1/admin/peers/{node_id}/test",
    params(("node_id" = String, Path, description = "Peer node identifier")),
    responses(
        (status = 200, description = "Diagnostic completed (check `ok` field)", body = RaTlsTestResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not authorized (admin required)"),
        (status = 404, description = "Peer not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn test_peer_ratls(
    AdminOnly(_user): AdminOnly,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<RaTlsTestResponse>, ApiError> {
    let target = format!("peer:{node_id}");
    let mut steps: Vec<DiagnosticStep> = Vec::new();

    // Resolve peer
    let peers = state.peer_registry.list_peers();
    let Some(peer) = peers.into_iter().find(|p| p.node_id == node_id) else {
        return Err(ApiError::not_found(format!("Peer not found: {node_id}")));
    };
    steps.push(DiagnosticStep {
        step: "peer_lookup",
        ok: true,
        detail: Some(format!("Resolved {} -> {}", peer.node_id, peer.url)),
    });

    let Some(client) = state.peer_registry.client_for(&node_id) else {
        steps.push(DiagnosticStep {
            step: "client_lookup",
            ok: false,
            detail: Some("No RA-TLS client built for this peer (check startup logs)".into()),
        });
        return Ok(Json(RaTlsTestResponse {
            ok: false,
            target,
            steps,
            wrapper_code: None,
            wrapper_code_meaning: None,
            observed_measurements: None,
            remediation: vec!["Restart the server. If the client still fails to build, /tmp/ra-tls.crt.pem may be missing."],
        }));
    };
    steps.push(DiagnosticStep {
        step: "client_lookup",
        ok: true,
        detail: None,
    });

    // Issue a HEAD request — TLS handshake fires before any HTTP semantics,
    // so an HTTP 401/404 still means the RA-TLS verification succeeded.
    let probe_url = format!("{}/api-doc/openapi.json", peer.url.trim_end_matches('/'));
    match client.get(&probe_url).send().await {
        Ok(resp) => {
            steps.push(DiagnosticStep {
                step: "tls_handshake",
                ok: true,
                detail: Some(format!("HTTP {}", resp.status())),
            });
            steps.push(DiagnosticStep {
                step: "verify_callback",
                ok: true,
                detail: Some("Peer's RA-TLS cert verified against configured policy".into()),
            });
            Ok(Json(RaTlsTestResponse {
                ok: true,
                target,
                steps,
                wrapper_code: Some(0),
                wrapper_code_meaning: Some(crate::discovery::ffi::decode_wrapper_code(0)),
                observed_measurements: None,
                remediation: vec![],
            }))
        }
        Err(e) => {
            // reqwest wraps the rustls/RA-TLS error inside its own; surface the chain.
            let mut chain = String::new();
            let mut src: Option<&dyn std::error::Error> = Some(&e);
            while let Some(err) = src {
                if !chain.is_empty() {
                    chain.push_str(" -> ");
                }
                chain.push_str(&err.to_string());
                src = err.source();
            }
            steps.push(DiagnosticStep {
                step: "tls_handshake",
                ok: false,
                detail: Some(chain.clone()),
            });
            // Try to detect whether this was an RA-TLS verifier failure vs. a
            // network / connect-level error.
            let is_ratls_failure =
                chain.contains("RA-TLS") || chain.contains("CertificateInvalid");
            let remediation: Vec<&'static str> = if is_ratls_failure {
                vec![
                    "Run /v1/admin/peers/self/test first — if self-test fails, fix the local verifier before debugging the peer.",
                    "If self-test passes, the peer's quote/platform is the problem: peer's MRENCLAVE may have changed, or peer's host is unknown to PCS.",
                    "Check the server logs at the same timestamp for 'sgx_qv_verify_quote failed: NNNNN' — the NNNNN is the inner quote3_error_t.",
                ]
            } else {
                vec![
                    "Connection-level failure (DNS, TCP, TLS handshake) — not an attestation policy rejection.",
                    "Verify the peer URL is reachable from inside the enclave (DNS, firewall, routing).",
                    "Check whether the peer is online and listening on the configured port.",
                ]
            };
            Ok(Json(RaTlsTestResponse {
                ok: false,
                target,
                steps,
                wrapper_code: None,
                wrapper_code_meaning: None,
                observed_measurements: None,
                remediation,
            }))
        }
    }
}

/// Decode a PEM-encoded certificate to DER. Picks the first CERTIFICATE block.
#[allow(dead_code)]
fn pem_to_der(pem: &str) -> Result<Vec<u8>, String> {
    use base64ct::{Base64, Encoding};
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let start = pem.find(BEGIN).ok_or("missing BEGIN CERTIFICATE")?;
    let after = &pem[start + BEGIN.len()..];
    let end = after.find(END).ok_or("missing END CERTIFICATE")?;
    let b64: String = after[..end].chars().filter(|c| !c.is_whitespace()).collect();
    Base64::decode_vec(&b64).map_err(|e| format!("base64 decode failed: {e}"))
}
