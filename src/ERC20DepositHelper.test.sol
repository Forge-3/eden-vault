pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {EdenTokenLL} from "./EdenTokenLL.sol";
import {ForgeToken} from "./ForgeToken.sol";
import {Vm} from "forge-std/Vm.sol";
import {CkErc20Deposit} from "./ERC20DepositHelper.sol";

contract ContractBTest is Test {
    address alice = address(0x1);
    address bob = address(0x2);
    address john = address(0x3);
    EdenTokenLL edenToken;
    ForgeToken forgeToken;
    CkErc20Deposit ckErc20Deposit;

    function setUp() public {
        vm.deal(alice, 1 ether);
        vm.deal(bob, 1 ether);
        vm.deal(john, 1 ether);
        vm.prank(alice);
        edenToken = new EdenTokenLL(
            10000000000000000000000000000,
            "tEDEN",
            "tEDN",
            alice,
            alice,
            0,
            address(0)
        );
        assertEq(10000000000000000000000000000, edenToken.balanceOf(alice));

        vm.prank(alice);
        forgeToken = new ForgeToken();

        vm.prank(alice);
        ckErc20Deposit = new CkErc20Deposit(alice);

        vm.prank(alice);
        edenToken.proposeLosslessTurnOff();

        vm.prank(alice);
        edenToken.executeLosslessTurnOff();
    }
/*
    function test_bob_transfer_from_erc20() public {
        vm.prank(alice);
        forgeToken.approve(bob, 10000000);

        vm.prank(bob);
        forgeToken.transferFrom(alice, john, 1);
    }

    function test_bob_transfer_from_eden() public {
        vm.prank(alice);
        edenToken.approve(bob, 10000000);

        vm.prank(bob);
        edenToken.transferFrom(alice, john, 10000000);
    }*/

    function test_deposit_eden() public {
        bytes32 x = 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef;

        vm.prank(alice);
        edenToken.approve(address(ckErc20Deposit), 10000000);

        vm.prank(alice);
        ckErc20Deposit.deposit(address(edenToken), 10000000, x);
    }
}