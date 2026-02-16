---
layout: default
title: Testing
parent: Contracts
nav_order: 2
---

# Contracts Testing

Tests live in:

- `apps/contracts/test/RelationalEuro.t.sol`

## Prerequisites

Install Foundry:

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Install dependencies for this workspace:

```bash
cd apps/contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge install foundry-rs/forge-std --no-git
```

`--no-git` avoids nested git submodule behavior in this monorepo.

## Run Test Suite

```bash
cd apps/contracts
forge test -vv
```

## Useful Commands

```bash
# Format check and compile
forge fmt --check
forge build
```
