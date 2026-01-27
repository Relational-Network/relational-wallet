#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

usage() {
    echo "Usage: build.sh ubuntu20"
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

image=""
codename=""

case "$1" in
    ubuntu20)
        image="ubuntu:20.04"
        codename="focal"
        ;;
    *)
        usage
        ;;
esac

docker build \
    --build-arg UBUNTU_IMAGE="${image}" \
    --build-arg UBUNTU_CODENAME="${codename}" \
    -t relationalnetwork/rust-server:"${codename}" \
    -f Dockerfile \
    ..
