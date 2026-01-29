// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Ownership enforcement for all storage operations.
//!
//! This module provides traits and implementations for strict ownership
//! validation. Every data access must pass through ownership checks.

use crate::auth::AuthenticatedUser;

use super::{StorageError, StorageResult};

/// Trait for resources that have an owner.
pub trait OwnedResource {
    /// Get the owner's user ID.
    fn owner_user_id(&self) -> &str;
}

/// Trait for enforcing ownership on storage operations.
pub trait OwnershipEnforcer {
    /// Verify that the user owns this resource.
    ///
    /// # Errors
    /// Returns `StorageError::PermissionDenied` if the user doesn't own the resource.
    fn verify_ownership(&self, user: &AuthenticatedUser) -> StorageResult<()>;
}

impl<T: OwnedResource> OwnershipEnforcer for T {
    fn verify_ownership(&self, user: &AuthenticatedUser) -> StorageResult<()> {
        if self.owner_user_id() == user.user_id {
            Ok(())
        } else {
            Err(StorageError::PermissionDenied {
                user_id: user.user_id.clone(),
                resource: "resource".to_string(),
            })
        }
    }
}

/// Extension trait for optional ownership verification.
/// TODO: Use when implementing admin views that need ownership checks on Results
#[allow(dead_code)]
pub trait OwnershipCheck<T> {
    /// Verify ownership and return the resource if authorized.
    fn verify_owner(self, user: &AuthenticatedUser) -> StorageResult<T>;
}

impl<T: OwnedResource> OwnershipCheck<T> for StorageResult<T> {
    fn verify_owner(self, user: &AuthenticatedUser) -> StorageResult<T> {
        let resource = self?;
        resource.verify_ownership(user)?;
        Ok(resource)
    }
}

impl<T: OwnedResource> OwnershipCheck<T> for Option<T> {
    fn verify_owner(self, user: &AuthenticatedUser) -> StorageResult<T> {
        match self {
            Some(resource) => {
                resource.verify_ownership(user)?;
                Ok(resource)
            }
            None => Err(StorageError::NotFoundResource {
                resource: "resource".to_string(),
                id: "unknown".to_string(),
            }),
        }
    }
}

/// Marker trait for resources that can be accessed without ownership check.
///
/// Use sparingly - only for truly public data or admin operations.
/// TODO: Use when implementing public endpoints
#[allow(dead_code)]
pub trait PublicResource {}

/// Trait for admin-level access that bypasses ownership checks.
/// TODO: Use when implementing admin-only operations
#[allow(dead_code)]
pub trait AdminAccess {
    /// Check if the user has admin privileges for this operation.
    fn check_admin_access(&self, user: &AuthenticatedUser) -> StorageResult<()> {
        if user.role.has_privilege(crate::auth::Role::Admin) {
            Ok(())
        } else {
            Err(StorageError::PermissionDenied {
                user_id: user.user_id.clone(),
                resource: "admin operation".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Role;

    struct TestResource {
        owner: String,
    }

    impl OwnedResource for TestResource {
        fn owner_user_id(&self) -> &str {
            &self.owner
        }
    }

    fn make_user(user_id: &str, role: Role) -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: user_id.to_string(),
            role,
            session_id: None,
            issuer: "test".to_string(),
            expires_at: 0,
        }
    }

    #[test]
    fn ownership_verification_passes_for_owner() {
        let resource = TestResource {
            owner: "user_123".to_string(),
        };
        let user = make_user("user_123", Role::Client);

        assert!(resource.verify_ownership(&user).is_ok());
    }

    #[test]
    fn ownership_verification_fails_for_non_owner() {
        let resource = TestResource {
            owner: "user_123".to_string(),
        };
        let user = make_user("user_456", Role::Client);

        let result = resource.verify_ownership(&user);
        assert!(matches!(result, Err(StorageError::PermissionDenied { .. })));
    }

    #[test]
    fn ownership_check_on_result() {
        let resource = TestResource {
            owner: "user_123".to_string(),
        };
        let user = make_user("user_123", Role::Client);

        let result: StorageResult<TestResource> = Ok(resource);
        assert!(result.verify_owner(&user).is_ok());
    }

    #[test]
    fn ownership_check_on_option_some() {
        let resource = TestResource {
            owner: "user_123".to_string(),
        };
        let user = make_user("user_123", Role::Client);

        let option: Option<TestResource> = Some(resource);
        assert!(option.verify_owner(&user).is_ok());
    }

    #[test]
    fn ownership_check_on_option_none() {
        let user = make_user("user_123", Role::Client);

        let option: Option<TestResource> = None;
        let result = option.verify_owner(&user);
        assert!(matches!(result, Err(StorageError::NotFoundResource { .. })));
    }

    struct AdminOp;
    impl AdminAccess for AdminOp {}

    #[test]
    fn admin_access_passes_for_admin() {
        let op = AdminOp;
        let user = make_user("admin_1", Role::Admin);

        assert!(op.check_admin_access(&user).is_ok());
    }

    #[test]
    fn admin_access_fails_for_client() {
        let op = AdminOp;
        let user = make_user("user_1", Role::Client);

        let result = op.check_admin_access(&user);
        assert!(matches!(result, Err(StorageError::PermissionDenied { .. })));
    }
}
