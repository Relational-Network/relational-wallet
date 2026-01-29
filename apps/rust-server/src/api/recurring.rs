// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

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
    let store = state.legacy_store.read().await;
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
    let mut store = state.legacy_store.write().await;
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

    let mut store = state.legacy_store.write().await;
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
    let mut store = state.legacy_store.write().await;
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
    let store = state.legacy_store.read().await;
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

    let mut store = state.legacy_store.write().await;
    store.update_last_paid_date(request)?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        Json,
    };
    use chrono::{Datelike, Utc};

    fn today() -> i32 {
        Utc::now().date_naive().num_days_from_ce()
    }

    fn sample_create_request(wallet_id: WalletAddress) -> CreateRecurringPaymentRequest {
        let today = today();
        CreateRecurringPaymentRequest {
            wallet_id,
            wallet_public_key: "pk1".into(),
            recipient: WalletAddress::from("recipient"),
            amount: 10.0,
            currency_code: "USD".into(),
            payment_start_date: today,
            frequency: 3,
            payment_end_date: today + 30,
        }
    }

    #[tokio::test]
    async fn create_recurring_payment_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");

        let status = create_recurring_payment(
            State(state.clone()),
            Json(sample_create_request(wallet_id.clone())),
        )
        .await
        .expect("create recurring payment succeeds");

        assert_eq!(status, StatusCode::CREATED);
        let stored = state.legacy_store.read().await.list_recurring(&wallet_id);
        assert_eq!(stored.len(), 1);
        let payment = &stored[0];
        assert_eq!(payment.wallet_id, wallet_id);
        assert_eq!(payment.amount, 10.0);
        assert_eq!(payment.currency_code, "USD");
        assert_eq!(payment.last_paid_date, -1);
    }

    #[tokio::test]
    async fn list_recurring_payments_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let other_wallet = WalletAddress::from("wallet_b");

        let mut expected = {
            let mut store = state.legacy_store.write().await;
            let first = store
                .create_recurring_payment(sample_create_request(wallet_id.clone()))
                .expect("create first");
            let mut second_request = sample_create_request(wallet_id.clone());
            second_request.amount = 15.5;
            let second = store
                .create_recurring_payment(second_request)
                .expect("create second");
            store
                .create_recurring_payment(sample_create_request(other_wallet))
                .expect("create other wallet");
            vec![first, second]
        };

        let Json(mut payments) = list_recurring_payments(
            State(state.clone()),
            Query(RecurringQuery {
                wallet_id: wallet_id.clone(),
            }),
        )
        .await
        .expect("list recurring succeeds");

        expected.sort_by(|a, b| a.id.cmp(&b.id));
        payments.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(payments, expected);
    }

    #[tokio::test]
    async fn update_recurring_payment_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");

        let payment = {
            let mut store = state.legacy_store.write().await;
            store
                .create_recurring_payment(sample_create_request(wallet_id.clone()))
                .expect("create payment")
        };

        let update_request = UpdateRecurringPaymentRequest {
            recurring_payment_id: String::new(), // overwritten in handler
            wallet_id: wallet_id.clone(),
            wallet_public_key: "pk2".into(),
            recipient: WalletAddress::from("new_recipient"),
            amount: 25.0,
            currency_code: "EUR".into(),
            payment_start_date: today(),
            frequency: 5,
            payment_end_date: today() + 10,
        };

        let status = update_recurring_payment(
            Path(payment.id.clone()),
            State(state.clone()),
            Json(update_request.clone()),
        )
        .await
        .expect("update recurring succeeds");

        assert_eq!(status, StatusCode::OK);

        let updated = state
            .legacy_store
            .read()
            .await
            .list_recurring(&wallet_id)
            .into_iter()
            .find(|p| p.id == payment.id)
            .expect("payment present");

        assert_eq!(updated.wallet_public_key, update_request.wallet_public_key);
        assert_eq!(updated.recipient, update_request.recipient);
        assert_eq!(updated.amount, update_request.amount);
        assert_eq!(updated.currency_code, update_request.currency_code);
        assert_eq!(
            updated.payment_start_date,
            update_request.payment_start_date
        );
        assert_eq!(updated.payment_end_date, update_request.payment_end_date);
        assert_eq!(updated.frequency, update_request.frequency);
    }

    #[tokio::test]
    async fn delete_recurring_payment_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");

        let payment = {
            let mut store = state.legacy_store.write().await;
            store
                .create_recurring_payment(sample_create_request(wallet_id.clone()))
                .expect("create payment")
        };

        let status = delete_recurring_payment(Path(payment.id.clone()), State(state.clone()))
            .await
            .expect("delete recurring succeeds");

        assert_eq!(status, StatusCode::NO_CONTENT);
        let payments = state.legacy_store.read().await.list_recurring(&wallet_id);
        assert!(payments.is_empty());
    }

    #[tokio::test]
    async fn recurring_payments_today_filters_correctly() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let today = today();

        let (due_one, due_two, future_payment) = {
            let mut store = state.legacy_store.write().await;

            let mut req1 = sample_create_request(wallet_id.clone());
            req1.payment_start_date = today - 2;
            req1.payment_end_date = today + 2;
            let due_one = store.create_recurring_payment(req1).expect("due one");

            let mut req2 = sample_create_request(wallet_id.clone());
            req2.payment_start_date = today;
            req2.payment_end_date = today + 1;
            let due_two = store.create_recurring_payment(req2).expect("due two");

            let mut future_req = sample_create_request(wallet_id.clone());
            future_req.payment_start_date = today + 1;
            future_req.payment_end_date = today + 10;
            let future_payment = store
                .create_recurring_payment(future_req)
                .expect("future payment");

            (due_one, due_two, future_payment)
        };

        update_last_paid_date(
            Path(due_two.id.clone()),
            State(state.clone()),
            Json(UpdateLastPaidDateRequest {
                recurring_payment_id: String::new(),
                last_paid_date: today - 3,
            }),
        )
        .await
        .expect("update last paid for due payment succeeds");

        let mut due_two = due_two;
        due_two.last_paid_date = today - 3;

        // Mark an existing payment as recently paid to ensure it is filtered out.
        update_last_paid_date(
            Path(future_payment.id.clone()),
            State(state.clone()),
            Json(UpdateLastPaidDateRequest {
                recurring_payment_id: String::new(), // overwritten in handler
                last_paid_date: today,
            }),
        )
        .await
        .expect("update last paid succeeds");

        let Json(mut due) = recurring_payments_today(State(state.clone()))
            .await
            .expect("fetch due today succeeds");

        due.sort_by(|a, b| a.id.cmp(&b.id));
        let mut expected = vec![due_one, due_two];
        expected.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(due, expected);
    }

    #[tokio::test]
    async fn update_last_paid_date_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let payment = {
            let mut store = state.legacy_store.write().await;
            store
                .create_recurring_payment(sample_create_request(wallet_id.clone()))
                .expect("create payment")
        };

        let new_date = today();
        let status = update_last_paid_date(
            Path(payment.id.clone()),
            State(state.clone()),
            Json(UpdateLastPaidDateRequest {
                recurring_payment_id: String::new(), // overwritten in handler
                last_paid_date: new_date,
            }),
        )
        .await
        .expect("update last paid succeeds");

        assert_eq!(status, StatusCode::OK);

        let updated = state
            .legacy_store
            .read()
            .await
            .list_recurring(&wallet_id)
            .into_iter()
            .find(|p| p.id == payment.id)
            .expect("payment exists");

        assert_eq!(updated.last_paid_date, new_date);
    }

    #[tokio::test]
    async fn update_last_paid_date_rejects_non_positive() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let payment = {
            let mut store = state.legacy_store.write().await;
            store
                .create_recurring_payment(sample_create_request(wallet_id.clone()))
                .expect("create payment")
        };

        let result = update_last_paid_date(
            Path(payment.id.clone()),
            State(state.clone()),
            Json(UpdateLastPaidDateRequest {
                recurring_payment_id: String::new(),
                last_paid_date: 0,
            }),
        )
        .await;

        match result {
            Err(err) => assert_eq!(err.status, StatusCode::BAD_REQUEST),
            Ok(_) => panic!("expected validation error for non-positive last_paid_date"),
        }
    }
}
