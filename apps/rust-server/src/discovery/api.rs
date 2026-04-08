// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! Discovery API endpoints for the internal two-phase VOPRF protocol.
//!
//! These endpoints are served on the main router under `/internal/discovery/`
//! and are only accessible via RA-TLS mutual authentication (no JWT auth).
//!
//! ## Endpoints
//!
//! - `POST /internal/discovery/evaluate` — Phase A: VOPRF evaluation
//! - `POST /internal/discovery/lookup` — Phase B: Token-based lookup

use axum::{extract::State, Json};
use base64ct::{Base64, Encoding};
use ring::aead;
use ring::hkdf;

use crate::error::ApiError;
use crate::models::{DiscoveryEvaluateRequest, DiscoveryEvaluateResponse, DiscoveryLookupRequest, DiscoveryLookupResponse};
use crate::state::AppState;

use super::voprf_ops::Cipher;

/// Fixed envelope size (bytes) for discovery lookup responses.
///
/// Both match and no-match responses are exactly this size to prevent
/// traffic analysis on match/no-match.
const ENVELOPE_SIZE: usize = 256;

/// Phase A: Evaluate a blinded VOPRF element.
///
/// The requesting peer sends a blinded element. This node evaluates it
/// using its VOPRF server key and returns the evaluation + proof.
/// The peer can then finalize to produce a token.
///
/// The server **never** learns the underlying email or hash — it only
/// sees the blinded element.
// TODO: Rate limiting for discovery evaluate — planned for proxy layer in production
pub async fn evaluate(
    State(state): State<AppState>,
    Json(body): Json<DiscoveryEvaluateRequest>,
) -> Result<Json<DiscoveryEvaluateResponse>, ApiError> {
    let voprf_server = &state.voprf_server;

    // Decode the blinded element from base64
    let blinded_bytes = Base64::decode_vec(&body.blinded_element)
        .map_err(|_| ApiError::bad_request("Invalid base64 in blinded_element"))?;

    let blinded_elem =
        voprf::BlindedElement::<Cipher>::deserialize(&blinded_bytes)
            .map_err(|_| ApiError::bad_request("Invalid VOPRF blinded element"))?;

    // Server evaluates the blinded element (infallible for single element)
    let eval_result = voprf_server.evaluate(&blinded_elem);

    // Encode results as base64
    let evaluated_base64 = Base64::encode_string(&eval_result.message.serialize());
    let proof_base64 = Base64::encode_string(&eval_result.proof.serialize());

    // Audit: log event metadata only (never log blinded elements or proofs)
    tracing::debug!("Served discovery evaluate request");

    Ok(Json(DiscoveryEvaluateResponse {
        evaluated_element: evaluated_base64,
        proof: proof_base64,
    }))
}

/// Phase B: Look up a finalized VOPRF token.
///
/// The requesting peer sends a finalized VOPRF token. This node checks
/// its local store and returns a fixed-size encrypted envelope:
/// - Match: AES-256-GCM encrypted `{ "public_address": "0x..." }` padded to ENVELOPE_SIZE
/// - No match: ENVELOPE_SIZE random bytes (GCM auth tag fails on decrypt)
///
/// Both responses are exactly (ENVELOPE_SIZE + 12 nonce + 16 tag) bytes
/// base64-encoded, preventing traffic analysis.
// TODO: Rate limiting for discovery lookup — planned for proxy layer in production
pub async fn lookup(
    State(state): State<AppState>,
    Json(body): Json<DiscoveryLookupRequest>,
) -> Result<Json<DiscoveryLookupResponse>, ApiError> {
    // Decode token from base64 and hex-encode for redb lookup
    let token_bytes = Base64::decode_vec(&body.token)
        .map_err(|_| ApiError::bad_request("Invalid base64 in token"))?;
    let token_hex = alloy::hex::encode(&token_bytes);

    let envelope = match state.voprf_store.lookup(&token_hex) {
        Ok(Some(public_address)) => {
            // Match: encrypt the address in a fixed-size envelope
            encrypt_envelope(&token_bytes, &public_address)?
        }
        _ => {
            // No match: random bytes of the same size
            random_envelope()
        }
    };

    let envelope_base64 = Base64::encode_string(&envelope);

    // Audit: log event metadata only
    tracing::debug!("Served discovery lookup request");

    Ok(Json(DiscoveryLookupResponse {
        envelope: envelope_base64,
    }))
}

/// Encrypt a public address into a fixed-size envelope.
///
/// Uses AES-256-GCM with a key derived from the VOPRF token via HKDF.
/// Both sides know the token, so both can derive the same key.
///
/// Layout: [12-byte nonce][encrypted padded data][16-byte GCM tag]
fn encrypt_envelope(token: &[u8], public_address: &str) -> Result<Vec<u8>, ApiError> {
    // Derive AES-256-GCM key from the token via HKDF-SHA256
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"discovery-envelope-key");
    let prk = salt.extract(token);
    let okm = prk
        .expand(&[b"aes-256-gcm"], &aead::AES_256_GCM)
        .map_err(|_| ApiError::internal("HKDF expand failed"))?;

    let key = aead::UnboundKey::from(okm);
    let sealing_key = aead::LessSafeKey::new(key);

    // Build plaintext: JSON padded to fill the envelope
    let json = serde_json::json!({ "public_address": public_address });
    let json_bytes = serde_json::to_vec(&json)
        .map_err(|_| ApiError::internal("JSON serialization failed"))?;

    // Pad plaintext to fixed size (ENVELOPE_SIZE - 12 nonce - 16 tag = 228 bytes)
    let plaintext_size = ENVELOPE_SIZE - 12 - 16; // 228 bytes
    if json_bytes.len() > plaintext_size {
        return Err(ApiError::internal("Address too long for envelope"));
    }
    let mut padded = vec![0u8; plaintext_size];
    padded[..json_bytes.len()].copy_from_slice(&json_bytes);
    // Store the actual JSON length in the last 2 bytes for unpadding
    let len = json_bytes.len() as u16;
    padded[plaintext_size - 2] = (len >> 8) as u8;
    padded[plaintext_size - 1] = (len & 0xFF) as u8;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    use k256::elliptic_curve::rand_core::{OsRng, RngCore};
    OsRng.fill_bytes(&mut nonce_bytes);

    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    sealing_key
        .seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut padded)
        .map_err(|_| ApiError::internal("AES-GCM encryption failed"))?;

    // Assemble: [nonce][ciphertext+tag]
    let mut envelope = Vec::with_capacity(ENVELOPE_SIZE);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&padded);

    debug_assert_eq!(envelope.len(), ENVELOPE_SIZE);

    Ok(envelope)
}

/// Generate a random envelope (indistinguishable from a real one).
fn random_envelope() -> Vec<u8> {
    use k256::elliptic_curve::rand_core::{OsRng, RngCore};
    let mut envelope = vec![0u8; ENVELOPE_SIZE];
    OsRng.fill_bytes(&mut envelope);
    envelope
}

/// Decrypt an envelope received from a peer during Phase B.
///
/// Returns `Some(public_address)` if the token matched (GCM auth succeeds),
/// or `None` if it was a random envelope (GCM auth fails).
pub fn decrypt_envelope(token: &[u8], envelope: &[u8]) -> Option<String> {
    if envelope.len() != ENVELOPE_SIZE {
        return None;
    }

    // Derive the same AES-256-GCM key from the token
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"discovery-envelope-key");
    let prk = salt.extract(token);
    let okm = prk
        .expand(&[b"aes-256-gcm"], &aead::AES_256_GCM)
        .ok()?;

    let key = aead::UnboundKey::from(okm);
    let opening_key = aead::LessSafeKey::new(key);

    // Split: [12-byte nonce][ciphertext+tag]
    let nonce_bytes: [u8; 12] = envelope[..12].try_into().ok()?;
    let mut ciphertext = envelope[12..].to_vec();

    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let plaintext = opening_key
        .open_in_place(nonce, aead::Aad::empty(), &mut ciphertext)
        .ok()?;

    // Unpad: last 2 bytes are the JSON length
    let plaintext_size = plaintext.len();
    if plaintext_size < 2 {
        return None;
    }
    let json_len =
        ((plaintext[plaintext_size - 2] as usize) << 8) | (plaintext[plaintext_size - 1] as usize);
    if json_len > plaintext_size - 2 {
        return None;
    }

    // Parse JSON
    let json_slice = &plaintext[..json_len];
    let value: serde_json::Value = serde_json::from_slice(json_slice).ok()?;
    value
        .get("public_address")
        .and_then(|v| v.as_str())
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_encrypt_decrypt_roundtrip() {
        let token = b"some-voprf-token-output-bytes-32";
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f4aB12";

        let envelope = encrypt_envelope(token, address).unwrap();
        assert_eq!(envelope.len(), ENVELOPE_SIZE);

        let decrypted = decrypt_envelope(token, &envelope);
        assert_eq!(decrypted, Some(address.to_string()));
    }

    #[test]
    fn random_envelope_fails_decrypt() {
        let token = b"some-voprf-token-output-bytes-32";
        let envelope = random_envelope();
        assert_eq!(envelope.len(), ENVELOPE_SIZE);

        // Decryption should fail (GCM auth tag mismatch)
        let result = decrypt_envelope(token, &envelope);
        assert!(result.is_none());
    }

    #[test]
    fn wrong_token_fails_decrypt() {
        let token1 = b"correct-token-output-bytes----32";
        let token2 = b"wrong---token-output-bytes----32";
        let address = "0xabc";

        let envelope = encrypt_envelope(token1, address).unwrap();
        let result = decrypt_envelope(token2, &envelope);
        assert!(result.is_none());
    }

    #[test]
    fn envelope_is_exactly_256_bytes() {
        let token = b"test-token-for-size-verification";
        let envelope = encrypt_envelope(token, "0x1234567890abcdef1234567890abcdef12345678").unwrap();
        assert_eq!(envelope.len(), 256);

        let random = random_envelope();
        assert_eq!(random.len(), 256);
    }
}
