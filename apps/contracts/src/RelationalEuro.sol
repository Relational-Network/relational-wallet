// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network
pragma solidity ^0.8.24;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC20Burnable} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

/// @title RelationalEuro
/// @notice Managed EUR stablecoin profile for initial deployment and demos.
/// @dev Non-upgradeable v1 token with role-based minting and pausing.
contract RelationalEuro is ERC20, ERC20Burnable, Pausable, AccessControl {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    error ZeroAddressRoleHolder();

    constructor(address admin, address minter, address pauser) ERC20("Relational Euro", "rEUR") {
        if (admin == address(0) || minter == address(0) || pauser == address(0)) {
            revert ZeroAddressRoleHolder();
        }

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(MINTER_ROLE, minter);
        _grantRole(PAUSER_ROLE, pauser);
    }

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function mint(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _mint(to, amount);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    /// @dev Enforces paused state across transfers, minting, and burning.
    function _update(address from, address to, uint256 value) internal override whenNotPaused {
        super._update(from, to, value);
    }
}
