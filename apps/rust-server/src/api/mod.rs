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
    models::{
        AutofundRequest, Bookmark, CreateBookmarkRequest, CreateRecurringPaymentRequest, Invite,
        RecurringPayment, RedeemInviteRequest, UpdateLastPaidDateRequest,
        UpdateRecurringPaymentRequest, WalletAddress,
    },
    state::AppState,
};

pub mod bookmarks;
pub mod invites;
pub mod recurring;
pub mod wallet;

pub fn router(state: AppState) -> Router {
    let v1_routes = Router::new()
        .route(
            "/bookmarks",
            get(bookmarks::list_bookmarks).post(bookmarks::create_bookmark),
        )
        .route(
            "/bookmarks/:bookmark_id",
            delete(bookmarks::delete_bookmark),
        )
        .route("/invite", get(invites::get_invite))
        .route("/invite/redeem", post(invites::redeem_invite))
        .route("/wallet/autofund", post(wallet::autofund_wallet))
        .route(
            "/recurring/payments",
            get(recurring::list_recurring_payments).post(recurring::create_recurring_payment),
        )
        .route(
            "/recurring/payment/:recurring_payment_id",
            delete(recurring::delete_recurring_payment).put(recurring::update_recurring_payment),
        )
        .route(
            "/recurring/payment/:recurring_payment_id/last-paid-date",
            put(recurring::update_last_paid_date),
        )
        .route(
            "/recurring/payments/today",
            get(recurring::recurring_payments_today),
        )
        .with_state(state);

    Router::new()
        .nest("/v1", v1_routes)
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        bookmarks::list_bookmarks,
        bookmarks::create_bookmark,
        bookmarks::delete_bookmark,
        invites::get_invite,
        invites::redeem_invite,
        wallet::autofund_wallet,
        recurring::list_recurring_payments,
        recurring::create_recurring_payment,
        recurring::update_recurring_payment,
        recurring::delete_recurring_payment,
        recurring::recurring_payments_today,
        recurring::update_last_paid_date
    ),
    components(
        schemas(
            Bookmark,
            Invite,
            RecurringPayment,
            WalletAddress,
            CreateBookmarkRequest,
            RedeemInviteRequest,
            AutofundRequest,
            CreateRecurringPaymentRequest,
            UpdateRecurringPaymentRequest,
            UpdateLastPaidDateRequest
        )
    ),
    tags(
        (name = "Bookmarks", description = "Bookmark management"),
        (name = "Invites", description = "Invite validation and redemption"),
        (name = "Wallet", description = "Wallet utilities"),
        (name = "Recurring", description = "Recurring payment scheduling")
    )
)]
struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn router_builds_with_all_routes() {
        let app = router(AppState::default());
        // Ensure the router can be converted into a service without panicking.
        let _ = app.into_make_service();
    }
}
