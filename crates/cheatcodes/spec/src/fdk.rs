// We don't document function parameters individually so we can't enable `missing_docs` for this
// module. Instead, we emit custom diagnostics in `#[derive(Cheatcode)]`.
#![allow(missing_docs)]

use super::*;
use alloy_sol_types::sol;
use foundry_macros::Cheatcode;

sol! {
/// Foundry Deployment Kit cheatcodes interface.
#[derive(Debug, Cheatcode)]
#[sol(abi)]
interface Fdk {
    /// Returns the FDK version string.
    #[cheatcode(group = Utilities, safety = Safe)]
    function fdkVersion() external pure returns (string memory);
    
    /// Loads a contract from the FDK registry.
    #[cheatcode(group = Custom, safety = Unsafe)]
    function loadContract(string calldata contractName) external pure returns (address);

    // function deployImmutable(string calldata contractName, bytes calldata constructorArgs) external pure returns (address);

    // function deployLogic(string calldata contractName, bytes calldata constructorArgs) external pure returns (address);

    // function deployProxy(string calldata contractName, bytes calldata constructorArgs, bytes calldata initializationData, address proxyAdmin) external pure returns (address);
    // function deployProxy(string calldata contractName, bytes calldata initializationData, address proxyAdmin) external pure returns (address);
    // function deployProxy(string calldata contractName, bytes calldata initializationData) external pure returns (address);
    // function deployProxy(string calldata contractName) external pure returns (address);

    // function upgradeProxy(string calldata contractName, bytes calldata constructorArgs, bytes calldata initializationData) external pure returns (address);
    // function upgradeProxy(string calldata contractName, bytes calldata initializationData) external pure returns (address);
    // function upgradeProxy(string calldata contractName) external pure returns (address);
}
}
