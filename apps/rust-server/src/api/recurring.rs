// SPDX-License-Identifier: AGPL-3.0-or-later 
// 
// Copyright (C) 2025 Relational Network 
// 
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet) 

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    error::ApiError,
    models::{
        CreateRecurringPaymentRequest, RecurringPayment, UpdateLastPaidDateRequest,
        UpdateRecurringPaymentRequest, WalletAddress,
    },
    state::AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct RecurringQuery {
    pub wallet_id: WalletAddress,
}

#[utoipa::path(
    get,
    path = "/v1/recurring/payments",
    params(RecurringQuery),
    tag = "Recurring",
    responses((status = 200, body = [RecurringPayment]))
)]
pub async fn list_recurring_payments(
    State(state): State<AppState>,
    Query(params): Query<RecurringQuery>,
) -> Result<Json<Vec<RecurringPayment>>, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.list_recurring(&params.wallet_id)))
}

#[utoipa::path(
    post,
    path = "/v1/recurring/payment",
    request_body = CreateRecurringPaymentRequest,
    tag = "Recurring",
    responses((status = 201))
)]
pub async fn create_recurring_payment(
    State(state): State<AppState>,
    Json(request): Json<CreateRecurringPaymentRequest>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.write().await;
    store.create_recurring_payment(request)?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    put,
    path = "/v1/recurring/payment/{recurring_payment_id}",
    params(
        (
            "recurring_payment_id" = String,
            Path,
            description = "Identifier of the recurring payment to update"
        )
    ),
    request_body = UpdateRecurringPaymentRequest,
    tag = "Recurring",
    responses((status = 200))
)]
pub async fn update_recurring_payment(
    Path(recurring_payment_id): Path<String>,
    State(state): State<AppState>,
    Json(mut request): Json<UpdateRecurringPaymentRequest>,
) -> Result<StatusCode, ApiError> {
    request.recurring_payment_id = recurring_payment_id;

    let mut store = state.store.write().await;
    store.update_recurring_payment(request)?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/v1/recurring/payment/{recurring_payment_id}",
    params(
        (
            "recurring_payment_id" = String,
            Path,
            description = "Identifier of the recurring payment to delete"
        )
    ),
    tag = "Recurring",
    responses((status = 204))
)]
pub async fn delete_recurring_payment(
    Path(recurring_payment_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.write().await;
    store.delete_recurring_payment(&recurring_payment_id)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/v1/recurring/payments/today",
    tag = "Recurring",
    responses((status = 200, body = [RecurringPayment]))
)]
pub async fn recurring_payments_today(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecurringPayment>>, ApiError> {
    let store = state.store.read().await;
    Ok(Json(store.recurring_due_today()))
}

#[utoipa::path(
    put,
    path = "/v1/recurring/payment/{recurring_payment_id}/last-paid-date",
    params(
        (
            "recurring_payment_id" = String,
            Path,
            description = "Identifier of the recurring payment to update"
        )
    ),
    request_body = UpdateLastPaidDateRequest,
    tag = "Recurring",
    responses((status = 200))
)]
pub async fn update_last_paid_date(
    Path(recurring_payment_id): Path<String>,
    State(state): State<AppState>,
    Json(mut request): Json<UpdateLastPaidDateRequest>,
) -> Result<StatusCode, ApiError> {
    request.recurring_payment_id = recurring_payment_id;

    let mut store = state.store.write().await;
    store.update_last_paid_date(request)?;
    Ok(StatusCode::OK)
}
