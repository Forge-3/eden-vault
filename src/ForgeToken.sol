// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";

contract ForgeToken is ERC20, ERC20Permit {
    constructor() ERC20("ForgeToken", "ckFT") ERC20Permit("ForgeToken") {
        _mint(msg.sender, 1_000_000_000_000_000_000);
    }
}
