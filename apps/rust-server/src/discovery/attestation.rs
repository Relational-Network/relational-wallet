// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! SGX attestation policy and custom rustls `ServerCertVerifier`.
//!
//! Uses Gramine's `libra_tls_verify_dcap.so` via FFI to verify peer
//! RA-TLS certificates. The library handles DCAP collateral fetching,
//! QE identity checks, TCB status evaluation, and quote signature
//! verification. We only check MRENCLAVE/MRSIGNER/ISV fields in the
//! measurement callback.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};

use super::ffi;

// =============================================================================
// Attestation Policy
// =============================================================================

/// Attestation policy for verifying a peer enclave's identity.
///
/// Checked inside the measurement callback during `ra_tls_verify_callback_der`.
/// Gramine's library handles debug mode rejection and TCB status checks
/// automatically in production builds.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AttestationPolicy {
    /// Expected MRENCLAVE (32 bytes). Must be an exact match.
    /// This is the hash of the enclave binary — ensures same code.
    #[serde(
        serialize_with = "serialize_hex_32",
        deserialize_with = "deserialize_hex_32"
    )]
    pub mrenclave: [u8; 32],

    /// Optional MRSIGNER pin (32 bytes). If set, must be an exact match.
    /// Same org → pin; different operators → rely on MRENCLAVE only.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_hex_32_opt",
        deserialize_with = "deserialize_hex_32_opt"
    )]
    pub mrsigner: Option<[u8; 32]>,

    /// Minimum ISV SVN (security version number). Reject if lower.
    #[serde(default)]
    pub min_isv_svn: u16,

    /// Expected ISV Product ID. Must be an exact match.
    #[serde(default)]
    pub isv_prod_id: u16,
}

// =============================================================================
// Hex Serialization Helpers
// =============================================================================

fn serialize_hex_32<S: serde::Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&alloy::hex::encode(bytes))
}

fn deserialize_hex_32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
    let hex_str: String = serde::Deserialize::deserialize(d)?;
    let bytes = alloy::hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| serde::de::Error::custom("expected 32 bytes (64 hex chars)"))?;
    Ok(arr)
}

fn serialize_hex_32_opt<S: serde::Serializer>(
    opt: &Option<[u8; 32]>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match opt {
        Some(bytes) => s.serialize_some(&alloy::hex::encode(bytes)),
        None => s.serialize_none(),
    }
}

fn deserialize_hex_32_opt<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<[u8; 32]>, D::Error> {
    let opt: Option<String> = serde::Deserialize::deserialize(d)?;
    match opt {
        Some(hex_str) => {
            let bytes = alloy::hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
            let arr: [u8; 32] = bytes
                .try_into()
                .map_err(|_| serde::de::Error::custom("expected 32 bytes (64 hex chars)"))?;
            Ok(Some(arr))
        }
        None => Ok(None),
    }
}

// =============================================================================
// Custom rustls ServerCertVerifier
// =============================================================================

/// A rustls `ServerCertVerifier` that uses Gramine's RA-TLS verification.
///
/// When connecting to a peer, rustls calls `verify_server_cert()` which
/// extracts the DER certificate and delegates to Gramine's
/// `ra_tls_verify_callback_der()` for full DCAP attestation verification.
#[derive(Debug)]
pub struct RaTlsServerVerifier {
    policy: AttestationPolicy,
    crypto_provider: Arc<CryptoProvider>,
}

impl RaTlsServerVerifier {
    /// Create a new verifier with the given attestation policy.
    pub fn new(policy: AttestationPolicy) -> Self {
        Self {
            policy,
            crypto_provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }
}

impl ServerCertVerifier for RaTlsServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        // Delegate to Gramine's DCAP RA-TLS verification
        ffi::verify_ratls_cert(end_entity.as_ref(), &self.policy).map_err(|e| {
            tracing::warn!(error = %e, "RA-TLS peer verification failed");
            TlsError::General(format!("RA-TLS attestation failed: {e}"))
        })?;

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.crypto_provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attestation_policy_serde_roundtrip() {
        let policy = AttestationPolicy {
            mrenclave: [0xab; 32],
            mrsigner: Some([0xcd; 32]),
            min_isv_svn: 1,
            isv_prod_id: 0,
        };

        let json = serde_json::to_string_pretty(&policy).unwrap();
        let parsed: AttestationPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.mrenclave, policy.mrenclave);
        assert_eq!(parsed.mrsigner, policy.mrsigner);
        assert_eq!(parsed.min_isv_svn, 1);
        assert_eq!(parsed.isv_prod_id, 0);
    }

    #[test]
    fn attestation_policy_without_mrsigner() {
        let json = r#"{
            "mrenclave": "abababababababababababababababababababababababababababababababab",
            "min_isv_svn": 0,
            "isv_prod_id": 0
        }"#;
        let policy: AttestationPolicy = serde_json::from_str(json).unwrap();
        assert!(policy.mrsigner.is_none());
        assert_eq!(policy.mrenclave, [0xab; 32]);
    }

    #[test]
    fn verifier_creation() {
        let policy = AttestationPolicy {
            mrenclave: [0x00; 32],
            mrsigner: None,
            min_isv_svn: 0,
            isv_prod_id: 0,
        };
        let _verifier = RaTlsServerVerifier::new(policy);
    }
}
