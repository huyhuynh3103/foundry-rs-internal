# FDK (Foundry Deployment Kit) Implementation

This module implements deployment management cheatcodes for Foundry, enabling systematic tracking and management of contract deployments across chains.

## Configuration

Add an `[fdk]` section to your `foundry.toml` to configure FDK behavior:

```toml
[fdk]
# Root directory for deployment artifacts
# Default: "deployments"
deployments_root = "deployments"

# Path to TransparentUpgradeableProxy contract
# Default: "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy"
transparent_proxy_path = "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy"

# Path to ProxyAdmin contract  
# Default: "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin"
proxy_admin_path = "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin"
```

### Custom Deployments Directory

To use a different directory for deployment artifacts:

```toml
[fdk]
deployments_root = "deploy"  # Will use ./deploy instead of ./deployments
```

### Custom Proxy Implementations

If you're using custom proxy contracts instead of OpenZeppelin's:

```toml
[fdk]
transparent_proxy_path = "src/proxies/CustomProxy.sol:CustomProxy"
proxy_admin_path = "src/proxies/CustomProxyAdmin.sol:CustomProxyAdmin"
```

## Features

### 1. Contract Loading
Load previously deployed contracts from the deployment registry:

```solidity
// Load contract by name (uses current chain)
address myContract = loadContract("MyContract");

// Load contract by chain alias
address myContract = loadContract("MyContract", "mainnet");

// Load contract by chain ID
address myContract = loadContract("MyContract", 1);
```

### 2. Immutable Contract Deployment
Deploy non-upgradeable contracts:

```solidity
address deployed = deployImmutable("Counter", abi.encode(initialValue));
```

### 3. Logic Contract Deployment
Deploy implementation contracts (for upgradeable patterns):

```solidity
address logic = deployLogic("MyContract", abi.encode(arg1, arg2));
```

The logic contract is saved as `{ContractName}Logic.json` in the deployments directory.

### 4. Proxy Deployment
Deploy TransparentUpgradeableProxy contracts with automatic ProxyAdmin management:

```solidity
// Full control: specify constructor args, init data, and ProxyAdmin
address proxy = deployProxy(
    "MyContract",
    abi.encode(constructorArg1, constructorArg2),
    abi.encodeCall(MyContract.initialize, (initArg1, initArg2)),
    proxyAdminAddress
);

// Auto ProxyAdmin: loads or deploys ProxyAdmin automatically
address proxy = deployProxy(
    "MyContract",
    abi.encodeCall(MyContract.initialize, (initArg))
);

// No initialization
address proxy = deployProxy("MyContract");
```

**How it works:**
1. Deploys the logic contract (implementation)
2. Gets or deploys a ProxyAdmin contract
3. Deploys a TransparentUpgradeableProxy pointing to the logic contract
4. Saves three deployment files:
   - `{ContractName}Logic.json` - the implementation contract
   - `{ContractName}Proxy.json` - the proxy contract
   - `{ContractName}.json` - alias to the proxy (for easy loading)

### 5. Proxy Upgrades
Upgrade existing proxies to new logic implementations:

```solidity
// With reinitialization
address newLogic = upgradeProxy(
    "MyContract",
    abi.encode(constructorArg),
    abi.encodeCall(MyContractV2.reinitialize, (newArg))
);

// Without reinitialization
address newLogic = upgradeProxy("MyContract");
```

**How it works:**
1. Loads the existing proxy address from `{ContractName}Proxy.json`
2. Deploys the new logic contract
3. Saves the new logic as `{ContractName}Logic.json`
4. Automatically calls `ProxyAdmin.upgradeAndCall(proxy, newLogic, initData)`
5. Returns the new logic address

The upgrade is executed atomically within the cheatcode, so no manual intervention is required.

## Deployment Artifacts

All deployments are tracked in `deployments/{chain_alias}/{contract_name}.json` files with the following metadata:

```json
{
  "name": "MyContract",
  "address": "0x...",
  "args": "0x...",           // constructor arguments (hex-encoded)
  "value": "0",               // ETH value sent (optional)
  "nonce": "5",               // deployer nonce at deployment
  "deployer": "0x...",        // deployer address
  "chainid": "1",             // chain ID
  "block_number": "12345678", // deployment block number
  "timestamp": "1234567890",  // deployment timestamp
  "abi": [...],               // contract ABI
  "devdoc": {...},            // developer documentation (from forge inspect)
  "userdoc": {...},           // user documentation (from forge inspect)
  "metadata": {...},          // contract metadata (from forge inspect)
  "storage_layout": {...},    // storage layout
  "bytecode": "0x...",        // creation bytecode
  "deployed_bytecode": "0x..." // runtime bytecode
}
```

## ProxyAdmin Management

The `ProxyAdmin` contract is automatically managed:

- **First proxy deployment**: Creates and deploys a new ProxyAdmin owned by the deployer
- **Subsequent deployments**: Reuses the existing ProxyAdmin from `deployments/{chain}/ProxyAdmin.json`
- **Manual specification**: Can specify a custom ProxyAdmin address in `deployProxy` calls

## Directory Structure

By default, deployments are saved to the `deployments/` directory (configurable via `fdk.deployments_root`):

```
deployments/              # Or custom path from fdk.deployments_root
├── mainnet/
│   ├── ProxyAdmin.json
│   ├── MyContract.json          # Points to proxy
│   ├── MyContractProxy.json     # Proxy contract
│   └── MyContractLogic.json     # Implementation contract
├── sepolia/
│   └── ...
└── optimism/
    └── ...
```

Each chain gets its own subdirectory, and each deployed contract gets a JSON artifact file with full deployment metadata.

## Implementation Details

### Artifact Generation (`src/fdk/artifact.rs`)
- Generates comprehensive deployment artifacts with all metadata
- Uses `forge inspect` to extract devdoc, userdoc, and metadata
- Loads contract ABIs and bytecode from compiled artifacts
- Captures deployment context (block, timestamp, nonce, deployer)

### Deployment Management (`src/fdk/cheatcode.rs`)
- Implements all deployment cheatcodes
- Manages in-memory address book for fast lookups
- Handles chain alias resolution and caching
- Integrates with OpenZeppelin's TransparentUpgradeableProxy pattern

## Requirements

- OpenZeppelin Contracts installed: `@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol`
- Forge must be available in PATH for `forge inspect` commands

## Examples

### Basic Immutable Deployment
```solidity
import {Script} from "forge-std/Script.sol";

contract DeployScript is Script {
    function run() external {
        vm.startBroadcast();
        
        address counter = deployImmutable("Counter", abi.encode(0));
        console.log("Counter deployed at:", counter);
        
        vm.stopBroadcast();
    }
}
```

### Upgradeable Contract Deployment
```solidity
contract DeployUpgradeableScript is Script {
    function run() external {
        vm.startBroadcast();
        
        // Deploy upgradeable contract
        bytes memory initData = abi.encodeCall(MyContract.initialize, (owner));
        address proxy = deployProxy("MyContract", initData);
        
        console.log("Proxy deployed at:", proxy);
        console.log("Logic:", loadContract("MyContractLogic"));
        console.log("ProxyAdmin:", loadContract("ProxyAdmin"));
        
        vm.stopBroadcast();
    }
}
```

### Cross-Chain Contract Loading
```solidity
contract CrossChainScript is Script {
    function run() external {
        // Load contract from different chains
        address mainnetContract = loadContract("MyContract", "mainnet");
        address optimismContract = loadContract("MyContract", "optimism");
        address arbitrumContract = loadContract("MyContract", "arbitrum");
        
        // Use addresses...
    }
}
```
