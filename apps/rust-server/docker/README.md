# rust-server — SGX Docker image

Reproducible Gramine + DCAP RA-TLS image for the Relational Wallet rust-server. Built and signed in CI; published to `ghcr.io/relational-network/rust-server:main` with `MRENCLAVE` pinned in [`../measurements.toml`](../measurements.toml).

## Run (production)

Use [`scripts/deploy-instance.sh`](../../../scripts/deploy-instance.sh) at the repo root — it pulls this image, runs it under systemd with a per-instance env file, and survives reboot. The raw `docker run` form is:

```bash
docker run --rm \
  --device /dev/sgx/enclave \
  --device /dev/sgx/provision \
  -v /var/run/aesmd:/var/run/aesmd \
  -v /var/lib/relational-wallet/data:/data \
  --env-file /etc/relational-wallet/rust-server.env \
  -p 127.0.0.1:8080:8080 \
  ghcr.io/relational-network/rust-server:main
```

The container starts as root only long enough to wire AESM and SGX device groups, then drops to UID/GID `10001`. Pre-create the data dir with that ownership:

```bash
sudo install -d -m 0750 -o 10001 -g 10001 /var/lib/relational-wallet/data
```

The signing key is **not** required at runtime — it is consumed at build time only.

## Build (locally)

Defaults to the host signing key at `$HOME/.config/gramine/enclave-key.pem` (generate with `gramine-sgx-gen-private-key` if missing). From the repo root:

```bash
cd apps/rust-server
make docker-build                    # uses default key
SGX_SIGNING_KEY=/path/to/prod.pem ./docker/build.sh ubuntu20
```

Build context is `apps/rust-server/`; see [`../.dockerignore`](../.dockerignore).

## Reproducibility

`MRSIGNER` is determined by the RSA-3072 signing key. `MRENCLAVE` is determined by binary + manifest + trusted files; the Dockerfile pins everything that affects it:

- Rust toolchain (`RUST_TOOLCHAIN`, matches `rust-toolchain.toml`)
- Gramine + SGX AESM packages (`GRAMINE_VERSION`, `SGX_AESM_VERSION`)
- Ubuntu apt snapshot (`UBUNTU_SNAPSHOT`)
- Build platform (`linux/amd64`, enforced in `build.sh`)
- Rust reproducibility env (`SOURCE_DATE_EPOCH`, fixed `RUSTFLAGS`, single codegen unit)
- Runtime UID/GID (`10001:10001`)
- SIGSTRUCT date (`--date 0000-00-00` to `gramine-sgx-sign`)
- `sgx.debug = false`

Override the apt snapshot if needed:

```bash
UBUNTU_SNAPSHOT=20260210T000000Z ./build.sh ubuntu20
```

## Verification

Inspect a built image's measurements:

```bash
make docker-sigstruct      # prints [enclave] block in measurements.toml field order
make verify-mrenclave      # rebuilds locally and diffs against measurements.toml
```

CI ([`.github/workflows/rust-server-ci.yml`](../../../.github/workflows/rust-server-ci.yml)) does the same on every push: builds with the `ENCLAVE_SIGNING_KEY` secret, asserts `debug_enclave = false`, `isv_prod_id = 0`, `isv_svn = 0`, and fails if `mr_enclave` drifts from `measurements.toml`. Pushes to `main` publish the image; PRs verify only.

To roll a new release:

1. `make docker-build && make docker-sigstruct`
2. Copy the printed `[enclave]` block into [`../measurements.toml`](../measurements.toml) **in the same PR** as the source change.
3. Merge — CI re-verifies and pushes the image.

Deploy by digest, not tag, when pinning a release:

```bash
sudo IMAGE=ghcr.io/relational-network/rust-server@sha256:<digest> \
     INSTANCE=wallet-001 DUCKDNS_TOKEN=... \
     bash scripts/deploy-instance.sh
```

---

SPDX-License-Identifier: AGPL-3.0-or-later · Copyright (C) 2026 Relational Network
