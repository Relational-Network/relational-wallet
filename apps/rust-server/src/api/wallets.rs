// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Wallet management API endpoints.
//!
//! These endpoints handle wallet creation, listing, retrieval, and deletion.
//! All operations require authentication and enforce ownership.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    audit_log,
    auth::Auth,
    error::ApiError,
    providers::email,
    state::AppState,
    storage::{
        AuditEventType, AuditRepository, EmailIndexRepository, OwnershipEnforcer, WalletMetadata,
        WalletRepository, WalletResponse, WalletStatus,
    },
};

/// Request to create a new wallet.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWalletRequest {
    /// Optional human-readable label for the wallet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Response after creating a wallet.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWalletResponse {
    /// The created wallet details.
    pub wallet: WalletResponse,
    /// Message indicating success.
    pub message: String,
}

/// Response containing a list of wallets.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletListResponse {
    /// List of wallets owned by the user.
    pub wallets: Vec<WalletResponse>,
    /// Total count of wallets.
    pub total: usize,
}

/// Response after deleting a wallet.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteWalletResponse {
    /// Message indicating success.
    pub message: String,
    /// The ID of the deleted wallet.
    pub wallet_id: String,
}

/// Create a new wallet for the authenticated user.
///
/// Generates a new p256 keypair inside the SGX enclave and stores it
/// encrypted on disk. Returns the wallet metadata (never the private key).
#[utoipa::path(
    post,
    path = "/v1/wallets",
    tag = "Wallets",
    security(("bearer_auth" = [])),
    request_body = CreateWalletRequest,
    responses(
        (status = 201, description = "Wallet created successfully", body = CreateWalletResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_wallet(
    Auth(user): Auth,
    State(state): State<AppState>,
    Json(request): Json<CreateWalletRequest>,
) -> Result<(StatusCode, Json<CreateWalletResponse>), ApiError> {
    let storage = state.storage();
    let tx_db = state
        .tx_db
        .as_ref()
        .expect("transaction database must be configured");

    // ── O(1) 1-wallet-per-user check via redb ──
    if let Ok(Some(_)) = tx_db.get_user_wallet(&user.user_id) {
        return Err(ApiError::conflict("You already have a wallet"));
    }

    // ── Fetch email from Clerk and enforce 1-wallet-per-email ──
    let mut email_lookup_key: Option<String> = None;
    if let Some(ref clerk) = state.clerk_client {
        let normalized_email = clerk
            .get_user_email(&user.user_id)
            .await
            .map_err(|e| ApiError::internal(&format!("Failed to fetch email: {}", e)))?;

        let sha256_hex = email::sha256_email(&normalized_email);
        let lookup_key = email::hmac_lookup_key(&state.email_hmac_key, &sha256_hex);

        // O(1) email uniqueness check
        let email_repo = EmailIndexRepository::new(tx_db.clone());
        if email_repo.exists(&lookup_key).map_err(|e| ApiError::internal(&format!("Email lookup failed: {}", e)))? {
            return Err(ApiError::conflict("A wallet already exists for this email"));
        }

        email_lookup_key = Some(lookup_key);
    }

    // Generate wallet ID
    let wallet_id = uuid::Uuid::new_v4().to_string();

    // Generate secp256k1 keypair (Ethereum/Avalanche compatible)
    let (private_key_pem, public_address) = generate_secp256k1_keypair()
        .map_err(|e| ApiError::internal(&format!("Key generation failed: {}", e)))?;

    // Create wallet metadata
    let metadata = WalletMetadata {
        wallet_id: wallet_id.clone(),
        owner_user_id: user.user_id.clone(),
        public_address: public_address.clone(),
        created_at: Utc::now(),
        status: WalletStatus::Active,
        label: request.label,
        email_lookup_key: email_lookup_key.clone(),
    };

    // Store wallet
    let repo = WalletRepository::new(&storage);
    repo.create(&metadata, private_key_pem.as_bytes())
        .map_err(|e| ApiError::internal(&format!("Failed to store wallet: {}", e)))?;

    // Register address → wallet_id in redb for the event indexer.
    if let Err(e) = tx_db.register_address(&public_address, &wallet_id) {
        tracing::warn!(
            error = %e,
            wallet_id = %wallet_id,
            "Failed to register wallet address in tx database"
        );
    }

    // Register user → wallet mapping (O(1))
    if let Err(e) = tx_db.register_user_wallet(&user.user_id, &wallet_id) {
        tracing::warn!(
            error = %e,
            wallet_id = %wallet_id,
            "Failed to register user→wallet mapping"
        );
    }

    // Register email lookup index (O(1))
    if let Some(ref lk) = email_lookup_key {
        let email_repo = EmailIndexRepository::new(tx_db.clone());
        if let Err(e) = email_repo.register(lk, &wallet_id, &public_address) {
            tracing::warn!(
                error = %e,
                wallet_id = %wallet_id,
                "Failed to register email lookup"
            );
        }
    }

    // Audit log
    audit_log!(
        &storage,
        AuditEventType::WalletCreated,
        &user,
        "wallet",
        &wallet_id
    );

    let response = CreateWalletResponse {
        wallet: WalletResponse::from(metadata),
        message: "Wallet created successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// List all wallets owned by the authenticated user.
#[utoipa::path(
    get,
    path = "/v1/wallets",
    tag = "Wallets",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of wallets", body = WalletListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_wallets(
    Auth(user): Auth,
    State(state): State<AppState>,
) -> Result<Json<WalletListResponse>, ApiError> {
    let storage = state.storage();
    let tx_db = state.tx_db.as_ref();
    let repo = WalletRepository::new(&storage);

    // O(1) path: look up user's wallet from redb, then fetch metadata directly
    let wallet_responses: Vec<WalletResponse> = if let Some(db) = tx_db {
        if let Ok(Some(wallet_id)) = db.get_user_wallet(&user.user_id) {
            match repo.get(&wallet_id) {
                Ok(meta) if meta.status != WalletStatus::Deleted => vec![meta.into()],
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        }
    } else {
        // Fallback: O(N) filesystem scan (no tx_db configured)
        let wallets = repo
            .list_by_owner(&user.user_id)
            .map_err(|e| ApiError::internal(&format!("Failed to list wallets: {}", e)))?;
        wallets.into_iter().map(Into::into).collect()
    };

    let total = wallet_responses.len();

    Ok(Json(WalletListResponse {
        wallets: wallet_responses,
        total,
    }))
}

/// Get a specific wallet by ID.
///
/// Only returns wallets owned by the authenticated user.
#[utoipa::path(
    get,
    path = "/v1/wallets/{wallet_id}",
    tag = "Wallets",
    security(("bearer_auth" = [])),
    params(
        ("wallet_id" = String, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Wallet details", body = WalletResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your wallet"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn get_wallet(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
) -> Result<Json<WalletResponse>, ApiError> {
    let storage = state.storage();
    let repo = WalletRepository::new(&storage);

    let metadata = repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {wallet_id} not found")))?;

    // Verify ownership
    metadata
        .verify_ownership(&user)
        .map_err(|_| ApiError::forbidden("You don't have permission to access this wallet"))?;

    // Audit access
    let audit_repo = AuditRepository::new(&storage);
    let _ = audit_repo.log(
        &crate::storage::AuditEvent::new(AuditEventType::WalletAccessed)
            .with_user(&user.user_id)
            .with_resource("wallet", &wallet_id),
    );

    Ok(Json(WalletResponse::from(metadata)))
}

/// Delete (soft-delete) a wallet.
///
/// Marks the wallet as deleted. The private key is retained for potential
/// recovery but the wallet cannot be used for new transactions.
#[utoipa::path(
    delete,
    path = "/v1/wallets/{wallet_id}",
    tag = "Wallets",
    security(("bearer_auth" = [])),
    params(
        ("wallet_id" = String, Path, description = "Wallet ID to delete")
    ),
    responses(
        (status = 200, description = "Wallet deleted successfully", body = DeleteWalletResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not your wallet"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn delete_wallet(
    Auth(user): Auth,
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
) -> Result<Json<DeleteWalletResponse>, ApiError> {
    let storage = state.storage();
    let repo = WalletRepository::new(&storage);

    // Get and verify ownership
    let metadata = repo
        .get(&wallet_id)
        .map_err(|_| ApiError::not_found(&format!("Wallet {wallet_id} not found")))?;

    metadata
        .verify_ownership(&user)
        .map_err(|_| ApiError::forbidden("You don't have permission to delete this wallet"))?;

    // Soft delete
    repo.soft_delete(&wallet_id)
        .map_err(|e| ApiError::internal(&format!("Failed to delete wallet: {}", e)))?;

    // Clean up redb index entries so user can create a new wallet
    if let Some(db) = state.tx_db.as_ref() {
        // Remove user→wallet mapping
        let _ = db.remove_user_wallet(&user.user_id);

        // Remove email→wallet mapping if wallet had an email
        if let Some(ref lookup_key) = metadata.email_lookup_key {
            let email_repo = EmailIndexRepository::new(db.clone());
            let _ = email_repo.remove(lookup_key);
        }

        // Remove address→wallet mapping
        let _ = db.remove_wallet_address(&metadata.public_address);
    }

    // Audit log
    audit_log!(
        &storage,
        AuditEventType::WalletDeleted,
        &user,
        "wallet",
        &wallet_id
    );

    Ok(Json(DeleteWalletResponse {
        message: "Wallet deleted successfully".to_string(),
        wallet_id,
    }))
}

/// Generate a secp256k1 keypair and derive Ethereum/Avalanche address.
///
/// Ethereum addresses are derived by:
/// 1. Generate secp256k1 private key
/// 2. Get uncompressed public key (65 bytes: 0x04 || x || y)
/// 3. Take keccak256 hash of the public key (without 0x04 prefix, so 64 bytes)
/// 4. Take the last 20 bytes of the hash
/// 5. Encode as hex with 0x prefix (42 characters total)
///
/// # Returns
/// A tuple of (private_key_pem, public_address) where:
/// - `private_key_pem`: PKCS#8 PEM-encoded private key for encrypted storage
/// - `public_address`: Ethereum-format address (0x + 40 hex chars)
fn generate_secp256k1_keypair() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>>
{
    use alloy::primitives::keccak256;
    use k256::ecdsa::SigningKey;
    use k256::elliptic_curve::rand_core::OsRng;
    use k256::pkcs8::EncodePrivateKey;

    // Generate random signing key (secp256k1)
    let signing_key = SigningKey::random(&mut OsRng);

    // Get verifying (public) key
    let verifying_key = signing_key.verifying_key();

    // Encode private key to PEM (PKCS#8 format) for encrypted storage
    let private_key_pem = signing_key
        .to_pkcs8_pem(k256::pkcs8::LineEnding::LF)
        .map_err(|e| format!("Failed to encode private key: {}", e))?;

    // Get uncompressed public key bytes (65 bytes: 0x04 prefix + 64 bytes of x,y coordinates)
    let public_key_uncompressed = verifying_key.to_encoded_point(false);
    let public_key_bytes = public_key_uncompressed.as_bytes();

    // Hash the public key coordinates (skip 0x04 prefix) using alloy's keccak256
    let hash = keccak256(&public_key_bytes[1..]);

    // Take the last 20 bytes of the hash as the address
    let address_bytes = &hash[12..]; // hash is 32 bytes, take last 20

    // Format as Ethereum address using alloy's hex encoding
    let public_address = format!("0x{}", alloy::hex::encode(address_bytes));

    Ok((private_key_pem.to_string(), public_address))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_keypair_produces_valid_ethereum_address() {
        let (private_key_pem, public_address) = generate_secp256k1_keypair().unwrap();

        // Private key should be PEM formatted
        assert!(private_key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(private_key_pem.contains("-----END PRIVATE KEY-----"));

        // Public address should be valid Ethereum format:
        // 0x prefix + 40 hex characters = 42 total
        assert!(public_address.starts_with("0x"));
        assert_eq!(
            public_address.len(),
            42,
            "Ethereum address must be 42 characters"
        );

        // All characters after 0x should be valid hex
        let hex_part = &public_address[2..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "Address must be valid hex"
        );
    }

    #[test]
    fn generate_keypair_produces_unique_addresses() {
        // Generate multiple keys and verify they're unique
        let mut addresses = std::collections::HashSet::new();
        for _ in 0..10 {
            let (_, addr) = generate_secp256k1_keypair().unwrap();
            assert!(addresses.insert(addr), "Generated duplicate address");
        }
    }

    #[test]
    fn generate_keypair_format_consistency() {
        // Generate multiple keys and verify format consistency
        for _ in 0..5 {
            let (pem, addr) = generate_secp256k1_keypair().unwrap();
            assert!(pem.starts_with("-----BEGIN PRIVATE KEY-----"));
            assert!(addr.starts_with("0x"));
            assert_eq!(addr.len(), 42);
        }
    }

    #[test]
    fn wallet_response_conversion() {
        let metadata = WalletMetadata {
            wallet_id: "w1".to_string(),
            owner_user_id: "user1".to_string(),
            public_address: "0xabc".to_string(),
            created_at: Utc::now(),
            status: WalletStatus::Active,
            label: Some("My Wallet".to_string()),
            email_lookup_key: None,
        };

        let response: WalletResponse = metadata.into();
        assert_eq!(response.wallet_id, "w1");
        assert_eq!(response.public_address, "0xabc");
        assert_eq!(response.label, Some("My Wallet".to_string()));
    }
}
