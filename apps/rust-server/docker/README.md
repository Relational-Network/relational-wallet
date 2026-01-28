# Rust Server Gramine Docker (Ubuntu 20.04) with DCAP RA-TLS

This folder contains a Gramine SGX image that builds and runs the Rust server on Ubuntu 20.04 (focal) with **DCAP remote attestation** and **RA-TLS** for secure HTTPS communication.

## Features

- **DCAP Attestation**: Uses Intel DCAP (Data Center Attestation Primitives) for remote attestation
- **RA-TLS**: TLS certificates are generated at runtime by `gramine-ratls` with attestation evidence embedded
- **HTTPS Only**: The server only accepts HTTPS connections (no HTTP fallback)

## Build

```bash
./build.sh ubuntu20
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
To use a different key path, mount it into the container and set `GRAMINE_SGX_SIGNING_KEY` to that path.

## Run

```bash
docker run --rm -it \
  --device /dev/sgx/enclave \
  --device /dev/sgx/provision \
  -p 8080:8080 \
  -v "$HOME/.config/gramine/enclave-key.pem:/keys/enclave-key.pem:ro" \
  -e GRAMINE_SGX_SIGNING_KEY=/keys/enclave-key.pem \
  relationalnetwork/rust-server:focal
```

If your host exposes legacy SGX device nodes, you may need to map `/dev/isgx` instead.

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
- The container signs the manifest on startup using your host key.
- The server binds to `0.0.0.0:8080` over **HTTPS** (set in the Gramine manifest).
- `sgx.debug = true` in the manifest - change to `false` for production.

## License

SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Relational Network
