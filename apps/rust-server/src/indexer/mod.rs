// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Event Indexer
//!
//! Background task that indexes ERC-20 Transfer events from the Avalanche
//! C-Chain into the embedded redb transaction database.
//!
//! ## Strategy
//!
//! 1. **ERC-20 Transfers**: Uses `eth_getLogs` with the Transfer(address,address,uint256)
//!    event topic, filtered to known token contract addresses.
//! 2. **Native AVAX Transfers**: Outgoing native sends are recorded at `send_transaction`
//!    time. Incoming native discovery runs on a slower poll by checking balance diffs.
//!
//! ## Checkpointing
//!
//! The indexer persists the last processed block in redb (`INDEXER_STATE` table).
//! On restart, it resumes from the checkpoint, avoiding full rescans.

use std::sync::Arc;
use std::time::Duration;

use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use tokio_util::sync::CancellationToken;

use crate::blockchain::{format_amount, NetworkConfig, AVAX_FUJI, REUR_TOKEN, USDC_TOKEN};
use crate::storage::repository::transactions::{StoredTransaction, TokenType, TxStatus};
use crate::storage::tx_cache::TxCache;
use crate::storage::tx_database::TxDatabase;

/// keccak256("Transfer(address,address,uint256)")
const TRANSFER_TOPIC: FixedBytes<32> = FixedBytes::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);

/// Default block chunk size per `eth_getLogs` query.
const DEFAULT_CHUNK_SIZE: u64 = 2000;

/// Default poll interval when caught up to chain head.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// How far back to look when starting fresh (no checkpoint).
const INITIAL_LOOKBACK_BLOCKS: u64 = 10_000;

/// ERC-20 event indexer that runs as a background tokio task.
pub struct EventIndexer {
    db: Arc<TxDatabase>,
    cache: Arc<TxCache>,
    network: NetworkConfig,
    poll_interval: Duration,
    chunk_size: u64,
    token_contracts: Vec<Address>,
}

impl EventIndexer {
    /// Create a new indexer for the given network and token contracts.
    pub fn new(
        db: Arc<TxDatabase>,
        cache: Arc<TxCache>,
        network: NetworkConfig,
        token_contracts: Vec<Address>,
    ) -> Self {
        Self {
            db,
            cache,
            network,
            poll_interval: DEFAULT_POLL_INTERVAL,
            chunk_size: DEFAULT_CHUNK_SIZE,
            token_contracts,
        }
    }

    /// Run the indexer loop until the cancellation token is triggered.
    ///
    /// This should be spawned as a background task:
    /// ```rust,ignore
    /// tokio::spawn(indexer.run(shutdown.clone()));
    /// ```
    pub async fn run(self, shutdown: CancellationToken) {
        tracing::info!(
            network = %self.network.name,
            contracts = self.token_contracts.len(),
            "Event indexer starting"
        );

        // Build alloy HTTP provider
        let provider = match ProviderBuilder::new()
            .connect_http(self.network.rpc_url.parse().expect("valid RPC URL"))
        {
            provider => provider,
        };

        loop {
            if shutdown.is_cancelled() {
                tracing::info!("Event indexer shutting down");
                return;
            }

            if let Err(e) = self.index_step(&provider).await {
                tracing::warn!(error = %e, "Indexer step failed, will retry");
            }

            tokio::select! {
                _ = tokio::time::sleep(self.poll_interval) => {},
                _ = shutdown.cancelled() => {
                    tracing::info!("Event indexer shutting down");
                    return;
                }
            }
        }
    }

    /// Execute one indexing step: fetch logs from checkpoint to head.
    async fn index_step<P: Provider + Clone>(&self, provider: &P) -> Result<(), IndexerError> {
        let network_key = self.network.name.to_lowercase().replace(' ', "_");
        let checkpoint = self.db.get_last_indexed_block(&network_key)?;

        let head = provider
            .get_block_number()
            .await
            .map_err(|e| IndexerError::Rpc(e.to_string()))?;

        // Determine start block
        let start = if checkpoint == 0 {
            head.saturating_sub(INITIAL_LOOKBACK_BLOCKS)
        } else {
            checkpoint + 1
        };

        if start > head {
            // Already caught up
            return Ok(());
        }

        // Process in chunks
        let mut from = start;
        while from <= head {
            if self.token_contracts.is_empty() {
                // No contracts to index, just update checkpoint
                break;
            }

            let to = (from + self.chunk_size - 1).min(head);

            let indexed = self.fetch_and_store_logs(provider, from, to).await?;
            if indexed > 0 {
                tracing::debug!(
                    from_block = from,
                    to_block = to,
                    events = indexed,
                    "Indexed ERC-20 transfer events"
                );
            }

            self.db.set_last_indexed_block(&network_key, to)?;
            from = to + 1;
        }

        // If no contracts, still update checkpoint to head
        if self.token_contracts.is_empty() {
            self.db.set_last_indexed_block(&network_key, head)?;
        }

        Ok(())
    }

    /// Fetch logs for a block range and store matching transfers.
    async fn fetch_and_store_logs<P: Provider + Clone>(
        &self,
        provider: &P,
        from_block: u64,
        to_block: u64,
    ) -> Result<usize, IndexerError> {
        // Build filter: Transfer events from our watched contracts
        let addresses: Vec<Address> = self.token_contracts.clone();

        let filter = Filter::new()
            .address(addresses)
            .event_signature(TRANSFER_TOPIC)
            .from_block(from_block)
            .to_block(to_block);

        let logs = provider
            .get_logs(&filter)
            .await
            .map_err(|e| IndexerError::Rpc(e.to_string()))?;

        let mut count = 0;

        for log in &logs {
            // Transfer event has 3 topics: [event_sig, from, to] and data = value
            if log.topics().len() < 3 {
                continue;
            }

            let from_topic = log.topics()[1];
            let to_topic = log.topics()[2];

            // Extract addresses from topics (last 20 bytes of 32-byte topic)
            let from_addr = format!("0x{}", alloy::hex::encode(&from_topic[12..]));
            let to_addr = format!("0x{}", alloy::hex::encode(&to_topic[12..]));

            // Decode value from log data
            let value = if log.data().data.len() >= 32 {
                U256::from_be_slice(&log.data().data[..32])
            } else {
                U256::ZERO
            };

            let contract_addr = log.address().to_string().to_lowercase();

            let tx_hash = log
                .transaction_hash
                .map(|h| format!("{h:#x}"))
                .unwrap_or_default();

            let block_number = log.block_number;

            if tx_hash.is_empty() {
                continue;
            }

            // Check if from or to is a registered wallet address
            let from_wallet = self.db.get_wallet_id_for_address(&from_addr)?;
            let to_wallet = self.db.get_wallet_id_for_address(&to_addr)?;

            if from_wallet.is_none() && to_wallet.is_none() {
                // Neither side belongs to us
                continue;
            }

            // Determine token metadata
            let (_symbol, decimals) = self.identify_token(&contract_addr);
            let amount_formatted = format_amount(value, decimals);

            let explorer_url = format!("{}/tx/{}", self.network.explorer_url, tx_hash);

            // Determine the wallet_id this record belongs to and the direction(s)
            let mut directions: Vec<(String, &str)> = Vec::new();

            if from_wallet.is_some() {
                directions.push((from_addr.clone(), "sent"));
            }
            if to_wallet.is_some() {
                directions.push((to_addr.clone(), "received"));
            }

            // Check if this tx already exists (avoid duplicates on re-index)
            if self.db.get_transaction(&tx_hash)?.is_some() {
                continue;
            }

            // Build StoredTransaction
            // Use the first matching wallet_id for the record
            let primary_wallet_id = from_wallet
                .as_deref()
                .or(to_wallet.as_deref())
                .unwrap_or("unknown")
                .to_string();

            let counterparty_wallet_id = if from_wallet.is_some() && to_wallet.is_some() {
                to_wallet.clone()
            } else {
                None
            };

            let mut stored_tx = StoredTransaction::new_pending(
                tx_hash.clone(),
                primary_wallet_id,
                counterparty_wallet_id,
                from_addr.clone(),
                to_addr.clone(),
                amount_formatted,
                TokenType::Erc20(contract_addr.clone()),
                self.network_name_short(),
                explorer_url,
            );

            // Mark as confirmed since we're reading from finalized logs
            stored_tx.status = TxStatus::Confirmed;
            stored_tx.block_number = block_number;

            if let Err(e) = self.db.upsert_transaction(&stored_tx, &directions) {
                tracing::warn!(
                    tx_hash = %tx_hash,
                    error = %e,
                    "Failed to store indexed transaction"
                );
                continue;
            }

            // Invalidate cache for affected wallets
            for (addr, _) in &directions {
                self.cache.invalidate(addr);
            }

            count += 1;
        }

        Ok(count)
    }

    /// Identify token symbol and decimals from contract address.
    fn identify_token(&self, contract_addr: &str) -> (&str, u8) {
        let addr_lower = contract_addr.to_lowercase();

        if USDC_TOKEN
            .fuji_address
            .map(|a| a.to_lowercase() == addr_lower)
            .unwrap_or(false)
            || USDC_TOKEN
                .mainnet_address
                .map(|a| a.to_lowercase() == addr_lower)
                .unwrap_or(false)
        {
            return (USDC_TOKEN.symbol, USDC_TOKEN.decimals);
        }

        if REUR_TOKEN
            .fuji_address
            .map(|a| a.to_lowercase() == addr_lower)
            .unwrap_or(false)
        {
            return (REUR_TOKEN.symbol, REUR_TOKEN.decimals);
        }

        // Unknown token — default to 18 decimals
        ("ERC20", 18)
    }

    /// Short network name for storage (e.g. "fuji").
    fn network_name_short(&self) -> String {
        if self.network.chain_id == AVAX_FUJI.chain_id {
            "fuji".to_string()
        } else {
            "mainnet".to_string()
        }
    }
}

/// Build the list of token contract addresses to monitor on Fuji.
pub fn fuji_token_contracts() -> Vec<Address> {
    let mut addrs = Vec::new();
    if let Some(addr_str) = USDC_TOKEN.fuji_address {
        if let Ok(addr) = addr_str.parse::<Address>() {
            addrs.push(addr);
        }
    }
    if let Some(addr_str) = REUR_TOKEN.fuji_address {
        if let Ok(addr) = addr_str.parse::<Address>() {
            addrs.push(addr);
        }
    }
    addrs
}

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Database error: {0}")]
    Db(#[from] crate::storage::tx_database::TxDbError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_topic_is_correct() {
        // keccak256("Transfer(address,address,uint256)")
        let expected = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        let actual = format!("0x{}", alloy::hex::encode(TRANSFER_TOPIC.as_slice()));
        assert_eq!(actual, expected);
    }

    #[test]
    fn fuji_token_contracts_parses() {
        let contracts = fuji_token_contracts();
        assert!(contracts.len() >= 1, "Should have at least USDC");
    }

    #[test]
    fn identify_token_usdc() {
        let db = Arc::new(
            TxDatabase::open(
                &std::env::temp_dir().join(format!("test-identify-{}.redb", uuid::Uuid::new_v4())),
            )
            .unwrap(),
        );
        let cache = Arc::new(TxCache::new(10, Duration::from_secs(60)));
        let indexer = EventIndexer::new(db, cache, AVAX_FUJI, fuji_token_contracts());

        let (symbol, decimals) =
            indexer.identify_token("0x5425890298aed601595a70AB815c96711a31Bc65");
        assert_eq!(symbol, "USDC");
        assert_eq!(decimals, 6);
    }

    #[test]
    fn identify_token_reur() {
        let db = Arc::new(
            TxDatabase::open(
                &std::env::temp_dir().join(format!("test-reur-{}.redb", uuid::Uuid::new_v4())),
            )
            .unwrap(),
        );
        let cache = Arc::new(TxCache::new(10, Duration::from_secs(60)));
        let indexer = EventIndexer::new(db, cache, AVAX_FUJI, fuji_token_contracts());

        let (symbol, decimals) =
            indexer.identify_token("0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63");
        assert_eq!(symbol, "rEUR");
        assert_eq!(decimals, 6);
    }
}
