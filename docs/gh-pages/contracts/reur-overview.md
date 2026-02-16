---
layout: default
title: rEUR Overview
parent: Contracts
nav_order: 1
---

# rEUR Overview

`RelationalEuro` is a managed ERC-20 token contract in:

- `apps/contracts/src/RelationalEuro.sol`

## Contract Profile

- Name: `Relational Euro`
- Symbol: `rEUR`
- Decimals: `6`
- Model: non-upgradeable v1 managed token

## Role Model

- `DEFAULT_ADMIN_ROLE`
- `MINTER_ROLE`
- `PAUSER_ROLE`

These roles are used for controlled minting and operational pause/unpause.

## Current Network Snapshot

- Network: Avalanche Fuji (`43113`)
- Contract: `0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63`
- Deployment tx: `0x89878d998b832bc06877990ea0f7e522b9a8bf1a389e8839013daa605d289f14`

## Notes

AML/KYC/compliance transfer restrictions are deferred in current phase and are not encoded in this contract version.
