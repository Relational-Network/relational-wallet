#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

set -e

/restart_aesm.sh

KEY_PATH="${GRAMINE_SGX_SIGNING_KEY:-/keys/enclave-key.pem}"

if [ ! -f /app/rust-server.manifest.sgx ]; then
    if [ ! -f "${KEY_PATH}" ]; then
        echo "Missing SGX signing key at ${KEY_PATH}." >&2
        echo "Mount your host key and set GRAMINE_SGX_SIGNING_KEY if needed." >&2
        exit 1
    fi

    gramine-sgx-sign \
        --manifest /app/rust-server.manifest \
        --output /app/rust-server.manifest.sgx \
        --key "${KEY_PATH}"
fi

exec gramine-sgx rust-server
