// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! Peer registry and RA-TLS client factory.
//!
//! Each peer is configured with a URL, VOPRF public key, and attestation
//! policy. The registry loads from `/data/system/peers.json` and provides
//! pre-configured `reqwest::Client` instances for each peer with RA-TLS
//! certificate verification.
//!
//! The registry supports runtime CRUD via `RwLock`, with self-skip
//! (automatically excludes peers whose VOPRF public key matches our own)
//! and JSON file persistence on the sealed filesystem.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::attestation::{AttestationPolicy, RaTlsServerVerifier};

// =============================================================================
// Peer Configuration
// =============================================================================

/// Configuration for a single peer enclave instance.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PeerConfig {
    /// Human-readable node identifier (e.g., "node-eu-1").
    pub node_id: String,

    /// HTTPS URL for the peer's API (e.g., "https://node-eu-1.example.com:8080").
    pub url: String,

    /// Base64-encoded VOPRF public key for proof verification.
    pub voprf_public_key: String,

    /// Attestation policy applied when verifying this peer's RA-TLS certificate.
    pub attestation_policy: AttestationPolicy,
}

// =============================================================================
// Peer Registry Inner (behind RwLock)
// =============================================================================

/// Mutable inner state of the peer registry.
struct PeerRegistryInner {
    peers: Vec<PeerConfig>,
    /// Pre-built reqwest clients keyed by node_id, each with the
    /// peer's attestation policy embedded in the TLS verifier.
    clients: HashMap<String, reqwest::Client>,
}

// =============================================================================
// Peer Registry
// =============================================================================

/// Registry of known peer instances with pre-built HTTP clients.
///
/// Thread-safe via `RwLock` — supports runtime CRUD from admin API.
/// Self-skip: peers whose `voprf_public_key` matches `own_voprf_public_key`
/// are automatically excluded from fan-out queries.
pub struct PeerRegistry {
    inner: RwLock<PeerRegistryInner>,
    /// Our own VOPRF public key (base64). Peers matching this are "self".
    own_voprf_public_key: String,
    /// Path to the peers.json file for persistence.
    file_path: PathBuf,
}

impl PeerRegistry {
    /// Load peer registry from a JSON file.
    ///
    /// Returns an empty registry if the file doesn't exist.
    /// Peers whose `voprf_public_key` matches `own_pk` are skipped.
    pub fn load(path: &Path, own_voprf_public_key: String) -> Result<Self, PeerRegistryError> {
        let all_peers = if path.exists() {
            let content =
                std::fs::read_to_string(path).map_err(|e| PeerRegistryError::Io(e.to_string()))?;
            let peers: Vec<PeerConfig> = serde_json::from_str(&content)
                .map_err(|e| PeerRegistryError::Parse(e.to_string()))?;
            peers
        } else {
            tracing::info!(
                path = %path.display(),
                "No peers.json found — running in local-only mode"
            );
            Vec::new()
        };

        // Filter out self
        let peers: Vec<PeerConfig> = all_peers
            .into_iter()
            .filter(|p| p.voprf_public_key != own_voprf_public_key)
            .collect();

        let mut clients = HashMap::new();
        for peer in &peers {
            match build_peer_client(peer) {
                Ok(client) => {
                    clients.insert(peer.node_id.clone(), client);
                }
                Err(e) => {
                    tracing::warn!(
                        node_id = %peer.node_id,
                        error = %e,
                        "Failed to build RA-TLS client for peer — skipping"
                    );
                }
            }
        }

        tracing::info!(count = peers.len(), "Loaded peer registry for discovery");

        Ok(Self {
            inner: RwLock::new(PeerRegistryInner { peers, clients }),
            own_voprf_public_key,
            file_path: path.to_path_buf(),
        })
    }

    /// Create an empty registry (no peers, local-only mode).
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(PeerRegistryInner {
                peers: Vec::new(),
                clients: HashMap::new(),
            }),
            own_voprf_public_key: String::new(),
            file_path: PathBuf::new(),
        }
    }

    /// Get a snapshot of all peer configurations (cloned).
    pub fn peers(&self) -> Vec<PeerConfig> {
        let inner = self.inner.read().expect("PeerRegistry lock poisoned");
        inner.peers.clone()
    }

    /// Whether any peers are configured.
    pub fn has_peers(&self) -> bool {
        let inner = self.inner.read().expect("PeerRegistry lock poisoned");
        !inner.peers.is_empty()
    }

    /// Get a cloned HTTP client for a specific peer.
    pub fn client_for(&self, node_id: &str) -> Option<reqwest::Client> {
        let inner = self.inner.read().expect("PeerRegistry lock poisoned");
        inner.clients.get(node_id).cloned()
    }

    /// Our own VOPRF public key (base64).
    pub fn own_public_key(&self) -> &str {
        &self.own_voprf_public_key
    }

    // ─── CRUD Operations ───

    /// Add a new peer. Returns error if `node_id` already exists or
    /// the peer's VOPRF public key matches our own.
    pub fn add_peer(&self, config: PeerConfig) -> Result<(), PeerRegistryError> {
        if config.voprf_public_key == self.own_voprf_public_key {
            return Err(PeerRegistryError::SelfSkip);
        }

        let client = build_peer_client(&config)?;

        let mut inner = self.inner.write().expect("PeerRegistry lock poisoned");
        if inner.peers.iter().any(|p| p.node_id == config.node_id) {
            return Err(PeerRegistryError::DuplicateNodeId(config.node_id.clone()));
        }

        inner.clients.insert(config.node_id.clone(), client);
        inner.peers.push(config);
        drop(inner);

        self.persist()?;
        Ok(())
    }

    /// Remove a peer by `node_id`. Returns error if not found.
    pub fn remove_peer(&self, node_id: &str) -> Result<(), PeerRegistryError> {
        let mut inner = self.inner.write().expect("PeerRegistry lock poisoned");
        let idx = inner
            .peers
            .iter()
            .position(|p| p.node_id == node_id)
            .ok_or_else(|| PeerRegistryError::NotFound(node_id.to_string()))?;

        inner.peers.remove(idx);
        inner.clients.remove(node_id);
        drop(inner);

        self.persist()?;
        Ok(())
    }

    /// Update an existing peer. The `node_id` must match an existing entry.
    /// Rebuilds the RA-TLS client with the new attestation policy.
    pub fn update_peer(&self, config: PeerConfig) -> Result<(), PeerRegistryError> {
        if config.voprf_public_key == self.own_voprf_public_key {
            return Err(PeerRegistryError::SelfSkip);
        }

        let client = build_peer_client(&config)?;

        let mut inner = self.inner.write().expect("PeerRegistry lock poisoned");
        let idx = inner
            .peers
            .iter()
            .position(|p| p.node_id == config.node_id)
            .ok_or_else(|| PeerRegistryError::NotFound(config.node_id.clone()))?;

        inner.peers[idx] = config.clone();
        inner.clients.insert(config.node_id.clone(), client);
        drop(inner);

        self.persist()?;
        Ok(())
    }

    /// List all peers (cloned snapshot).
    pub fn list_peers(&self) -> Vec<PeerConfig> {
        self.peers()
    }

    /// Persist the current peer list to the JSON file.
    fn persist(&self) -> Result<(), PeerRegistryError> {
        if self.file_path.as_os_str().is_empty() {
            return Ok(()); // Empty path = in-memory only (tests)
        }

        // Include self-skip note
        let inner = self.inner.read().expect("PeerRegistry lock poisoned");
        let json = serde_json::to_string_pretty(&inner.peers)
            .map_err(|e| PeerRegistryError::Parse(e.to_string()))?;
        drop(inner);

        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PeerRegistryError::Io(e.to_string()))?;
        }
        std::fs::write(&self.file_path, json).map_err(|e| PeerRegistryError::Io(e.to_string()))?;

        tracing::debug!(path = %self.file_path.display(), "Persisted peer registry");
        Ok(())
    }
}

// =============================================================================
// Client Factory
// =============================================================================

/// Build a `reqwest::Client` for a specific peer with RA-TLS verification.
///
/// The client's TLS configuration uses a custom `ServerCertVerifier` that
/// calls into Gramine's `libra_tls_verify_dcap.so` for DCAP attestation
/// verification, then checks MRENCLAVE/MRSIGNER/ISV measurements against
/// the peer's attestation policy.
fn build_peer_client(peer: &PeerConfig) -> Result<reqwest::Client, PeerRegistryError> {
    // Build rustls ClientConfig with our custom RA-TLS verifier
    let verifier = RaTlsServerVerifier::new(peer.attestation_policy.clone());

    // Load our own RA-TLS cert for mutual attestation (peer verifies us too)
    let own_certs = crate::tls::load_ratls_certificate(crate::tls::RA_TLS_CERT_PATH)
        .map_err(|e| PeerRegistryError::Tls(format!("Failed to load own RA-TLS cert: {e}")))?;
    let own_key = crate::tls::load_ratls_private_key(crate::tls::RA_TLS_KEY_PATH)
        .map_err(|e| PeerRegistryError::Tls(format!("Failed to load own RA-TLS key: {e}")))?;

    let tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_client_auth_cert(own_certs, own_key)
        .map_err(|e| PeerRegistryError::Tls(format!("TLS client config error: {e}")))?;

    let client = reqwest::Client::builder()
        .use_preconfigured_tls(tls_config)
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| PeerRegistryError::Client(e.to_string()))?;

    tracing::debug!(
        node_id = %peer.node_id,
        url = %peer.url,
        "Built RA-TLS client for peer"
    );

    Ok(client)
}

// =============================================================================
// Error Type
// =============================================================================

/// Errors from peer registry operations.
#[derive(Debug, thiserror::Error)]
pub enum PeerRegistryError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("JSON parse error: {0}")]
    Parse(String),
    #[error("TLS error: {0}")]
    Tls(String),
    #[error("Client build error: {0}")]
    Client(String),
    #[error("Peer not found: {0}")]
    NotFound(String),
    #[error("Duplicate node_id: {0}")]
    DuplicateNodeId(String),
    #[error("Cannot add self as peer")]
    SelfSkip,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry() {
        let registry = PeerRegistry::empty();
        assert!(!registry.has_peers());
        assert!(registry.peers().is_empty());
    }

    #[test]
    fn load_nonexistent_file_returns_empty() {
        let registry = PeerRegistry::load(
            Path::new("/nonexistent/peers.json"),
            "my-own-pk".to_string(),
        )
        .unwrap();
        assert!(!registry.has_peers());
    }

    #[test]
    fn peer_config_serde_roundtrip() {
        let config = PeerConfig {
            node_id: "node-eu-1".to_string(),
            url: "https://node-eu-1.example.com:8080".to_string(),
            voprf_public_key: "dGVzdC1rZXk=".to_string(),
            attestation_policy: AttestationPolicy {
                mrenclave: [0xab; 32],
                mrsigner: None,
                min_isv_svn: 0,
                isv_prod_id: 0,
            },
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: PeerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_id, "node-eu-1");
        assert_eq!(parsed.url, "https://node-eu-1.example.com:8080");
        assert_eq!(parsed.voprf_public_key, "dGVzdC1rZXk=");
    }

    #[test]
    fn peer_registry_json_format() {
        let peers = vec![
            PeerConfig {
                node_id: "node-1".to_string(),
                url: "https://node1:8080".to_string(),
                voprf_public_key: "a2V5MQ==".to_string(),
                attestation_policy: AttestationPolicy {
                    mrenclave: [0x01; 32],
                    mrsigner: Some([0x02; 32]),
                    min_isv_svn: 1,
                    isv_prod_id: 0,
                },
            },
            PeerConfig {
                node_id: "node-2".to_string(),
                url: "https://node2:8080".to_string(),
                voprf_public_key: "a2V5Mg==".to_string(),
                attestation_policy: AttestationPolicy {
                    mrenclave: [0x03; 32],
                    mrsigner: None,
                    min_isv_svn: 0,
                    isv_prod_id: 0,
                },
            },
        ];

        let json = serde_json::to_string_pretty(&peers).unwrap();
        let parsed: Vec<PeerConfig> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].node_id, "node-1");
        assert_eq!(parsed[1].node_id, "node-2");
    }

    #[test]
    fn self_skip_filters_own_key() {
        let own_pk = "my-own-pk-base64".to_string();
        let tmp = std::env::temp_dir().join(format!("test-peers-{}.json", uuid::Uuid::new_v4()));
        let peers = vec![
            PeerConfig {
                node_id: "self-node".to_string(),
                url: "https://self:8080".to_string(),
                voprf_public_key: own_pk.clone(),
                attestation_policy: AttestationPolicy {
                    mrenclave: [0x01; 32],
                    mrsigner: None,
                    min_isv_svn: 0,
                    isv_prod_id: 0,
                },
            },
            PeerConfig {
                node_id: "other-node".to_string(),
                url: "https://other:8080".to_string(),
                voprf_public_key: "other-pk".to_string(),
                attestation_policy: AttestationPolicy {
                    mrenclave: [0x02; 32],
                    mrsigner: None,
                    min_isv_svn: 0,
                    isv_prod_id: 0,
                },
            },
        ];
        std::fs::write(&tmp, serde_json::to_string(&peers).unwrap()).unwrap();

        // Loading will try to build RA-TLS clients which will fail outside SGX,
        // but the self-skip filtering happens before client building.
        // The "other-node" will fail client build but self-node should be filtered.
        let registry = PeerRegistry::load(&tmp, own_pk).unwrap();
        // Only "other-node" should remain (self filtered), but client build
        // may have warned. The peer list should have exactly 1 entry.
        let peers = registry.peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "other-node");

        std::fs::remove_file(&tmp).ok();
    }
}
