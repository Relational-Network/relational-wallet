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

## Header Checks

Use the repo script to verify SPDX and copyright headers (any year by default):

```bash
./scripts/check_headers.sh
```

To require a specific year (e.g., 2026):

```bash
REQUIRED_YEAR=2026 ./scripts/check_headers.sh
```

To update the year across headers (e.g., 2025 -> 2026):

```bash
./scripts/update_header_year.sh 2025 2026
```


