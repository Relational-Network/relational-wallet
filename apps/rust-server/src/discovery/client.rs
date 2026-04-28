// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! Two-phase parallel discovery client for querying peer enclaves.
//!
//! Implements the fan-out query protocol:
//! 1. Phase A (parallel): blind → POST evaluate → verify proof → finalize → token
//! 2. Phase B (parallel): POST lookup with token → attempt AES-GCM decrypt
//!
//! Wall-clock time ≈ 2× slowest-peer RTT (both phases parallelized).

use std::sync::Arc;

use base64ct::{Base64, Encoding};

use super::api::decrypt_envelope;
use super::peer::PeerRegistry;
use super::voprf_ops;
use crate::models::{
    DiscoveryEvaluateRequest, DiscoveryEvaluateResponse, DiscoveryLookupRequest,
    DiscoveryLookupResponse,
};

// =============================================================================
// Discovery Client
// =============================================================================

/// Client for querying peer enclaves via the two-phase VOPRF protocol.
///
/// Holds the peer registry with pre-built HTTP clients and performs
/// the blind → evaluate → finalize → lookup sequence in parallel
/// across all configured peers.
pub struct DiscoveryClient {
    peer_registry: Arc<PeerRegistry>,
}

impl DiscoveryClient {
    /// Create a new discovery client.
    pub fn new(peer_registry: Arc<PeerRegistry>) -> Self {
        Self { peer_registry }
    }

    /// Query all peers for an email hash and return the resolved address, if any.
    ///
    /// This implements the full two-phase protocol:
    /// - Phase A: For each peer, blind the input → POST evaluate → verify proof → finalize
    /// - Phase B: For each peer, POST lookup with the finalized token → try decrypt
    ///
    /// Returns the first (and ideally only) match, or `None` if no peer has a wallet
    /// for this email.
    pub async fn query(&self, email_sha256: &str) -> Result<Option<String>, DiscoveryError> {
        if !self.peer_registry.has_peers() {
            // DEBUG-VOPRF: remove after debugging
            tracing::info!("[VOPRF-DBG] query: no peers configured, returning None");
            // END DEBUG-VOPRF
            return Ok(None);
        }
        let peers = self.peer_registry.peers();

        let input = alloy::hex::decode(email_sha256)
            .map_err(|_| DiscoveryError::InvalidInput("Invalid hex in email_sha256".into()))?;

        // DEBUG-VOPRF: remove after debugging — query inputs
        tracing::info!(
            email_sha256_hex = %email_sha256,
            input_len_bytes = input.len(),
            input_bytes_hex = %alloy::hex::encode(&input),
            peer_count = peers.len(),
            "[VOPRF-DBG] query: starting Phase A fan-out"
        );
        // END DEBUG-VOPRF

        // ── Phase A: Evaluate (parallel across all peers) ──
        let mut phase_a_futures = Vec::with_capacity(peers.len());

        // We need to blind separately for each peer (fresh randomness each time)
        for peer in &peers {
            let client = self
                .peer_registry
                .client_for(&peer.node_id)
                .ok_or_else(|| {
                    DiscoveryError::PeerError(format!("No client for peer {}", peer.node_id))
                })?;

            let peer_url = peer.url.clone();
            let peer_pk = peer.voprf_public_key.clone();
            let peer_node_id = peer.node_id.clone();
            let input_clone = input.clone();

            phase_a_futures.push(tokio::spawn(async move {
                evaluate_peer(client, &peer_url, &peer_node_id, &peer_pk, &input_clone).await
            }));
        }

        // Collect Phase A results
        let mut phase_b_inputs: Vec<(String, Vec<u8>, reqwest::Client)> = Vec::new();

        for (i, future) in phase_a_futures.into_iter().enumerate() {
            match future.await {
                Ok(Ok((token, client))) => {
                    let peer = &peers[i];
                    phase_b_inputs.push((peer.url.clone(), token, client));
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, peer_index = i, "Phase A failed for peer");
                }
                Err(e) => {
                    tracing::warn!(error = %e, peer_index = i, "Phase A task panicked");
                }
            }
        }

        if phase_b_inputs.is_empty() {
            tracing::debug!("All Phase A evaluations failed — no peers responded");
            return Ok(None);
        }

        // ── Phase B: Lookup (parallel across successful Phase A peers) ──
        let mut phase_b_futures = Vec::with_capacity(phase_b_inputs.len());

        for (peer_url, token, client) in phase_b_inputs {
            let token_clone = token.clone();
            phase_b_futures.push(tokio::spawn(async move {
                lookup_peer(client, &peer_url, &token_clone).await
            }));
        }

        // Collect Phase B results — first match wins
        for future in phase_b_futures {
            match future.await {
                Ok(Ok(Some(address))) => {
                    tracing::debug!("Discovery query resolved — address found on a peer");
                    return Ok(Some(address));
                }
                Ok(Ok(None)) => {
                    // No match from this peer — continue
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "Phase B failed for a peer");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Phase B task panicked");
                }
            }
        }

        // No peer had a match
        Ok(None)
    }
}

/// Phase A: Blind input, send to peer for evaluation, verify proof, finalize.
async fn evaluate_peer(
    client: reqwest::Client,
    peer_url: &str,
    peer_node_id: &str,
    peer_pk_base64: &str,
    input: &[u8],
) -> Result<(Vec<u8>, reqwest::Client), DiscoveryError> {
    // DEBUG-VOPRF: remove after debugging — Phase A start per peer
    tracing::info!(
        peer_node_id = %peer_node_id,
        peer_url = %peer_url,
        peer_pk_base64 = %peer_pk_base64,
        input_len = input.len(),
        "[VOPRF-DBG] Phase A: starting for peer"
    );
    // END DEBUG-VOPRF

    // Blind the input (fresh randomness)
    let blind_result =
        voprf_ops::blind(input).map_err(|e| DiscoveryError::VoprfError(e.to_string()))?;

    // POST to peer's evaluate endpoint
    let url = format!("{peer_url}/internal/discovery/evaluate");
    let request = DiscoveryEvaluateRequest {
        blinded_element: blind_result.blinded_element_base64,
    };

    // DEBUG-VOPRF: remove after debugging — blinded element sent
    tracing::info!(
        peer_node_id = %peer_node_id,
        url = %url,
        blinded_element = %request.blinded_element,
        "[VOPRF-DBG] Phase A: POST evaluate"
    );
    // END DEBUG-VOPRF

    let response = client.post(&url).json(&request).send().await.map_err(|e| {
        DiscoveryError::PeerError(format!("Phase A request to {peer_node_id} failed: {e}"))
    })?;

    if !response.status().is_success() {
        return Err(DiscoveryError::PeerError(format!(
            "Phase A: {peer_node_id} returned {}",
            response.status()
        )));
    }

    let eval_response: DiscoveryEvaluateResponse = response.json().await.map_err(|e| {
        DiscoveryError::PeerError(format!(
            "Phase A: failed to parse response from {peer_node_id}: {e}"
        ))
    })?;

    // DEBUG-VOPRF: remove after debugging — evaluated element + proof received
    tracing::info!(
        peer_node_id = %peer_node_id,
        evaluated_element = %eval_response.evaluated_element,
        proof = %eval_response.proof,
        "[VOPRF-DBG] Phase A: received evaluate response"
    );
    // END DEBUG-VOPRF

    // Finalize: verify proof and compute the VOPRF token
    let token = voprf_ops::finalize(
        &blind_result.state,
        input,
        &eval_response.evaluated_element,
        &eval_response.proof,
        peer_pk_base64,
    )
    .map_err(|e| {
        DiscoveryError::VoprfError(format!("Finalization failed for {peer_node_id}: {e}"))
    })?;

    // DEBUG-VOPRF: remove after debugging — token derived from VOPRF finalize
    tracing::info!(
        peer_node_id = %peer_node_id,
        token_hex = %alloy::hex::encode(&token),
        token_len = token.len(),
        "[VOPRF-DBG] Phase A: finalized — derived token for this peer"
    );
    // END DEBUG-VOPRF

    Ok((token, client))
}

/// Phase B: Send finalized token to peer for lookup, attempt to decrypt envelope.
async fn lookup_peer(
    client: reqwest::Client,
    peer_url: &str,
    token: &[u8],
) -> Result<Option<String>, DiscoveryError> {
    let url = format!("{peer_url}/internal/discovery/lookup");
    let token_base64 = Base64::encode_string(token);

    // DEBUG-VOPRF: remove after debugging — Phase B start
    tracing::info!(
        peer_url = %peer_url,
        token_hex = %alloy::hex::encode(token),
        token_base64 = %token_base64,
        "[VOPRF-DBG] Phase B: POST lookup"
    );
    // END DEBUG-VOPRF

    let request = DiscoveryLookupRequest {
        token: token_base64,
    };

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| DiscoveryError::PeerError(format!("Phase B request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(DiscoveryError::PeerError(format!(
            "Phase B: peer returned {}",
            response.status()
        )));
    }

    let lookup_response: DiscoveryLookupResponse = response.json().await.map_err(|e| {
        DiscoveryError::PeerError(format!("Phase B: failed to parse response: {e}"))
    })?;

    // Decode the envelope and attempt decryption
    let envelope = Base64::decode_vec(&lookup_response.envelope)
        .map_err(|_| DiscoveryError::PeerError("Invalid base64 in envelope".into()))?;

    // Try to decrypt: GCM auth success → match, GCM auth fail → no match
    let decrypted = decrypt_envelope(token, &envelope);

    // DEBUG-VOPRF: remove after debugging — Phase B decrypt outcome
    tracing::info!(
        peer_url = %peer_url,
        envelope_len = envelope.len(),
        decrypt_matched = decrypted.is_some(),
        "[VOPRF-DBG] Phase B: decrypt attempt complete \
         (Some = token was present on peer, None = random envelope / wrong token)"
    );
    // END DEBUG-VOPRF

    Ok(decrypted)
}

// =============================================================================
// Error Type
// =============================================================================

/// Errors from discovery queries.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("VOPRF error: {0}")]
    VoprfError(String),

    #[error("Peer communication error: {0}")]
    PeerError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_error_display() {
        let err = DiscoveryError::PeerError("connection refused".into());
        assert!(err.to_string().contains("connection refused"));

        let err = DiscoveryError::VoprfError("bad proof".into());
        assert!(err.to_string().contains("bad proof"));
    }
}
