// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

contract StrictCounter {
    uint256 public number;

    constructor() {
        number = 0;
    }

    function update(uint256 newNumber) public {
        if (newNumber != number + 1) {
            revert("Invalid number");
        }
        number = newNumber;
    }

    function reset() public {
        number = 0;
    }

    function getCount() public view returns (uint256) {
        return number;
    }
}
