// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    auth::Role,
    blockchain::{TokenBalance, WalletBalanceResponse},
    models::{
        Bookmark, CreateBookmarkRequest, CreateRecurringPaymentRequest, Invite,
        RecurringPayment, RedeemInviteRequest, UpdateLastPaidDateRequest,
        UpdateRecurringPaymentRequest, WalletAddress,
    },
    state::AppState,
};

pub mod admin;
pub mod balance;
pub mod bookmarks;
pub mod health;
pub mod invites;
pub mod recurring;
pub mod users;
pub mod wallets;

pub fn router(state: AppState) -> Router {
    let v1_routes = Router::new()
        // User endpoints (auth required)
        .route("/users/me", get(users::get_current_user))
        // Wallet lifecycle endpoints (auth required)
        .route(
            "/wallets",
            get(wallets::list_wallets).post(wallets::create_wallet),
        )
        .route(
            "/wallets/{wallet_id}",
            get(wallets::get_wallet).delete(wallets::delete_wallet),
        )
        // Wallet balance endpoints
        .route(
            "/wallets/{wallet_id}/balance",
            get(balance::get_wallet_balance),
        )
        .route(
            "/wallets/{wallet_id}/balance/native",
            get(balance::get_native_balance),
        )
        // Bookmark endpoints
        .route(
            "/bookmarks",
            get(bookmarks::list_bookmarks).post(bookmarks::create_bookmark),
        )
        .route(
            "/bookmarks/{bookmark_id}",
            delete(bookmarks::delete_bookmark),
        )
        // Invite endpoints
        .route("/invite", get(invites::get_invite))
        .route("/invite/redeem", post(invites::redeem_invite))
        // Recurring payment endpoints
        .route(
            "/recurring/payments",
            get(recurring::list_recurring_payments).post(recurring::create_recurring_payment),
        )
        .route(
            "/recurring/payment/{recurring_payment_id}",
            delete(recurring::delete_recurring_payment).put(recurring::update_recurring_payment),
        )
        .route(
            "/recurring/payment/{recurring_payment_id}/last-paid-date",
            put(recurring::update_last_paid_date),
        )
        .route(
            "/recurring/payments/today",
            get(recurring::recurring_payments_today),
        )
        // Admin endpoints (admin role required)
        .route("/admin/stats", get(admin::get_system_stats))
        .route("/admin/wallets", get(admin::list_all_wallets))
        .route("/admin/users", get(admin::list_all_users))
        .route("/admin/audit/events", get(admin::query_audit_logs))
        .route("/admin/health", get(admin::get_detailed_health))
        .route(
            "/admin/wallets/{wallet_id}/suspend",
            post(admin::suspend_wallet),
        )
        .route(
            "/admin/wallets/{wallet_id}/activate",
            post(admin::activate_wallet),
        )
        .with_state(state.clone());

    Router::new()
        // Health endpoints (no auth required, but need state for JWKS check)
        .route("/health", get(health::health))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        // API v1 routes
        .nest("/v1", v1_routes)
        // Swagger UI
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(
    paths(
        // User endpoints
        users::get_current_user,
        // Wallet lifecycle endpoints
        wallets::create_wallet,
        wallets::list_wallets,
        wallets::get_wallet,
        wallets::delete_wallet,
        // Wallet balance endpoints
        balance::get_wallet_balance,
        balance::get_native_balance,
        // Bookmark endpoints
        bookmarks::list_bookmarks,
        bookmarks::create_bookmark,
        bookmarks::delete_bookmark,
        // Invite endpoints
        invites::get_invite,
        invites::redeem_invite,
        // Recurring payment endpoints
        recurring::list_recurring_payments,
        recurring::create_recurring_payment,
        recurring::update_recurring_payment,
        recurring::delete_recurring_payment,
        recurring::recurring_payments_today,
        recurring::update_last_paid_date,
        // Admin endpoints
        admin::get_system_stats,
        admin::list_all_wallets,
        admin::list_all_users,
        admin::query_audit_logs,
        admin::get_detailed_health,
        admin::suspend_wallet,
        admin::activate_wallet,
        // Health endpoints
        health::health,
        health::liveness,
        health::readiness
    ),
    components(
        schemas(
            // Auth schemas
            Role,
            users::UserMeResponse,
            // Wallet lifecycle schemas
            wallets::CreateWalletRequest,
            wallets::CreateWalletResponse,
            wallets::WalletListResponse,
            wallets::DeleteWalletResponse,
            crate::storage::WalletResponse,
            crate::storage::WalletStatus,
            // Wallet balance schemas
            balance::BalanceResponse,
            balance::NativeBalanceResponse,
            TokenBalance,
            WalletBalanceResponse,
            // Admin schemas
            admin::SystemStatsResponse,
            admin::AdminWalletItem,
            admin::AdminWalletListResponse,
            admin::AdminUserSummary,
            admin::AdminUserListResponse,
            admin::AuditLogResponse,
            admin::DetailedHealthResponse,
            admin::StorageHealth,
            crate::storage::AuditEvent,
            crate::storage::AuditEventType,
            // Data schemas
            Bookmark,
            Invite,
            RecurringPayment,
            WalletAddress,
            CreateBookmarkRequest,
            RedeemInviteRequest,
            CreateRecurringPaymentRequest,
            UpdateRecurringPaymentRequest,
            UpdateLastPaidDateRequest,
            // Health schemas
            health::HealthResponse,
            health::HealthChecks,
            health::ReadyResponse
        )
    ),
    tags(
        (name = "Users", description = "User identity and authentication"),
        (name = "Wallets", description = "Wallet lifecycle management"),
        (name = "Bookmarks", description = "Bookmark management"),
        (name = "Invites", description = "Invite validation and redemption"),
        (name = "Recurring", description = "Recurring payment scheduling"),
        (name = "Admin", description = "Admin-only system management"),
        (name = "Health", description = "Liveness and readiness checks")
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

/// Security scheme for OpenAPI documentation
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
            let scheme = Http::builder()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .description(Some("Clerk JWT token"))
                .build();
            components.add_security_scheme("bearer", SecurityScheme::Http(scheme));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn router_builds_with_all_routes() {
        let app = router(AppState::default());
        // Ensure the router can be converted into a service without panicking.
        let _ = app.into_make_service();
    }

    #[test]
    fn generate_openapi_json() {
        use std::fs;
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        fs::write("/tmp/openapi_generated.json", &json).unwrap();
        assert!(json.contains("openapi"));
    }
}
