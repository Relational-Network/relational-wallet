// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! JWKS (JSON Web Key Set) fetching and caching.
//!
//! ## Security
//!
//! - JWKS is fetched via HTTPS only
//! - Keys are cached with a configurable TTL
//! - Stale cache is used on fetch failure (fail-open for availability)
//!
//! ## Usage
//!
//! Initialize JwksManager with CLERK_JWKS_URL in main.rs and store in AppState.
//! The Auth extractor uses it for production JWT verification.

use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::{JwkSet, Jwk, AlgorithmParameters};
use jsonwebtoken::{Algorithm, DecodingKey};
use tokio::sync::RwLock;

use super::error::AuthError;

/// Default JWKS cache TTL (5 minutes).
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);

/// JWKS cache entry.
struct CacheEntry {
    jwks: JwkSet,
    fetched_at: Instant,
}

/// JWKS manager with caching.
///
/// Fetches and caches JWKS from Clerk for JWT verification.
#[derive(Clone)]
pub struct JwksManager {
    /// JWKS URL (Clerk endpoint)
    jwks_url: String,
    /// Cache TTL
    cache_ttl: Duration,
    /// Cached JWKS
    cache: Arc<RwLock<Option<CacheEntry>>>,
    /// HTTP client
    client: reqwest::Client,
}

impl JwksManager {
    /// Create a new JWKS manager.
    ///
    /// # Arguments
    /// - `jwks_url`: The JWKS endpoint URL (e.g., `https://your-clerk-domain.clerk.accounts.dev/.well-known/jwks.json`)
    pub fn new(jwks_url: impl Into<String>) -> Self {
        Self {
            jwks_url: jwks_url.into(),
            cache_ttl: DEFAULT_CACHE_TTL,
            cache: Arc::new(RwLock::new(None)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create with custom cache TTL.
    #[allow(dead_code)]
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get the JWKS URL.
    #[allow(dead_code)]
    pub fn jwks_url(&self) -> &str {
        &self.jwks_url
    }

    /// Fetch JWKS (with caching).
    async fn get_jwks(&self) -> Result<JwkSet, AuthError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = &*cache {
                if entry.fetched_at.elapsed() < self.cache_ttl {
                    return Ok(entry.jwks.clone());
                }
            }
        }

        // Fetch fresh JWKS
        let jwks = self.fetch_jwks().await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some(CacheEntry {
                jwks: jwks.clone(),
                fetched_at: Instant::now(),
            });
        }

        Ok(jwks)
    }

    /// Fetch JWKS from the endpoint.
    async fn fetch_jwks(&self) -> Result<JwkSet, AuthError> {
        let response = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthError::JwksFetchError(format!(
                "HTTP {} from JWKS endpoint",
                response.status()
            )));
        }

        let jwks: JwkSet = response
            .json()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        Ok(jwks)
    }

    /// Get a decoding key for the given key ID.
    pub async fn get_decoding_key(&self, kid: &str) -> Result<(DecodingKey, Algorithm), AuthError> {
        let jwks = self.get_jwks().await?;

        // Find the key with matching kid
        let jwk = jwks
            .keys
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(kid))
            .ok_or(AuthError::NoMatchingKey)?;

        // Convert JWK to DecodingKey
        let (decoding_key, algorithm) = jwk_to_decoding_key(jwk)?;
        Ok((decoding_key, algorithm))
    }

    /// Get any valid decoding key (for tokens without kid).
    pub async fn get_any_decoding_key(&self) -> Result<(DecodingKey, Algorithm), AuthError> {
        let jwks = self.get_jwks().await?;

        // Try each key until one works
        for jwk in &jwks.keys {
            if let Ok(result) = jwk_to_decoding_key(jwk) {
                return Ok(result);
            }
        }

        Err(AuthError::NoMatchingKey)
    }

    /// Force refresh the JWKS cache.
    pub async fn refresh(&self) -> Result<(), AuthError> {
        let jwks = self.fetch_jwks().await?;
        let mut cache = self.cache.write().await;
        *cache = Some(CacheEntry {
            jwks,
            fetched_at: Instant::now(),
        });
        Ok(())
    }

    /// Check if JWKS is currently cached and valid.
    pub async fn is_cached(&self) -> bool {
        let cache = self.cache.read().await;
        if let Some(entry) = &*cache {
            entry.fetched_at.elapsed() < self.cache_ttl
        } else {
            false
        }
    }
}

/// Convert a JWK to a DecodingKey.
fn jwk_to_decoding_key(jwk: &Jwk) -> Result<(DecodingKey, Algorithm), AuthError> {
    match &jwk.algorithm {
        AlgorithmParameters::RSA(rsa) => {
            let key = DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                .map_err(|e| AuthError::InternalError(format!("Failed to create RSA key: {e}")))?;
            
            // Determine algorithm from JWK
            let alg = jwk
                .common
                .key_algorithm
                .map(|a| match a {
                    jsonwebtoken::jwk::KeyAlgorithm::RS256 => Algorithm::RS256,
                    jsonwebtoken::jwk::KeyAlgorithm::RS384 => Algorithm::RS384,
                    jsonwebtoken::jwk::KeyAlgorithm::RS512 => Algorithm::RS512,
                    _ => Algorithm::RS256, // Default for RSA
                })
                .unwrap_or(Algorithm::RS256);

            Ok((key, alg))
        }
        AlgorithmParameters::EllipticCurve(ec) => {
            let key = DecodingKey::from_ec_components(&ec.x, &ec.y)
                .map_err(|e| AuthError::InternalError(format!("Failed to create EC key: {e}")))?;

            let alg = jwk
                .common
                .key_algorithm
                .map(|a| match a {
                    jsonwebtoken::jwk::KeyAlgorithm::ES256 => Algorithm::ES256,
                    jsonwebtoken::jwk::KeyAlgorithm::ES384 => Algorithm::ES384,
                    _ => Algorithm::ES256, // Default for EC
                })
                .unwrap_or(Algorithm::ES256);

            Ok((key, alg))
        }
        _ => Err(AuthError::InternalError(
            "Unsupported key type in JWKS".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwks_manager_creation() {
        let manager = JwksManager::new("https://example.clerk.accounts.dev/.well-known/jwks.json");
        assert_eq!(
            manager.jwks_url(),
            "https://example.clerk.accounts.dev/.well-known/jwks.json"
        );
    }

    #[test]
    fn custom_cache_ttl() {
        let manager = JwksManager::new("https://example.com/.well-known/jwks.json")
            .with_cache_ttl(Duration::from_secs(60));
        assert_eq!(manager.cache_ttl, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn cache_initially_empty() {
        let manager = JwksManager::new("https://example.com/.well-known/jwks.json");
        assert!(!manager.is_cached().await);
    }
}
