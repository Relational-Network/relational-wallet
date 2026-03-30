---
layout: default
title: rEUR Overview
parent: Smart Contracts
nav_order: 1
---

# RelationalEuro (rEUR)
{: .fs-7 }

A managed ERC-20 stablecoin representing euros on the Avalanche C-Chain. Minted via fiat on-ramp, burned via fiat off-ramp.
{: .fs-5 .fw-300 }

---

## Contract Specification

| Property | Value |
|:---------|:------|
| **Contract** | `RelationalEuro` |
| **Symbol** | `rEUR` |
| **Decimals** | `6` |
| **Standard** | ERC-20 + Burnable + Pausable + AccessControl |
| **Source** | `apps/contracts/src/RelationalEuro.sol` |
| **Compiler** | Solc ^0.8.24, EVM Paris, optimizer 200 runs |
| **License** | MIT (contract) / AGPL-3.0 (platform) |

---

## Inherited Interfaces

```
RelationalEuro
├── ERC20               (OpenZeppelin)  — standard token
├── ERC20Burnable       (OpenZeppelin)  — holders can burn their tokens
├── Pausable            (OpenZeppelin)  — emergency pause mechanism
└── AccessControl       (OpenZeppelin)  — role-based permissions
```

---

## ABI Reference

### Read Functions

| Function | Returns | Description |
|:---------|:--------|:------------|
| `name()` | `string` | Token name: `"Relational Euro"` |
| `symbol()` | `string` | Token symbol: `"rEUR"` |
| `decimals()` | `uint8` | Always `6` |
| `totalSupply()` | `uint256` | Total tokens in circulation |
| `balanceOf(address)` | `uint256` | Balance of an address (raw, 6 decimals) |
| `allowance(owner, spender)` | `uint256` | Remaining allowance |
| `paused()` | `bool` | Whether transfers are paused |
| `hasRole(bytes32, address)` | `bool` | Check role membership |

### Write Functions

| Function | Role Required | Description |
|:---------|:-------------|:------------|
| `mint(address to, uint256 amount)` | `MINTER_ROLE` | Mint tokens to an address |
| `burn(uint256 amount)` | Any holder | Burn caller's own tokens |
| `burnFrom(address, uint256)` | Approved | Burn tokens from an allowance |
| `pause()` | `PAUSER_ROLE` | Pause all transfers |
| `unpause()` | `PAUSER_ROLE` | Resume transfers |
| `transfer(address, uint256)` | Any holder | Transfer tokens |
| `approve(address, uint256)` | Any holder | Set allowance |
| `grantRole(bytes32, address)` | `DEFAULT_ADMIN_ROLE` | Grant a role |
| `revokeRole(bytes32, address)` | `DEFAULT_ADMIN_ROLE` | Revoke a role |

---

## Role Identifiers

```bash
# Compute role hashes (Foundry cast)
cast keccak "MINTER_ROLE"
# 0x9f2df0fed2c77648de5860a4cc508cd0818c85b8b8a1ab4ceeef8d981c8956a6

cast keccak "PAUSER_ROLE"
# 0x65d7a28e3265b37a6474929f336521b332c1681b933f6cb9f3376673440d862a

# DEFAULT_ADMIN_ROLE is always bytes32(0)
DEFAULT_ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000
```

---

## Key Invariants

1. **Mint authority**: Only addresses with `MINTER_ROLE` can increase supply. The reserve wallet inside the SGX enclave holds this role.
2. **Burn by holders**: Any token holder can burn their own balance. The off-ramp flow calls `burn` before initiating the TrueLayer payout.
3. **Pause blocks everything**: When paused, all `transfer`, `transferFrom`, `mint`, `burn`, and `burnFrom` calls revert.
4. **No transfer restrictions**: v1 does not implement AML/KYC transfer restrictions. Compliance is enforced at the service layer.
5. **Role hierarchy**: `DEFAULT_ADMIN_ROLE` can grant/revoke all other roles. Compromise of the admin key requires immediate role rotation.

---

## Integration with the Wallet Service

The enclave backend interacts with rEUR via:

| Operation | Caller | Function |
|:----------|:-------|:---------|
| On-ramp settlement | Reserve wallet (in SGX) | `mint(user_wallet, amount)` |
| Off-ramp initiation | Reserve wallet (in SGX) | `burnFrom(user_wallet, amount)` |
| Balance query | Any (via RPC) | `balanceOf(address)` |
| Token send | User wallet (in SGX) | `transfer(recipient, amount)` |

---

## Reading Balance On-Chain

```bash
# Human-readable balance (divide raw by 10^6)
cast call 0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63 \
  "balanceOf(address)(uint256)" \
  "0xYourAddress" \
  --rpc-url https://api.avax-test.network/ext/bc/C/rpc
# 10000000 = 10.0 rEUR
```

---

## Future Roadmap

| Phase | Feature |
|:------|:--------|
| v2 | Transfer compliance hooks (allowlist/denylist) |
| v2 | AML/KYC enforcement at contract level |
| v3 | Upgradeability via transparent proxy |
| v3 | Multi-collateral backing |
