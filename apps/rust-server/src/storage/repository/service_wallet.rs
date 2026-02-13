// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Repository for enclave-managed fiat reserve service wallet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::super::{EncryptedStorage, StorageError, StorageResult};

const SERVICE_WALLET_ID: &str = "fiat_service_wallet";

/// Persisted metadata for the enclave-managed fiat reserve service wallet.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FiatServiceWalletMetadata {
    /// Stable identifier for this service wallet record.
    pub wallet_id: String,
    /// Public EVM address controlled by enclave-held key.
    pub public_address: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Repository for fiat service-wallet lifecycle and key access.
pub struct FiatServiceWalletRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> FiatServiceWalletRepository<'a> {
    /// Create repository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Check if service-wallet metadata exists.
    pub fn exists(&self) -> bool {
        self.storage.exists(self.storage.paths().fiat_service_wallet_meta())
    }

    /// Load service-wallet metadata.
    pub fn get(&self) -> StorageResult<FiatServiceWalletMetadata> {
        let path = self.storage.paths().fiat_service_wallet_meta();
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(
                "Fiat service wallet metadata".to_string(),
            ));
        }
        self.storage.read_json(path)
    }

    /// Create service wallet if missing, otherwise return existing record.
    ///
    /// Key material is generated inside the enclave and stored in encrypted `/data`.
    pub fn bootstrap(&self) -> StorageResult<FiatServiceWalletMetadata> {
        if self.exists() {
            return self.get();
        }

        let (private_key_pem, public_address) = generate_secp256k1_keypair()
            .map_err(|e| StorageError::SerializationError(format!("key generation failed: {e}")))?;

        let now = Utc::now();
        let metadata = FiatServiceWalletMetadata {
            wallet_id: SERVICE_WALLET_ID.to_string(),
            public_address,
            created_at: now,
            updated_at: now,
        };

        self.storage
            .create_dir(self.storage.paths().fiat_service_wallet_dir())?;
        self.storage
            .write_json(self.storage.paths().fiat_service_wallet_meta(), &metadata)?;
        self.storage.write_raw(
            self.storage.paths().fiat_service_wallet_key(),
            private_key_pem.as_bytes(),
        )?;

        Ok(metadata)
    }

    /// Read service-wallet private key bytes (PEM).
    pub fn read_private_key(&self) -> StorageResult<Vec<u8>> {
        let path = self.storage.paths().fiat_service_wallet_key();
        if !self.storage.exists(&path) {
            return Err(StorageError::NotFound(
                "Fiat service wallet private key".to_string(),
            ));
        }
        self.storage.read_raw(path)
    }
}

/// Generate secp256k1 keypair and derive EVM address.
fn generate_secp256k1_keypair() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>>
{
    use alloy::primitives::keccak256;
    use k256::ecdsa::SigningKey;
    use k256::elliptic_curve::rand_core::OsRng;
    use k256::pkcs8::EncodePrivateKey;

    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let private_key_pem = signing_key
        .to_pkcs8_pem(k256::pkcs8::LineEnding::LF)
        .map_err(|e| format!("failed to encode private key: {e}"))?;

    let public_key_uncompressed = verifying_key.to_encoded_point(false);
    let public_key_bytes = public_key_uncompressed.as_bytes();
    let hash = keccak256(&public_key_bytes[1..]);
    let address_bytes = &hash[12..];
    let public_address = format!("0x{}", alloy::hex::encode(address_bytes));

    Ok((private_key_pem.to_string(), public_address))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use std::env;
    use std::fs;

    fn test_storage() -> EncryptedStorage {
        let test_dir = env::temp_dir().join(format!(
            "test-service-wallet-repo-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = StoragePaths::new(&test_dir);
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("initialize test storage");
        storage
    }

    fn cleanup(storage: &EncryptedStorage) {
        let _ = fs::remove_dir_all(storage.paths().root());
    }

    #[test]
    fn bootstrap_is_idempotent() {
        let storage = test_storage();
        let repo = FiatServiceWalletRepository::new(&storage);

        let one = repo.bootstrap().expect("first bootstrap");
        let two = repo.bootstrap().expect("second bootstrap");

        assert_eq!(one.wallet_id, SERVICE_WALLET_ID);
        assert_eq!(one.public_address, two.public_address);

        cleanup(&storage);
    }

    #[test]
    fn bootstrap_writes_readable_private_key() {
        let storage = test_storage();
        let repo = FiatServiceWalletRepository::new(&storage);

        let _ = repo.bootstrap().expect("bootstrap");
        let key = repo.read_private_key().expect("read key");
        let pem = String::from_utf8(key).expect("utf8");
        assert!(pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(pem.contains("-----END PRIVATE KEY-----"));

        cleanup(&storage);
    }
}

