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
        AuditEvent, AuditEventType, AuditRepository, BookmarkRepository, InviteRepository,
        RecurringRepository, WalletRepository, WalletStatus,
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
    /// Total number of invites.
    pub total_invites: usize,
    /// Number of redeemed invites.
    pub redeemed_invites: usize,
    /// Total number of recurring payments.
    pub total_recurring_payments: usize,
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
    /// Number of recurring payments.
    pub recurring_payment_count: usize,
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

lazy_static::lazy_static! {
    static ref SERVER_START: std::time::Instant = std::time::Instant::now();
}

/// Initialize the server start time. Call this at startup.
/// Note: SERVER_START is lazily initialized, so this is optional.
#[allow(dead_code)]
pub fn init_server_start_time() {
    // Access the lazy static to initialize it
    let _ = *SERVER_START;
}

// ============================================================================
// Handlers
// ============================================================================

/// Get system statistics.
///
/// Returns aggregate statistics about the system including wallet counts,
/// invite usage, and storage metrics. Admin only.
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
    let wallet_repo = WalletRepository::new(&storage);
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
    let bookmark_repo = BookmarkRepository::new(&storage);
    let total_bookmarks = bookmark_repo.list_all().unwrap_or_default().len();

    // Count invites
    let invite_repo = InviteRepository::new(&storage);
    let all_invites = invite_repo.list_all().unwrap_or_default();
    let redeemed_invites = all_invites.iter().filter(|i| i.redeemed).count();

    // Count recurring payments
    let recurring_repo = RecurringRepository::new(&storage);
    let total_recurring = recurring_repo.list_all().unwrap_or_default().len();

    // Audit log
    audit_log!(&storage, AuditEventType::AdminAccess, &user);

    Ok(Json(SystemStatsResponse {
        total_wallets: all_wallets.len(),
        active_wallets,
        suspended_wallets,
        deleted_wallets,
        total_bookmarks,
        total_invites: all_invites.len(),
        redeemed_invites,
        total_recurring_payments: total_recurring,
        uptime_seconds: SERVER_START.elapsed().as_secs(),
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
    let wallet_repo = WalletRepository::new(&storage);

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

    Ok(Json(AdminWalletListResponse { wallets: items, total }))
}

/// List all unique users with their resource counts.
///
/// Returns a summary of all users who have wallets, bookmarks, or recurring payments.
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
    let wallet_repo = WalletRepository::new(&storage);
    let bookmark_repo = BookmarkRepository::new(&storage);
    let recurring_repo = RecurringRepository::new(&storage);

    let wallets = wallet_repo.list_all_wallets().unwrap_or_default();
    let bookmarks = bookmark_repo.list_all().unwrap_or_default();
    let recurring = recurring_repo.list_all().unwrap_or_default();

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
                recurring_payment_count: 0,
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
                recurring_payment_count: 0,
            });
        entry.bookmark_count += 1;
    }

    for payment in &recurring {
        let entry = user_map
            .entry(payment.owner_user_id.clone())
            .or_insert_with(|| AdminUserSummary {
                user_id: payment.owner_user_id.clone(),
                wallet_count: 0,
                bookmark_count: 0,
                recurring_payment_count: 0,
            });
        entry.recurring_payment_count += 1;
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
    let audit_repo = AuditRepository::new(&storage);

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
        status: if exists && writable { "healthy" } else { "degraded" }.to_string(),
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
    let wallet_repo = WalletRepository::new(&storage);

    let mut wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {} not found", wallet_id)))?;

    wallet.status = WalletStatus::Suspended;
    wallet_repo
        .update(&wallet)
        .map_err(|e| ApiError::internal(&format!("Failed to suspend wallet: {}", e)))?;

    // Audit log
    let audit_repo = AuditRepository::new(&storage);
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
    let wallet_repo = WalletRepository::new(&storage);

    let mut wallet = wallet_repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {} not found", wallet_id)))?;

    wallet.status = WalletStatus::Active;
    wallet_repo
        .update(&wallet)
        .map_err(|e| ApiError::internal(&format!("Failed to activate wallet: {}", e)))?;

    // Audit log
    let audit_repo = AuditRepository::new(&storage);
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
            total_invites: 5,
            redeemed_invites: 3,
            total_recurring_payments: 7,
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
        let params: AuditQueryParams = serde_json::from_str(r#"{
            "start_date": "2026-01-01",
            "end_date": "2026-01-31",
            "user_id": "user_123",
            "limit": 50
        }"#)
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
