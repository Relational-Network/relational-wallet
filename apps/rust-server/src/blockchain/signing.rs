// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Transaction signing module for Avalanche C-Chain.
//!
//! This module handles conversion of stored PEM private keys to signers
//! and provides transaction signing capabilities within the SGX enclave.

use alloy::{
    network::EthereumWallet,
    signers::local::PrivateKeySigner,
};
use k256::SecretKey;

use super::client::AvaxClientError;

/// Parse a private key from PEM format to hex string.
///
/// The wallet stores keys in PKCS#8 PEM format. This function extracts
/// the raw key bytes and converts them to hex for use with alloy's signer.
///
/// # Arguments
/// * `pem_bytes` - The PEM-encoded private key bytes
///
/// # Returns
/// * `Ok(String)` - Hex-encoded private key (64 characters, no 0x prefix)
/// * `Err(AvaxClientError)` - If PEM parsing fails
pub fn pem_to_hex(pem_bytes: &[u8]) -> Result<String, AvaxClientError> {
    let pem_str = std::str::from_utf8(pem_bytes)
        .map_err(|e| AvaxClientError::InvalidPrivateKey(format!("Invalid UTF-8: {}", e)))?;

    // Parse the PEM to get the DER-encoded key
    let pem = pem::parse(pem_str)
        .map_err(|e| AvaxClientError::InvalidPrivateKey(format!("Invalid PEM: {}", e)))?;

    // Parse as PKCS#8 private key
    let secret_key = SecretKey::from_sec1_der(pem.contents())
        .or_else(|_| {
            // Try parsing as PKCS#8 if SEC1 fails
            parse_pkcs8_to_secret_key(pem.contents())
        })
        .map_err(|e| AvaxClientError::InvalidPrivateKey(format!("Invalid key format: {}", e)))?;

    // Convert to hex
    let key_bytes = secret_key.to_bytes();
    Ok(alloy::hex::encode(key_bytes))
}

/// Parse PKCS#8 DER to extract the secret key.
fn parse_pkcs8_to_secret_key(der: &[u8]) -> Result<SecretKey, String> {
    // PKCS#8 format wraps the key with algorithm identifiers
    // For secp256k1, the raw key is typically at offset 36 (after headers)
    // We use k256's built-in parsing
    use k256::pkcs8::DecodePrivateKey;
    SecretKey::from_pkcs8_der(der)
        .map_err(|e| e.to_string())
}

/// Create a signer from PEM-encoded private key.
///
/// # Arguments
/// * `pem_bytes` - The PEM-encoded private key bytes from wallet storage
///
/// # Returns
/// * `Ok(PrivateKeySigner)` - A signer ready to sign transactions
/// * `Err(AvaxClientError)` - If key parsing fails
pub fn signer_from_pem(pem_bytes: &[u8]) -> Result<PrivateKeySigner, AvaxClientError> {
    let hex_key = pem_to_hex(pem_bytes)?;
    super::client::AvaxClient::create_signer(&hex_key)
}

/// Create an Ethereum wallet from PEM-encoded private key.
///
/// This is a convenience function that combines `signer_from_pem` and
/// `AvaxClient::create_wallet`.
///
/// # Arguments
/// * `pem_bytes` - The PEM-encoded private key bytes from wallet storage
///
/// # Returns
/// * `Ok(EthereumWallet)` - A wallet ready for transaction signing
/// * `Err(AvaxClientError)` - If key parsing fails
pub fn wallet_from_pem(pem_bytes: &[u8]) -> Result<EthereumWallet, AvaxClientError> {
    let signer = signer_from_pem(pem_bytes)?;
    Ok(super::client::AvaxClient::create_wallet(signer))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test PEM key generated the same way as in wallet creation
    const TEST_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgxK7Fx7YPvb0O6HlNZjXL
8LYqkLOTqPjSvBmPf1RzGhehRANCAATMiVOx5kXz7Np1tKhQU0qkRbRww/oGxjzM
Q5rHgr5XmGlxwvwGRrr7XJO3YQRvJKy7wXPM8sS5BYw0JI0ZP6J4
-----END PRIVATE KEY-----"#;

    #[test]
    fn test_pem_to_hex() {
        let result = pem_to_hex(TEST_PEM.as_bytes());
        assert!(result.is_ok(), "Failed to parse PEM: {:?}", result.err());
        
        let hex = result.unwrap();
        assert_eq!(hex.len(), 64, "Hex key should be 64 characters");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "Should be valid hex");
    }

    #[test]
    fn test_signer_from_pem() {
        let result = signer_from_pem(TEST_PEM.as_bytes());
        assert!(result.is_ok(), "Failed to create signer: {:?}", result.err());
    }

    #[test]
    fn test_wallet_from_pem() {
        let result = wallet_from_pem(TEST_PEM.as_bytes());
        assert!(result.is_ok(), "Failed to create wallet: {:?}", result.err());
    }
}
