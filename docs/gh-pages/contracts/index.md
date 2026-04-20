---
layout: default
title: Smart Contracts
nav_order: 5
has_children: true
permalink: /contracts/
---

# Smart Contracts
{: .fs-8 }

The `RelationalEuro (rEUR)` ERC-20 token powering fiat settlement on Avalanche C-Chain.
{: .fs-5 .fw-300 }

---

## Deployed Contract

| Property | Value |
|:---------|:------|
| **Name** | Relational Euro |
| **Symbol** | rEUR |
| **Network** | Avalanche Fuji (testnet) |
| **Chain ID** | `43113` |
| **Address** | [`0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`](https://testnet.snowtrace.io/address/0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63) |
| **Deployment Tx** | [`0x8987...289f14`](https://testnet.snowtrace.io/tx/0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14) |
| **Deployed** | February 11, 2026 |
| **Compiler** | Solc 0.8.24, optimizer 200 runs, EVM Paris |
| **Framework** | Foundry |

---

## Overview

rEUR is a managed ERC-20 stablecoin representing euros on the Avalanche C-Chain. It is minted when users deposit euros via the TrueLayer fiat on-ramp, and burned when users withdraw via the off-ramp.

The contract inherits from OpenZeppelin:
- **ERC20** --- Standard token interface
- **ERC20Burnable** --- Holders can burn their own tokens
- **Pausable** --- Admin can pause all transfers in emergencies
- **AccessControl** --- Role-based permission system

---

## Role Model

| Role | Identifier | Capabilities |
|:-----|:-----------|:-------------|
| `DEFAULT_ADMIN_ROLE` | `0x000...000` | Grant/revoke any role |
| `MINTER_ROLE` | `keccak256("MINTER_ROLE")` | Mint new tokens to any address |
| `PAUSER_ROLE` | `keccak256("PAUSER_ROLE")` | Pause and unpause all transfers |

The reserve wallet (inside the SGX enclave) holds the `MINTER_ROLE` to mint rEUR during on-ramp settlement.

---

## Key Behaviors

| Behavior | Description |
|:---------|:------------|
| **Minting** | Only `MINTER_ROLE` can mint. Called during fiat on-ramp settlement. |
| **Burning** | Any holder can burn their own tokens. Called during fiat off-ramp. |
| **Pausing** | `PAUSER_ROLE` can pause all transfers, minting, and burning. Emergency use only. |
| **Transfers** | Standard ERC-20 transfers. No transfer restrictions in v1. |
| **Decimals** | 6 (matching USDC convention for EUR-pegged tokens) |

---

## Quick Reference

```bash
# Query token info
cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 "name()(string)" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
# "Relational Euro"

cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 "symbol()(string)" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
# "rEUR"

cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 "totalSupply()(uint256)" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
```

---

## Sub-pages

- [**rEUR Overview**](/relational-wallet/contracts/reur-overview) --- Contract specification, interface, and invariants
- [**Testing**](/relational-wallet/contracts/testing) --- Foundry test suite and coverage
- [**Deployment (Fuji)**](/relational-wallet/contracts/deployment-fuji) --- Deployment runbook and post-deploy verification
