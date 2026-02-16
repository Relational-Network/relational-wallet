---
layout: default
title: Deployment (Fuji)
parent: Contracts
nav_order: 3
---

# Deployment (Fuji)

Deployment script:

- `apps/contracts/script/DeployFuji.s.sol`

## 1) Configure Environment

```bash
cd apps/contracts
cp .env.example .env
set -a
source .env
set +a
```

At minimum, provide:

- `FUJI_RPC_URL`
- deployer private key env var used by your script profile

## 2) Deploy

```bash
cd apps/contracts
forge script script/DeployFuji.s.sol:DeployFuji \
  --rpc-url "$FUJI_RPC_URL" \
  --broadcast
```

## 3) Post-Deploy Verification

Set deployed address:

```bash
TOKEN_ADDRESS=0xREPLACE_WITH_DEPLOYED_ADDRESS
```

Check symbol/decimals:

```bash
cast call "$TOKEN_ADDRESS" "symbol()(string)" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN_ADDRESS" "decimals()(uint8)" --rpc-url "$FUJI_RPC_URL"
```

Check role hashes:

```bash
MINTER_ROLE=$(cast keccak "MINTER_ROLE")
PAUSER_ROLE=$(cast keccak "PAUSER_ROLE")
DEFAULT_ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000
```

Check role members:

```bash
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$DEFAULT_ADMIN_ROLE" "$ADMIN_ADDRESS" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$MINTER_ROLE" "$MINTER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
cast call "$TOKEN_ADDRESS" "hasRole(bytes32,address)(bool)" "$PAUSER_ROLE" "$PAUSER_ADDRESS" --rpc-url "$FUJI_RPC_URL"
```
