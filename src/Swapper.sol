// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

interface IUniswapV2Factory {
    function createPair(address tokenA, address tokenB) external returns (address pair);
}

interface IUniswapV2Pair {
    function token0() external view returns (address);
    function token1() external view returns (address);
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external;
}

interface IUniswapV2Router02 {
    function factory() external view returns (address);

    function addLiquidity(
        address tokenA,
        address tokenB,
        uint256 amountADesired,
        uint256 amountBDesired,
        uint256 amountAMin,
        uint256 amountBMin,
        address to,
        uint256 deadline
    ) external returns (uint256 amountA, uint256 amountB, uint256 liquidity);
}

contract TokenA is ERC20 {
    constructor() ERC20("Token A", "TKA") {
        _mint(msg.sender, 1_000_000_000_000_000_000_000_000);
    }
}

contract TokenB is ERC20 {
    constructor() ERC20("Token B", "TKB") {
        _mint(msg.sender, 1_000_000_000_000_000_000_000_000);
    }
}

contract Swapper {
    TokenA public tokenA;
    TokenB public tokenB;
    IUniswapV2Pair public pair;
    IUniswapV2Router02 public router;
    address public immutable owner;
    IUniswapV2Factory public factory;

    constructor(address _router) {
        owner = msg.sender;
        router = IUniswapV2Router02(_router);
        factory = IUniswapV2Factory(router.factory());
        
        // Deploy tokens
        tokenA = new TokenA();
        tokenB = new TokenB();
        
        // Create pair
        pair = IUniswapV2Pair(
            factory.createPair(address(tokenA), address(tokenB))
        );
        
        // Approve router to spend tokens
        tokenA.approve(address(router), type(uint256).max);
        tokenB.approve(address(router), type(uint256).max);
        
        // Add liquidity
        uint256 amountA = tokenA.balanceOf(address(this)) / 2;
        uint256 amountB = tokenB.balanceOf(address(this)) / 2;
        
        router.addLiquidity(
            address(tokenA),
            address(tokenB),
            amountA,
            amountB,
            0,
            0,
            address(this),
            block.timestamp + 1
        );

        tokenA.transfer(msg.sender, tokenA.balanceOf(address(this)));
        tokenB.transfer(msg.sender, tokenB.balanceOf(address(this)));
    }

    function getReserves() external view returns (uint256 reserveA, uint256 reserveB) {
        (uint112 reserve0, uint112 reserve1, ) = pair.getReserves();
        reserveA = uint256(reserve0);
        reserveB = uint256(reserve1);
    }

    function getTokens() external view returns (address, address) {
        return (address(tokenA), address(tokenB));
    }

    function getPair() external view returns (address) {
        return address(pair);
    }

    function swap(uint256 amountIn, bool aToB) external {
        // if (msg.sender != owner) {
        //     revert();
        // }
        if (aToB) {
            tokenA.transferFrom(msg.sender, address(pair), amountIn);
        } else {
            tokenB.transferFrom(msg.sender, address(pair), amountIn);
        }
        pair.swap(1, 1, msg.sender, "");
    }
}