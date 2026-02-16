// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! LRU cache for transaction history first-page lookups.
//!
//! Caches the first page of transactions per wallet address to avoid
//! repeated redb reads for the most common query pattern.

use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;

use super::repository::transactions::StoredTransaction;

/// Cached entry: list of transactions + insertion timestamp.
struct CacheEntry {
    transactions: Vec<(StoredTransaction, String)>, // (tx, direction)
    inserted_at: Instant,
}

/// In-process LRU cache for hot wallet transaction lookups.
pub struct TxCache {
    cache: Mutex<LruCache<String, CacheEntry>>,
    ttl: Duration,
}

impl TxCache {
    /// Create a new cache with the given capacity and TTL.
    ///
    /// - `capacity`: Max number of wallet addresses to cache.
    /// - `ttl`: Time-to-live for each cache entry.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap()),
            )),
            ttl,
        }
    }

    /// Get the cached first page for a wallet address.
    ///
    /// Returns `None` if not cached or expired.
    pub fn get_first_page(&self, wallet_address: &str) -> Option<Vec<(StoredTransaction, String)>> {
        let key = wallet_address.to_lowercase();
        let mut cache = self.cache.lock().ok()?;
        if let Some(entry) = cache.get(&key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.transactions.clone());
            }
            // Expired — remove it
            cache.pop(&key);
        }
        None
    }

    /// Store the first page for a wallet address.
    pub fn put_first_page(&self, wallet_address: &str, txs: Vec<(StoredTransaction, String)>) {
        let key = wallet_address.to_lowercase();
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(
                key,
                CacheEntry {
                    transactions: txs,
                    inserted_at: Instant::now(),
                },
            );
        }
    }

    /// Invalidate the cache for a specific wallet address.
    pub fn invalidate(&self, wallet_address: &str) {
        let key = wallet_address.to_lowercase();
        if let Ok(mut cache) = self.cache.lock() {
            cache.pop(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::transactions::{StoredTransaction, TokenType};

    fn sample_tx() -> (StoredTransaction, String) {
        let tx = StoredTransaction::new_pending(
            "0xabc".to_string(),
            "wallet-1".to_string(),
            None,
            "0x1111111111111111111111111111111111111111".to_string(),
            "0x2222222222222222222222222222222222222222".to_string(),
            "5.0".to_string(),
            TokenType::Native,
            "fuji".to_string(),
            "https://testnet.snowtrace.io/tx/0xabc".to_string(),
        );
        (tx, "sent".to_string())
    }

    #[test]
    fn cache_put_and_get() {
        let cache = TxCache::new(10, Duration::from_secs(300));
        let addr = "0xABCD";
        let data = vec![sample_tx()];

        assert!(cache.get_first_page(addr).is_none());

        cache.put_first_page(addr, data.clone());

        let result = cache.get_first_page(addr).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.tx_hash, "0xabc");
    }

    #[test]
    fn cache_invalidate() {
        let cache = TxCache::new(10, Duration::from_secs(300));
        let addr = "0xABCD";
        cache.put_first_page(addr, vec![sample_tx()]);
        assert!(cache.get_first_page(addr).is_some());

        cache.invalidate(addr);
        assert!(cache.get_first_page(addr).is_none());
    }

    #[test]
    fn cache_ttl_expiry() {
        let cache = TxCache::new(10, Duration::from_millis(1));
        cache.put_first_page("0xABCD", vec![sample_tx()]);

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(5));

        assert!(cache.get_first_page("0xABCD").is_none());
    }

    #[test]
    fn cache_case_insensitive() {
        let cache = TxCache::new(10, Duration::from_secs(300));
        cache.put_first_page("0xABCD", vec![sample_tx()]);

        // Should find by lowercase
        assert!(cache.get_first_page("0xabcd").is_some());
    }
}
