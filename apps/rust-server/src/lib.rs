// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Relational Wallet - Custodial Avalanche Wallet Service
//!
//! This crate provides a TEE-backed custodial wallet service using Intel SGX
//! for secure key management, with Avalanche as the settlement layer.
//!
//! ## Modules
//!
//! - `api` - HTTP API handlers (Axum)
//! - `auth` - Authentication and authorization (Clerk JWT)
//! - `blockchain` - Avalanche C-Chain integration
//! - `storage` - Encrypted storage (Gramine sealed FS)

pub mod api;
pub mod auth;
pub mod blockchain;
pub mod config;
pub mod error;
pub mod models;
pub mod state;
pub mod storage;
pub mod tls;
