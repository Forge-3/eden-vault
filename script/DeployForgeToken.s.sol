// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/ForgeToken.sol";

contract DeployForgeToken is Script {
    function run() external {
        // Start broadcasting to send transactions
        vm.startBroadcast();

        // Deploy the contract
        ForgeToken myContract = new ForgeToken();

        // Log the deployed contract address
        console.log("Deployed MyContract at:", address(myContract));

        // Stop broadcasting
        vm.stopBroadcast();
    }
}
