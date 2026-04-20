---
layout: default
title: Deployment (Fuji)
parent: Smart Contracts
nav_order: 3
---

# Deployment Runbook (Fuji Testnet)
{: .fs-7 }

Step-by-step instructions for deploying the rEUR contract to Avalanche Fuji. The current mainnet-equivalent deployment at `0x76568...1A63` was completed February 11, 2026.
{: .fs-5 .fw-300 }

---

## Current Deployment

| Property | Value |
|:---------|:------|
| **Network** | Avalanche Fuji (testnet, Chain ID 43113) |
| **Address** | [`0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`](https://testnet.snowtrace.io/address/0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63) |
| **Deployment Tx** | [`0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14`](https://testnet.snowtrace.io/tx/0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14) |
| **Deployed** | February 11, 2026 |

---

## Prerequisites

- Foundry installed (see [Testing](/relational-wallet/contracts/testing#prerequisites))
- Avalanche Fuji RPC URL
- Deployer private key with testnet AVAX for gas
- Get testnet AVAX: [Fuji Faucet](https://faucet.avax.network/)

---

## 1. Configure Environment

```bash
cd apps/contracts
cp .env.example .env
```

Edit `.env` with:

```env
FUJI_RPC_URL=https://api.avax-test.network/ext/bc/C/rpc
DEPLOYER_PRIVATE_KEY=0x...           # Deployer wallet private key
ADMIN_ADDRESS=0x...                  # Will receive DEFAULT_ADMIN_ROLE
MINTER_ADDRESS=0x...                 # Will receive MINTER_ROLE (use reserve wallet address)
PAUSER_ADDRESS=0x...                 # Will receive PAUSER_ROLE
```

Load environment:

```bash
set -a
source .env
set +a
```

---

## 2. Compile and Test

Always run tests before deploying:

```bash
forge build
forge test -vv
```

All 5 tests must pass before proceeding.
{: .warning }

---

## 3. Deploy

```bash
forge script script/DeployFuji.s.sol:DeployFuji \
  --rpc-url "$FUJI_RPC_URL" \
  --private-key "$DEPLOYER_PRIVATE_KEY" \
  --broadcast \
  --verify
```

The deployment script:
1. Deploys `RelationalEuro` with constructor args `(admin, minter, pauser)`
2. Broadcasts the deployment transaction
3. Optionally verifies on Snowtrace (requires `SNOWTRACE_API_KEY`)

Note the deployed contract address from the output.

---

## 4. Post-Deploy Verification

Set the deployed address:

```bash
export TOKEN_ADDRESS=0x...  # From deployment output
export RPC=https://api.avax-test.network/ext/bc/C/rpc
```

### Verify Metadata

```bash
cast call "$TOKEN_ADDRESS" "name()(string)" --rpc-url "$RPC"
# "Relational Euro"

cast call "$TOKEN_ADDRESS" "symbol()(string)" --rpc-url "$RPC"
# "rEUR"

cast call "$TOKEN_ADDRESS" "decimals()(uint8)" --rpc-url "$RPC"
# 6

cast call "$TOKEN_ADDRESS" "totalSupply()(uint256)" --rpc-url "$RPC"
# 0 (no tokens minted at deployment)
```

### Verify Roles

```bash
MINTER_ROLE=$(cast keccak "MINTER_ROLE")
PAUSER_ROLE=$(cast keccak "PAUSER_ROLE")
DEFAULT_ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000

# Check admin has DEFAULT_ADMIN_ROLE
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" \
  "$DEFAULT_ADMIN_ROLE" "$ADMIN_ADDRESS" --rpc-url "$RPC"
# true

# Check minter has MINTER_ROLE
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" \
  "$MINTER_ROLE" "$MINTER_ADDRESS" --rpc-url "$RPC"
# true

# Check pauser has PAUSER_ROLE
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" \
  "$PAUSER_ROLE" "$PAUSER_ADDRESS" --rpc-url "$RPC"
# true
```

### Smoke Test: Mint and Burn

```bash
# Mint 1 rEUR (1_000_000 raw units) to test address
cast send "$TOKEN_ADDRESS" "mint(address,uint256)" \
  "$TEST_ADDRESS" 1000000 \
  --private-key "$MINTER_PRIVATE_KEY" \
  --rpc-url "$RPC"

# Verify balance
cast call "$TOKEN_ADDRESS" "balanceOf(address)(uint256)" \
  "$TEST_ADDRESS" --rpc-url "$RPC"
# 1000000

# Burn tokens from test address
cast send "$TOKEN_ADDRESS" "burn(uint256)" 1000000 \
  --private-key "$TEST_PRIVATE_KEY" \
  --rpc-url "$RPC"
```

---

## 5. Update Backend Configuration

After deployment, update the backend environment:

```env
REUR_CONTRACT_ADDRESS_FUJI=0x<new_address>
```

And update the docs/frontend with the new address.

---

## Re-Deployment Notes

If you need to redeploy:

- The contract is **non-upgradeable** — a new deployment creates a new address
- The old contract's tokens become worthless unless you migrate holders
- Always coordinate with the reserve wallet configuration before pointing the backend to a new contract
- Update `REUR_CONTRACT_ADDRESS_FUJI` in all environment files and CI secrets
