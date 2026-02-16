// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use utoipa::OpenApi;

use crate::{
    auth::Role,
    blockchain::{TokenBalance, WalletBalanceResponse},
    models::{
        Bookmark, CreateBookmarkRequest, CreateRecurringPaymentRequest, Invite, RecurringPayment,
        RedeemInviteRequest, UpdateLastPaidDateRequest, UpdateRecurringPaymentRequest,
        WalletAddress,
    },
    state::AppState,
    storage::{
        FiatDirection, FiatRequestStatus, StoredFiatRequest, StoredTransaction, TokenType, TxStatus,
    },
};

pub mod admin;
pub mod balance;
pub mod bookmarks;
pub mod fiat;
pub mod health;
pub mod invites;
pub mod recurring;
pub mod transactions;
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
        // Transaction endpoints
        .route(
            "/wallets/{wallet_id}/estimate",
            post(transactions::estimate_gas),
        )
        .route(
            "/wallets/{wallet_id}/send",
            post(transactions::send_transaction),
        )
        .route(
            "/wallets/{wallet_id}/transactions",
            get(transactions::list_transactions),
        )
        .route(
            "/wallets/{wallet_id}/transactions/{tx_hash}",
            get(transactions::get_transaction_status),
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
        // Fiat request stubs
        .route("/fiat/providers", get(fiat::list_fiat_providers))
        .route(
            "/fiat/providers/truelayer/webhook",
            post(fiat::truelayer_webhook),
        )
        .route("/fiat/onramp/requests", post(fiat::create_onramp_request))
        .route("/fiat/offramp/requests", post(fiat::create_offramp_request))
        .route("/fiat/requests", get(fiat::list_fiat_requests))
        .route("/fiat/requests/{request_id}", get(fiat::get_fiat_request))
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
        .route(
            "/admin/fiat/service-wallet",
            get(fiat::get_fiat_service_wallet),
        )
        .route(
            "/admin/fiat/service-wallet/bootstrap",
            post(fiat::bootstrap_fiat_service_wallet),
        )
        .route("/admin/fiat/reserve/topup", post(fiat::topup_fiat_reserve))
        .route(
            "/admin/fiat/reserve/transfer",
            post(fiat::transfer_fiat_reserve),
        )
        .route(
            "/admin/fiat/requests/{request_id}/sync",
            post(fiat::sync_fiat_request_admin),
        )
        .with_state(state.clone());

    Router::new()
        // Health endpoints (no auth required, but need state for JWKS check)
        .route("/health", get(health::health))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        // API v1 routes
        .nest("/v1", v1_routes)
        // Swagger/OpenAPI docs
        .route("/api-doc/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui_index))
        .route("/docs/", get(swagger_ui_index))
        .route("/docs/{*rest}", get(swagger_ui_asset))
        .layer(build_cors_layer())
        .with_state(state)
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn swagger_ui_index() -> Response {
    serve_swagger_ui("index.html")
}

async fn swagger_ui_asset(Path(rest): Path<String>) -> Response {
    serve_swagger_ui(&rest)
}

fn serve_swagger_ui(path: &str) -> Response {
    let config = Arc::new(utoipa_swagger_ui::Config::from("/api-doc/openapi.json"));
    let asset_path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path
    };

    match utoipa_swagger_ui::serve(asset_path, config) {
        Ok(Some(file)) => {
            let content_type = file.content_type;
            let body = file.bytes.into_owned();

            // Make relative asset links resolve correctly for both /docs and /docs/.
            if asset_path == "index.html" {
                match String::from_utf8(body) {
                    Ok(html) => {
                        let html = if html.contains("<base href=\"/docs/\"") {
                            html
                        } else {
                            html.replacen("<head>", "<head>\n    <base href=\"/docs/\" />", 1)
                        };
                        return (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], html)
                            .into_response();
                    }
                    Err(error) => {
                        return (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, content_type)],
                            error.into_bytes(),
                        )
                            .into_response();
                    }
                }
            }

            (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Build CORS layer from environment configuration.
///
/// - If `CORS_ALLOWED_ORIGINS` is set, only those origins are allowed.
///   Multiple origins can be comma-separated (e.g., `https://app.example.com,https://staging.example.com`).
/// - If not set, falls back to permissive CORS (development only).
fn build_cors_layer() -> CorsLayer {
    if let Ok(origins) = std::env::var("CORS_ALLOWED_ORIGINS") {
        let allowed: Vec<_> = origins
            .split(',')
            .map(|s| s.trim().parse().expect("Invalid CORS origin"))
            .collect();
        tracing::info!(origins = %origins, "CORS: restricting to configured origins");
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(allowed))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    } else {
        tracing::warn!("CORS_ALLOWED_ORIGINS not set - using permissive CORS (development only)");
        CorsLayer::permissive()
    }
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
        // Transaction endpoints
        transactions::estimate_gas,
        transactions::send_transaction,
        transactions::list_transactions,
        transactions::get_transaction_status,
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
        // Fiat endpoints
        fiat::list_fiat_providers,
        fiat::truelayer_webhook,
        fiat::create_onramp_request,
        fiat::create_offramp_request,
        fiat::list_fiat_requests,
        fiat::get_fiat_request,
        fiat::get_fiat_service_wallet,
        fiat::bootstrap_fiat_service_wallet,
        fiat::topup_fiat_reserve,
        fiat::transfer_fiat_reserve,
        fiat::sync_fiat_request_admin,
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
            // Transaction schemas
            transactions::EstimateGasRequest,
            transactions::EstimateGasResponse,
            transactions::SendTransactionRequest,
            transactions::SendTransactionResponse,
            transactions::TransactionListResponse,
            transactions::TransactionSummary,
            transactions::TransactionStatusResponse,
            StoredTransaction,
            TokenType,
            TxStatus,
            // Fiat schemas
            fiat::CreateFiatRequest,
            fiat::FiatProviderSummary,
            fiat::FiatProviderListResponse,
            fiat::FiatRequestResponse,
            fiat::FiatRequestListResponse,
            fiat::FiatServiceWalletStatusResponse,
            fiat::ReserveTopUpRequest,
            fiat::ReserveTransferRequest,
            fiat::ReserveTransactionResponse,
            fiat::FiatSyncResponse,
            FiatDirection,
            FiatRequestStatus,
            StoredFiatRequest,
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
        (name = "Transactions", description = "Transaction signing and sending"),
        (name = "Bookmarks", description = "Bookmark management"),
        (name = "Invites", description = "Invite validation and redemption"),
        (name = "Recurring", description = "Recurring payment scheduling"),
        (name = "Fiat", description = "Fiat on-ramp/off-ramp provider integrations"),
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
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_builds_with_all_routes() {
        let app = router(AppState::default());
        // Ensure the router can be converted into a service without panicking.
        let _ = app.into_make_service();
    }

    #[tokio::test]
    async fn docs_route_serves_without_redirect() {
        let app = router(AppState::default());
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/docs/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn generate_openapi_json() {
        use std::fs;
        let json = ApiDoc::openapi().to_pretty_json().unwrap();
        fs::write("/tmp/openapi_generated.json", &json).unwrap();
        assert!(json.contains("openapi"));
    }
}
