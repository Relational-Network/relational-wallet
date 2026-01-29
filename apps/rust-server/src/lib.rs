// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Relational Wallet - Custodial Avalanche Wallet Service
//!
//! This crate provides a **TEE-backed custodial wallet service** using Intel SGX
//! (via Gramine) for secure key management, with Avalanche C-Chain as the
//! settlement layer.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     SGX Enclave (Gramine)                       │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
//! │  │  Axum API   │  │   Auth      │  │   Encrypted Storage     │  │
//! │  │  (HTTPS)    │  │  (Clerk)    │  │   (/data - sealed)      │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │              Blockchain Client (Avalanche C-Chain)          ││
//! │  └─────────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`api`] - HTTP API handlers built on Axum with OpenAPI documentation
//! - [`auth`] - Clerk JWT authentication with JWKS verification
//! - [`blockchain`] - Avalanche C-Chain client for balance queries
//! - [`config`] - Runtime configuration constants
//! - [`error`] - API error types with HTTP status mapping
//! - [`models`] - Request/response data structures
//! - [`state`] - Application state shared across handlers
//! - [`storage`] - Gramine encrypted filesystem repositories
//! - [`tls`] - RA-TLS certificate loading utilities
//!
//! ## Security Model
//!
//! 1. **Private Key Isolation**: All secp256k1 keys are generated and stored
//!    inside the enclave. They never leave unencrypted.
//!
//! 2. **Encrypted Storage**: The `/data` directory is mounted as Gramine's
//!    encrypted filesystem, bound to the enclave's MRSIGNER.
//!
//! 3. **Remote Attestation**: DCAP RA-TLS certificates prove the enclave
//!    identity to clients.
//!
//! 4. **JWT Verification**: All API calls (except health) require Clerk JWTs
//!    verified against JWKS.
//!
//! ## Lightweight Design
//!
//! This crate is optimized for enclave execution:
//! - Minimal dependencies (no `lazy_static`, uses `std::sync::OnceLock`)
//! - Pure Rust crypto (`k256`, `sha3`) - no C dependencies
//! - Async-first with Tokio for efficient I/O

pub mod api;
pub mod auth;
pub mod blockchain;
pub mod config;
pub mod error;
pub mod models;
pub mod state;
pub mod storage;
pub mod tls;
