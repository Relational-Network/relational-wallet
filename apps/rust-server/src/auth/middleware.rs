// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Authentication middleware for Axum.
//!
//! This module provides middleware-based authentication as an alternative to
//! the extractor-based approach in `extractor.rs`.
//!
//! ## Current Status
//!
//! This module is implemented but not currently used. The `Auth` extractor
//! in `extractor.rs` handles authentication directly using AppState,
//! which provides better ergonomics for most handlers.
//!
//! This middleware approach is available if you need to:
//! - Apply authentication to an entire router subtree
//! - Pre-authenticate before handler execution
//! - Use a different authentication flow

#![allow(dead_code)] // Middleware approach not currently used

use std::sync::Arc;

use axum::{
    extract::Request,
    http::header::AUTHORIZATION,
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, decode_header, Validation};

use super::jwks::JwksManager;
use super::{claims::ClerkClaims, AuthError, AuthenticatedUser};

/// Clock skew tolerance (60 seconds).
const CLOCK_SKEW_LEEWAY: u64 = 60;

/// Authentication configuration.
#[derive(Clone)]
pub struct AuthConfig {
    /// JWKS manager for key fetching
    pub jwks: Arc<JwksManager>,
    /// Expected issuer (Clerk instance URL)
    pub issuer: String,
    /// Expected audience (optional)
    pub audience: Option<String>,
}

impl AuthConfig {
    /// Create a new auth configuration.
    ///
    /// # Arguments
    /// - `jwks_url`: URL to fetch JWKS from
    /// - `issuer`: Expected token issuer (your Clerk instance URL)
    pub fn new(jwks_url: impl Into<String>, issuer: impl Into<String>) -> Self {
        Self {
            jwks: Arc::new(JwksManager::new(jwks_url)),
            issuer: issuer.into(),
            audience: None,
        }
    }

    /// Set the expected audience.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }
}

/// Create the authentication middleware layer.
///
/// # Usage
///
/// ```rust,ignore
/// let auth_config = AuthConfig::new(
///     "https://your-clerk.clerk.accounts.dev/.well-known/jwks.json",
///     "https://your-clerk.clerk.accounts.dev",
/// );
///
/// let app = Router::new()
///     .route("/protected", get(protected_handler))
///     .layer(axum::middleware::from_fn_with_state(
///         auth_config.clone(),
///         auth_middleware,
///     ));
/// ```
pub fn auth_layer(
    _config: AuthConfig,
) -> axum::middleware::FromFnLayer<
    fn(
        axum::extract::State<AuthConfig>,
        Request,
        Next,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>,
    AuthConfig,
    (axum::extract::State<AuthConfig>, Request, Next),
> {
    // Note: The type signature here is complex because of axum's middleware system.
    // In practice, use `axum::middleware::from_fn_with_state(config, auth_middleware)` directly.
    todo!("Use axum::middleware::from_fn_with_state(config, auth_middleware) directly")
}

/// Authentication middleware function.
pub async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<AuthConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = match request.headers().get(AUTHORIZATION) {
        Some(header) => header,
        None => return AuthError::MissingAuthHeader.into_response(),
    };

    // Parse Bearer token
    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return AuthError::InvalidAuthHeader.into_response(),
    };

    let token = match auth_str.strip_prefix("Bearer ") {
        Some(t) => t.trim(),
        None => return AuthError::InvalidAuthHeader.into_response(),
    };

    // Validate token and extract user
    match validate_token(token, &config).await {
        Ok(user) => {
            // Add authenticated user to request extensions
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        Err(e) => e.into_response(),
    }
}

/// Validate a JWT token and return the authenticated user.
async fn validate_token(token: &str, config: &AuthConfig) -> Result<AuthenticatedUser, AuthError> {
    // Decode header to get kid
    let header = decode_header(token).map_err(|_| AuthError::MalformedToken)?;

    // Get decoding key from JWKS
    let (decoding_key, algorithm) = if let Some(kid) = &header.kid {
        config.jwks.get_decoding_key(kid).await?
    } else {
        // No kid, try any key
        config.jwks.get_any_decoding_key().await?
    };

    // Build validation
    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[&config.issuer]);
    validation.leeway = CLOCK_SKEW_LEEWAY;

    if let Some(aud) = &config.audience {
        validation.set_audience(&[aud]);
    } else {
        validation.validate_aud = false;
    }

    // Decode and validate token
    let token_data =
        decode::<ClerkClaims>(token, &decoding_key, &validation).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidSignature,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => AuthError::InvalidIssuer,
            jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::InvalidAudience,
            jsonwebtoken::errors::ErrorKind::ImmatureSignature => AuthError::TokenNotYetValid,
            _ => AuthError::MalformedToken,
        })?;

    Ok(AuthenticatedUser::from_claims(token_data.claims))
}

/// Middleware that skips authentication for certain paths.
///
/// Use this for health check endpoints that should be accessible without auth.
pub async fn skip_auth_for_paths(request: Request, next: Next, skip_paths: &[&str]) -> Response {
    let path = request.uri().path();

    for skip_path in skip_paths {
        if path.starts_with(skip_path) {
            return next.run(request).await;
        }
    }

    // Path requires auth - this will be handled by the auth middleware
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_config_creation() {
        let config = AuthConfig::new(
            "https://example.clerk.accounts.dev/.well-known/jwks.json",
            "https://example.clerk.accounts.dev",
        );
        assert_eq!(config.issuer, "https://example.clerk.accounts.dev");
        assert!(config.audience.is_none());
    }

    #[test]
    fn auth_config_with_audience() {
        let config = AuthConfig::new(
            "https://example.clerk.accounts.dev/.well-known/jwks.json",
            "https://example.clerk.accounts.dev",
        )
        .with_audience("my-app");
        assert_eq!(config.audience, Some("my-app".to_string()));
    }
}
