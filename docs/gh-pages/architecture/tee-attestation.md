---
layout: default
title: TEE & Attestation
parent: Architecture
nav_order: 3
---

# TEE & Remote Attestation
{: .fs-7 }

How Intel SGX, Gramine, and DCAP RA-TLS work together to provide hardware-enforced security guarantees.
{: .fs-5 .fw-300 }

---

## What is a TEE?

A **Trusted Execution Environment** (TEE) is a hardware-isolated region of a processor that protects code and data from the rest of the system. Intel SGX (Software Guard Extensions) is the TEE technology used by Relational Wallet.

### SGX Security Properties

| Property | Description |
|:---------|:------------|
| **Memory encryption** | Enclave memory is encrypted by the CPU. The host OS, hypervisor, and other processes cannot read it. |
| **Memory integrity** | Hardware prevents tampering with enclave memory pages. |
| **Attestation** | The CPU can produce a cryptographic proof that specific code is running inside a genuine enclave. |
| **Sealing** | Data can be encrypted with keys derived from the enclave's identity, making it inaccessible outside the enclave. |

### What SGX does NOT protect against

| Limitation | Mitigation |
|:-----------|:-----------|
| Side-channel attacks | Gramine includes mitigations; security-critical paths avoid secret-dependent branching |
| Denial of service (host can kill the enclave) | Monitoring + auto-restart; data persisted to sealed storage |
| Bugs in enclave code | Code review, testing, deterministic builds |
| Supply chain attacks on CPU | Out of scope; requires trust in Intel hardware |

---

## Gramine

[Gramine](https://gramineproject.io/) is a library OS that allows unmodified Linux applications to run inside Intel SGX enclaves. Relational Wallet uses Gramine to run the Axum Rust server inside SGX without modifying the application code.

### How Gramine Works

```
┌────────────────────────────────────────────┐
│              Intel SGX Enclave             │
│                                            │
│  ┌──────────────────────────────────────┐  │
│  │         Gramine Library OS           │  │
│  │  • Linux system call emulation       │  │
│  │  • Encrypted filesystem              │  │
│  │  • RA-TLS certificate generation     │  │
│  │  • Thread management                 │  │
│  │                                      │  │
│  │  ┌────────────────────────────────┐  │  │
│  │  │  Relational Wallet Binary      │  │  │
│  │  │  (Axum + Tokio + Application)  │  │  │
│  │  └────────────────────────────────┘  │  │
│  └──────────────────────────────────────┘  │
│                                            │
└────────────────────────────────────────────┘
        │                    │
        ▼                    ▼
  Host filesystem      SGX driver
  (encrypted I/O)    (/dev/sgx/*)
```

### Gramine Manifest

The Gramine manifest (`rust-server.manifest.template`) defines:

| Setting | Purpose |
|:--------|:--------|
| `sgx.debug` | `true` for development, `false` for production |
| `sgx.enclave_size` | Memory allocated to the enclave |
| `sgx.thread_num` | Number of threads available inside the enclave |
| `fs.mounts` | Encrypted filesystem mount points (e.g., `/data`) |
| `sgx.trusted_files` | Files whose integrity is verified before loading |
| `sgx.allowed_files` | Files accessible but not integrity-checked |

---

## Enclave Measurements

SGX uses cryptographic measurements to identify enclaves:

### MRENCLAVE

A SHA-256 hash of the enclave's initial code, data, and configuration. **Any change** to the binary, manifest, or configuration produces a different MRENCLAVE.

```
MRENCLAVE = SHA256(code + data + heap + stack + manifest settings)
```

This is the primary identity used for verification. Clients can check the MRENCLAVE before connecting to ensure they are communicating with the expected code.

### MRSIGNER

A hash of the RSA public key used to sign the enclave. All enclaves signed with the same key share the same MRSIGNER.

```
MRSIGNER = SHA256(signing_public_key)
```

### Verification in CI

The GitHub Actions CI pipeline verifies MRENCLAVE consistency:

```yaml
# Extract MRENCLAVE from Docker build
docker cp container:/app/rust-server.sig /tmp/
gramine-sgx-sigstruct-view /tmp/rust-server.sig

# Compare against pinned value in measurements.toml
make verify-mrenclave
```

If the built MRENCLAVE differs from the pinned value in `measurements.toml`, the CI build fails. This ensures that only intentional code changes can alter the enclave identity.

### Deterministic Builds

To ensure reproducible MRENCLAVE values, the Docker build uses:

| Technique | Purpose |
|:----------|:--------|
| `SOURCE_DATE_EPOCH` | Fixed timestamp for deterministic builds |
| Single codegen unit | Ensures consistent Rust compilation output |
| Pinned Ubuntu snapshot | `20260210T000000Z` for reproducible apt packages |
| Pinned Rust toolchain | Exact version 1.92.0 |
| Pinned Gramine version | Exact version 1.8 |

---

## DCAP Remote Attestation

Relational Wallet uses **DCAP** (Data Center Attestation Primitives) for remote attestation, which does not require Intel's centralized EPID service.

### Attestation Flow

```
┌──────────┐                           ┌─────────────────┐
│  Client   │                           │  SGX Enclave    │
│           │                           │  (Gramine)      │
│           │    1. TLS ClientHello     │                 │
│           │ ─────────────────────────►│                 │
│           │                           │  2. Generate    │
│           │                           │     RA-TLS cert │
│           │                           │     with DCAP   │
│           │                           │     quote       │
│           │    3. TLS ServerHello     │                 │
│           │       + RA-TLS cert       │                 │
│           │ ◄─────────────────────────│                 │
│           │                           │                 │
│  4. Verify│                           │                 │
│     cert  │                           │                 │
│     chain │                           │                 │
│     + DCAP│                           │                 │
│     quote │                           │                 │
│           │                           │                 │
│  5. Check │                           │                 │
│  MRENCLAVE│                           │                 │
│  matches  │                           │                 │
│  expected │                           │                 │
│           │    6. Encrypted channel   │                 │
│           │ ◄════════════════════════►│                 │
└──────────┘                           └─────────────────┘
```

### What the RA-TLS Certificate Contains

The RA-TLS certificate is a standard X.509 certificate with additional SGX-specific extensions:

| Extension | Content |
|:----------|:--------|
| **DCAP Quote** | Signed attestation evidence from the CPU |
| **MRENCLAVE** | Hash of the enclave code and configuration |
| **MRSIGNER** | Hash of the enclave signing key |
| **ISV Product ID** | Enclave product identifier |
| **ISV SVN** | Enclave security version number |
| **Report Data** | Contains the TLS public key hash (binds attestation to TLS session) |

### Verification Steps

A client verifying an RA-TLS connection:

1. **TLS handshake**: Standard TLS 1.3 handshake with the RA-TLS certificate
2. **Quote verification**: Verify the DCAP quote signature using Intel's root CA
3. **Freshness check**: Verify the quote is recent (prevents replay)
4. **MRENCLAVE check**: Compare the enclave measurement against the expected value
5. **Report data binding**: Verify the TLS public key matches the report data (prevents MITM)

---

## Encrypted Filesystem

Gramine provides an encrypted filesystem that is transparent to the application:

```
Application writes to /data/wallets/abc/meta.json
  │
  ▼
Gramine intercepts the write system call
  │
  ▼
Encrypts the data with a key derived from the enclave identity
  │
  ▼
Writes encrypted bytes to the host filesystem
  │
  ▼
On read: decrypts transparently back to the application
```

### Key Derivation

| Mode | Key Source | Binding |
|:-----|:----------|:--------|
| **Development** | Persistent 16-byte file | `data/.dev_storage_key` |
| **Production** | `_sgx_mrsigner` | Sealed to enclave signer identity |

In production, the encryption key is derived from the enclave's signing identity. This means:
- Data encrypted by one enclave version can be read by another version **signed with the same key**
- Changing the signing key makes all previously encrypted data inaccessible
- The host OS cannot derive the encryption key

---

## Security Guarantees Summary

| Guarantee | Mechanism |
|:----------|:----------|
| Private keys cannot be extracted | SGX memory isolation + no export API |
| Data at rest is encrypted | Gramine encrypted FS sealed to enclave identity |
| Clients can verify the server code | DCAP RA-TLS with MRENCLAVE verification |
| Code cannot be tampered with | MRENCLAVE changes with any code/config modification |
| Network traffic is encrypted | RA-TLS (TLS 1.3 with attestation evidence) |
| Authentication is cryptographic | Clerk JWT with JWKS signature verification |
| All actions are auditable | Append-only audit logs inside encrypted storage |
