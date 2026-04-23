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
use std::sync::{Mutex, OnceLock};

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

    /// When set, the measurement callback skips all policy checks and
    /// records the observed values for later retrieval. Used by the
    /// self-test diagnostic to determine whether the local DCAP
    /// verification infrastructure works without depending on a known
    /// expected MRENCLAVE.
    static DRY_RUN_MODE: RefCell<bool> = const { RefCell::new(false) };

    /// Measurements observed during a dry-run verification.
    static OBSERVED_MEASUREMENTS: RefCell<Option<ObservedMeasurements>> = const { RefCell::new(None) };
}

/// Measurements extracted from a peer's RA-TLS quote during verification.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ObservedMeasurements {
    pub mrenclave: String,
    pub mrsigner: String,
    pub isv_prod_id: u16,
    pub isv_svn: u16,
}

/// RAII guard that unconditionally clears `DRY_RUN_MODE` and
/// `OBSERVED_MEASUREMENTS` for the current thread on drop. Prevents a
/// panic between the set-true and set-false points of a dry-run from
/// leaking the flag onto the worker thread, which would cause every
/// subsequent `verify_ratls_cert` call on that worker to silently
/// accept any MRENCLAVE/MRSIGNER.
struct DryRunGuard;

impl Drop for DryRunGuard {
    fn drop(&mut self) {
        DRY_RUN_MODE.with(|d| *d.borrow_mut() = false);
        OBSERVED_MEASUREMENTS.with(|m| *m.borrow_mut() = None);
    }
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
    // Dry-run: record the observed values, skip all policy checks.
    let dry_run = DRY_RUN_MODE.with(|d| *d.borrow());
    if dry_run {
        let report_mrenclave = std::slice::from_raw_parts(mrenclave, 32);
        let report_mrsigner = std::slice::from_raw_parts(mrsigner, 32);
        let report_isv_prod_id = u16::from_le_bytes([*isv_prod_id, *isv_prod_id.add(1)]);
        let report_isv_svn = u16::from_le_bytes([*isv_svn, *isv_svn.add(1)]);
        OBSERVED_MEASUREMENTS.with(|m| {
            *m.borrow_mut() = Some(ObservedMeasurements {
                mrenclave: alloy::hex::encode(report_mrenclave),
                mrsigner: alloy::hex::encode(report_mrsigner),
                isv_prod_id: report_isv_prod_id,
                isv_svn: report_isv_svn,
            });
        });
        return 0;
    }

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

    // Defensively clear DRY_RUN_MODE on this worker before invoking the
    // enforcing callback. Belt-and-suspenders: even if a prior dry-run
    // somehow leaked the flag (e.g. from a panic that bypassed the
    // RAII guard in verify_ratls_cert_dry_run_detailed), production
    // verification must never short-circuit policy checks.
    DRY_RUN_MODE.with(|d| *d.borrow_mut() = false);

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

/// Result of a dry-run RA-TLS verification, including any stderr output
/// captured from the underlying C library so the inner `quote3_error_t`
/// is observable without grepping process logs.
#[derive(Debug, Clone)]
pub struct DryRunResult {
    pub observed: Option<ObservedMeasurements>,
    pub captured_stderr: String,
    /// Wrapper return code from `ra_tls_verify_callback_der` (0 = success).
    pub wrapper_code: i32,
    /// Inner `quote3_error_t` code parsed from captured stderr if present.
    pub quote3_error: Option<u32>,
}

/// Process-wide lock serializing fd 2 redirection in `capture_stderr`.
///
/// `capture_stderr` mutates a process-global file descriptor (stderr).
/// Without serialization, two concurrent calls can interleave their
/// `dup`/`dup2`/`close` sequences such that one call's `dup` of fd 2
/// captures the other call's pipe write end as the "saved" stderr —
/// and after the first call drops its pipe read end, the second call's
/// restore points fd 2 at a pipe with no reader, permanently breaking
/// stderr (and risking SIGPIPE-induced process termination).
static CAPTURE_STDERR_LOCK: Mutex<()> = Mutex::new(());

/// RAII guard that restores fd 2 from `saved_fd` on drop. Ensures a
/// panic inside the captured closure still puts stderr back, instead
/// of leaving fd 2 pointing at our internal pipe.
struct StderrRestoreGuard {
    saved_fd: libc::c_int,
}

impl Drop for StderrRestoreGuard {
    fn drop(&mut self) {
        if self.saved_fd >= 0 {
            unsafe {
                libc::dup2(self.saved_fd, 2);
                libc::close(self.saved_fd);
            }
        }
    }
}

/// Capture everything written to fd 2 (stderr) by `f`. Restores the
/// original fd 2 on return — including across panics in `f` — and
/// serializes concurrent calls via a process-wide mutex.
///
/// # Safety
///
/// Manipulates global file descriptors. The mutex prevents two
/// `capture_stderr` calls from racing, but other code that writes
/// directly to fd 2 from a different thread during `f`'s execution
/// will still have its output captured here.
fn capture_stderr<F: FnOnce() -> R, R>(f: F) -> (R, String) {
    use std::io::Read;
    use std::os::fd::FromRawFd;

    // Serialize fd 2 manipulation across threads. Recover from poisoning
    // because a prior panic that left fd 2 mid-swap will have been
    // unwound by the RAII guard below.
    let _lock = CAPTURE_STDERR_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    // Flush Rust's stderr before swapping the fd.
    let _ = std::io::Write::flush(&mut std::io::stderr());

    let saved = unsafe { libc::dup(2) };
    if saved < 0 {
        return (f(), String::new());
    }

    let mut pipe_fds: [libc::c_int; 2] = [0, 0];
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        unsafe {
            libc::close(saved);
        }
        return (f(), String::new());
    }
    let (read_fd, write_fd) = (pipe_fds[0], pipe_fds[1]);

    if unsafe { libc::dup2(write_fd, 2) } < 0 {
        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
            libc::close(saved);
        }
        return (f(), String::new());
    }
    unsafe {
        libc::close(write_fd);
    }

    // Run the closure under an RAII guard that restores fd 2 even if
    // `f` panics. `catch_unwind` lets us read the pipe and re-raise.
    let guard = StderrRestoreGuard { saved_fd: saved };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let _ = std::io::Write::flush(&mut std::io::stderr());
    drop(guard); // Restores fd 2 -> saved, closes saved.

    // Read everything available from the read end (non-blocking).
    let flags = unsafe { libc::fcntl(read_fd, libc::F_GETFL) };
    if flags >= 0 {
        unsafe {
            libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    let mut captured = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let _ = file.read_to_string(&mut captured);
    drop(file);

    match result {
        Ok(r) => (r, captured),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Parse `sgx_qv_verify_quote failed: NNNNN` (decimal) from captured stderr.
fn parse_quote3_error(stderr: &str) -> Option<u32> {
    let needle = "sgx_qv_verify_quote failed: ";
    let idx = stderr.find(needle)?;
    let tail = &stderr[idx + needle.len()..];
    let end = tail
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(tail.len());
    tail[..end].parse::<u32>().ok()
}

/// Verify an RA-TLS certificate in dry-run mode.
///
/// Runs the full DCAP verification stack (collateral fetch, quote signature
/// check, TCB policy from `RA_TLS_ALLOW_*` env vars) but skips the
/// MRENCLAVE/MRSIGNER/ISV policy checks. On success, returns the actual
/// measurements observed in the quote.
///
/// This is the basis of the `/admin/peers/self/test` self-diagnostic:
/// if the local enclave's own RA-TLS cert verifies under dry-run, then
/// the local DCAP verification infrastructure works correctly. Any
/// remaining peer-test failure is attributable to the peer's platform
/// or network reachability, not our verifier setup.
#[allow(dead_code)] // Convenience wrapper around the detailed variant; kept for callers
                    // that don't need stderr capture.
pub fn verify_ratls_cert_dry_run(der: &[u8]) -> Result<ObservedMeasurements, RaTlsError> {
    verify_ratls_cert_dry_run_detailed(der).and_then(|r| {
        if r.wrapper_code != 0 {
            let mut detail = format!(
                "DCAP quote verification failed with code {}",
                r.wrapper_code
            );
            if let Some(q3) = r.quote3_error {
                let (name, hint) = decode_quote3_error(q3);
                detail.push_str(&format!(
                    " — inner quote3_error_t = {q3} (0x{q3:04X}) {name}: {hint}"
                ));
            } else if !r.captured_stderr.is_empty() {
                detail.push_str(&format!(" — stderr: {}", r.captured_stderr.trim()));
            }
            Err(RaTlsError::VerificationFailed {
                code: r.wrapper_code,
                detail,
            })
        } else {
            r.observed.ok_or_else(|| RaTlsError::VerificationFailed {
                code: 0,
                detail: "verification returned success but no measurements were recorded".into(),
            })
        }
    })
}

/// Like `verify_ratls_cert_dry_run` but returns full diagnostic info
/// including captured stderr and the parsed `quote3_error_t`.
pub fn verify_ratls_cert_dry_run_detailed(der: &[u8]) -> Result<DryRunResult, RaTlsError> {
    let lib = get_ratls_lib().map_err(|msg| RaTlsError::LibraryNotAvailable(msg.to_string()))?;

    DRY_RUN_MODE.with(|d| *d.borrow_mut() = true);
    OBSERVED_MEASUREMENTS.with(|m| *m.borrow_mut() = None);
    CALLBACK_ERROR.with(|e| *e.borrow_mut() = None);
    // Reset both thread-locals on every exit path, including panics.
    let _dry_run_guard = DryRunGuard;

    unsafe {
        (lib.set_measurement_callback)(Some(measurement_callback));
    }

    let (result, captured_stderr) =
        capture_stderr(|| unsafe { (lib.verify_callback_der)(der.as_ptr(), der.len()) });

    let observed = OBSERVED_MEASUREMENTS.with(|m| m.borrow_mut().take());

    // Echo captured stderr back to real stderr so server logs still see it.
    if !captured_stderr.is_empty() {
        eprint!("{captured_stderr}");
    }

    let quote3_error = parse_quote3_error(&captured_stderr);

    Ok(DryRunResult {
        observed,
        captured_stderr,
        wrapper_code: result,
        quote3_error,
    })
}

/// Decode a wrapper return code from `ra_tls_verify_callback_der`.
///
/// The library returns mbedTLS error codes (negative values). The most
/// common is `-9984` (`MBEDTLS_ERR_X509_CERT_VERIFY_FAILED`), which is
/// emitted whenever any inner verification step fails. The specific
/// `quote3_error_t` code (e.g., `0xE03A`) is logged to stderr by the
/// library just before returning the wrapper code.
pub fn decode_wrapper_code(code: i32) -> &'static str {
    // mbedTLS x509 error codes (see include/mbedtls/x509.h). Values are
    // negative; the bare hex base is shown in the comments for grep.
    match code {
        0 => "success",
        // -0x2700
        -9984 => "MBEDTLS_ERR_X509_CERT_VERIFY_FAILED — DCAP verification or measurement check failed (look for 'sgx_qv_verify_quote failed: NNNNN' in stderr for the inner quote3_error_t code)",
        // -0x2180
        -8576 => "MBEDTLS_ERR_X509_INVALID_FORMAT — RA-TLS certificate is not a valid X.509 DER",
        // -0x2200
        -8704 => "MBEDTLS_ERR_X509_INVALID_VERSION — RA-TLS certificate uses an unsupported X.509 version field",
        // -0x2500
        -9472 => "MBEDTLS_ERR_X509_INVALID_EXTENSIONS — RA-TLS certificate is missing the SGX quote extension",
        _ => "unknown wrapper code",
    }
}

/// Decode a `quote3_error_t` value (the inner sgx_qv_verify_quote error)
/// into a human-readable identifier and remediation hint.
///
/// These are the values logged to stderr by Gramine's `ra_tls_verify_callback`
/// when `sgx_qv_verify_quote` fails. The dry-run helper auto-extracts
/// the value from captured stderr; this lookup turns it into operator
/// guidance.
pub fn decode_quote3_error(code: u32) -> (&'static str, &'static str) {
    match code {
        0xE001 => ("SGX_QL_ERROR_UNEXPECTED", "Internal QVL error. Restart aesmd and az-dcap-client."),
        0xE011 => ("SGX_QL_NO_PLATFORM_CERT_DATA", "PCS has no PCK collateral for the peer's platform. Verify the peer is on a registered SGX-capable host."),
        0xE019 => ("SGX_QL_NETWORK_ERROR", "Collateral fetch over HTTPS failed. Check enclave egress to PCCS / Azure PCS, and that /app/ca-certificates.crt is reachable."),
        0xE01B => ("SGX_QL_NO_QUOTE_COLLATERAL_DATA", "PCS returned empty collateral. Likely unsupported platform or stale cache. Try restarting aesmd."),
        0xE022 => ("SGX_QL_PCK_CERT_CHAIN_ERROR", "PCK certificate chain failed validation. Check enclave system clock and that Intel SGX Root CA is reachable."),
        0xE024 => ("SGX_QL_TCBINFO_MISMATCH", "TCB Info FMSPC does not match the PCK cert. Stale collateral cache — clear ~/.az-dcap-client and restart."),
        0xE034 => ("SGX_QL_UNABLE_TO_GET_COLLATERAL", "Collateral provider (libdcap_quoteprov / az-dcap-client) failed to fetch. Most often a network issue from inside the enclave."),
        0xE039 => ("SGX_QL_QEIDENTITY_CHAIN_ERROR", "QE Identity certificate chain failed validation. Check system clock; refresh collateral cache."),
        0xE03A => ("SGX_QL_TCBINFO_CHAIN_ERROR", "TCB Info certificate chain failed validation. Most common causes: (1) wrong system clock inside the enclave, (2) corrupted collateral cache (clear ~/.az-dcap-client/cache), (3) TLS verification failed during collateral fetch (check that CA bundle is mounted at a libcurl-discoverable path)."),
        0xE040 => ("SGX_QL_SERVICE_UNAVAILABLE", "Quote provider service unreachable. Restart aesmd inside the container."),
        0xE041 => ("SGX_QL_NETWORK_FAILURE", "Network failure during collateral fetch. Check enclave DNS/egress."),
        0xE047 => ("SGX_QL_PLATFORM_UNKNOWN", "PCS does not know the peer's platform (unregistered FMSPC). The peer's host is not in Intel/Azure's PCS database."),
        _ => ("UNKNOWN_QUOTE3_ERROR", "Unrecognized quote3_error_t code. See Intel DCAP source: external/dcap_source/QuoteVerification/dcap_quoteverify/sgx_dcap_quoteverify_export.h"),
    }
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
    fn decode_wrapper_code_pins_mbedtls_constants() {
        // Pin the numeric values to their canonical mbedTLS x509 codes so a
        // future typo (e.g. mixing up INVALID_FORMAT vs INVALID_VERSION,
        // both of which fall in the -0x21XX/-0x22XX range) is caught here
        // instead of misleading an operator in the admin diagnostic UI.
        assert_eq!(decode_wrapper_code(0), "success");
        assert!(decode_wrapper_code(-9984).starts_with("MBEDTLS_ERR_X509_CERT_VERIFY_FAILED"));
        assert!(decode_wrapper_code(-8576).starts_with("MBEDTLS_ERR_X509_INVALID_FORMAT"));
        assert!(decode_wrapper_code(-8704).starts_with("MBEDTLS_ERR_X509_INVALID_VERSION"));
        assert!(decode_wrapper_code(-9472).starts_with("MBEDTLS_ERR_X509_INVALID_EXTENSIONS"));
        assert_eq!(decode_wrapper_code(-12345), "unknown wrapper code");
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
