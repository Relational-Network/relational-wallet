// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! # Runtime Configuration Constants
//!
//! This module defines environment variable names and default values used
//! throughout the application. Configuration is loaded from the environment
//! at startup.
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `DATA_DIR` | Root directory for encrypted storage | `/data` |
//! | `HOST` | Server bind address | `0.0.0.0` |
//! | `PORT` | Server bind port | `8080` |
//! | `CLERK_JWKS_URL` | Clerk JWKS endpoint for JWT verification | Required for production |
//! | `CLERK_ISSUER` | Expected JWT issuer claim | Required for production |
//! | `CLERK_AUDIENCE` | Expected JWT audience claim | Optional |
//! | `LOG_FORMAT` | Logging format (`json` or `pretty`) | `pretty` |
//! | `RUST_LOG` | Log level filter | `info,tower_http=debug` |

/// Environment variable name for the encrypted data directory path.
///
/// The data directory is mounted as Gramine's encrypted filesystem in the
/// manifest. All wallet keys, bookmarks, and audit logs are stored here.
///
/// # Default
/// `/data` (set in Gramine manifest as encrypted mount point)
pub const DATA_DIR_ENV: &str = "DATA_DIR";
