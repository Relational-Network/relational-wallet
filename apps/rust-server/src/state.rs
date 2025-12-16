// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2025 Relational Network
//
// Derived from Nautilus Wallet (https://github.com/ntls-io/nautilus-wallet)

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::store::InMemoryStore;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<InMemoryStore>>,
}

impl AppState {
    pub fn new(store: InMemoryStore) -> Self {
        Self {
            store: Arc::new(RwLock::new(store)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(InMemoryStore::new())
    }
}
