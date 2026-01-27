---
layout: default
title: Rust Server
parent: Installation
nav_order: 3
---

# Rust Server (Gramine SGX)

This guide covers two supported setups:

- **Docker (SGX)** — Run inside a Gramine SGX container on Ubuntu 20.04.
- **Native (SGX)** — Build and run on the host with Gramine SGX.

## Prerequisites (both modes)

- Intel SGX-capable host with `/dev/sgx/enclave` and `/dev/sgx/provision`.
- Gramine installed on the host.
- Gramine SGX signing key on the host:
  ```bash
  gramine-sgx-gen-private-key
  ```
  This creates `$HOME/.config/gramine/enclave-key.pem`.

## Docker (SGX)

### Prerequisites

- Docker installed and available to `sudo`.

### Build the image

From the repo root:

```bash
cd apps/rust-server
make docker-build
```

This builds `relationalnetwork/rust-server:focal` (Ubuntu 20.04).

### Run the container

```bash
cd apps/rust-server
make docker-run
```

The container binds to `0.0.0.0:8080` and publishes port `8080` to the host.

### Stop the container

```bash
cd apps/rust-server
make docker-stop
```

### Custom key path

If your key is not in the default location, mount it and set `GRAMINE_SGX_SIGNING_KEY` in the run command. See `apps/rust-server/docker/README.md` for a full example.

## Native (SGX)

### Build

```bash
cd apps/rust-server
make SGX=1
```

This generates `rust-server.manifest`, `rust-server.manifest.sgx`, and `rust-server.sig`.

### Run

```bash
cd apps/rust-server
gramine-sgx rust-server
```

The server binds to `127.0.0.1:8080` by default. Override with `HOST`/`PORT` as needed.
