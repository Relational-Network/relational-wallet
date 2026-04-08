// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! # VOPRF Cross-Instance Discovery Protocol (Phase 2)
//!
//! Enables privacy-preserving email→wallet resolution across multiple
//! enclave instances using RFC 9497 VOPRF (Verifiable Oblivious Pseudorandom
//! Function) with Ristretto255.
//!
//! ## Architecture
//!
//! Each instance maintains:
//! - A VOPRF server key (sealed by Gramine encrypted FS, per-node)
//! - A redb table mapping VOPRF tokens to public addresses
//! - A peer registry with RA-TLS attestation policies
//!
//! ## Two-Phase Protocol
//!
//! ```text
//! Node X (querying)                    Node N (responding)
//! ─────────────────                    ───────────────────
//! Phase A: VOPRF Evaluate
//!   blind(SHA-256(email)) ──RA-TLS──► evaluate(blinded) → proof
//!                         ◄─────────  { evaluated, proof }
//!   verify(proof) → finalize → token
//!
//! Phase B: Token Lookup
//!   token ────────RA-TLS─────────────► lookup(token) → encrypt(address)
//!                         ◄─────────  { envelope }  (fixed 256 bytes)
//!   decrypt(envelope) → address or ∅
//! ```
//!
//! ## Security Properties
//!
//! - Server never sees raw email or SHA-256 hash (only blinded element)
//! - Per-node keys: compromise of node N ≠ compromise of node M
//! - RA-TLS with DCAP attestation between all nodes
//! - Fixed-size envelopes prevent traffic analysis on match/no-match
//! - All keys sealed by Gramine encrypted FS

pub mod api;
pub mod attestation;
pub mod client;
pub mod ffi;
pub mod peer;
pub mod store;
pub mod voprf_ops;

pub use client::DiscoveryClient;
pub use peer::{PeerConfig, PeerRegistry};
pub use store::VoprfTokenStore;
pub use voprf_ops::VoprfServerWrapper;
