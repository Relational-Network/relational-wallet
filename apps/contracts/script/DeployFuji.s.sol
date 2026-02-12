// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {RelationalEuro} from "../src/RelationalEuro.sol";

/// @notice Deploy RelationalEuro to Avalanche Fuji and print role assignments.
contract DeployFuji is Script {
    function run() external returns (RelationalEuro token) {
        uint256 privateKey = vm.envUint("PRIVATE_KEY");
        address admin = vm.envAddress("ADMIN_ADDRESS");
        address minter = vm.envAddress("MINTER_ADDRESS");
        address pauser = vm.envAddress("PAUSER_ADDRESS");

        vm.startBroadcast(privateKey);
        token = new RelationalEuro(admin, minter, pauser);
        vm.stopBroadcast();

        console2.log("RelationalEuro deployed at:", address(token));
        console2.log("ADMIN_ADDRESS:", admin);
        console2.log("MINTER_ADDRESS:", minter);
        console2.log("PAUSER_ADDRESS:", pauser);
        console2.logBytes32(token.DEFAULT_ADMIN_ROLE());
        console2.logBytes32(token.MINTER_ROLE());
        console2.logBytes32(token.PAUSER_ROLE());
    }
}
