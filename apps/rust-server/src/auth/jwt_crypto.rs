// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Custom `jsonwebtoken` crypto provider backed by `ring`.
//!
//! We only verify JWTs in this service, so we keep jsonwebtoken's parsing and
//! claim validation while routing signature checks through `ring` to avoid the
//! vulnerable/transitively vulnerable bundled backends.

use jsonwebtoken::{
    crypto::{CryptoProvider, JwkUtils, JwtSigner, JwtVerifier},
    errors::{new_error, Error as JwtError, ErrorKind, Result as JwtResult},
    signature::{Error as SignatureError, Verifier},
    Algorithm, AlgorithmFamily, DecodingKey, DecodingKeyKind, EncodingKey,
};
use ring::signature::{
    self as ring_signature, EcdsaVerificationAlgorithm, RsaParameters, UnparsedPublicKey,
};

static JWT_CRYPTO_PROVIDER: CryptoProvider = CryptoProvider {
    signer_factory: unsupported_signer,
    verifier_factory: new_verifier,
    jwk_utils: JwkUtils::new_unimplemented(),
};

/// Install the process-wide JWT crypto provider once.
pub fn ensure_provider_installed() {
    let _ = JWT_CRYPTO_PROVIDER.install_default();
}

fn unsupported_signer(_algorithm: &Algorithm, _key: &EncodingKey) -> JwtResult<Box<dyn JwtSigner>> {
    Err(new_error(ErrorKind::Provider(
        "JWT signing is not supported by the custom ring provider".to_string(),
    )))
}

fn new_verifier(algorithm: &Algorithm, key: &DecodingKey) -> JwtResult<Box<dyn JwtVerifier>> {
    match algorithm {
        Algorithm::RS256 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::RS256,
            &ring_signature::RSA_PKCS1_2048_8192_SHA256,
        )?)),
        Algorithm::RS384 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::RS384,
            &ring_signature::RSA_PKCS1_2048_8192_SHA384,
        )?)),
        Algorithm::RS512 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::RS512,
            &ring_signature::RSA_PKCS1_2048_8192_SHA512,
        )?)),
        Algorithm::PS256 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::PS256,
            &ring_signature::RSA_PSS_2048_8192_SHA256,
        )?)),
        Algorithm::PS384 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::PS384,
            &ring_signature::RSA_PSS_2048_8192_SHA384,
        )?)),
        Algorithm::PS512 => Ok(Box::new(RsaVerifier::new(
            key,
            Algorithm::PS512,
            &ring_signature::RSA_PSS_2048_8192_SHA512,
        )?)),
        Algorithm::ES256 => Ok(Box::new(EcdsaVerifier::new(
            key,
            Algorithm::ES256,
            &ring_signature::ECDSA_P256_SHA256_FIXED,
        )?)),
        Algorithm::ES384 => Ok(Box::new(EcdsaVerifier::new(
            key,
            Algorithm::ES384,
            &ring_signature::ECDSA_P384_SHA384_FIXED,
        )?)),
        _ => Err(new_error(ErrorKind::InvalidAlgorithm)),
    }
}

struct RsaVerifier {
    key: DecodingKey,
    algorithm: Algorithm,
    parameters: &'static RsaParameters,
}

impl RsaVerifier {
    fn new(
        key: &DecodingKey,
        algorithm: Algorithm,
        parameters: &'static RsaParameters,
    ) -> JwtResult<Self> {
        if key.family() != AlgorithmFamily::Rsa {
            return Err(new_error(ErrorKind::InvalidKeyFormat));
        }

        Ok(Self {
            key: key.clone(),
            algorithm,
            parameters,
        })
    }
}

impl Verifier<Vec<u8>> for RsaVerifier {
    fn verify(&self, msg: &[u8], signature: &Vec<u8>) -> Result<(), SignatureError> {
        match self.key.kind() {
            DecodingKeyKind::SecretOrDer(bytes) => UnparsedPublicKey::new(self.parameters, bytes)
                .verify(msg, signature)
                .map_err(signature_error),
            DecodingKeyKind::RsaModulusExponent { n, e } => {
                ring_signature::RsaPublicKeyComponents {
                    n: n.as_slice(),
                    e: e.as_slice(),
                }
                .verify(self.parameters, msg, signature)
                .map_err(signature_error)
            }
        }
    }
}

impl JwtVerifier for RsaVerifier {
    fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
}

struct EcdsaVerifier {
    key: DecodingKey,
    algorithm: Algorithm,
    parameters: &'static EcdsaVerificationAlgorithm,
}

impl EcdsaVerifier {
    fn new(
        key: &DecodingKey,
        algorithm: Algorithm,
        parameters: &'static EcdsaVerificationAlgorithm,
    ) -> JwtResult<Self> {
        if key.family() != AlgorithmFamily::Ec {
            return Err(new_error(ErrorKind::InvalidKeyFormat));
        }

        Ok(Self {
            key: key.clone(),
            algorithm,
            parameters,
        })
    }
}

impl Verifier<Vec<u8>> for EcdsaVerifier {
    fn verify(&self, msg: &[u8], signature: &Vec<u8>) -> Result<(), SignatureError> {
        UnparsedPublicKey::new(self.parameters, self.key.as_bytes())
            .verify(msg, signature)
            .map_err(signature_error)
    }
}

impl JwtVerifier for EcdsaVerifier {
    fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
}

fn signature_error(source: ring::error::Unspecified) -> SignatureError {
    SignatureError::from_source(JwtError::from(ErrorKind::Provider(source.to_string())))
}
