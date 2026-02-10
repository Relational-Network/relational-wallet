# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# SGX-only Docker image with DCAP RA-TLS attestation support
#
# Deterministic build: the enclave is signed at BUILD TIME so that
# MRENCLAVE and MRSIGNER are baked into the image and never change
# at runtime.  The signing key is NOT needed to run the container.
#
# Everything is pinned for reproducibility:
#   - Ubuntu base image pinned to SHA256 digest
#   - Rust toolchain pinned to exact version
#   - Gramine + SGX packages pinned to exact versions

ARG UBUNTU_IMAGE=ubuntu:20.04@sha256:8feb4d8ca5354def3d8fce243717141ce31e2c428701f6682bd2fafe15388214
ARG UBUNTU_SNAPSHOT=20260210T000000Z
ARG CA_CERTS_VERSION=20240203~20.04.1

# ---------------------------------------------------------------------------
# Stage 1 — Rust build (pinned toolchain for reproducibility)
# ---------------------------------------------------------------------------
FROM ${UBUNTU_IMAGE} AS builder

ARG UBUNTU_SNAPSHOT
ARG CA_CERTS_VERSION
ARG TARGETPLATFORM

RUN test "${TARGETPLATFORM}" = "linux/amd64" || \
    (echo "error: SGX image must be built for linux/amd64 (got ${TARGETPLATFORM})" >&2; exit 1)

# Install CA roots before switching to snapshot mirrors (which redirect to HTTPS).
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates=${CA_CERTS_VERSION} && \
    rm -rf /var/lib/apt/lists/*

# Freeze Ubuntu package resolution to a timestamped snapshot so transitive
# dependency versions cannot drift over time.
RUN printf 'Acquire::Check-Valid-Until "false";\n' > /etc/apt/apt.conf.d/99ubuntu-snapshot && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" > /etc/apt/sources.list && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal-updates main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" >> /etc/apt/sources.list && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal-security main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" >> /etc/apt/sources.list

# Pin all builder packages for reproducibility.
# To update: run the base image and check apt-cache policy <pkg>
ARG BUILD_ESSENTIAL_VERSION=12.8ubuntu1.1
ARG CLANG_VERSION=1:10.0-50~exp1
ARG CURL_VERSION=7.68.0-1ubuntu2.25

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    curl=${CURL_VERSION} \
    ca-certificates=${CA_CERTS_VERSION} \
    build-essential=${BUILD_ESSENTIAL_VERSION} \
    clang=${CLANG_VERSION} && \
    rm -rf /var/lib/apt/lists/*

# Pin the exact Rust toolchain so MRENCLAVE is reproducible.
# Update this version deliberately — never use "stable".
ARG RUST_TOOLCHAIN=1.92.0

# Pin the rustup installer by SHA256 so builds don't depend on a mutable URL.
# To update: curl -sSf https://sh.rustup.rs -o rustup-init.sh && sha256sum rustup-init.sh
ARG RUSTUP_SHA256=17247e4bcacf6027ec2e11c79a72c494c9af69ac8d1abcc1b271fa4375a106c2

ENV CARGO_HOME=/opt/cargo
ENV RUSTUP_HOME=/opt/rustup
ENV PATH=/opt/cargo/bin:${PATH}
ENV SOURCE_DATE_EPOCH=1704067200
ENV TZ=UTC
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV CARGO_INCREMENTAL=0
ENV CARGO_BUILD_JOBS=1
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
ENV CARGO_PROFILE_RELEASE_DEBUG=0
ENV CARGO_PROFILE_RELEASE_STRIP=symbols
ENV ZERO_AR_DATE=1
ENV RUSTFLAGS="-Ccodegen-units=1 -Cdebuginfo=0 -Cstrip=symbols -Clink-arg=-Wl,--build-id=none -Cllvm-args=-rng-seed=0 --remap-path-prefix=/app=."
ENV CFLAGS="-g0 -frandom-seed=relational-wallet -Wno-builtin-macro-redefined -D__DATE__=\"redacted\" -D__TIME__=\"redacted\" -D__TIMESTAMP__=\"redacted\""
ENV CXXFLAGS="-g0 -frandom-seed=relational-wallet -Wno-builtin-macro-redefined -D__DATE__=\"redacted\" -D__TIME__=\"redacted\" -D__TIMESTAMP__=\"redacted\""

RUN curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh && \
    echo "${RUSTUP_SHA256}  /tmp/rustup-init.sh" | sha256sum -c - && \
    sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain ${RUST_TOOLCHAIN} && \
    rm /tmp/rustup-init.sh

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src
RUN cargo fetch --locked
RUN CARGO_NET_OFFLINE=true cargo build --release --locked --jobs 1

# ---------------------------------------------------------------------------
# Stage 2 — Runtime image + enclave signing
# ---------------------------------------------------------------------------
ARG UBUNTU_CODENAME=focal

FROM ${UBUNTU_IMAGE}

# ARGs are reset after each FROM
ARG UBUNTU_CODENAME=focal
ARG UBUNTU_SNAPSHOT
ARG CA_CERTS_VERSION
ARG TARGETPLATFORM
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV TZ=UTC
ENV PYTHONHASHSEED=0

RUN test "${TARGETPLATFORM}" = "linux/amd64" || \
    (echo "error: SGX image must be built for linux/amd64 (got ${TARGETPLATFORM})" >&2; exit 1)

# Install CA roots before switching to snapshot mirrors (which redirect to HTTPS).
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates=${CA_CERTS_VERSION} && \
    rm -rf /var/lib/apt/lists/*

# Freeze Ubuntu package resolution to a timestamped snapshot so transitive
# dependency versions cannot drift over time.
RUN printf 'Acquire::Check-Valid-Until "false";\n' > /etc/apt/apt.conf.d/99ubuntu-snapshot && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" > /etc/apt/sources.list && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal-updates main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" >> /etc/apt/sources.list && \
    printf 'deb [check-valid-until=no] https://snapshot.ubuntu.com/ubuntu/%s focal-security main restricted universe multiverse\n' "${UBUNTU_SNAPSHOT}" >> /etc/apt/sources.list

# Pin runtime utility packages.
ARG RT_CURL_VERSION=7.68.0-1ubuntu2.25
ARG GNUPG2_VERSION=2.2.19-3ubuntu2.5
ARG BINUTILS_VERSION=2.34-6ubuntu1.11

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    curl=${RT_CURL_VERSION} \
    gnupg2=${GNUPG2_VERSION} \
    binutils=${BINUTILS_VERSION} && \
    rm -rf /var/lib/apt/lists/*

# Pin GPG keyrings by SHA256 so builds don't break on key rotation
# and we verify we're trusting the same keys every time.
# To update: download the file and sha256sum it.
ARG GRAMINE_KEYRING_SHA256=29c8a028a14f11e3ac3f2ca2669d6ef717656e64d375cd74617319a75db94f13
ARG INTEL_SGX_DEB_SHA256=92f96f84281031d889deb81060c44325f0481aee621ae47a15ae1df4431b4a23

RUN curl -fsSLo /usr/share/keyrings/gramine-keyring.gpg https://packages.gramineproject.io/gramine-keyring-${UBUNTU_CODENAME}.gpg && \
    echo "${GRAMINE_KEYRING_SHA256}  /usr/share/keyrings/gramine-keyring.gpg" | sha256sum -c - && \
    echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/gramine-keyring.gpg] https://packages.gramineproject.io/ '${UBUNTU_CODENAME}' main' > /etc/apt/sources.list.d/gramine.list

RUN curl -fsSLo /usr/share/keyrings/intel-sgx-deb.key https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key && \
    echo "${INTEL_SGX_DEB_SHA256}  /usr/share/keyrings/intel-sgx-deb.key" | sha256sum -c - && \
    echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx-deb.key] https://download.01.org/intel-sgx/sgx_repo/ubuntu '${UBUNTU_CODENAME}' main' > /etc/apt/sources.list.d/intel-sgx.list

# Pin Gramine + SGX package versions for reproducible MRENCLAVE.
# Bump these deliberately when upgrading.
# Query current versions: dpkg-query -W -f '${Package}=${Version}\n' gramine sgx-aesm-service
ARG GRAMINE_VERSION=1.8
ARG SGX_AESM_VERSION=2.25.100.3-focal1
ARG SGX_DCAP_QV_VERSION=1.22.100.3-focal1
ARG PSMISC_VERSION=23.3-1

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    gramine=${GRAMINE_VERSION} \
    gramine-ratls-dcap=${GRAMINE_VERSION} \
    sgx-aesm-service=${SGX_AESM_VERSION} \
    libsgx-aesm-launch-plugin=${SGX_AESM_VERSION} \
    libsgx-aesm-quote-ex-plugin=${SGX_AESM_VERSION} \
    libsgx-aesm-ecdsa-plugin=${SGX_AESM_VERSION} \
    libsgx-dcap-quote-verify=${SGX_DCAP_QV_VERSION} \
    psmisc=${PSMISC_VERSION} && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/relational-rust-server /app/relational-rust-server
COPY rust-server.manifest.template /app/rust-server.manifest.template

# Generate the Gramine manifest and rewrite sgx.trusted_files as an explicit
# sorted list to avoid nondeterminism from recursive directory traversal order.
RUN ENTRYPOINT=$(which gramine-ratls) && \
    gramine-manifest \
    -Dentrypoint=${ENTRYPOINT} \
    -Dlog_level=error \
    -Darch_libdir=/lib/x86_64-linux-gnu \
    -Dapp_dir=/app \
    -Dself_exe=/app/relational-rust-server \
    -Dra_type=dcap \
    -Ddata_dir=/data \
    -Disvprodid=0 \
    -Disvsvn=0 \
    /app/rust-server.manifest.template /app/rust-server.manifest && \
    GRAMINE_RUNTIME_DIR=$(gramine-runtimedir 2>/dev/null || echo /usr/lib/x86_64-linux-gnu/gramine/runtime/glibc) && \
    { \
      echo 'sgx.trusted_files = ['; \
      echo '    "file:/app/relational-rust-server",'; \
      echo "    \"file:${ENTRYPOINT}\","; \
      find "${GRAMINE_RUNTIME_DIR}" /lib/x86_64-linux-gnu \
        -mindepth 1 \( -type f -o -type l \) -printf '%p\n' \
        | LC_ALL=C sort -u \
        | sed 's#^#    "file:#; s#$#",#'; \
      echo ']'; \
    } > /tmp/trusted_files.toml && \
    awk ' \
      BEGIN { in_block=0; replaced=0 } \
      /^sgx\.trusted_files[[:space:]]*=[[:space:]]*\[/ { \
        if (!replaced) { \
          while ((getline line < "/tmp/trusted_files.toml") > 0) print line; \
          close("/tmp/trusted_files.toml"); \
          replaced=1; \
        } \
        in_block=1; \
        next; \
      } \
      in_block && /^\]/ { in_block=0; next } \
      !in_block { print } \
    ' /app/rust-server.manifest > /app/rust-server.manifest.det && \
    mv /app/rust-server.manifest.det /app/rust-server.manifest && \
    rm -f /tmp/trusted_files.toml

# ---------- SIGN THE ENCLAVE AT BUILD TIME ----------
# The signing key is injected via Docker build secret so it never
# ends up in a layer.  This produces rust-server.manifest.sgx and
# rust-server.sig which lock MRENCLAVE and MRSIGNER into the image.
#
# Usage:  docker build --secret id=sgx-key,src=/path/to/enclave-key.pem ...
RUN --mount=type=secret,id=sgx-key,required=true \
    gramine-sgx-sign \
        --manifest /app/rust-server.manifest \
        --output /app/rust-server.manifest.sgx \
        --key /run/secrets/sgx-key

RUN mkdir -p /var/run/aesmd/ /data

COPY docker/restart_aesm.sh /restart_aesm.sh
COPY docker/start.sh /start.sh

ENV HOST=0.0.0.0
EXPOSE 8080

ENTRYPOINT ["/start.sh"]
