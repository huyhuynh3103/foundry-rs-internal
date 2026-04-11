// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.6.2 <0.9.0;
pragma experimental ABIEncoderV2;

address constant FDK_CHEATCODE_ADDRESS = address(bytes20(uint160(uint256(keccak256("fdk cheat code")))));

interface Fdk {
    function fdkVersion() external pure returns (string memory);
}
