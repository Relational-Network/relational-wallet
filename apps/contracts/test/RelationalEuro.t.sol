// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {RelationalEuro} from "../src/RelationalEuro.sol";

contract RelationalEuroTest is Test {
    RelationalEuro internal token;

    address internal admin = makeAddr("admin");
    address internal minter = makeAddr("minter");
    address internal pauser = makeAddr("pauser");
    address internal user = makeAddr("user");
    address internal recipient = makeAddr("recipient");

    function setUp() external {
        token = new RelationalEuro(admin, minter, pauser);
    }

    function testMetadata() external view {
        assertEq(token.name(), "Relational Euro");
        assertEq(token.symbol(), "rEUR");
        assertEq(token.decimals(), 6);
    }

    function testOnlyMinterCanMint() external {
        vm.prank(user);
        vm.expectRevert();
        token.mint(user, 1_000_000);

        vm.prank(minter);
        token.mint(user, 1_000_000);
        assertEq(token.balanceOf(user), 1_000_000);
    }

    function testOnlyPauserCanPauseAndUnpause() external {
        vm.prank(user);
        vm.expectRevert();
        token.pause();

        vm.prank(pauser);
        token.pause();
        assertTrue(token.paused());

        vm.prank(user);
        vm.expectRevert();
        token.unpause();

        vm.prank(pauser);
        token.unpause();
        assertFalse(token.paused());
    }

    function testTransfersBlockedWhilePausedAndResumeAfterUnpause() external {
        vm.prank(minter);
        token.mint(user, 5_000_000);

        vm.prank(pauser);
        token.pause();

        vm.prank(user);
        vm.expectRevert();
        token.transfer(recipient, 1_000_000);

        vm.prank(pauser);
        token.unpause();

        vm.prank(user);
        assertTrue(token.transfer(recipient, 1_000_000));
        assertEq(token.balanceOf(recipient), 1_000_000);
    }

    function testMintAndBurnAdjustTotalSupply() external {
        vm.prank(minter);
        token.mint(user, 10_000_000);
        assertEq(token.totalSupply(), 10_000_000);

        vm.prank(user);
        token.burn(2_500_000);

        assertEq(token.totalSupply(), 7_500_000);
        assertEq(token.balanceOf(user), 7_500_000);
    }
}
