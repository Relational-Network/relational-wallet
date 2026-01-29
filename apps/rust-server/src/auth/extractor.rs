// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Axum extractor for authenticated users.
//!
//! Use the `Auth` extractor in handlers to require authentication:
//!
//! ```rust,ignore
//! async fn my_handler(Auth(user): Auth) -> impl IntoResponse {
//!     // user is AuthenticatedUser
//! }
//! ```

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{decode, decode_header, Validation};
use serde::Deserialize;

use super::{AuthenticatedUser, AuthError, Role};
use crate::state::AppState;

/// Clock skew tolerance (60 seconds).
const CLOCK_SKEW_LEEWAY: u64 = 60;

/// Minimal JWT claims for decoding Clerk tokens.
#[derive(Debug, Deserialize)]
struct JwtClaims {
    /// Subject (user ID)
    sub: String,
    /// Issued at timestamp
    #[serde(default)]
    #[allow(dead_code)]
    iat: i64,
    /// Expiration timestamp
    #[serde(default)]
    exp: i64,
    /// Issuer
    #[serde(default)]
    iss: String,
    /// Session ID (Clerk-specific)
    #[serde(default)]
    sid: Option<String>,
    /// Audience (validated by jsonwebtoken crate, not read directly)
    #[serde(default)]
    #[allow(dead_code)]
    aud: Option<serde_json::Value>,
    /// Clerk public metadata containing role
    #[serde(default, rename = "publicMetadata")]
    public_metadata: Option<PublicMetadata>,
}

/// Clerk public metadata structure.
#[derive(Debug, Deserialize, Default)]
struct PublicMetadata {
    /// User's role (set in Clerk dashboard)
    #[serde(default)]
    role: Option<String>,
}

/// Extractor for authenticated users.
///
/// This extractor validates the JWT from the Authorization header
/// and provides the authenticated user information.
///
/// ## Authentication Modes
///
/// - **Production mode** (CLERK_JWKS_URL set): Full JWT verification against Clerk JWKS
/// - **Development mode** (no CLERK_JWKS_URL): Structure validation only (no signature check)
///
/// # Example
///
/// ```rust,ignore
/// async fn list_wallets(
///     Auth(user): Auth,
///     State(state): State<AppState>,
/// ) -> Result<Json<Vec<Wallet>>, ApiError> {
///     // user.user_id contains the authenticated user's ID
///     // user.role contains their role
/// }
/// ```
pub struct Auth(pub AuthenticatedUser);

impl FromRequestParts<AppState> for Auth {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // First check if middleware already set the user
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>().cloned() {
            return Ok(Auth(user));
        }

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(AuthError::MissingAuthHeader)?
            .to_str()
            .map_err(|_| AuthError::InvalidAuthHeader)?;

        // Extract Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidAuthHeader)?;

        // Decode and verify the JWT
        let user = verify_jwt(token, &state.auth_config).await?;

        Ok(Auth(user))
    }
}

/// Verify JWT and extract user information.
///
/// In production mode (JWKS configured), verifies signature against Clerk JWKS.
/// In development mode, only validates structure (no signature verification).
async fn verify_jwt(token: &str, auth_config: &crate::state::AuthConfig) -> Result<AuthenticatedUser, AuthError> {
    // Check if production mode is enabled
    if let Some(ref jwks) = auth_config.jwks {
        // Production mode: full JWKS verification
        verify_jwt_production(token, jwks, auth_config).await
    } else {
        // Development mode: decode without signature verification
        verify_jwt_development(token)
    }
}

/// Production JWT verification with JWKS.
async fn verify_jwt_production(
    token: &str,
    jwks: &super::JwksManager,
    auth_config: &crate::state::AuthConfig,
) -> Result<AuthenticatedUser, AuthError> {
    // Decode header to get kid (key ID)
    let header = decode_header(token).map_err(|_| AuthError::MalformedToken)?;

    // Get decoding key from JWKS
    let (decoding_key, algorithm) = if let Some(kid) = &header.kid {
        jwks.get_decoding_key(kid).await?
    } else {
        // No kid in header, try any key
        jwks.get_any_decoding_key().await?
    };

    // Build validation
    let mut validation = Validation::new(algorithm);
    validation.leeway = CLOCK_SKEW_LEEWAY;

    // Validate issuer if configured
    if let Some(ref issuer) = auth_config.issuer {
        validation.set_issuer(&[issuer]);
    } else {
        validation.validate_aud = false;
    }

    // Validate audience if configured
    if let Some(ref audience) = auth_config.audience {
        validation.set_audience(&[audience]);
    } else {
        validation.validate_aud = false;
    }

    // Decode and validate token
    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidSignature,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => AuthError::InvalidIssuer,
            jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::InvalidAudience,
            jsonwebtoken::errors::ErrorKind::ImmatureSignature => AuthError::TokenNotYetValid,
            _ => AuthError::MalformedToken,
        })?;

    let claims = token_data.claims;

    // Extract role from public metadata (default to Client)
    let role = claims
        .public_metadata
        .as_ref()
        .and_then(|m| m.role.as_ref())
        .and_then(|r| Role::from_str(r))
        .unwrap_or(Role::Client);

    Ok(AuthenticatedUser {
        user_id: claims.sub,
        role,
        session_id: claims.sid,
        issuer: claims.iss,
        expires_at: claims.exp,
    })
}

/// Development JWT verification (no signature check).
///
/// WARNING: This should only be used in development environments.
fn verify_jwt_development(token: &str) -> Result<AuthenticatedUser, AuthError> {
    // Use the dangerous decode API to skip signature verification
    let token_data = jsonwebtoken::dangerous::insecure_decode::<JwtClaims>(token)
        .map_err(|_e| AuthError::MalformedToken)?;

    let claims = token_data.claims;

    // Check expiration manually
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if claims.exp > 0 && claims.exp < now - CLOCK_SKEW_LEEWAY as i64 {
        return Err(AuthError::TokenExpired);
    }

    // Extract role from public metadata (default to Client)
    let role = claims
        .public_metadata
        .as_ref()
        .and_then(|m| m.role.as_ref())
        .and_then(|r| Role::from_str(r))
        .unwrap_or(Role::Client);

    Ok(AuthenticatedUser {
        user_id: claims.sub,
        role,
        session_id: claims.sid,
        issuer: claims.iss,
        expires_at: claims.exp,
    })
}

/// Extractor that requires a specific role.
///
/// TODO: Use this when implementing role-specific endpoints
///
/// # Example
///
/// ```rust,ignore
/// async fn admin_only(
///     RequireRole(user, Role::Admin): RequireRole<{ Role::Admin as u8 }>,
/// ) -> impl IntoResponse {
///     // Only admins can reach here
/// }
/// ```
#[allow(dead_code)]
pub struct RequireRole<const R: u8>(pub AuthenticatedUser);

impl<const R: u8> FromRequestParts<AppState> for RequireRole<R> {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let Auth(user) = Auth::from_request_parts(parts, state).await?;

        // Convert const to Role
        let required_role = match R {
            0 => Role::Admin,
            1 => Role::Client,
            2 => Role::Support,
            3 => Role::Auditor,
            _ => Role::Admin, // Safest default
        };

        if !user.has_role(required_role) {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(RequireRole(user))
    }
}

/// Extractor that requires admin role.
pub struct AdminOnly(pub AuthenticatedUser);

impl FromRequestParts<AppState> for AdminOnly {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let Auth(user) = Auth::from_request_parts(parts, state).await?;

        if !user.is_admin() {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(AdminOnly(user))
    }
}

/// Optional authentication extractor.
///
/// Returns `None` if no valid authentication is present, instead of rejecting.
/// TODO: Use for public endpoints that can optionally show user-specific data
#[allow(dead_code)]
pub struct OptionalAuth(pub Option<AuthenticatedUser>);

impl FromRequestParts<AppState> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // Try to authenticate, but don't fail if it doesn't work
        match Auth::from_request_parts(parts, state).await {
            Ok(Auth(user)) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, AuthConfig};
    use crate::storage::{EncryptedStorage, StoragePaths};
    use axum::http::Request;
    use tempfile::TempDir;

    /// Helper to create a test AppState with no JWKS (development mode)
    fn create_test_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let paths = StoragePaths::new(temp_dir.path());
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().expect("Failed to initialize storage");
        
        let state = AppState::new(storage)
            .with_auth_config(AuthConfig {
                jwks: None,
                issuer: Some("test".to_string()),
                audience: None,
            });
        (state, temp_dir)
    }

    /// Helper to create a test JWT token (unsigned, for testing only)
    fn create_test_jwt(user_id: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        
        let header = r#"{"alg":"RS256","typ":"JWT"}"#;
        let claims = format!(
            r#"{{"sub":"{}","iat":1609459200,"exp":9999999999,"iss":"test","sid":"sess_123"}}"#,
            user_id
        );
        
        let header_b64 = URL_SAFE_NO_PAD.encode(header.as_bytes());
        let claims_b64 = URL_SAFE_NO_PAD.encode(claims.as_bytes());
        
        // For testing, signature doesn't matter since we use development mode
        format!("{}.{}.fake_signature", header_b64, claims_b64)
    }

    #[tokio::test]
    async fn auth_extractor_requires_auth_header() {
        let (state, _temp_dir) = create_test_state();
        let mut parts = Request::builder()
            .uri("/test")
            .body(())
            .unwrap()
            .into_parts()
            .0;

        // Without auth header, should fail
        let result = Auth::from_request_parts(&mut parts, &state).await;
        assert!(matches!(result, Err(AuthError::MissingAuthHeader)));
    }

    #[tokio::test]
    async fn auth_extractor_succeeds_with_jwt() {
        let (state, _temp_dir) = create_test_state();
        let token = create_test_jwt("user_123");
        let mut parts = Request::builder()
            .uri("/test")
            .header("Authorization", format!("Bearer {}", token))
            .body(())
            .unwrap()
            .into_parts()
            .0;

        let result = Auth::from_request_parts(&mut parts, &state).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.user_id, "user_123");
    }

    #[tokio::test]
    async fn auth_extractor_prefers_extensions() {
        let (state, _temp_dir) = create_test_state();
        // If middleware already set the user, use that
        let mut parts = Request::builder()
            .uri("/test")
            .body(())
            .unwrap()
            .into_parts()
            .0;

        let user = AuthenticatedUser {
            user_id: "user_from_middleware".to_string(),
            role: Role::Admin,
            session_id: None,
            issuer: "middleware".to_string(),
            expires_at: 0,
        };
        parts.extensions.insert(user.clone());

        let result = Auth::from_request_parts(&mut parts, &state).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.user_id, "user_from_middleware");
    }

    #[tokio::test]
    async fn admin_only_rejects_non_admin() {
        let (state, _temp_dir) = create_test_state();
        let mut parts = Request::builder()
            .uri("/test")
            .body(())
            .unwrap()
            .into_parts()
            .0;

        let user = AuthenticatedUser {
            user_id: "user_123".to_string(),
            role: Role::Client, // Not admin
            session_id: None,
            issuer: "test".to_string(),
            expires_at: 0,
        };
        parts.extensions.insert(user);

        let result = AdminOnly::from_request_parts(&mut parts, &state).await;
        assert!(matches!(result, Err(AuthError::InsufficientPermissions)));
    }

    #[tokio::test]
    async fn optional_auth_returns_none_without_user() {
        let (state, _temp_dir) = create_test_state();
        let mut parts = Request::builder()
            .uri("/test")
            .body(())
            .unwrap()
            .into_parts()
            .0;

        let result = OptionalAuth::from_request_parts(&mut parts, &state).await;
        assert!(result.is_ok());
        assert!(result.unwrap().0.is_none());
    }
}
