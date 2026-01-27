# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

ARG UBUNTU_IMAGE=ubuntu:20.04

FROM ${UBUNTU_IMAGE} AS builder

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y curl ca-certificates build-essential && \
    rm -rf /var/lib/apt/lists/*

ENV CARGO_HOME=/opt/cargo
ENV RUSTUP_HOME=/opt/rustup
ENV PATH=/opt/cargo/bin:/opt/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:${PATH}

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo fetch
RUN cargo build --release

ARG UBUNTU_CODENAME=focal

FROM ${UBUNTU_IMAGE}

# ARGs cannot be grouped since each FROM in a Dockerfile initiates a new build
# stage, resulting in the loss of ARG values from earlier stages.
ARG UBUNTU_CODENAME=focal

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y curl gnupg2 binutils

RUN curl -fsSLo /usr/share/keyrings/gramine-keyring.gpg https://packages.gramineproject.io/gramine-keyring-${UBUNTU_CODENAME}.gpg && \
    echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/gramine-keyring.gpg] https://packages.gramineproject.io/ '${UBUNTU_CODENAME}' main' > /etc/apt/sources.list.d/gramine.list

RUN curl -fsSLo /usr/share/keyrings/intel-sgx-deb.key https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key && \
    echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx-deb.key] https://download.01.org/intel-sgx/sgx_repo/ubuntu '${UBUNTU_CODENAME}' main' > /etc/apt/sources.list.d/intel-sgx.list

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y gramine \
    sgx-aesm-service \
    libsgx-aesm-launch-plugin \
    libsgx-aesm-quote-ex-plugin \
    libsgx-aesm-ecdsa-plugin \
    libsgx-dcap-quote-verify \
    psmisc && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/relational-rust-server /app/relational-rust-server
COPY rust-server.manifest.template /app/rust-server.manifest.template

RUN gramine-manifest \
    -Dlog_level=error \
    -Darch_libdir=/lib/x86_64-linux-gnu \
    -Dapp_dir=/app \
    -Dself_exe=/app/relational-rust-server \
    /app/rust-server.manifest.template /app/rust-server.manifest

RUN mkdir -p /var/run/aesmd/

COPY docker/restart_aesm.sh /restart_aesm.sh
COPY docker/start.sh /start.sh

ENV HOST=0.0.0.0
EXPOSE 8080

ENTRYPOINT ["/start.sh"]
