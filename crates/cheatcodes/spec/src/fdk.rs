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
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadContract(string calldata contractName) external pure returns (address);

    /// Loads a contract from the FDK registry by chain alias.
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadContract(string calldata contractName, string calldata chainAlias) external pure returns (address);

    /// Loads a contract from the FDK registry by chain ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadContract(string calldata contractName, uint256 chainId) external pure returns (address);

    /// Deploys an immutable contract.
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployImmutable(string calldata artifact, bytes calldata constructorArgs) external pure returns (address);
    /// Deploys an immutable contract (no constructor args).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployImmutable(string calldata artifact) external pure returns (address);

    /// Deploys a logic contract (implementation).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployLogic(string calldata artifact, bytes calldata constructorArgs) external pure returns (address);

    /// Deploys a TransparentUpgradeableProxy with a new logic contract.
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployProxy(string calldata artifact, bytes calldata constructorArgs, bytes calldata initializationData, address proxyAdmin) external pure returns (address);

    /// Deploys a TransparentUpgradeableProxy with a new logic contract (no constructor args).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployProxy(string calldata artifact, bytes calldata initializationData, address proxyAdmin) external pure returns (address);

    /// Deploys a TransparentUpgradeableProxy with a new logic contract (loads ProxyAdmin from deployments).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployProxy(string calldata artifact, bytes calldata initializationData) external pure returns (address);

    /// Deploys a TransparentUpgradeableProxy with a new logic contract (no init data, loads ProxyAdmin).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function deployProxy(string calldata artifact) external pure returns (address);

    /// Upgrades an existing proxy to a new logic contract.
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function upgradeProxy(string calldata artifact, bytes calldata constructorArgs, bytes calldata initializationData) external pure returns (address);

    /// Upgrades an existing proxy to a new logic contract (no constructor args).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function upgradeProxy(string calldata artifact, bytes calldata initializationData) external pure returns (address);

    /// Upgrades an existing proxy to a new logic contract (no init data).
    /// @param artifact Contract identifier - can be: name ("MyToken"), path ("contracts/Token.sol"), or full ("contracts/Token.sol:MyToken")
    #[cheatcode(group = Scripting, safety = Safe)]
    function upgradeProxy(string calldata artifact) external pure returns (address);

    // ============================================================================
    // Contract Configuration Management
    // ============================================================================

    /// Stores contract configuration for the current chain.
    #[cheatcode(group = Scripting, safety = Safe)]
    function storeConfig(string calldata contractName, bytes calldata config) external pure;

    /// Stores contract configuration for a specific chain by alias.
    #[cheatcode(group = Scripting, safety = Safe)]
    function storeConfig(string calldata contractName, string calldata chainAlias, bytes calldata config) external pure;

    /// Stores contract configuration for a specific chain by ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function storeConfig(string calldata contractName, uint256 chainId, bytes calldata config) external pure;

    /// Loads contract configuration for the current chain.
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadConfig(string calldata contractName) external view returns (bytes memory);

    /// Loads contract configuration for a specific chain by alias.
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadConfig(string calldata contractName, string calldata chainAlias) external view returns (bytes memory);

    /// Loads contract configuration for a specific chain by ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function loadConfig(string calldata contractName, uint256 chainId) external view returns (bytes memory);

    /// Set contract address for a specific chain by ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function setContract(string calldata contractName, uint256 chainId, address contractAddr) external view;

    /// Set contract address for a specific chain by ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function setContract(string calldata contractName, string calldata chainAlias, address contractAddr) external view;

    /// Set contract address for a specific chain by ID.
    #[cheatcode(group = Scripting, safety = Safe)]
    function setContract(string calldata contractName, address contractAddr) external view;
}
}
