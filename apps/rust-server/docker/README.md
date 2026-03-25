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

The container starts as root only long enough to bootstrap AESM and prepare
runtime permissions, then drops to a fixed non-root service user (`relational`,
UID/GID `10001`) before launching `gramine-sgx`.

When bind-mounting `/data`, pre-provision the host directory for that user:

```bash
sudo install -d -m 0750 -o 10001 -g 10001 /path/to/data
```

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
- Fixed service UID/GID for runtime privilege dropping (`10001:10001`)

### How to pin `measurements.toml`

Use the same production signing key every time. The Dockerfile already passes
`--date 0000-00-00` to `gramine-sgx-sign`.

```bash
cd apps/rust-server
make docker-build
make docker-sigstruct
```

Record only the audit-critical values in `apps/rust-server/measurements.toml`:
- pinned build inputs: base image digest, Ubuntu snapshot, Gramine, Rust, SIGSTRUCT date
- core SIGSTRUCT fields: `mr_enclave`, `mr_signer`, `isv_prod_id`, `isv_svn`, `debug_enclave`

`make docker-sigstruct` prints the `[enclave]` block in the same field order.

### Recommended CI check

For CI, store the enclave RSA key in a repo/org secret such as
`ENCLAVE_SIGNING_KEY`, then:

1. Build the Docker image from `apps/rust-server` using
   `apps/rust-server/docker/Dockerfile`.
2. Inject the signing key as a BuildKit secret named `sgx-key`.
3. Run `gramine-sgx-sigstruct-view /app/rust-server.sig` inside the built image.
4. Compare the built `mr_enclave` against `apps/rust-server/measurements.toml`.
5. Fail the workflow if it differs.

This repository's `.github/workflows/rust-server-ci.yml` follows that model.
Configure the `ENCLAVE_SIGNING_KEY` GitHub secret to enable the signed-image
build path. With that secret configured, the workflow compares the built
`mr_enclave` against `apps/rust-server/measurements.toml` and fails on
unexpected drift.

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
- The container entrypoint starts as root, wires SGX device groups when needed, then runs `gramine-sgx rust-server` as the non-root `relational` user.
- The entrypoint does not auto-migrate `/data` ownership; bind-mounted data must already be writable by UID/GID `10001`.
- This image installs Gramine and Intel SGX AESM/DCAP packages from their official apt repos.
- The enclave is signed at **build time** — no signing key is needed at runtime.
- The server binds to `0.0.0.0:8080` over **HTTPS** (set in the Gramine manifest).
- `sgx.debug = false` is enforced in Docker builds — no action needed.

## License

SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Relational Network
