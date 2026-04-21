# contracts — Relational Euro (`rEUR`)

Foundry workspace for the v1 non-upgradeable Euro stablecoin used by rust-server fiat settlement.

| | |
|---|---|
| Name | `Relational Euro` |
| Symbol | `rEUR` |
| Decimals | `6` |
| Standard | Managed ERC-20 (`ERC20`, `ERC20Burnable`, `Pausable`, `AccessControl`) |
| Roles | `DEFAULT_ADMIN_ROLE`, `MINTER_ROLE`, `PAUSER_ROLE` |

AML/KYC and transfer compliance are out of scope in this phase.

## Deployments

| Network | Address | Tx |
|---------|---------|----|
| Fuji | [`0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`](https://testnet.snowtrace.io/address/0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63) | [`0x89878d99…`](https://testnet.snowtrace.io/tx/0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14) |
| Avalanche Mainnet | — | — |

The Fuji address is wired into rust-server via `REUR_CONTRACT_ADDRESS_FUJI` in [`../rust-server/.env.example`](../rust-server/.env.example).

## Prerequisites

```bash
curl -L https://foundry.paradigm.xyz | bash && foundryup
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge install foundry-rs/forge-std --no-git
```

`--no-git` avoids nested submodules in this monorepo.

## Test

```bash
forge test -vv
```

## Deploy

```bash
cp .env.example .env    # fill PRIVATE_KEY, ADMIN/MINTER/PAUSER, FUJI_RPC_URL
set -a && source .env && set +a

forge script script/DeployFuji.s.sol:DeployFuji \
  --rpc-url "$FUJI_RPC_URL" \
  --broadcast
```

The script prints the deployed address, role IDs, and role holders. Record new deployments in the table above and update `REUR_CONTRACT_ADDRESS_*` in rust-server's env template.

## Verify roles

```bash
TOKEN=0x<deployed-address>
MINTER_ROLE=$(cast keccak "MINTER_ROLE")
PAUSER_ROLE=$(cast keccak "PAUSER_ROLE")
ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000

cast call "$TOKEN" "hasRole(bytes32,address)(bool)" "$ADMIN_ROLE"  "$ADMIN_ADDRESS"  --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN" "hasRole(bytes32,address)(bool)" "$MINTER_ROLE" "$MINTER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN" "hasRole(bytes32,address)(bool)" "$PAUSER_ROLE" "$PAUSER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
```

---

SPDX-License-Identifier: AGPL-3.0-or-later · Copyright (C) 2026 Relational Network
