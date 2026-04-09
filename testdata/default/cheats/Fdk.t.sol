// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.18;

import "utils/Test.sol";
import "utils/Fdk.sol";

interface IFdkUnknown {
    function unknown() external;
}

contract FdkTest is Test {
    Fdk internal constant fdk = Fdk(FDK_CHEATCODE_ADDRESS);

    function testFdkVersion() public {
        assertEq(fdk.fdkVersion(), "0.1.0");
    }

    function testFdkUnknownSelectorReverts() public {
        vm.expectRevert(
            bytes(
                "unknown FDK cheatcode with selector 0x3d0a387c; you may have a mismatch between the `Fdk` interface (likely in `fdk-std`) and the `forge` version"
            )
        );
        IFdkUnknown(FDK_CHEATCODE_ADDRESS).unknown();
    }
}
