#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# Start script for SGX container with DCAP RA-TLS attestation
# This script:
# 1. Restarts AESM service for SGX attestation
# 2. Verifies the enclave is pre-signed (done at build time)
# 3. Launches the server via gramine-sgx (which invokes gramine-ratls)
#
# NOTE: The enclave is signed at docker build time.  No signing key is
# needed at runtime.  This guarantees that MRENCLAVE and MRSIGNER are
# fixed for every container started from the same image.

set -e

/restart_aesm.sh

if [ ! -f /app/rust-server.manifest.sgx ]; then
    echo "FATAL: /app/rust-server.manifest.sgx not found." >&2
    echo "The enclave must be signed at build time." >&2
    echo "Rebuild the image with: --secret id=sgx-key,src=/path/to/enclave-key.pem" >&2
    exit 1
fi

echo "Starting Rust server with DCAP RA-TLS attestation..."
echo "Server will be available at https://0.0.0.0:8080"

# gramine-sgx will execute the manifest which uses gramine-ratls as entrypoint
# gramine-ratls generates TLS cert/key with DCAP attestation evidence,
# then executes the actual Rust server binary
exec gramine-sgx rust-server
