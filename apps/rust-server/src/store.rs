// SPDX-License-Identifier: AGPL-3.0-or-later 
// 
// Copyright (C) 2025 Relational Network 
// 
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet) 

 
use std::collections::HashMap;

use chrono::{Datelike, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{
    AutofundRequest, Bookmark, CreateBookmarkRequest, CreateRecurringPaymentRequest, Invite,
    RecurringPayment, RedeemInviteRequest, UpdateLastPaidDateRequest,
    UpdateRecurringPaymentRequest, WalletAddress,
};

#[derive(Default)]
pub struct InMemoryStore {
    bookmarks: HashMap<String, Bookmark>,
    invites: HashMap<String, Invite>,
    recurring: HashMap<String, RecurringPayment>,
    pub autofund_log: Vec<AutofundRequest>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list_bookmarks(&self, wallet_id: &WalletAddress) -> Vec<Bookmark> {
        self.bookmarks
            .values()
            .filter(|bookmark| &bookmark.wallet_id == wallet_id)
            .cloned()
            .collect()
    }

    pub fn create_bookmark(&mut self, request: CreateBookmarkRequest) -> Bookmark {
        let id = Uuid::new_v4().to_string();
        let bookmark = Bookmark {
            id: id.clone(),
            wallet_id: request.wallet_id,
            name: request.name,
            address: request.address,
        };
        self.bookmarks.insert(id, bookmark.clone());
        bookmark
    }

    pub fn delete_bookmark(&mut self, bookmark_id: &str) -> Result<(), ApiError> {
        if self.bookmarks.remove(bookmark_id).is_some() {
            Ok(())
        } else {
            Err(ApiError::not_found("Bookmark not found"))
        }
    }

    pub fn invite_by_code(&self, invite_code: &str) -> Result<Invite, ApiError> {
        let invite = self
            .invites
            .values()
            .find(|invite| invite.code == invite_code)
            .cloned()
            .ok_or_else(|| ApiError::not_found("Invite not found"))?;

        if invite.redeemed {
            Err(ApiError::unprocessable(
                "This invite code has already been redeemed.",
            ))
        } else {
            Ok(invite)
        }
    }

    pub fn redeem_invite(&mut self, request: RedeemInviteRequest) -> Result<(), ApiError> {
        match self.invites.get_mut(&request.invite_id) {
            Some(invite) => {
                if invite.redeemed {
                    Err(ApiError::unprocessable(
                        "This invite code has already been redeemed.",
                    ))
                } else {
                    invite.redeemed = true;
                    Ok(())
                }
            }
            None => Err(ApiError::not_found("Invite not found")),
        }
    }

    pub fn insert_invite(&mut self, code: impl Into<String>, redeemed: bool) -> Invite {
        let id = Uuid::new_v4().to_string();
        let invite = Invite {
            id: id.clone(),
            code: code.into(),
            redeemed,
        };
        self.invites.insert(id, invite.clone());
        invite
    }

    pub fn list_recurring(&self, wallet_id: &WalletAddress) -> Vec<RecurringPayment> {
        self.recurring
            .values()
            .filter(|payment| &payment.wallet_id == wallet_id)
            .cloned()
            .collect()
    }

    pub fn create_recurring_payment(
        &mut self,
        request: CreateRecurringPaymentRequest,
    ) -> Result<RecurringPayment, ApiError> {
        validate_date_range(
            request.payment_start_date,
            request.payment_end_date,
            request.frequency,
        )?;

        let id = Uuid::new_v4().to_string();
        let payment = RecurringPayment {
            id: id.clone(),
            wallet_id: request.wallet_id,
            wallet_public_key: request.wallet_public_key,
            recipient: request.recipient,
            amount: request.amount,
            currency_code: request.currency_code,
            payment_start_date: request.payment_start_date,
            frequency: request.frequency,
            payment_end_date: request.payment_end_date,
            last_paid_date: -1,
        };
        self.recurring.insert(id, payment.clone());
        Ok(payment)
    }

    pub fn update_recurring_payment(
        &mut self,
        request: UpdateRecurringPaymentRequest,
    ) -> Result<(), ApiError> {
        validate_date_range(
            request.payment_start_date,
            request.payment_end_date,
            request.frequency,
        )?;

        let Some(payment) = self.recurring.get_mut(&request.recurring_payment_id) else {
            return Err(ApiError::not_found("Recurring payment not found"));
        };

        payment.wallet_id = request.wallet_id;
        payment.wallet_public_key = request.wallet_public_key;
        payment.recipient = request.recipient;
        payment.amount = request.amount;
        payment.currency_code = request.currency_code;
        payment.payment_start_date = request.payment_start_date;
        payment.frequency = request.frequency;
        payment.payment_end_date = request.payment_end_date;

        Ok(())
    }

    pub fn delete_recurring_payment(&mut self, payment_id: &str) -> Result<(), ApiError> {
        if self.recurring.remove(payment_id).is_some() {
            Ok(())
        } else {
            Err(ApiError::not_found("Recurring payment not found"))
        }
    }

    pub fn update_last_paid_date(&mut self, request: UpdateLastPaidDateRequest) -> Result<(), ApiError> {
        if request.last_paid_date <= 0 {
            return Err(ApiError::bad_request(
                "last_paid_date must be a positive ordinal date",
            ));
        }

        let Some(payment) = self.recurring.get_mut(&request.recurring_payment_id) else {
            return Err(ApiError::not_found("Recurring payment not found"));
        };

        payment.last_paid_date = request.last_paid_date;
        Ok(())
    }

    pub fn recurring_due_today(&self) -> Vec<RecurringPayment> {
        let today = Utc::now().date_naive().num_days_from_ce();

        self.recurring
            .values()
            .filter(|payment| {
                payment.payment_start_date <= today
                    && today <= payment.payment_end_date
                    && (payment.last_paid_date == -1
                        || today - payment.last_paid_date >= payment.frequency)
            })
            .cloned()
            .collect()
    }

    pub fn log_autofund(&mut self, request: AutofundRequest) {
        self.autofund_log.push(request);
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
