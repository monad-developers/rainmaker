// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IERC721} from "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import {ERC721} from "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import {ERC721URIStorage} from "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import {Base64} from "@openzeppelin/contracts/utils/Base64.sol";

/**
 * @title Soulbound
 * @dev An NFT that can be minted to an address but cannot be transferred (soulbound)
 */
contract Soulbound is ERC721, ERC721URIStorage {
    // Counter for token IDs
    uint256 private _nextTokenId;
    
    address public owner;

    /// @dev Only callable by the contract owner.
    error OnlyOwner();

    /// @dev Cannot transfer soulbound tokens
    error NonTransferable();

    // Mapping for token URIs
    mapping(uint256 => string) private _tokenURIs;

    modifier onlyOwner() {
        if (msg.sender != owner) revert OnlyOwner();
        _;
    }
        
    constructor(string memory name, string memory symbol) ERC721(name, symbol) {
        owner = msg.sender;
    }

    /**
     * @dev Mint a new soulbound token to a recipient
     * @param to The address to mint the token to
     * @param uri The token URI
     */
    function mint(address to, string memory uri) external onlyOwner {
        uint256 tokenId = _nextTokenId++;
        _mint(to, tokenId);
        _setTokenURI(tokenId, uri);
    }

    /**
     * @dev Batch mint soulbound tokens to multiple recipients
     * @param recipients Array of recipient addresses
     * @param uri The token URI to use for all tokens
     */
    function batchMint(address[] calldata recipients, string memory uri) external onlyOwner {
        require(recipients.length > 0, "No recipients provided");

        for (uint256 i = 0; i < recipients.length; i++) {
            uint256 tokenId = _nextTokenId++;
            _mint(recipients[i], tokenId);
            _setTokenURI(tokenId, uri);
        }
    }

    /**
     * @dev Override transfer functions to prevent transfers of soulbound tokens
     */
    function transferFrom(address, address, uint256) public pure override (ERC721, IERC721) {
        revert NonTransferable();
    }

    /**
     * @dev Implementation of the {IERC721Metadata-tokenURI} function.
     */
    function tokenURI(uint256 tokenId)
        public
        view
        override(ERC721, ERC721URIStorage)
        returns (string memory)
    {
        return super.tokenURI(tokenId);
    }

    /**
     * @dev See {IERC165-supportsInterface}.
     */
    function supportsInterface(bytes4 interfaceId)
        public
        view
        override(ERC721, ERC721URIStorage)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}