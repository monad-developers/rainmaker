// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {Test, console} from "forge-std/Test.sol";
import {Swapper} from "../src/Swapper.sol";

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function approve(address spender, uint256 amount) external returns (bool);
}

contract SwapperTest is Test {
    Swapper public swapper;
    address public tokenA;
    address public tokenB;
    
    // Mock Uniswap router address
    address constant ROUTER = 0xfB8e1C3b833f9E67a71C859a132cf783b645e436;
    
    function setUp() public {
        // Deploy the Swapper contract
        swapper = new Swapper(ROUTER);
        
        (tokenA, tokenB) = swapper.getTokens();
        
        // Approve tokens for spending
        IERC20(tokenA).approve(address(swapper), type(uint256).max);
        IERC20(tokenB).approve(address(swapper), type(uint256).max);
    }
    
    function testSwapGasUsage() public {
        // Get initial pair reserves
        (uint256 reserveA, uint256 reserveB) = swapper.getReserves();
        console.log("Initial reserves - TokenA:", reserveA);
        console.log("Initial reserves - TokenB:", reserveB);
        
        // Get token balances before swap
        uint256 balanceA = IERC20(tokenA).balanceOf(address(this));
        uint256 balanceB = IERC20(tokenB).balanceOf(address(this));
        console.log("Initial balances - TokenA:", balanceA);
        console.log("Initial balances - TokenB:", balanceB);
        
        // Measure gas usage for swap (aToB = false, so swapping TokenB)
        uint256 gas = gasleft();
        swapper.swap(10, false);
        uint256 gasUsed = gas - gasleft();
        console.log("Gas used for swap:", gasUsed);
    }
    
}
