// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{Datelike, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    auth::Auth,
    error::ApiError,
    models::{
        CreateRecurringPaymentRequest, RecurringPayment, UpdateLastPaidDateRequest,
        UpdateRecurringPaymentRequest, WalletAddress,
    },
    state::AppState,
    storage::repository::recurring::{PaymentFrequency, PaymentStatus, RecurringRepository, StoredRecurringPayment},
};

#[derive(Deserialize, IntoParams)]
pub struct RecurringQuery {
    pub wallet_id: WalletAddress,
}

/// Convert a stored payment to the API response model.
fn to_api_model(stored: &StoredRecurringPayment) -> RecurringPayment {
    RecurringPayment {
        id: stored.id.clone(),
        wallet_id: WalletAddress::from(stored.wallet_id.clone()),
        wallet_public_key: stored.wallet_public_key.clone(),
        recipient: WalletAddress::from(stored.recipient.clone()),
        amount: stored.amount,
        currency_code: stored.currency_code.clone(),
        payment_start_date: stored.payment_start_date,
        frequency: stored.frequency.into(),
        payment_end_date: stored.payment_end_date,
        last_paid_date: stored.last_paid_date,
    }
}

fn validate_date_range(
    payment_start_date: i32,
    payment_end_date: i32,
    frequency: i32,
) -> Result<(), ApiError> {
    if payment_start_date <= 0 || payment_end_date <= 0 {
        return Err(ApiError::bad_request(
            "payment_start_date and payment_end_date must be positive ordinal dates",
        ));
    }

    if frequency <= 0 {
        return Err(ApiError::bad_request(
            "frequency must be a positive number of days",
        ));
    }

    if payment_start_date > payment_end_date {
        return Err(ApiError::bad_request(
            "payment_start_date must be on or before payment_end_date",
        ));
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/recurring/payments",
    params(RecurringQuery),
    tag = "Recurring",
    responses((status = 200, body = [RecurringPayment]))
)]
pub async fn list_recurring_payments(
    Auth(user): Auth,
    State(state): State<AppState>,
    Query(params): Query<RecurringQuery>,
) -> Result<Json<Vec<RecurringPayment>>, ApiError> {
    let repo = RecurringRepository::new(&state.storage);
    let payments = repo.list_by_owner(&user.user_id).map_err(|e| {
        ApiError::internal(format!("Failed to list payments: {e}"))
    })?;

    // Filter by wallet_id if specified
    let filtered: Vec<RecurringPayment> = payments
        .iter()
        .filter(|p| p.wallet_id == params.wallet_id.0)
        .map(to_api_model)
        .collect();

    Ok(Json(filtered))
}

#[utoipa::path(
    post,
    path = "/v1/recurring/payment",
    request_body = CreateRecurringPaymentRequest,
    tag = "Recurring",
    responses((status = 201))
)]
pub async fn create_recurring_payment(
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<CreateRecurringPaymentRequest>,
) -> Result<StatusCode, ApiError> {
    validate_date_range(
        request.payment_start_date,
        request.payment_end_date,
        request.frequency,
    )?;

    let now = Utc::now();
    let payment = StoredRecurringPayment {
        id: uuid::Uuid::new_v4().to_string(),
        wallet_id: request.wallet_id.0.clone(),
        owner_user_id: user.user_id.clone(),
        wallet_public_key: request.wallet_public_key,
        recipient: request.recipient.0,
        amount: request.amount,
        currency_code: request.currency_code,
        frequency: PaymentFrequency::from(request.frequency),
        payment_start_date: request.payment_start_date,
        payment_end_date: request.payment_end_date,
        last_paid_date: -1,
        status: PaymentStatus::Active,
        created_at: now,
        updated_at: now,
    };

    let repo = RecurringRepository::new(&state.storage);
    repo.create(&payment).map_err(|e| {
        ApiError::internal(format!("Failed to create payment: {e}"))
    })?;

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
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<UpdateRecurringPaymentRequest>,
) -> Result<StatusCode, ApiError> {
    validate_date_range(
        request.payment_start_date,
        request.payment_end_date,
        request.frequency,
    )?;

    let repo = RecurringRepository::new(&state.storage);
    
    // Verify ownership
    let mut payment = repo.verify_ownership(&recurring_payment_id, &user.user_id).map_err(|_| {
        ApiError::not_found("Recurring payment not found")
    })?;

    payment.wallet_id = request.wallet_id.0;
    payment.wallet_public_key = request.wallet_public_key;
    payment.recipient = request.recipient.0;
    payment.amount = request.amount;
    payment.currency_code = request.currency_code;
    payment.payment_start_date = request.payment_start_date;
    payment.frequency = PaymentFrequency::from(request.frequency);
    payment.payment_end_date = request.payment_end_date;
    payment.updated_at = Utc::now();

    repo.update(&payment).map_err(|e| {
        ApiError::internal(format!("Failed to update payment: {e}"))
    })?;

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
    Auth(user): Auth,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let repo = RecurringRepository::new(&state.storage);
    
    // Verify ownership before deleting
    repo.verify_ownership(&recurring_payment_id, &user.user_id).map_err(|_| {
        ApiError::not_found("Recurring payment not found")
    })?;

    repo.delete(&recurring_payment_id).map_err(|e| {
        ApiError::internal(format!("Failed to delete payment: {e}"))
    })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/v1/recurring/payments/today",
    tag = "Recurring",
    responses((status = 200, body = [RecurringPayment]))
)]
pub async fn recurring_payments_today(
    Auth(user): Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<RecurringPayment>>, ApiError> {
    let today = Utc::now().date_naive().num_days_from_ce();
    let repo = RecurringRepository::new(&state.storage);
    
    // Get all payments due today for this user
    let payments = repo.list_by_owner(&user.user_id).map_err(|e| {
        ApiError::internal(format!("Failed to list payments: {e}"))
    })?;

    let due: Vec<RecurringPayment> = payments
        .iter()
        .filter(|p| {
            p.status == PaymentStatus::Active
                && p.payment_start_date <= today
                && today <= p.payment_end_date
                && (p.last_paid_date == -1
                    || today - p.last_paid_date >= i32::from(p.frequency))
        })
        .map(to_api_model)
        .collect();

    Ok(Json(due))
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
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<UpdateLastPaidDateRequest>,
) -> Result<StatusCode, ApiError> {
    if request.last_paid_date <= 0 {
        return Err(ApiError::bad_request(
            "last_paid_date must be a positive ordinal date",
        ));
    }

    let repo = RecurringRepository::new(&state.storage);
    
    // Verify ownership
    repo.verify_ownership(&recurring_payment_id, &user.user_id).map_err(|_| {
        ApiError::not_found("Recurring payment not found")
    })?;

    repo.update_last_paid_date(&recurring_payment_id, request.last_paid_date).map_err(|e| {
        ApiError::internal(format!("Failed to update last paid date: {e}"))
    })?;

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
    use crate::auth::AuthenticatedUser;

    fn today() -> i32 {
        Utc::now().date_naive().num_days_from_ce()
    }

    fn mock_auth(user_id: &str) -> Auth {
        Auth(AuthenticatedUser {
            user_id: user_id.to_string(),
            role: crate::auth::Role::Client,
            session_id: None,
            issuer: "https://test.clerk.dev".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        })
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
        let user_id = "user-test-1";

        let status = create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id.clone())),
        )
        .await
        .expect("create recurring payment succeeds");

        assert_eq!(status, StatusCode::CREATED);
        
        // Verify it was stored
        let repo = RecurringRepository::new(&state.storage);
        let payments = repo.list_by_owner(user_id).expect("list payments");
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].wallet_id, wallet_id.0);
        assert_eq!(payments[0].amount, 10.0);
    }

    #[tokio::test]
    async fn list_recurring_payments_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let other_wallet = WalletAddress::from("wallet_b");
        let user_id = "user-test-2";

        // Create payments for the same user
        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id.clone())),
        )
        .await
        .expect("create first");

        let mut second_request = sample_create_request(wallet_id.clone());
        second_request.amount = 15.5;
        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(second_request),
        )
        .await
        .expect("create second");

        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(other_wallet)),
        )
        .await
        .expect("create other wallet");

        let Json(payments) = list_recurring_payments(
            mock_auth(user_id),
            State(state.clone()),
            Query(RecurringQuery {
                wallet_id: wallet_id.clone(),
            }),
        )
        .await
        .expect("list recurring succeeds");

        assert_eq!(payments.len(), 2);
    }

    #[tokio::test]
    async fn update_recurring_payment_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-3";

        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id.clone())),
        )
        .await
        .expect("create payment");

        // Get the created payment's ID
        let repo = RecurringRepository::new(&state.storage);
        let payments = repo.list_by_owner(user_id).expect("list payments");
        let payment_id = payments[0].id.clone();

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
            Path(payment_id.clone()),
            mock_auth(user_id),
            State(state.clone()),
            Json(update_request.clone()),
        )
        .await
        .expect("update recurring succeeds");

        assert_eq!(status, StatusCode::OK);

        let updated = repo.get(&payment_id).expect("get payment");
        assert_eq!(updated.wallet_public_key, update_request.wallet_public_key);
        assert_eq!(updated.recipient, update_request.recipient.0);
        assert_eq!(updated.amount, update_request.amount);
    }

    #[tokio::test]
    async fn delete_recurring_payment_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-4";

        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id.clone())),
        )
        .await
        .expect("create payment");

        let repo = RecurringRepository::new(&state.storage);
        let payments = repo.list_by_owner(user_id).expect("list payments");
        let payment_id = payments[0].id.clone();

        let status = delete_recurring_payment(Path(payment_id), mock_auth(user_id), State(state.clone()))
            .await
            .expect("delete recurring succeeds");

        assert_eq!(status, StatusCode::NO_CONTENT);
        
        let remaining = repo.list_by_owner(user_id).expect("list payments");
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn recurring_payments_today_filters_correctly() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-5";
        let today = today();

        // Create a payment that is due today
        let mut req1 = sample_create_request(wallet_id.clone());
        req1.payment_start_date = today - 2;
        req1.payment_end_date = today + 2;
        create_recurring_payment(mock_auth(user_id), State(state.clone()), Json(req1))
            .await
            .expect("due one");

        // Create a payment that starts in the future
        let mut future_req = sample_create_request(wallet_id.clone());
        future_req.payment_start_date = today + 1;
        future_req.payment_end_date = today + 10;
        create_recurring_payment(mock_auth(user_id), State(state.clone()), Json(future_req))
            .await
            .expect("future payment");

        let Json(due) = recurring_payments_today(mock_auth(user_id), State(state.clone()))
            .await
            .expect("fetch due today succeeds");

        // Only the first payment should be due
        assert_eq!(due.len(), 1);
    }

    #[tokio::test]
    async fn update_last_paid_date_success() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-6";

        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id)),
        )
        .await
        .expect("create payment");

        let repo = RecurringRepository::new(&state.storage);
        let payments = repo.list_by_owner(user_id).expect("list payments");
        let payment_id = payments[0].id.clone();

        let new_date = today();
        let status = update_last_paid_date(
            Path(payment_id.clone()),
            mock_auth(user_id),
            State(state.clone()),
            Json(UpdateLastPaidDateRequest {
                recurring_payment_id: String::new(),
                last_paid_date: new_date,
            }),
        )
        .await
        .expect("update last paid succeeds");

        assert_eq!(status, StatusCode::OK);

        let updated = repo.get(&payment_id).expect("get payment");
        assert_eq!(updated.last_paid_date, new_date);
    }

    #[tokio::test]
    async fn update_last_paid_date_rejects_non_positive() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-7";

        create_recurring_payment(
            mock_auth(user_id),
            State(state.clone()),
            Json(sample_create_request(wallet_id)),
        )
        .await
        .expect("create payment");

        let repo = RecurringRepository::new(&state.storage);
        let payments = repo.list_by_owner(user_id).expect("list payments");
        let payment_id = payments[0].id.clone();

        let result = update_last_paid_date(
            Path(payment_id),
            mock_auth(user_id),
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

    #[tokio::test]
    async fn validation_rejects_invalid_dates() {
        let state = AppState::default();
        let wallet_id = WalletAddress::from("wallet_a");
        let user_id = "user-test-8";

        // Test start date = 0
        let mut req = sample_create_request(wallet_id.clone());
        req.payment_start_date = 0;
        let result = create_recurring_payment(mock_auth(user_id), State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(e) if e.status == StatusCode::BAD_REQUEST));

        // Test frequency = 0
        let mut req = sample_create_request(wallet_id.clone());
        req.frequency = 0;
        let result = create_recurring_payment(mock_auth(user_id), State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(e) if e.status == StatusCode::BAD_REQUEST));

        // Test start > end
        let mut req = sample_create_request(wallet_id);
        req.payment_start_date = 30;
        req.payment_end_date = 20;
        let result = create_recurring_payment(mock_auth(user_id), State(state.clone()), Json(req)).await;
        assert!(matches!(result, Err(e) if e.status == StatusCode::BAD_REQUEST));
    }
}
