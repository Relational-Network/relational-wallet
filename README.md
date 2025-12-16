# Relational Wallet

Relational Wallet is an MVP non-custodial stablecoin wallet that uses **Trusted Execution Environments (TEEs)** for key isolation and transaction signing, with **Avalanche** as the settlement ledger.

This repository is organized as a monorepo containing a TEE-backed enclave for wallet operations and a lightweight front-end for user interaction.

## Components

- **[Wallet Enclave](apps/wallet-enclave/)** — TEE-based key management and transaction signing.
- **[Wallet Web](apps/wallet-web/)** — Minimal web interface integrating the Wallet SDK.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

You may copy, modify, and redistribute this work under the terms of the AGPL-3.0.
A full copy of the license can be found in the `LICENSE` file or at:

👉 https://www.gnu.org/licenses/agpl-3.0.html

## Origin and Attribution

This project is a ground-up rewrite and expansion inspired by and derived from
the open-source **Nautilus Wallet** project:
https://github.com/ntls-io/nautilus-wallet

Nautilus Wallet is licensed under the GNU Affero General Public License v3.0
(AGPL-3.0). While this repository does not directly fork the original codebase,
multiple components and architectural concepts are derived from the Nautilus
Wallet design and implementation and reimplemented in a new architecture.
