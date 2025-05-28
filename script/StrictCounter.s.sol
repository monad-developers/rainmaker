// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {StrictCounter} from "../src/StrictCounter.sol";

contract StrictCounterScript is Script {
    StrictCounter public counter;

    function setUp() public {}

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        counter = new StrictCounter();

        vm.stopBroadcast();
    }
}
