// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! VOPRF key management and cryptographic operations.
//!
//! Uses the `voprf` crate (RustCrypto, RFC 9497) with Ristretto255
//! for privacy-preserving email→wallet discovery.
//!
//! ## Key Storage
//!
//! The VOPRF server key is persisted at `/data/system/voprf_server_key.bin`,
//! sealed by Gramine's encrypted filesystem. Each node has its own key —
//! there is no shared secret across instances.

use std::path::Path;

use base64ct::{Base64, Encoding};
use k256::elliptic_curve::rand_core::OsRng;
use voprf::*;

/// Re-export the VOPRF error type with a more descriptive name to avoid
/// ambiguity with `std::error::Error`.
pub type VoprfError = voprf::Error;

/// The VOPRF cipher suite: Ristretto255 + SHA-512 (RFC 9497 default).
pub type Cipher = Ristretto255;

// =============================================================================
// Server Wrapper
// =============================================================================

/// Wrapper around `VoprfServer` that handles serialization and key management.
pub struct VoprfServerWrapper {
    server: VoprfServer<Cipher>,
}

impl VoprfServerWrapper {
    /// Generate a fresh VOPRF server with a random key.
    pub fn generate() -> Self {
        let server = VoprfServer::<Cipher>::new(&mut OsRng)
            .expect("VOPRF server key generation should never fail");
        Self { server }
    }

    /// Serialize the server state (including secret key) for sealed storage.
    pub fn serialize(&self) -> Vec<u8> {
        self.server.serialize().to_vec()
    }

    /// Deserialize a previously-stored server state.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, VoprfError> {
        let server = VoprfServer::<Cipher>::deserialize(bytes)?;
        Ok(Self { server })
    }

    /// Load from file or generate a new key if the file doesn't exist.
    ///
    /// The file is stored in Gramine's encrypted FS, so the key is sealed
    /// to the enclave identity.
    pub fn load_or_generate(path: &Path) -> Result<Self, VoprfKeyError> {
        if path.exists() {
            let bytes = std::fs::read(path).map_err(|e| VoprfKeyError::Io(e.to_string()))?;
            let wrapper =
                Self::deserialize(&bytes).map_err(|e| VoprfKeyError::Deserialize(e.to_string()))?;
            tracing::info!(path = %path.display(), "Loaded VOPRF server key");
            Ok(wrapper)
        } else {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let wrapper = Self::generate();
            std::fs::write(path, wrapper.serialize())
                .map_err(|e| VoprfKeyError::Io(e.to_string()))?;
            tracing::info!(path = %path.display(), "Generated and stored new VOPRF server key");
            Ok(wrapper)
        }
    }

    /// Get the public key for sharing with peers (base64-encoded).
    pub fn public_key_base64(&self) -> String {
        let pk = self.server.get_public_key();
        let pk_bytes = <Cipher as CipherSuite>::Group::serialize_elem(pk);
        Base64::encode_string(&pk_bytes)
    }

    /// Get the raw public key bytes.
    #[allow(dead_code)]
    pub fn public_key_bytes(&self) -> Vec<u8> {
        let pk = self.server.get_public_key();
        <Cipher as CipherSuite>::Group::serialize_elem(pk).to_vec()
    }

    /// Evaluate a blinded element (server-side, for incoming peer queries).
    ///
    /// Returns the evaluation result and proof that the evaluation was
    /// performed correctly using this server's key.
    ///
    /// Uses `blind_evaluate` which handles proof generation internally.
    pub fn evaluate(
        &self,
        blinded_element: &BlindedElement<Cipher>,
    ) -> VoprfServerEvaluateResult<Cipher> {
        self.server.blind_evaluate(&mut OsRng, blinded_element)
    }

    /// Compute a local VOPRF token for storage at wallet creation.
    ///
    /// Performs the full protocol locally: blind → evaluate → finalize.
    /// The resulting token is stored in the VOPRF token database so
    /// peers can look it up during Phase B of the discovery protocol.
    pub fn compute_local_token(&self, input: &[u8]) -> Result<Vec<u8>, VoprfError> {
        // Client-side: blind the input
        let client_blind_result = VoprfClient::<Cipher>::blind(input, &mut OsRng)?;

        // Server-side: evaluate the blinded element (infallible for single element)
        let server_evaluate_result = self
            .server
            .blind_evaluate(&mut OsRng, &client_blind_result.message);

        // Client-side: finalize to get the token
        let output = client_blind_result.state.finalize(
            input,
            &server_evaluate_result.message,
            &server_evaluate_result.proof,
            self.server.get_public_key(),
        )?;

        Ok(output.to_vec())
    }
}

// =============================================================================
// Client-Side Operations (for querying peers)
// =============================================================================

/// Result of blinding an input on the client side.
pub struct BlindResult {
    /// The blinded element to send to the peer.
    pub blinded_element_base64: String,
    /// Opaque state needed for finalization (kept locally).
    pub state: VoprfClientBlindResult<Cipher>,
}

/// Blind an input for sending to a peer's evaluate endpoint.
///
/// Returns the blinded element (base64) and the client state needed
/// for finalization after receiving the evaluation.
pub fn blind(input: &[u8]) -> Result<BlindResult, VoprfError> {
    let blind_result = VoprfClient::<Cipher>::blind(input, &mut OsRng)?;

    let blinded_bytes = blind_result.message.serialize();
    let blinded_base64 = Base64::encode_string(&blinded_bytes);

    Ok(BlindResult {
        blinded_element_base64: blinded_base64,
        state: blind_result,
    })
}

/// Finalize a VOPRF evaluation to produce the token.
///
/// Called after receiving the evaluated element and proof from the peer.
pub fn finalize(
    blind_result: &VoprfClientBlindResult<Cipher>,
    input: &[u8],
    evaluated_base64: &str,
    proof_base64: &str,
    peer_public_key_base64: &str,
) -> Result<Vec<u8>, VoprfError> {
    let evaluated_bytes = Base64::decode_vec(evaluated_base64)
        .map_err(|_| VoprfError::Input)?;
    let proof_bytes = Base64::decode_vec(proof_base64)
        .map_err(|_| VoprfError::Input)?;
    let pk_bytes = Base64::decode_vec(peer_public_key_base64)
        .map_err(|_| VoprfError::Input)?;

    let evaluated = EvaluationElement::<Cipher>::deserialize(&evaluated_bytes)?;
    let proof = Proof::<Cipher>::deserialize(&proof_bytes)?;
    let public_key = <Cipher as CipherSuite>::Group::deserialize_elem(&pk_bytes)?;

    let output = blind_result.state.finalize(input, &evaluated, &proof, public_key)?;

    Ok(output.to_vec())
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors from VOPRF key management.
#[derive(Debug, thiserror::Error)]
pub enum VoprfKeyError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_serialize_roundtrip() {
        let server = VoprfServerWrapper::generate();
        let bytes = server.serialize();
        let restored = VoprfServerWrapper::deserialize(&bytes).unwrap();

        // Public keys should match
        assert_eq!(server.public_key_base64(), restored.public_key_base64());
    }

    #[test]
    fn compute_local_token_deterministic_with_same_key() {
        let server = VoprfServerWrapper::generate();
        let input = b"test@example.com-sha256-hash";

        // VOPRF uses random blinding, so the intermediate blinded elements differ,
        // but the final output is deterministic for the same input + server key.
        let token1 = server.compute_local_token(input).unwrap();
        let token2 = server.compute_local_token(input).unwrap();
        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        // VOPRF output is deterministic: F(key, input) always produces the same result
        assert_eq!(token1, token2, "VOPRF output should be deterministic for same key+input");
    }

    #[test]
    fn full_protocol_flow() {
        let server = VoprfServerWrapper::generate();
        let input = b"sha256-of-email";

        // Store a local token (simulates wallet creation)
        let stored_token = server.compute_local_token(input).unwrap();
        assert!(!stored_token.is_empty());

        // Simulate peer query: blind → evaluate → finalize
        let blind_result = blind(input).unwrap();
        assert!(!blind_result.blinded_element_base64.is_empty());

        // Deserialize the blinded element for the server
        let blinded_bytes = Base64::decode_vec(&blind_result.blinded_element_base64).unwrap();
        let blinded_elem = BlindedElement::<Cipher>::deserialize(&blinded_bytes).unwrap();

        // Server evaluates
        let eval_result = server.evaluate(&blinded_elem);
        let eval_base64 = Base64::encode_string(&eval_result.message.serialize());
        let proof_base64 = Base64::encode_string(&eval_result.proof.serialize());
        let pk_base64 = server.public_key_base64();

        // Client finalizes
        let token = finalize(
            &blind_result.state,
            input,
            &eval_base64,
            &proof_base64,
            &pk_base64,
        )
        .unwrap();

        // The finalized token should match the locally-computed one
        // (VOPRF determinism property: same input + same server key → same output,
        //  regardless of random blinding factor)
        assert_eq!(token, stored_token);
    }

    #[test]
    fn load_or_generate_creates_new_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("system/voprf_key.bin");

        // First call: generates
        let server1 = VoprfServerWrapper::load_or_generate(&path).unwrap();
        assert!(path.exists());

        // Second call: loads
        let server2 = VoprfServerWrapper::load_or_generate(&path).unwrap();
        assert_eq!(server1.public_key_base64(), server2.public_key_base64());
    }

    #[test]
    fn public_key_base64_is_valid() {
        let server = VoprfServerWrapper::generate();
        let pk = server.public_key_base64();
        // Should be valid base64
        assert!(Base64::decode_vec(&pk).is_ok());
    }
}
