# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network

# SGX-only Makefile for Rust server with DCAP RA-TLS attestation
# This build requires Intel SGX hardware and DCAP infrastructure.
# No non-SGX builds are supported.

# Use fixed x86_64-linux-gnu path (clang may report x86_64-pc-linux-gnu,
# which does not exist on Ubuntu/Gramine systems).
ARCH_LIBDIR ?= /lib/x86_64-linux-gnu

# Find gramine-ratls binary (required for RA-TLS)
ENTRYPOINT ?= $(firstword $(wildcard /usr/bin/gramine-ratls /usr/local/bin/gramine-ratls /bin/gramine-ratls))

# SELF_EXE = target/release/relational-rust-server # For production release builds (see Dockerfile)
SELF_EXE = target/debug/relational-rust-server
SRCS = $(shell find src -type f)
APP_DIR = $(abspath $(patsubst %/,%,$(dir $(SELF_EXE))))

# Directory for encrypted persistent storage (on host)
# This will be mounted as /data inside the enclave with Gramine encryption
# Override per command if desired, for example:
#   make DATA_DIR=/opt/local-docker/data/rust-server docker-run
DATA_DIR ?= $(abspath data)
DOCKER_APP_UID ?= 10001
DOCKER_APP_GID ?= 10001
SIGNING_KEY ?= $(HOME)/.config/gramine/enclave-key.pem

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

.PHONY: help
help:
	@echo "Common targets:"
	@echo "  make dev-check         - fast host compile check (no SGX runtime)"
	@echo "  make dev-build         - host dev build (target/debug)"
	@echo "  make dev-test          - run Rust tests with --features dev"
	@echo "  make                   - build SGX artifacts (.manifest/.sgx/.sig)"
	@echo "  make start-rust-server - run inside SGX enclave (loads .env if present)"
	@echo "  make docker-build      - build SGX Docker image"
	@echo "  make docker-data-dir   - create host data dir (override with DATA_DIR=/path)"
	@echo "  make docker-run        - run SGX Docker image (override with DATA_DIR=/path)"
	@echo "  make docker-stop       - stop running SGX Docker container"
	@echo "  make show-measurements - inspect local SIGSTRUCT measurements"
	@echo "  make verify-mrenclave  - build signed Docker image locally and compare against measurements.toml"
	@echo "  make docker-sigstruct  - inspect SIGSTRUCT from Docker image"
	@echo "  make clean             - remove generated manifest/signing artifacts"
	@echo "  make distclean         - clean + remove target/ and Cargo.lock"

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
	cargo dev-build

.PHONY: dev-build
dev-build:
	cargo dev-build

.PHONY: dev-check
dev-check:
	cargo dev-check

.PHONY: dev-test
dev-test:
	cargo dev-test

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
		--output $<.sgx \
		--date 0000-00-00

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

.PHONY: docker-data-dir
# Example custom path:
#   make DATA_DIR=/opt/local-docker/data/rust-server docker-data-dir
docker-data-dir:
	sudo install -d -m 0750 -o $(DOCKER_APP_UID) -g $(DOCKER_APP_GID) "$(DATA_DIR)"

.PHONY: docker-run
# Override DATA_DIR for a deployment-like local bind mount, for example:
#   make DATA_DIR=/opt/local-docker/data/rust-server docker-run
# Exit 137 (SIGKILL) is expected because Gramine SGX does not propagate
# SIGTERM to the enclave, so Docker force-stops the container after the
# configured stop-timeout. This is harmless: Gramine's encrypted filesystem
# flushes writes synchronously.
#
# Auth & CORS env vars are loaded from .env via --env-file.
# This avoids the problem of `sudo` stripping the user's environment.
# Create .env from .env.example if you haven't already.
docker-run: docker-data-dir
	@if [ ! -f .env ]; then \
		echo "error: .env file not found. Copy .env.example to .env and fill in the values."; \
		exit 1; \
	fi
	@status=0; \
	sudo docker run --rm -it \
		--name rust-server-sgx \
		--device /dev/sgx/enclave \
		--device /dev/sgx/provision \
		--stop-timeout 30 \
		-p 8080:8080 \
		-v "$(DATA_DIR):/data" \
		--env-file .env \
		relationalnetwork/rust-server:focal || status=$$?; \
	case "$$status" in \
		0|130|137) exit 0 ;; \
		*) exit "$$status" ;; \
	esac

.PHONY: show-measurements
show-measurements: rust-server.sig
	@gramine-sgx-sigstruct-view rust-server.sig | grep -E "mr_enclave|mr_signer|isv_prod_id|isv_svn"

.PHONY: verify-mrenclave
verify-mrenclave:
	@if [ ! -f "$(SIGNING_KEY)" ]; then \
		echo "ERROR: Signing key not found at $(SIGNING_KEY)"; \
		echo "Set SIGNING_KEY=/path/to/enclave-key.pem"; \
		exit 1; \
	fi
	@EXPECTED_MR=$$(sed -n 's/^mr_enclave = "\(.*\)"$$/\1/p' measurements.toml | head -1); \
	if ! printf '%s\n' "$$EXPECTED_MR" | grep -Eq '^[0-9a-f]{64}$$'; then \
		echo "ERROR: measurements.toml does not contain a pinned mr_enclave yet."; \
		echo "Run 'make docker-build' and 'make docker-sigstruct', then record the value in measurements.toml."; \
		exit 1; \
	fi
	@echo "=== Building locally (no cache) ==="
	@cd docker && sudo DOCKER_BUILDKIT=1 SGX_SIGNING_KEY="$(SIGNING_KEY)" docker build \
		--platform linux/amd64 \
		--no-cache \
		--build-arg UBUNTU_CODENAME=focal \
		--secret id=sgx-key,src="$(SIGNING_KEY)" \
		-t relationalnetwork/rust-server:verify-local \
		-f Dockerfile \
		.. >/dev/null
	@BUILT_MR=$$(sudo docker run --rm \
		--entrypoint gramine-sgx-sigstruct-view \
		relationalnetwork/rust-server:verify-local \
		/app/rust-server.sig 2>/dev/null | grep 'mr_enclave:' | awk '{print $$2}' | head -1); \
	EXPECTED_MR=$$(sed -n 's/^mr_enclave = "\(.*\)"$$/\1/p' measurements.toml | head -1); \
	echo "Built:    $$BUILT_MR"; \
	echo "Expected: $$EXPECTED_MR  (measurements.toml)"; \
	if [ "$$BUILT_MR" = "$$EXPECTED_MR" ]; then \
		echo "MATCH - MRENCLAVE matches measurements.toml"; \
	else \
		echo "MISMATCH - MRENCLAVE differs from measurements.toml!"; \
		echo "If intentional, update measurements.toml in the same PR."; \
		exit 1; \
	fi

.PHONY: docker-stop
docker-stop:
	sudo docker stop -t 30 rust-server-sgx

.PHONY: docker-sigstruct
docker-sigstruct:
	@echo "Extracting enclave measurements from Docker image..."
	@sudo docker create --name rust-server-sigstruct relationalnetwork/rust-server:focal >/dev/null 2>&1
	@sudo docker cp rust-server-sigstruct:/app/rust-server.sig /tmp/rust-server-docker.sig >/dev/null 2>&1
	@sudo docker rm rust-server-sigstruct >/dev/null 2>&1
	@echo "[enclave]"
	@gramine-sgx-sigstruct-view --verbose --output-format=toml /tmp/rust-server-docker.sig \
		| awk ' \
			/^mr_enclave = / { mr_enclave = $$0 } \
			/^mr_signer = / { mr_signer = $$0 } \
			/^isv_prod_id = / { isv_prod_id = $$0 } \
			/^isv_svn = / { isv_svn = $$0 } \
			/^debug_enclave = / { debug_enclave = $$0 } \
			END { \
				print mr_enclave; \
				print mr_signer; \
				print isv_prod_id; \
				print isv_svn; \
				print debug_enclave; \
			} \
		'
	@sudo rm -f /tmp/rust-server-docker.sig

.PHONY: clean
clean:
	$(RM) -rf *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/ Cargo.lock
