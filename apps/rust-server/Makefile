# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Relational Network


ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

SELF_EXE = target/release/relational-rust-server

.PHONY: all
all: $(SELF_EXE) rust-server.manifest
ifeq ($(SGX),1)
all: rust-server.manifest.sgx rust-server.sig
endif

ifeq ($(DEBUG),1)
GRAMINE_LOG_LEVEL = debug
else
GRAMINE_LOG_LEVEL = error
endif

# Note that we're compiling in release mode regardless of the DEBUG setting passed
# to Make, as compiling in debug mode results in an order of magnitude's difference in
# performance that makes testing by running a benchmark with ab painful. The primary goal
# of the DEBUG setting is to control Gramine's loglevel.
-include $(SELF_EXE).d # See also: .cargo/config.toml
$(SELF_EXE): Cargo.toml
	cargo build --release

rust-server.manifest: rust-server.manifest.template $(SELF_EXE)
	gramine-manifest \
		-Dlog_level=$(GRAMINE_LOG_LEVEL) \
		-Darch_libdir=$(ARCH_LIBDIR) \
		-Dself_exe=$(SELF_EXE) \
		$< $@

rust-server.manifest.sgx rust-server.sig &: rust-server.manifest
	gramine-sgx-sign \
		--manifest $< \
		--output $<.sgx

ifeq ($(SGX),)
GRAMINE = gramine-direct
else
GRAMINE = gramine-sgx
endif

.PHONY: start-wallet
start-wallet: all
	$(GRAMINE) rust-server

.PHONY: clean
clean:
	$(RM) -rf *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/ Cargo.lock
