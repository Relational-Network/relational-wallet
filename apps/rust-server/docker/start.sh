#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# Start script for SGX container with DCAP RA-TLS attestation.
# This script:
# 1. Starts AESM as root for SGX attestation support
# 2. Prepares SGX device groups and the persistent /data mount
# 3. Drops privileges to the fixed service user before launching gramine-sgx
#
# NOTE: The enclave is signed at docker build time.  No signing key is
# needed at runtime.  This guarantees that MRENCLAVE and MRSIGNER are
# fixed for every container started from the same image.

set -eu

APP_USER=relational
DATA_DIR=/data
STARTUP_UMASK=${RELATIONAL_STARTUP_UMASK:-027}

log() {
    printf '%s\n' "$*"
}

warn() {
    printf 'warning: %s\n' "$*" >&2
}

fatal() {
    printf 'FATAL: %s\n' "$*" >&2
    exit 1
}

app_uid() {
    id -u "${APP_USER}"
}

app_gid() {
    id -g "${APP_USER}"
}

user_in_group() {
    _user=$1
    _group=$2
    id -nG "${_user}" | tr ' ' '\n' | grep -Fx "${_group}" >/dev/null 2>&1
}

ensure_runtime_group() {
    _gid=$1
    _label=$2

    if [ "${_gid}" = "0" ]; then
        warn "${_label} is owned by GID 0; refusing to add ${APP_USER} to the root group. Non-root SGX access may require host-side device permission changes."
        return 0
    fi

    _group_name=$(getent group "${_gid}" | cut -d: -f1 || true)
    if [ -z "${_group_name}" ]; then
        _group_name="relational-sgx-${_gid}"
        groupadd --gid "${_gid}" "${_group_name}"
    fi

    if user_in_group "${APP_USER}" "${_group_name}"; then
        return 0
    fi

    usermod -a -G "${_group_name}" "${APP_USER}"
}

configure_sgx_access() {
    _seen_gids=" "

    for _device in /dev/sgx/enclave /dev/sgx/provision; do
        if [ ! -e "${_device}" ]; then
            continue
        fi

        _gid=$(stat -c '%g' "${_device}")
        case " ${_seen_gids} " in
            *" ${_gid} "*) continue ;;
        esac
        _seen_gids="${_seen_gids}${_gid} "

        ensure_runtime_group "${_gid}" "${_device}"
    done
}

run_as_app() {
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "${APP_USER}" -- "$@"
        return
    fi

    if command -v setpriv >/dev/null 2>&1; then
        setpriv --reuid "$(app_uid)" --regid "$(app_gid)" --init-groups "$@"
        return
    fi

    if command -v su >/dev/null 2>&1; then
        su -s /bin/sh "${APP_USER}" -c 'exec "$@"' sh "$@"
        return
    fi

    fatal "No supported privilege drop helper found (expected one of: runuser, setpriv, su)."
}

exec_as_app() {
    if command -v setpriv >/dev/null 2>&1; then
        exec setpriv --reuid "$(app_uid)" --regid "$(app_gid)" --init-groups "$@"
    fi

    if command -v runuser >/dev/null 2>&1; then
        exec runuser -u "${APP_USER}" -- "$@"
    fi

    if command -v su >/dev/null 2>&1; then
        exec su -s /bin/sh "${APP_USER}" -c 'exec "$@"' sh "$@"
    fi

    fatal "No supported privilege drop helper found (expected one of: setpriv, runuser, su)."
}

prepare_data_dir() {
    if [ -e "${DATA_DIR}" ] && [ ! -d "${DATA_DIR}" ]; then
        fatal "${DATA_DIR} exists but is not a directory."
    fi

    mkdir -p "${DATA_DIR}"

    chmod 0750 "${DATA_DIR}" 2>/dev/null || true

    if ! run_as_app test -w "${DATA_DIR}"; then
        fatal "${DATA_DIR} is not writable by ${APP_USER}. Pre-create the mount with UID $(app_uid) and GID $(app_gid)."
    fi
}

if [ "$(id -u)" -ne 0 ]; then
    fatal "This container must start as root so it can bootstrap AESM and then drop to ${APP_USER}."
fi

if ! id "${APP_USER}" >/dev/null 2>&1; then
    fatal "Required runtime user ${APP_USER} is missing from the image."
fi

umask "${STARTUP_UMASK}"

/restart_aesm.sh

if [ ! -f /app/rust-server.manifest.sgx ]; then
    fatal "/app/rust-server.manifest.sgx not found. Rebuild the image with: --secret id=sgx-key,src=/path/to/enclave-key.pem"
fi

configure_sgx_access
prepare_data_dir

log "Starting Rust server with DCAP RA-TLS attestation as ${APP_USER}..."
log "Server will be available at https://0.0.0.0:8080"

# gramine-sgx will execute the manifest which uses gramine-ratls as entrypoint.
# gramine-ratls generates TLS cert/key with DCAP attestation evidence, then
# executes the actual Rust server binary inside the enclave.
exec_as_app gramine-sgx rust-server
