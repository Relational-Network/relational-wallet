// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! JWT claims and authenticated user representation.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::roles::Role;

/// Claims extracted from a Clerk JWT.
///
/// Clerk JWTs contain standard OIDC claims plus custom claims.
/// See: https://clerk.com/docs/backend-requests/handling/manual-jwt
///
/// Note: The actual JWT verification uses JwtClaims in extractor.rs.
/// This struct provides a full representation for reference.
/// Fields must exist for serde JWT deserialization
/// even though they're not read
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ClerkClaims {
    /// Subject (user ID) - the canonical Clerk user identifier
    pub sub: String,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Not before timestamp (optional)
    #[serde(default)]
    pub nbf: Option<i64>,

    /// Issuer (should be your Clerk instance URL)
    pub iss: String,

    /// Audience (optional, application-specific)
    #[serde(default)]
    pub aud: Option<String>,

    /// Clerk session ID
    #[serde(default)]
    pub sid: Option<String>,

    /// Authorized party (optional)
    #[serde(default)]
    pub azp: Option<String>,

    /// Custom metadata (from Clerk user metadata)
    #[serde(default)]
    pub metadata: Option<UserMetadata>,

    /// Organization memberships (if using Clerk organizations)
    #[serde(default)]
    pub org_memberships: Option<Vec<OrgMembership>>,
}

/// User metadata from Clerk.
///
/// Note: Role extraction is implemented in extractor.rs via PublicMetadata.
/// This struct is kept for documentation and future expansion.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserMetadata {
    /// User's role (set in Clerk public metadata)
    #[serde(default)]
    pub role: Option<String>,

    /// Additional custom fields
    /// TODO: Define specific fields as needed or keep as a flexible map for future use.
    #[allow(dead_code)]
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Organization membership from Clerk.
///
/// Note: Multi-tenant support is planned for future releases.
/// TODO: Implement organization membership handling when multi-tenancy is added.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct OrgMembership {
    /// Organization ID
    pub org_id: String,
    /// Role in the organization
    pub role: String,
}

/// Authenticated user information extracted from JWT.
///
/// This is the primary type used throughout the application to represent
/// the authenticated user making a request.
/// TODO (issuer, expires_at): Set in constructor; available for logging/middleware
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticatedUser {
    /// Canonical user ID (Clerk `sub` claim)
    pub user_id: String,

    /// User's role
    pub role: Role,

    /// Session ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Original issuer (used for validation, not serialized)
    #[serde(skip)]
    pub issuer: String,

    /// Token expiration (Unix timestamp, used for validation, not serialized)
    #[serde(skip)]
    pub expires_at: i64,
}

impl AuthenticatedUser {
    /// Create from Clerk claims.
    pub fn from_claims(claims: ClerkClaims) -> Self {
        // Extract role from metadata or default to Client
        let role = claims
            .metadata
            .as_ref()
            .and_then(|m| m.role.as_ref())
            .and_then(|r| Role::from_str(r))
            .unwrap_or(Role::Client);

        Self {
            user_id: claims.sub,
            role,
            session_id: claims.sid,
            issuer: claims.iss,
            expires_at: claims.exp,
        }
    }

    /// Check if the user has the required role.
    /// TODO: role-specific endpoints and permissions can be implemented using this method.
    #[allow(dead_code)]
    pub fn has_role(&self, required: Role) -> bool {
        self.role.has_privilege(required)
    }

    /// Check if this user is an admin.
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims() -> ClerkClaims {
        ClerkClaims {
            sub: "user_123".to_string(),
            iat: 1700000000,
            exp: 1700003600,
            nbf: None,
            iss: "https://clerk.example.com".to_string(),
            aud: Some("my-app".to_string()),
            sid: Some("sess_abc".to_string()),
            azp: None,
            metadata: Some(UserMetadata {
                role: Some("admin".to_string()),
                extra: Default::default(),
            }),
            org_memberships: None,
        }
    }

    #[test]
    fn from_claims_extracts_user_id() {
        let claims = sample_claims();
        let user = AuthenticatedUser::from_claims(claims);
        assert_eq!(user.user_id, "user_123");
    }

    #[test]
    fn from_claims_extracts_role_from_metadata() {
        let claims = sample_claims();
        let user = AuthenticatedUser::from_claims(claims);
        assert_eq!(user.role, Role::Admin);
    }

    #[test]
    fn from_claims_defaults_to_client_role() {
        let mut claims = sample_claims();
        claims.metadata = None;
        let user = AuthenticatedUser::from_claims(claims);
        assert_eq!(user.role, Role::Client);
    }

    #[test]
    fn has_role_checks_privilege() {
        let claims = sample_claims();
        let user = AuthenticatedUser::from_claims(claims);

        // Admin has all privileges
        assert!(user.has_role(Role::Admin));
        assert!(user.has_role(Role::Client));
        assert!(user.has_role(Role::Support));
    }
}
