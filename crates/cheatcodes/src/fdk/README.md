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

### Multisig Wallet Support

Configure multisig wallet addresses to enable safe transaction handling:

```toml
[fdk.multisig_wallets]
mainnet = "0x1234567890123456789012345678901234567890"
sepolia = "0x0987654321098765432109876543210987654321"
optimism = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
```

When the transaction sender is a configured multisig wallet:
1. **Transaction is NOT broadcast** (requires propose/approve/execute flow)
2. **Transaction details are logged** to `deployments/{chain}/multisig/tx_{timestamp}.json`
3. **Transaction is simulated** using prank to verify it will work
4. **Console output** provides clear instructions for next steps

Example multisig transaction log:
```json
{
  "from": "0x1234567890123456789012345678901234567890",
  "to": "0xProxyAdmin...",
  "data": "0x9623609d...",
  "value": "0",
  "chain_id": 1,
  "description": "Upgrade proxy 0x... to implementation 0x... via ProxyAdmin 0x..."
}
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

## Multisig Wallet Integration

### Overview

When deploying or upgrading contracts from a multisig wallet, transactions cannot be broadcast directly (they require propose → approve → execute flow). FDK automatically detects multisig senders and:

1. **Logs transactions** instead of broadcasting
2. **Simulates transactions** using prank to verify correctness
3. **Provides transaction details** for proposing to the multisig

### Configuration

Define your multisig wallet addresses in `foundry.toml`:

```toml
[fdk.multisig_wallets]
mainnet = "0x1234567890123456789012345678901234567890"
sepolia = "0x9876543210987654321098765432109876543210"
optimism = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
```

### How It Works

When `upgradeProxy` (or other functions) detect the sender is a multisig:

**Console Output:**
```
══════════════════════════════════════════════════════════
🔐 MULTISIG TRANSACTION DETECTED
══════════════════════════════════════════════════════════
From (Multisig): 0x1234...
To:              0xProxyAdmin...
Value:           0 wei
Calldata:        0x9623609d...
Description:     Upgrade proxy 0x... to implementation 0x...
Chain ID:        1
Saved to:        deployments/mainnet/multisig/tx_1234567890.json
══════════════════════════════════════════════════════════
⚠️  This transaction was NOT broadcast.
   You need to:
   1. Propose this transaction to the multisig
   2. Gather required approvals
   3. Execute through the multisig wallet
══════════════════════════════════════════════════════════

🔍 Simulating transaction with prank...
✅ Simulation successful! Transaction will work when executed from multisig.
```

**Transaction File** (`deployments/mainnet/multisig/tx_1234567890.json`):
```json
{
  "from": "0x1234567890123456789012345678901234567890",
  "to": "0xProxyAdmin...",
  "data": "0x9623609d000000...",
  "value": "0",
  "chain_id": 1,
  "description": "Upgrade proxy 0x... to implementation 0x... via ProxyAdmin 0x..."
}
```

### Multisig Deployment Workflow

1. **Configure multisig in `foundry.toml`:**
```toml
[fdk.multisig_wallets]
mainnet = "0xYourSafeMultisig..."
```

2. **Run deployment script:**
```solidity
contract UpgradeScript is Script {
    function run() external {
        address multisig = vm.envAddress("MULTISIG_ADDRESS");
        vm.startBroadcast(multisig);
        
        // This will be logged, not broadcast
        upgradeProxy("MyContract", initData);
        
        vm.stopBroadcast();
    }
}
```

3. **Check simulation results** in console output

4. **Propose transactions** from the logged JSON files to your multisig:
   - Safe (Gnosis Safe): Use the Safe UI or CLI
   - Hardware wallet multisig: Use your multisig tool
   - Custom multisig: Parse the JSON and submit

5. **Gather approvals** from other signers

6. **Execute** the approved transaction

### Benefits

- **No accidental broadcasts** from multisig addresses
- **Pre-flight simulation** catches errors before proposing
- **Audit trail** with timestamped transaction logs
- **Clear instructions** in console output
- **Portable transaction data** in standard JSON format

### Directory Structure

```
deployments/
├── mainnet/
│   ├── multisig/
│   │   ├── tx_1712345678.json   # First multisig tx
│   │   ├── tx_1712345890.json   # Second multisig tx
│   │   └── tx_1712346000.json   # Third multisig tx
│   ├── ProxyAdmin.json
│   └── MyContract.json
└── optimism/
    └── multisig/
        └── tx_1712340000.json
```

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

### Multisig Wallet Deployment

When deploying or upgrading from a multisig wallet:

```solidity
contract MultisigUpgradeScript is Script {
    function run() external {
        // Configure multisig as sender in foundry.toml:
        // [fdk.multisig_wallets]
        // mainnet = "0xYourMultisig..."
        
        vm.startBroadcast(multisigAddress);
        
        // This will be logged instead of broadcast
        address newLogic = upgradeProxy(
            "MyContract",
            abi.encodeCall(MyContractV2.reinitialize, (newParam))
        );
        
        vm.stopBroadcast();
        
        // Output:
        // 🔐 MULTISIG TRANSACTION DETECTED
        // From (Multisig): 0xYourMultisig...
        // To:              0xProxyAdmin...
        // Calldata:        0x9623609d...
        // Saved to:        deployments/mainnet/multisig/tx_1234567890.json
        // ✅ Simulation successful!
    }
}
```

**Workflow:**
1. Run the script - transactions are logged, not broadcast
2. Review the logged transaction JSON files
3. Propose each transaction to your multisig (Safe, Gnosis, etc.)
4. Gather required approvals from other signers
5. Execute the approved transaction through the multisig UI/CLI

**Benefits:**
- Transactions are simulated to catch errors before proposing
- All transaction details are saved for audit trail
- No accidental broadcasts from multisig (which would fail)
- Clean separation between deployment logic and multisig execution

### Contract Configuration Management

Store and load contract configurations across chains. Useful for deploying the same contract with the same settings on multiple chains:

```solidity
contract DeployWithConfigScript is Script {
    struct AxieConfig {
        uint256 withdrawalLimit;
        address[] allowedAddresses;
        uint256 cooldownPeriod;
    }
    
    function run() external {
        // Define configuration once
        AxieConfig memory config = AxieConfig({
            withdrawalLimit: 1000 ether,
            allowedAddresses: new address[](2),
            cooldownPeriod: 7 days
        });
        config.allowedAddresses[0] = 0x1234...;
        config.allowedAddresses[1] = 0x5678...;
        
        // Store config for current chain
        fdk.storeConfig("Axie", abi.encode(config));
        
        // Store for specific chains
        fdk.storeConfig("Axie", "mainnet", abi.encode(config));
        fdk.storeConfig("Axie", "optimism", abi.encode(config));
        fdk.storeConfig("Axie", 42161, abi.encode(config)); // Arbitrum by ID
        
        vm.startBroadcast();
        
        // Deploy with stored config
        bytes memory configBytes = fdk.loadConfig("Axie");
        AxieConfig memory loadedConfig = abi.decode(configBytes, (AxieConfig));
        
        address axie = deployImmutable("Axie", abi.encode(
            loadedConfig.withdrawalLimit,
            loadedConfig.allowedAddresses,
            loadedConfig.cooldownPeriod
        ));
        
        vm.stopBroadcast();
    }
}
```

**Cross-chain Configuration:**
```solidity
contract DeployAcrossChainsScript is Script {
    function run() external {
        // Same config can be deployed on different chains at different times
        bytes memory config = fdk.loadConfig("Axie", "mainnet");
        
        // Deploy to current chain using mainnet's config
        vm.startBroadcast();
        deployImmutable("Axie", config);
        vm.stopBroadcast();
    }
}
```

**API:**
- `storeConfig(contractName, config)` - Store for current chain
- `storeConfig(contractName, chainAlias, config)` - Store for specific chain by alias
- `storeConfig(contractName, chainId, config)` - Store for specific chain by ID
- `loadConfig(contractName)` - Load from current chain
- `loadConfig(contractName, chainAlias)` - Load from specific chain by alias
- `loadConfig(contractName, chainId)` - Load from specific chain by ID

**Use Cases:**
- Deploy contracts with identical configurations across multiple chains
- Share configuration between different deployment scripts
- Version control your contract configurations as code
- Ensure consistency in multi-chain deployments

**Complete Examples:**

See `config_example.sol` for comprehensive examples including:
- Storing configuration for multiple chains
- Loading and deploying with stored config
- Cross-chain configuration sharing
- Environment-based configurations
- Integration with upgradeable deployments
- Shared configuration libraries
