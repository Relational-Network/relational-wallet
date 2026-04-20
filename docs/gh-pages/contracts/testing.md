---
layout: default
title: Testing
parent: Smart Contracts
nav_order: 2
---

# Contract Testing
{: .fs-7 }

The rEUR contract is tested with Foundry's `forge test` framework. All 5 tests must pass before deployment.
{: .fs-5 .fw-300 }

---

## Prerequisites

### Install Foundry

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Verify:

```bash
forge --version
# forge 0.2.0 (...)
```

### Install Dependencies

```bash
cd apps/contracts
forge install OpenZeppelin/openzeppelin-contracts --no-git
forge install foundry-rs/forge-std --no-git
```

`--no-git` is required to avoid nested submodule behavior in this monorepo.
{: .note }

---

## Running Tests

```bash
cd apps/contracts

# Run all tests with verbose output
forge test -vv

# Run with maximum verbosity (shows stack traces on failure)
forge test -vvvv

# Run a specific test by name
forge test --match-test testOnlyMinterCanMint -vv

# Run with gas reporting
forge test --gas-report
```

---

## Test Suite

Tests live in `apps/contracts/test/RelationalEuro.t.sol`.

### `testMetadata`

Verifies the token's basic properties are correctly set at deployment.

```
✓ name() == "Relational Euro"
✓ symbol() == "rEUR"
✓ decimals() == 6
```

### `testOnlyMinterCanMint`

Verifies that only addresses with `MINTER_ROLE` can mint tokens.

```
✓ Minter can mint to any address
✓ Non-minter call reverts with AccessControl error
✓ totalSupply increases by minted amount
```

### `testOnlyPauserCanPauseAndUnpause`

Verifies pause/unpause access control.

```
✓ Pauser can pause the contract
✓ Pauser can unpause the contract
✓ Non-pauser pause call reverts
✓ Non-pauser unpause call reverts
```

### `testTransfersBlockedWhilePausedAndResumeAfterUnpause`

Verifies that pausing blocks all transfers and unpausing restores them.

```
✓ Transfer succeeds when not paused
✓ Transfer reverts when paused (EnforcedPause error)
✓ Transfer succeeds again after unpause
```

### `testMintAndBurnAdjustTotalSupply`

Verifies that minting and burning correctly adjust the total supply.

```
✓ Mint N tokens → totalSupply increases by N
✓ Burn M tokens from holder → totalSupply decreases by M
✓ Holder balance reflects mint and burn
```

---

## Expected Output

```
Ran 5 tests for test/RelationalEuro.t.sol:RelationalEuroTest
[PASS] testMintAndBurnAdjustTotalSupply() (gas: 65123)
[PASS] testMetadata() (gas: 12450)
[PASS] testOnlyMinterCanMint() (gas: 45678)
[PASS] testOnlyPauserCanPauseAndUnpause() (gas: 38901)
[PASS] testTransfersBlockedWhilePausedAndResumeAfterUnpause() (gas: 72345)
Suite result: ok. 5 passed; 0 failed; 0 skipped
```

---

## Other Useful Commands

```bash
# Compile contracts
forge build

# Check formatting
forge fmt --check

# Apply formatting
forge fmt

# Static analysis (if slither installed)
slither src/RelationalEuro.sol

# Coverage (if lcov installed)
forge coverage --report lcov
```

---

## Adding New Tests

Test files follow the Foundry convention:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {RelationalEuro} from "../src/RelationalEuro.sol";

contract RelationalEuroTest is Test {
    RelationalEuro token;
    address admin = address(this);
    address minter = address(0x1);

    function setUp() public {
        token = new RelationalEuro(admin, minter, admin);
    }

    function testYourNewTest() public {
        // arrange, act, assert
    }
}
```

Run with `forge test --match-test testYourNewTest -vv`.
