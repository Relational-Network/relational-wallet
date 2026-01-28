# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# SGX-only Makefile for Rust server with DCAP RA-TLS attestation
# This build requires Intel SGX hardware and DCAP infrastructure.
# No non-SGX builds are supported.

ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

# Find gramine-ratls binary (required for RA-TLS)
ENTRYPOINT ?= $(firstword $(wildcard /usr/bin/gramine-ratls /usr/local/bin/gramine-ratls /bin/gramine-ratls))

SELF_EXE = target/release/relational-rust-server
SRCS = $(shell find src -type f)
APP_DIR = $(abspath $(patsubst %/,%,$(dir $(SELF_EXE))))

# RA type is always DCAP - no EPID support
RA_TYPE = dcap

# SGX enclave identity values
ISVPRODID ?= 0
ISVSVN ?= 0

ifeq ($(DEBUG),1)
GRAMINE_LOG_LEVEL = debug
else
GRAMINE_LOG_LEVEL = error
endif

.PHONY: all
all: $(SELF_EXE) rust-server.manifest rust-server.manifest.sgx rust-server.sig

# Note that we're compiling in release mode regardless of the DEBUG setting passed
# to Make, as compiling in debug mode results in an order of magnitude's difference in
# performance that makes testing by running a benchmark with ab painful. The primary goal
# of the DEBUG setting is to control Gramine's loglevel.
-include $(SELF_EXE).d # See also: .cargo/config.toml
$(SELF_EXE): Cargo.toml Cargo.lock $(SRCS)
	cargo build --release

rust-server.manifest: rust-server.manifest.template $(SELF_EXE)
	@if [ -z "$(ENTRYPOINT)" ]; then \
		echo "error: gramine-ratls not found; install gramine-ratls-dcap or set ENTRYPOINT=/path/to/gramine-ratls"; \
		exit 1; \
	fi
	gramine-manifest \
		-Dentrypoint=$(ENTRYPOINT) \
		-Dlog_level=$(GRAMINE_LOG_LEVEL) \
		-Darch_libdir=$(ARCH_LIBDIR) \
		-Dapp_dir=$(APP_DIR) \
		-Dself_exe=$(SELF_EXE) \
		-Dra_type=$(RA_TYPE) \
		-Disvprodid=$(ISVPRODID) \
		-Disvsvn=$(ISVSVN) \
		$< $@

rust-server.manifest.sgx rust-server.sig &: rust-server.manifest
	gramine-sgx-sign \
		--manifest $< \
		--output $<.sgx

.PHONY: start-rust-server
start-rust-server: all
	gramine-sgx rust-server

.PHONY: docker-build
docker-build:
	cd docker && sudo ./build.sh ubuntu20

.PHONY: docker-run
docker-run:
	sudo docker run --rm -it \
		--name rust-server-sgx \
		--device /dev/sgx/enclave \
		--device /dev/sgx/provision \
		-p 8080:8080 \
		-v "$$HOME/.config/gramine/enclave-key.pem:/keys/enclave-key.pem:ro" \
		-e GRAMINE_SGX_SIGNING_KEY=/keys/enclave-key.pem \
		relationalnetwork/rust-server:focal

.PHONY: docker-stop
docker-stop:
	sudo docker stop rust-server-sgx

.PHONY: clean
clean:
	$(RM) -rf *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/ Cargo.lock
