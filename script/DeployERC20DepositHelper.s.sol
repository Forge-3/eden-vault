// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/ERC20DepositHelper.sol";

contract DeployERC20DepositHelper is Script {
    function run() external {
        // Start broadcasting to send transactions
        vm.startBroadcast();

        // Deploy the contract
        CkErc20Deposit myContract = new CkErc20Deposit(0x7187CC02Be5f744eE15653A4ea3F13FeC23E1a7B);

        // Log the deployed contract address
        console.log("Deployed MyContract at:", address(myContract));

        // Stop broadcasting
        vm.stopBroadcast();
    }
}
