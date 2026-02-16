// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Fiat Request Poller
//!
//! Background task that periodically syncs pending fiat requests with the
//! TrueLayer provider API. This ensures payout/payment status transitions
//! happen server-side even when no user is actively polling from the frontend.
//!
//! ## Strategy
//!
//! Every `poll_interval` (default 30 s) the poller:
//! 1. Lists all non-terminal fiat requests (Queued, AwaitingProvider,
//!    AwaitingUserDeposit, SettlementPending, ProviderPending).
//! 2. For each request, calls the existing `sync_and_persist_request` which
//!    performs provider status polling, deposit detection, and settlement.
//! 3. Skips requests whose `last_provider_sync_at` is less than the minimum
//!    sync interval, avoiding duplicate work when the frontend is also polling.
//!
//! ## Shutdown
//!
//! Uses `tokio_util::sync::CancellationToken` for graceful shutdown, following
//! the same pattern as the `EventIndexer`.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::storage::EncryptedStorage;

/// Default interval between polling sweeps.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Background fiat request poller that syncs pending requests with TrueLayer.
pub struct FiatPoller {
    storage: Arc<EncryptedStorage>,
    poll_interval: Duration,
}

impl FiatPoller {
    /// Create a new poller for the given encrypted storage.
    pub fn new(storage: Arc<EncryptedStorage>) -> Self {
        Self {
            storage,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }

    /// Run the poller loop until the cancellation token is triggered.
    ///
    /// Should be spawned as a background task:
    /// ```rust,ignore
    /// tokio::spawn(poller.run(shutdown.clone()));
    /// ```
    pub async fn run(self, shutdown: CancellationToken) {
        info!(
            interval_secs = self.poll_interval.as_secs(),
            "Fiat request poller starting"
        );

        loop {
            if shutdown.is_cancelled() {
                info!("Fiat request poller shutting down");
                return;
            }

            self.poll_step().await;

            tokio::select! {
                _ = tokio::time::sleep(self.poll_interval) => {},
                _ = shutdown.cancelled() => {
                    info!("Fiat request poller shutting down");
                    return;
                }
            }
        }
    }

    /// Execute one polling sweep: find pending requests and sync each.
    async fn poll_step(&self) {
        let pending_ids = crate::api::fiat::list_pending_request_ids(&self.storage);

        if pending_ids.is_empty() {
            return;
        }

        info!(
            count = pending_ids.len(),
            "Fiat poller: syncing pending requests"
        );

        for request_id in &pending_ids {
            match crate::api::fiat::sync_and_persist_request(&self.storage, request_id).await {
                Ok(record) => {
                    info!(
                        request_id = %record.request_id,
                        status = ?record.status,
                        provider_reference = ?record.provider_reference,
                        "Fiat poller: synced request"
                    );
                }
                Err(e) => {
                    warn!(
                        request_id = %request_id,
                        error = %e.message,
                        "Fiat poller: failed to sync request"
                    );
                }
            }
        }
    }
}
