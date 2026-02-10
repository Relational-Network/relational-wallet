# Rust Server Gramine Docker (Ubuntu 20.04) with DCAP RA-TLS

This folder contains a Gramine SGX image that builds and runs the Rust server on Ubuntu 20.04 (focal) with **DCAP remote attestation** and **RA-TLS** for secure HTTPS communication.

## Features

- **DCAP Attestation**: Uses Intel DCAP (Data Center Attestation Primitives) for remote attestation
- **RA-TLS**: TLS certificates are generated at runtime by `gramine-ratls` with attestation evidence embedded
- **HTTPS Only**: The server only accepts HTTPS connections (no HTTP fallback)
- **Deterministic Build**: Enclave is signed at build time — MRENCLAVE and MRSIGNER are fixed per image

## Build

```bash
./build.sh ubuntu20
```

The signing key defaults to `$HOME/.config/gramine/enclave-key.pem`.
Override with the `SGX_SIGNING_KEY` environment variable:

```bash
SGX_SIGNING_KEY=/path/to/production-key.pem ./build.sh ubuntu20
```

To override the pinned Ubuntu snapshot timestamp:

```bash
UBUNTU_SNAPSHOT=20260210T000000Z ./build.sh ubuntu20
```

This builds the image as:

```
relationalnetwork/rust-server:focal
```

Note: the Docker build context is `apps/rust-server` (see `apps/rust-server/.dockerignore`).

## Generate a signing key (SGX only)

If you have not already, generate a key on your host:

```bash
gramine-sgx-gen-private-key
```

By default, the key is stored at `$HOME/.config/gramine/enclave-key.pem`.

**Important:** Use the same key for all production builds to preserve MRSIGNER.
Store it securely (e.g., in a hardware security module or encrypted vault).

## Run

The signing key is **NOT needed at runtime** — it is only used during `docker build`.

```bash
docker run --rm -it \
  --device /dev/sgx/enclave \
  --device /dev/sgx/provision \
  -p 8080:8080 \
  relationalnetwork/rust-server:focal
```

If your host exposes legacy SGX device nodes, you may need to map `/dev/isgx` instead.

## Deterministic Enclave Measurements

| Measurement | What controls it | How to keep it stable |
|-------------|-----------------|----------------------|
| **MRSIGNER** | The RSA-3072 signing key | Use the same key for every build |
| **MRENCLAVE** | Binary + manifest + trusted files | Pin Rust toolchain + Gramine versions |

The following are pinned in the Dockerfile:
- Rust toolchain version (via `RUST_TOOLCHAIN` build arg, matches `rust-toolchain.toml`)
- Gramine package version (via `GRAMINE_VERSION` build arg)
- SGX AESM package version (via `SGX_AESM_VERSION` build arg)
- Ubuntu apt snapshot timestamp (via `UBUNTU_SNAPSHOT` build arg)
- Build target platform (`linux/amd64`, enforced in `build.sh`)
- Rust reproducibility env (`SOURCE_DATE_EPOCH`, single codegen unit/job, fixed `RUSTFLAGS`)

## DCAP Configuration

For DCAP attestation to work, you need:

1. **Intel SGX DCAP driver** installed on the host
2. The container needs access to `/dev/sgx/enclave` and `/dev/sgx/provision`

The container installs the following DCAP packages:
- `gramine-ratls-dcap` - RA-TLS library with DCAP support

## TLS Certificate Details

At startup, `gramine-ratls` generates:
- Certificate: `/tmp/ra-tls.crt.pem` (inside the enclave)
- Private key: `/tmp/ra-tls.key.pem` (inside the enclave)

The certificate contains SGX attestation evidence that clients can verify using the RA-TLS verification library.

## Notes

- AESM is started in the container via `restart_aesm.sh`.
- The container entrypoint runs `gramine-sgx rust-server` which invokes `gramine-ratls` first.
- This image installs Gramine and Intel SGX AESM/DCAP packages from their official apt repos.
- The enclave is signed at **build time** — no signing key is needed at runtime.
- The server binds to `0.0.0.0:8080` over **HTTPS** (set in the Gramine manifest).
- `sgx.debug = false` is enforced in Docker builds — no action needed.

## License

SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Relational Network
