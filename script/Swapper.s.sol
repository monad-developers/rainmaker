// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {Script, console} from "forge-std/Script.sol";
import {Swapper} from "../src/Swapper.sol";

contract DeploySwapper is Script {
    // Mainnet UniswapV2 Router address - replace with your target network's router address
    address constant ROUTER = 0xfB8e1C3b833f9E67a71C859a132cf783b645e436;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // Deploy Swapper
        Swapper swapper = new Swapper(ROUTER);
        
        // Get token addresses
        (address tokenA, address tokenB) = swapper.getTokens();
        
        // Get pair reserves
        (uint256 reserveA, uint256 reserveB) = swapper.getReserves();
        
        IERC20(tokenA).approve(address(swapper), type(uint256).max);
        IERC20(tokenB).approve(address(swapper), type(uint256).max);

        // Get deployer balances
        uint256 balanceA = IERC20(tokenA).balanceOf(swapper.owner());
        uint256 balanceB = IERC20(tokenB).balanceOf(swapper.owner());

        vm.stopBroadcast();

        // Log results
        console.log("Swapper deployed at:", address(swapper));
        console.log("Swapper owner:", swapper.owner());
        console.log("TokenA address:", tokenA);
        console.log("TokenB address:", tokenB);
        console.log("Pair reserves - TokenA:", reserveA);
        console.log("Pair reserves - TokenB:", reserveB);
        console.log("Deployer TokenA balance:", balanceA);
        console.log("Deployer TokenB balance:", balanceB);


    }
}

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
}
