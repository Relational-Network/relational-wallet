# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# SGX-only Makefile for Rust server with DCAP RA-TLS attestation
# This build requires Intel SGX hardware and DCAP infrastructure.
# No non-SGX builds are supported.

ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

# Find gramine-ratls binary (required for RA-TLS)
ENTRYPOINT ?= $(firstword $(wildcard /usr/bin/gramine-ratls /usr/local/bin/gramine-ratls /bin/gramine-ratls))

# SELF_EXE = target/release/relational-rust-server # For production release builds (see Dockerfile)
SELF_EXE = target/debug/relational-rust-server
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

# Persistent dev storage key (survives enclave rebuilds unlike _sgx_mrsigner)
# Generate once: python3 -c "import os; open('data/.dev_storage_key','wb').write(os.urandom(16))"
DEV_KEY_FILE = $(DATA_DIR)/.dev_storage_key

# Read the dev key as hex for the manifest template
DEV_STORAGE_KEY = $(shell xxd -p -c 16 $(DEV_KEY_FILE) 2>/dev/null || echo "")

.PHONY: all
all: $(SELF_EXE) $(DEV_KEY_FILE) rust-server.manifest rust-server.manifest.sgx rust-server.sig

# Generate a persistent 16-byte dev storage key if it doesn't exist
$(DEV_KEY_FILE): | $(DATA_DIR)
	@if [ ! -f "$(DEV_KEY_FILE)" ]; then \
		python3 -c "import os; open('$(DEV_KEY_FILE)', 'wb').write(os.urandom(16))"; \
		echo "Generated persistent dev storage key at $(DEV_KEY_FILE)"; \
	fi

# Dev builds for faster iteration. The DEBUG setting controls Gramine's loglevel only.
# Production release builds are handled by Docker (see docker/build.sh).
-include $(SELF_EXE).d # See also: .cargo/config.toml
$(SELF_EXE): Cargo.toml Cargo.lock $(SRCS)
	cargo build --features dev

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
		-Ddev_storage_key=$(DEV_STORAGE_KEY) \
		$< $@

rust-server.manifest.sgx rust-server.sig &: rust-server.manifest
	gramine-sgx-sign \
		--manifest $< \
		--output $<.sgx

.PHONY: start-rust-server
start-rust-server: all
	@if [ -f .env ]; then \
		echo "Loading environment from .env"; \
		if grep -nE '^[[:space:]]*[^#[:space:]][^=]*=.*<[^>]+>' .env >/dev/null 2>&1; then \
			echo "error: .env contains placeholder syntax like <...> which breaks shell parsing."; \
			echo "replace placeholder values with literal URLs/values."; \
			grep -nE '^[[:space:]]*[^#[:space:]][^=]*=.*<[^>]+>' .env; \
			exit 1; \
		fi; \
		set -a; . ./.env; set +a; \
	fi; \
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
	#
	# Auth & CORS env vars are loaded from .env via --env-file.
	# This avoids the problem of `sudo` stripping the user's environment.
	# Create .env from .env.example if you haven't already.
	@if [ ! -f .env ]; then \
		echo "error: .env file not found. Copy .env.example to .env and fill in the values."; \
		exit 1; \
	fi
	-sudo docker run --rm -it \
		--name rust-server-sgx \
		--device /dev/sgx/enclave \
		--device /dev/sgx/provision \
		--stop-timeout 30 \
		-p 8080:8080 \
		-v "$(DATA_DIR):/data" \
		--env-file .env \
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
