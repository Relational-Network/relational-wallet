# Rust Server Gramine Docker (Ubuntu 20.04)

This folder contains a Gramine SGX image that builds and runs the Rust server on Ubuntu 20.04 (focal).

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

## Notes

- AESM is started in the container via `restart_aesm.sh`.
- The container entrypoint runs `gramine-sgx rust-server` by default.
- This image installs Gramine and Intel SGX AESM packages from their official apt repos.
- The container signs the manifest on startup using your host key.
- The server binds to `0.0.0.0:8080` (set in the Gramine manifest).

## License

SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Relational Network
