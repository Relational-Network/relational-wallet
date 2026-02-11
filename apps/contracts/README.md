<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Relational Network -->

# Relational Euro (`rEUR`) Contracts

This workspace contains the v1 non-upgradeable Euro stablecoin contract and deployment tooling.

## Contract Profile

- Name: `Relational Euro`
- Symbol: `rEUR`
- Decimals: `6`
- Standard: managed ERC20 (`ERC20`, `ERC20Burnable`, `Pausable`, `AccessControl`)
- Roles:
  - `DEFAULT_ADMIN_ROLE`
  - `MINTER_ROLE`
  - `PAUSER_ROLE`

AML/KYC and transfer compliance restrictions are intentionally out of scope in this phase.

## Prerequisites

1. Install Foundry:

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

2. From this directory, install dependencies:

```bash
cd apps/contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge install foundry-rs/forge-std --no-git
```

`--no-git` is recommended in this monorepo to avoid nested git submodules.

## Run Tests

```bash
cd apps/contracts
forge test -vv
```

## Deploy to Avalanche Fuji

1. Create and fill your env file:

```bash
cd apps/contracts
cp .env.example .env
```

2. Export variables:

```bash
set -a
source .env
set +a
```

3. Run deployment:

```bash
forge script script/DeployFuji.s.sol:DeployFuji \
  --rpc-url "$FUJI_RPC_URL" \
  --broadcast
```

The script prints:
- deployed contract address
- admin/minter/pauser addresses
- role IDs

## Verify Assigned Roles

Set token address after deployment:

```bash
TOKEN_ADDRESS=0xREPLACE_WITH_DEPLOYED_ADDRESS
```

Check role constants:

```bash
MINTER_ROLE=$(cast keccak "MINTER_ROLE")
PAUSER_ROLE=$(cast keccak "PAUSER_ROLE")
DEFAULT_ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000
```

Verify role membership:

```bash
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$DEFAULT_ADMIN_ROLE" "$ADMIN_ADDRESS" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$MINTER_ROLE" "$MINTER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$PAUSER_ROLE" "$PAUSER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
```

## Address Registry

After deployment, update:

1. `TODO.md` -> `Deployment Address Registry` table.
2. This file with known deployment addresses.

### Known Deployments

| Network | Contract Address | Tx Hash | Notes |
|---|---|---|---|
| Fuji | `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63` | `0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14` | Deployed on Feb 11, 2026 |
| Avalanche Mainnet | TBD | TBD | Future |

## Deferred Integration

Backend/SGX integration and frontend token wiring are intentionally deferred to a later phase.
