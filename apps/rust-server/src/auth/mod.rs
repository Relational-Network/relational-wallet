// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Authentication Module
//!
//! This module provides Clerk JWT authentication for the Relational Wallet API.
//!
//! ## Auth Flow
//!
//! 1. Frontend (Next.js) authenticates user with Clerk
//! 2. Frontend sends `Authorization: Bearer <Clerk JWT>`
//! 3. Enclave server:
//!    - Fetches Clerk JWKS via HTTPS
//!    - Verifies JWT signature, expiry, issuer, audience
//!    - Extracts:
//!      - `sub` → canonical `user_id`
//!      - role claims (custom or group claims)
//!
//! ## Security
//!
//! - All non-health endpoints require authentication
//! - JWT verification uses HTTPS-only JWKS fetching
//! - JWKS is cached with TTL for performance
//! - Clock skew tolerance is 60 seconds

pub mod claims;
pub mod error;
pub mod extractor;
pub mod jwks;
pub mod middleware;
pub mod roles;

pub use claims::AuthenticatedUser;
pub use error::AuthError;
pub use extractor::{Auth, AdminOnly};
pub use jwks::JwksManager;
pub use roles::Role;
