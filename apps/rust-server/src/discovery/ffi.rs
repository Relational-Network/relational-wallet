// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

//! FFI bindings to Gramine's `libra_tls_verify_dcap.so` via runtime `dlopen`.
//!
//! This library ships with the `gramine-ratls-dcap` package and handles all
//! the heavy lifting: DCAP collateral fetching, QE identity verification,
//! TCB status evaluation, and SGX quote signature verification.
//!
//! We load the library at runtime using `dlopen`/`dlsym` instead of a
//! link-time `extern "C"` block. This allows the binary to compile and
//! run outside SGX (where the library does not exist) and only fail
//! gracefully when RA-TLS verification is actually attempted.
//!
//! We only need a thin measurement callback to check MRENCLAVE/MRSIGNER/ISV
//! fields against our per-peer attestation policy.

use std::cell::RefCell;
use std::ffi::CString;
use std::sync::OnceLock;

use super::attestation::AttestationPolicy;

// =============================================================================
// Type Aliases for FFI Function Pointers
// =============================================================================

/// Signature of `ra_tls_set_measurement_callback`.
type SetMeasurementCallbackFn = unsafe extern "C" fn(
    callback: Option<
        unsafe extern "C" fn(
            mrenclave: *const u8,
            mrsigner: *const u8,
            isv_prod_id: *const u8,
            isv_svn: *const u8,
        ) -> i32,
    >,
);

/// Signature of `ra_tls_verify_callback_der`.
type VerifyCallbackDerFn = unsafe extern "C" fn(der_crt: *const u8, der_crt_size: usize) -> i32;

// =============================================================================
// Runtime-Loaded Library Handle
// =============================================================================

/// Holds function pointers loaded via `dlopen`/`dlsym`.
struct RaTlsLib {
    set_measurement_callback: SetMeasurementCallbackFn,
    verify_callback_der: VerifyCallbackDerFn,
}

// SAFETY: The function pointers are loaded from a shared library and are
// valid for the lifetime of the process. The library handle is never closed.
unsafe impl Send for RaTlsLib {}
unsafe impl Sync for RaTlsLib {}

/// Search paths for the RA-TLS DCAP verification library.
const LIB_SEARCH_PATHS: &[&str] = &[
    "libra_tls_verify_dcap.so",
    "/usr/lib/x86_64-linux-gnu/gramine/runtime/glibc/libra_tls_verify_dcap.so",
    "/usr/lib/x86_64-linux-gnu/gramine/direct/libra_tls_verify_dcap.so",
];

/// One-time loaded library result.
static RA_TLS_LIB: OnceLock<Result<RaTlsLib, String>> = OnceLock::new();

/// Attempt to load the RA-TLS DCAP library at runtime.
fn load_ratls_lib() -> Result<RaTlsLib, String> {
    for &path in LIB_SEARCH_PATHS {
        let c_path = CString::new(path).map_err(|e| format!("Invalid lib path: {e}"))?;

        // SAFETY: dlopen with RTLD_NOW | RTLD_LOCAL. Returns null on failure.
        let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            let err = unsafe { libc::dlerror() };
            let err_msg = if err.is_null() {
                "unknown error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned() }
            };
            tracing::debug!(path = path, error = %err_msg, "dlopen failed — trying next path");
            continue;
        }

        // Load ra_tls_set_measurement_callback
        let sym_name =
            CString::new("ra_tls_set_measurement_callback").expect("CString::new failed");
        let sym = unsafe { libc::dlsym(handle, sym_name.as_ptr()) };
        if sym.is_null() {
            return Err(format!(
                "dlsym(ra_tls_set_measurement_callback) failed in {path}"
            ));
        }
        let set_measurement_callback: SetMeasurementCallbackFn =
            unsafe { std::mem::transmute(sym) };

        // Load ra_tls_verify_callback_der
        let sym_name = CString::new("ra_tls_verify_callback_der").expect("CString::new failed");
        let sym = unsafe { libc::dlsym(handle, sym_name.as_ptr()) };
        if sym.is_null() {
            return Err(format!(
                "dlsym(ra_tls_verify_callback_der) failed in {path}"
            ));
        }
        let verify_callback_der: VerifyCallbackDerFn = unsafe { std::mem::transmute(sym) };

        tracing::info!(path = path, "Loaded RA-TLS DCAP verification library");
        // NOTE: We intentionally never call dlclose — the library stays
        // loaded for the lifetime of the process.
        return Ok(RaTlsLib {
            set_measurement_callback,
            verify_callback_der,
        });
    }

    Err(format!(
        "libra_tls_verify_dcap.so not found in any search path: {:?}",
        LIB_SEARCH_PATHS
    ))
}

/// Get the loaded RA-TLS library, or the error explaining why it isn't available.
fn get_ratls_lib() -> Result<&'static RaTlsLib, &'static str> {
    let result = RA_TLS_LIB.get_or_init(load_ratls_lib);
    match result {
        Ok(lib) => Ok(lib),
        Err(msg) => Err(msg.as_str()),
    }
}

// =============================================================================
// Thread-Local Policy for Callback
// =============================================================================

thread_local! {
    /// Thread-local attestation policy used by the measurement callback.
    ///
    /// This allows concurrent peer verification with different policies
    /// per thread. The policy is set before calling `verify_ratls_cert()`
    /// and read inside the callback.
    static CURRENT_POLICY: RefCell<Option<AttestationPolicy>> = const { RefCell::new(None) };

    /// Thread-local result storage for the callback.
    static CALLBACK_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

// =============================================================================
// Measurement Callback
// =============================================================================

/// C-ABI measurement callback invoked by `ra_tls_verify_callback_der`.
///
/// Reads the thread-local `CURRENT_POLICY` and compares the report fields.
/// Returns 0 on success, 1 on failure.
unsafe extern "C" fn measurement_callback(
    mrenclave: *const u8,
    mrsigner: *const u8,
    isv_prod_id: *const u8,
    isv_svn: *const u8,
) -> i32 {
    CURRENT_POLICY.with(|policy_cell| {
        let policy = policy_cell.borrow();
        let Some(ref policy) = *policy else {
            CALLBACK_ERROR.with(|e| {
                *e.borrow_mut() = Some("No attestation policy set".to_string());
            });
            return 1;
        };

        // MRENCLAVE: exact 32-byte match
        let report_mrenclave = std::slice::from_raw_parts(mrenclave, 32);
        if report_mrenclave != policy.mrenclave {
            CALLBACK_ERROR.with(|e| {
                *e.borrow_mut() = Some(format!(
                    "MRENCLAVE mismatch: expected {}, got {}",
                    alloy::hex::encode(policy.mrenclave),
                    alloy::hex::encode(report_mrenclave)
                ));
            });
            return 1;
        }

        // MRSIGNER: optional pin (32-byte match if set)
        if let Some(ref expected_mrsigner) = policy.mrsigner {
            let report_mrsigner = std::slice::from_raw_parts(mrsigner, 32);
            if report_mrsigner != expected_mrsigner.as_slice() {
                CALLBACK_ERROR.with(|e| {
                    *e.borrow_mut() = Some(format!(
                        "MRSIGNER mismatch: expected {}, got {}",
                        alloy::hex::encode(expected_mrsigner),
                        alloy::hex::encode(report_mrsigner)
                    ));
                });
                return 1;
            }
        }

        // ISV_PROD_ID: exact match (2 bytes, little-endian u16)
        let report_isv_prod_id = u16::from_le_bytes([*isv_prod_id, *isv_prod_id.add(1)]);
        if report_isv_prod_id != policy.isv_prod_id {
            CALLBACK_ERROR.with(|e| {
                *e.borrow_mut() = Some(format!(
                    "ISV_PROD_ID mismatch: expected {}, got {}",
                    policy.isv_prod_id, report_isv_prod_id
                ));
            });
            return 1;
        }

        // ISV_SVN: must be >= minimum
        let report_isv_svn = u16::from_le_bytes([*isv_svn, *isv_svn.add(1)]);
        if report_isv_svn < policy.min_isv_svn {
            CALLBACK_ERROR.with(|e| {
                *e.borrow_mut() = Some(format!(
                    "ISV_SVN too low: expected >= {}, got {}",
                    policy.min_isv_svn, report_isv_svn
                ));
            });
            return 1;
        }

        0 // Success
    })
}

// =============================================================================
// Safe Rust Wrapper
// =============================================================================

/// Errors from RA-TLS verification.
#[derive(Debug, thiserror::Error)]
pub enum RaTlsError {
    #[error("RA-TLS verification failed (code {code}): {detail}")]
    VerificationFailed { code: i32, detail: String },

    #[error("Measurement callback failed: {0}")]
    MeasurementMismatch(String),

    #[error("RA-TLS library not available: {0}")]
    LibraryNotAvailable(String),
}

/// Verify a DER-encoded RA-TLS certificate against an attestation policy.
///
/// This function:
/// 1. Loads the RA-TLS library (once, via `dlopen`)
/// 2. Sets the thread-local attestation policy
/// 3. Registers the measurement callback with Gramine's library
/// 4. Calls `ra_tls_verify_callback_der()` which does all DCAP verification
/// 5. Clears the policy and returns the result
///
/// Returns `RaTlsError::LibraryNotAvailable` if the shared library cannot
/// be loaded (i.e., running outside SGX/Gramine).
///
/// # Safety
///
/// Calls into C FFI via dlopen'd function pointers. The certificate bytes
/// must be a valid DER-encoded X.509. Thread-safe: uses thread-local
/// storage for the policy.
pub fn verify_ratls_cert(der: &[u8], policy: &AttestationPolicy) -> Result<(), RaTlsError> {
    let lib = get_ratls_lib().map_err(|msg| RaTlsError::LibraryNotAvailable(msg.to_string()))?;

    // Set thread-local policy for the callback
    CURRENT_POLICY.with(|p| {
        *p.borrow_mut() = Some(policy.clone());
    });
    CALLBACK_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });

    // Register our measurement callback
    unsafe {
        (lib.set_measurement_callback)(Some(measurement_callback));
    }

    // Call Gramine's DCAP verification
    let result = unsafe { (lib.verify_callback_der)(der.as_ptr(), der.len()) };

    // Clean up thread-local
    CURRENT_POLICY.with(|p| {
        *p.borrow_mut() = None;
    });

    if result != 0 {
        // Check if our measurement callback set a specific error
        let detail = CALLBACK_ERROR.with(|e| e.borrow_mut().take());
        if let Some(msg) = detail {
            return Err(RaTlsError::MeasurementMismatch(msg));
        }
        return Err(RaTlsError::VerificationFailed {
            code: result,
            detail: format!("DCAP quote verification failed with code {result}"),
        });
    }

    Ok(())
}

/// Check whether the RA-TLS library is available at runtime.
///
/// Returns `true` if `libra_tls_verify_dcap.so` was successfully loaded.
pub fn is_ratls_available() -> bool {
    get_ratls_lib().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_local_policy_lifecycle() {
        // Policy starts as None
        CURRENT_POLICY.with(|p| {
            assert!(p.borrow().is_none());
        });

        // Set a policy
        let policy = AttestationPolicy {
            mrenclave: [0xab; 32],
            mrsigner: None,
            min_isv_svn: 0,
            isv_prod_id: 0,
        };
        CURRENT_POLICY.with(|p| {
            *p.borrow_mut() = Some(policy);
        });

        // Verify it's set
        CURRENT_POLICY.with(|p| {
            assert!(p.borrow().is_some());
        });

        // Clear it
        CURRENT_POLICY.with(|p| {
            *p.borrow_mut() = None;
        });
        CURRENT_POLICY.with(|p| {
            assert!(p.borrow().is_none());
        });
    }

    #[test]
    fn ratls_verify_fake_cert_fails_gracefully() {
        // With a fake cert, verification should fail regardless of whether
        // the RA-TLS library is available or not.
        let policy = AttestationPolicy {
            mrenclave: [0x00; 32],
            mrsigner: None,
            min_isv_svn: 0,
            isv_prod_id: 0,
        };
        let result = verify_ratls_cert(b"fake-cert-data", &policy);
        assert!(
            result.is_err(),
            "Expected an error for fake cert data, got Ok"
        );
        // The error could be LibraryNotAvailable (no Gramine) or
        // VerificationFailed (library loaded but cert is garbage).
        match result {
            Err(RaTlsError::LibraryNotAvailable(_)) => {
                // Expected outside SGX
            }
            Err(RaTlsError::VerificationFailed { .. }) => {
                // Expected when library is available but cert is invalid
            }
            Err(RaTlsError::MeasurementMismatch(_)) => {
                // Shouldn't happen with garbage data, but acceptable
            }
            Ok(_) => unreachable!("Already asserted is_err"),
        }
    }
}
