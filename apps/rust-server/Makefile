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

# Directory for encrypted persistent storage (on host)
# This will be mounted as /data inside the enclave with Gramine encryption
DATA_DIR ?= $(abspath data)

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

# Create data directory for encrypted storage
$(DATA_DIR):
	mkdir -p $(DATA_DIR)

rust-server.manifest: rust-server.manifest.template $(SELF_EXE) | $(DATA_DIR)
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
		-Ddata_dir=$(DATA_DIR) \
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
	cd docker && sudo SGX_SIGNING_KEY="$${SGX_SIGNING_KEY:-$$HOME/.config/gramine/enclave-key.pem}" ./build.sh ubuntu20

.PHONY: docker-run
docker-run: | $(DATA_DIR)
	# Exit 137 (SIGKILL) is expected — Gramine SGX does not propagate
	# SIGTERM to the enclave, so the container is always force-killed
	# after the stop-timeout.  This is harmless: Gramine's encrypted
	# filesystem flushes writes synchronously.
	-sudo docker run --rm -it \
		--name rust-server-sgx \
		--device /dev/sgx/enclave \
		--device /dev/sgx/provision \
		--stop-timeout 30 \
		-p 8080:8080 \
		-v "$(DATA_DIR):/data" \
		relationalnetwork/rust-server:focal

.PHONY: docker-stop
docker-stop:
	sudo docker stop -t 30 rust-server-sgx

.PHONY: docker-sigstruct
docker-sigstruct:
	@echo "Extracting SIGSTRUCT from Docker image..."
	@sudo docker create --name rust-server-sigstruct relationalnetwork/rust-server:focal >/dev/null 2>&1
	@sudo docker cp rust-server-sigstruct:/app/rust-server.sig /tmp/rust-server-docker.sig >/dev/null 2>&1
	@sudo docker rm rust-server-sigstruct >/dev/null 2>&1
	@echo "---"
	@gramine-sgx-sigstruct-view --verbose --output-format=toml /tmp/rust-server-docker.sig
	@sudo rm -f /tmp/rust-server-docker.sig

.PHONY: clean
clean:
	$(RM) -rf *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/ Cargo.lock
