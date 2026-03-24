// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Email normalization, hashing, and HMAC utilities for email-linked wallets.
//!
//! ## Normalization Spec (Frozen)
//!
//! Rules applied in order:
//! 1. Trim leading/trailing whitespace
//! 2. Lowercase the entire address
//! 3. NFC Unicode normalization
//! 4. Validate RFC 5322 format
//!
//! These rules **must not change** once deployed — if rules change,
//! existing lookup keys break.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

type HmacSha256 = Hmac<Sha256>;

/// Errors from email operations.
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Invalid email format: {0}")]
    InvalidFormat(String),
}

/// Normalize email per frozen spec: trim → lowercase → NFC → validate.
///
/// Identical output to the TypeScript `normalizeEmail()` in `lib/emailHash.ts`.
pub fn normalize_email(email: &str) -> Result<String, EmailError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(EmailError::InvalidFormat("empty email".to_string()));
    }

    let lowered = trimmed.to_lowercase();
    let nfc: String = lowered.nfc().collect();

    // RFC 5322 basic validation: local@domain.tld
    if !is_valid_email(&nfc) {
        return Err(EmailError::InvalidFormat(format!("'{nfc}' is not a valid email address")));
    }

    Ok(nfc)
}

/// SHA-256 hash of a normalized email string (hex-encoded).
///
/// Same output as the frontend's `hashEmail()` — cross-platform verified.
pub fn sha256_email(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    alloy::hex::encode(result)
}

/// HMAC-SHA256(node_secret, email_sha256_hex) → lookup key (hex-encoded).
///
/// Two-layer protection:
/// - Layer 1 (client-side): SHA-256 strips PII before transit
/// - Layer 2 (server-side): HMAC-SHA256 resists offline brute-force if DB leaks
pub fn hmac_lookup_key(hmac_key: &[u8; 32], email_sha256: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(hmac_key)
        .expect("HMAC can take key of any size");
    mac.update(email_sha256.as_bytes());
    let result = mac.finalize();
    alloy::hex::encode(result.into_bytes())
}

/// Validate email_hash is a valid hex-encoded SHA-256 (64 hex chars).
pub fn validate_email_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

/// Basic RFC 5322 email validation.
fn is_valid_email(email: &str) -> bool {
    // Must contain exactly one @
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    // Local part must not be empty
    if local.is_empty() {
        return false;
    }

    // Domain must contain at least one dot, must not be empty
    if domain.is_empty() || !domain.contains('.') {
        return false;
    }

    // Domain parts must not be empty
    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.iter().any(|p| p.is_empty()) {
        return false;
    }

    // No whitespace allowed
    if email.chars().any(|c| c.is_whitespace()) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Normalization test vectors (frozen spec) ──────────────────

    #[test]
    fn normalize_case_fold() {
        assert_eq!(normalize_email("Alice@Example.COM").unwrap(), "alice@example.com");
    }

    #[test]
    fn normalize_trim() {
        assert_eq!(normalize_email("  bob@test.org  ").unwrap(), "bob@test.org");
    }

    #[test]
    fn normalize_unicode_nfc() {
        // NFD café → NFC café
        let nfd_cafe = "cafe\u{0301}@example.com";
        let result = normalize_email(nfd_cafe).unwrap();
        assert_eq!(result, "caf\u{00e9}@example.com");
    }

    #[test]
    fn normalize_admin_gmail() {
        assert_eq!(normalize_email("ADMIN@GMAIL.COM").unwrap(), "admin@gmail.com");
    }

    #[test]
    fn normalize_rejects_invalid() {
        assert!(normalize_email("not-an-email").is_err());
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_email("").is_err());
    }

    // ── SHA-256 tests ─────────────────────────────────────────────

    #[test]
    fn sha256_deterministic() {
        let hash1 = sha256_email("alice@example.com");
        let hash2 = sha256_email("alice@example.com");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn sha256_different_for_different_emails() {
        let hash1 = sha256_email("alice@example.com");
        let hash2 = sha256_email("bob@example.com");
        assert_ne!(hash1, hash2);
    }

    // ── HMAC tests ────────────────────────────────────────────────

    #[test]
    fn hmac_deterministic() {
        let key = [0u8; 32];
        let hash = sha256_email("alice@example.com");
        let hmac1 = hmac_lookup_key(&key, &hash);
        let hmac2 = hmac_lookup_key(&key, &hash);
        assert_eq!(hmac1, hmac2);
        assert_eq!(hmac1.len(), 64);
    }

    #[test]
    fn hmac_different_for_different_emails() {
        let key = [0u8; 32];
        let hash1 = sha256_email("alice@example.com");
        let hash2 = sha256_email("bob@example.com");
        let hmac1 = hmac_lookup_key(&key, &hash1);
        let hmac2 = hmac_lookup_key(&key, &hash2);
        assert_ne!(hmac1, hmac2);
    }

    #[test]
    fn hmac_different_for_different_keys() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let hash = sha256_email("alice@example.com");
        let hmac1 = hmac_lookup_key(&key1, &hash);
        let hmac2 = hmac_lookup_key(&key2, &hash);
        assert_ne!(hmac1, hmac2);
    }

    // ── email_hash validation ─────────────────────────────────────

    #[test]
    fn validate_email_hash_accepts_valid() {
        let hash = sha256_email("alice@example.com");
        assert!(validate_email_hash(&hash));
    }

    #[test]
    fn validate_email_hash_rejects_short() {
        assert!(!validate_email_hash("abc123"));
    }

    #[test]
    fn validate_email_hash_rejects_non_hex() {
        assert!(!validate_email_hash(&"g".repeat(64)));
    }
}
