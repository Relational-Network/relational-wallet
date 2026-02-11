// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! User roles for authorization.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// User roles for authorization.
///
/// ## Role Hierarchy
///
/// - `Admin` - Full access to all endpoints and wallets
/// - `Client` - Normal user, can only access own wallets
/// - `Support` - Read-only access to metadata (no private keys)
/// - `Auditor` - Read-only access to audit logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full administrative access
    Admin,
    /// Normal client user (owns wallets)
    Client,
    /// Support staff (read-only metadata)
    Support,
    /// Auditor (read-only audit logs)
    Auditor,
}

impl Role {
    /// Check if this role has at least the privileges of the required role.
    pub fn has_privilege(&self, required: Role) -> bool {
        match (self, required) {
            // Admin can do anything
            (Role::Admin, _) => true,
            // Client can do Client things
            (Role::Client, Role::Client) => true,
            // Support can read metadata
            (Role::Support, Role::Support) => true,
            // Auditor can read audits
            (Role::Auditor, Role::Auditor) => true,
            // Everything else is denied
            _ => false,
        }
    }

    /// Parse role from string (case-insensitive).
    /// Used when extracting roles from Clerk public metadata.
    pub fn from_str(s: &str) -> Option<Role> {
        match s.to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "client" => Some(Role::Client),
            "support" => Some(Role::Support),
            "auditor" => Some(Role::Auditor),
            _ => None,
        }
    }
}

impl Default for Role {
    /// Default role is Client (least privilege for authenticated users).
    fn default() -> Self {
        Role::Client
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Client => write!(f, "client"),
            Role::Support => write!(f, "support"),
            Role::Auditor => write!(f, "auditor"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_all_privileges() {
        assert!(Role::Admin.has_privilege(Role::Admin));
        assert!(Role::Admin.has_privilege(Role::Client));
        assert!(Role::Admin.has_privilege(Role::Support));
        assert!(Role::Admin.has_privilege(Role::Auditor));
    }

    #[test]
    fn client_only_has_client_privilege() {
        assert!(!Role::Client.has_privilege(Role::Admin));
        assert!(Role::Client.has_privilege(Role::Client));
        assert!(!Role::Client.has_privilege(Role::Support));
        assert!(!Role::Client.has_privilege(Role::Auditor));
    }

    #[test]
    fn from_str_parses_correctly() {
        assert_eq!(Role::from_str("admin"), Some(Role::Admin));
        assert_eq!(Role::from_str("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::from_str("Client"), Some(Role::Client));
        assert_eq!(Role::from_str("unknown"), None);
    }

    #[test]
    fn default_role_is_client() {
        assert_eq!(Role::default(), Role::Client);
    }
}
