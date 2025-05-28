// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {Script} from "lib/forge-std/src/Script.sol";
import {Soulbound} from "../src/Soulbound.sol";

contract SoulboundDeploy is Script {
    function run() public returns (Soulbound) {
        // Start broadcasting transactions
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        string memory name = "Monad Games Cipher P.O.G.";
        string memory symbol = "POG";

        // Deploy the Soulbound contract
        Soulbound soulbound = new Soulbound(name, symbol);

        // Stop broadcasting transactions
        vm.stopBroadcast();

        // Return the deployed contract
        return soulbound;
    }
}