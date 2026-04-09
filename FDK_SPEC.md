# FDK -- Foundry Deployment Kit

Two deliverables, built sequentially:

1. **`fdk`** (Rust CLI) -- standalone tool at `/Users/huy.huynh/repo/rust/fdk/`. Orchestrates `forge`/`cast` externally, TOML configuration, artifact management, proxy utilities, initialization validation. **Built first.**
2. **`fdk-std`** (Solidity library) -- separate repo (e.g. `github.com/axieinfinity/fdk-std`), installed via `forge install`. Pure Solidity, reads `fdk.toml` via `vm.parseToml()`, provides `BaseMigration`/`_deployProxy`/`_upgradeProxy` API. Works like forge-std. **Built later.**

Both share the same `fdk.toml` config format.

---

## Part 1: `fdk` Rust CLI

### Usage Examples

```bash
# Initialize project
cd my-foundry-project/
fdk init    # creates fdk.toml with Ronin defaults

# Deploy contracts
fdk deploy MyToken --type immutable --network ronin-testnet \
  --constructor-args "MyToken,MTK,1000000000000000000000"

fdk deploy MyContract --type proxy --network ronin-testnet \
  --admin 0x505d91E8fd2091794b45b27f86C045529fa92CD7 \
  --init "initialize(address,uint256)" \
  --init-args "0xAbC123...,100"

# Upgrade proxy
fdk upgrade MyContract --network ronin-testnet \
  --init "reinitialize(uint256)" --init-args "2"

# Run migration script
fdk migrate script/DeployMyContract.s.sol \
  --network ronin-testnet --broadcast --generate-artifacts --validate-init

# Proxy inspection
fdk proxy admin 0x1234... --network ronin-testnet
fdk proxy impl 0x1234... --network ronin-testnet
fdk proxy hierarchy 0x1234... --network ronin-testnet

# Registry
fdk registry list --network ronin-testnet
fdk registry get MyContract --network ronin-testnet

# Validate initialization
fdk validate 0x1234... --network ronin-testnet
fdk validate --all --network ronin-testnet

# Network info
fdk network list
fdk network info ronin-testnet
```

### CLI Commands

```
fdk init                              # Scaffold fdk.toml
fdk deploy <contract>                 # Deploy contract
    --type immutable|logic|proxy
    --network <name>
    --admin <address>
    --init <sig> --init-args <args>
    --constructor-args <args>
    --value <ether>
    --sender <address> | --trezor | --keystore <path>
fdk upgrade <contract>                # Upgrade proxy
    --network <name>
    --proxy <address>
    --init <sig> --init-args <args>
fdk migrate <script.s.sol>            # Wrap forge script
    --network <name>
    --broadcast
    --generate-artifacts
    --validate-init
    --args <key=val,...>
fdk registry list|get|set             # Deployment registry
fdk validate [addr|--all]             # Init guard validation
fdk network list|info                 # Network management
fdk proxy admin|impl|hierarchy        # Proxy utilities
```

### `fdk.toml`

```toml
[project]
deployments_path = "deployments"
artifacts_path = "out"
src_path = "src"
generate_artifacts = true
validate_initialization = true
proxy_type = "TransparentProxyOZv4_9_5"

[wallet]
type = "env"
env_var = "DEPLOYER"

[networks.localhost]
chain_id = 31337
rpc_url = "http://127.0.0.1:8545"
explorer = ""
block_time = 3
default_sender = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

[networks.ronin-testnet]
chain_id = 2021
rpc_url = "${RONIN_TESTNET_RPC}"
explorer = "https://saigon-app.roninchain.com/"
block_time = 3

[networks.ronin-mainnet]
chain_id = 2020
rpc_url = "${RONIN_MAINNET_RPC}"
explorer = "https://app.roninchain.com/"
block_time = 3

[contracts.ronin-testnet]
ProxyAdmin = "0x505d91E8fd2091794b45b27f86C045529fa92CD7"
Multicall3 = "0xcA11bde05977b3631167028862bE2a173976CA11"
WRON = "0xA959726154953bAe111746E265E6d754F48570E6"
WRONHelper = "0x2D3Aa3503B4EB3EEea370e2e089E3DEe43D5091C"
WETH = "0x29C6F8349A028E1bdfC68BFa08BDee7bC5D47E16"
AXS = "0x3C4e17b9056272Ce1b49F6900d8cFD6171a1869d"
Scatter = "0xFc4090C0A3c07155484Da061B9d9cB8650e6A8cC"
KatanaRouter = "0xDa44546C0715ae78D454fE8B84f0235081584Fe0"
KatanaFactory = "0x86587380C4c815Ba0066c90aDB2B45CC9C15E72c"
KatanaGovernance = "0x247F12836A421CDC5e22B93Bf5A9AAa0f521f986"
AffiliateRouter = "0x4a913d50E618Ee9F61FfA288D8f8040D489d2360"
PermissionedRouter = "0x3BD36748D17e322cFB63417B059Bcc1059012D83"
USDC = "0x067FBFf8990c58Ab90BaE3c97241C5d736053F77"
Axie = "0xcaCA1c072D26E46686d932686015207FbE08FdB8"
Pyth = "0xA2aa501b19aff244D90cc15a4Cf739D2725B5729"
ERC721BatchTransfer = "0x2E889348bD37f192063Bfec8Ff39bD3635949e20"
RoninGovernanceAdmin = "0x53Ea388CB72081A3a397114a43741e7987815896"
RoninValidatorSet = "0x54B3AC74a90E64E8dDE60671b6fE8F8DDf18eC9d"
Profile = "0x3b67c8D22a91572a6AB18acC9F70787Af04A4043"
RoninVRFCoordinator = "0xA60c1e07fa030E4B49Eb54950ADb298Ab94dD312"

[contracts.ronin-mainnet]
ProxyAdmin = "0xA3e7d085E65CB0B916f6717da876b7bE5cC92f03"
Multicall2 = "0xC76d0d0D3Aa608190f78db02Bf2f5AeF374fC0b9"
Multicall3 = "0xcA11bde05977b3631167028862bE2a173976CA11"
WRON = "0xe514d9DEB7966c8BE0ca922de8a064264eA6bcd4"
WRONHelper = "0xCAF3E62b27a3dF0766721d1959d22b066E1a57F1"
WETH = "0xc99a6A985eD2Cac1ef41640596C5A5f9F4E19Ef5"
AXS = "0x97a9107C1793BC407d6F527b77e7fff4D812bece"
Scatter = "0x5d518933351a0bC14B24B329b33b813565608769"
KatanaRouter = "0x7D0556D55ca1a92708681e2e231733EBd922597D"
KatanaFactory = "0xB255D6A720BB7c39fee173cE22113397119cB930"
KatanaGovernance = "0x2C1726346d83cBF848bD3C2B208ec70d32a9E44a"
AffiliateRouter = "0x77F96cF7b98B963fB8A9b84787806D396d953b2b"
PermissionedRouter = "0xC05AFC8c9353c1dd5f872EcCFaCD60fd5A2a9aC7"
SCMultisig = "0x9D05D1F5b0424F8fDE534BC196FFB6Dd211D902a"
USDC = "0x0B7007c13325C48911F73A2daD5FA5dCBf808aDc"
Axie = "0x32950db2a7164aE833121501C797D79E7B79d74C"
Pyth = "0x2880aB155794e7179c9eE2e38200202908C17B43"
ERC721BatchTransfer = "0x2368dfED532842dB89b470fdE9Fd584d48D4F644"
RoninGovernanceAdmin = "0x946397deDFd2f79b75a72B322944a21C3240c9c3"
RoninValidatorSet = "0x617c5d73662282EA7FfD231E020eCa6D2B0D552f"
Profile = "0x840EBf1CA767CB690029E91856A357a43B85d035"
RoninVRFCoordinator = "0x16A62a921e7fEC5Bf867fF5c805b662Db757B778"
```

### Crate Structure

```
fdk/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cmd/                    # CLI subcommands
│   │   ├── mod.rs
│   │   ├── init.rs
│   │   ├── deploy.rs
│   │   ├── upgrade.rs
│   │   ├── migrate.rs
│   │   ├── registry.rs
│   │   ├── validate.rs
│   │   ├── network.rs
│   │   └── proxy.rs
│   ├── config/                 # TOML config loading
│   │   ├── mod.rs
│   │   ├── network.rs
│   │   ├── wallet.rs
│   │   ├── contract.rs
│   │   └── project.rs
│   ├── core/                   # Business logic
│   │   ├── mod.rs
│   │   ├── deployer.rs         # forge create orchestration
│   │   ├── upgrader.rs         # Proxy upgrade (hierarchy + cast send)
│   │   ├── proxy.rs            # EIP-1967 slots + hierarchy walk via alloy
│   │   ├── artifact.rs         # JSON artifact generation
│   │   ├── init_guard.rs       # OZv4/v5 initialization validation
│   │   └── registry.rs         # exported_address file I/O
│   ├── forge/                  # Shell command wrappers
│   │   ├── mod.rs
│   │   ├── create.rs
│   │   ├── script.rs
│   │   ├── inspect.rs
│   │   └── cast.rs
│   └── rpc/                    # alloy provider management
│       ├── mod.rs
│       └── provider.rs
```

### Dependencies

```toml
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
alloy-primitives = "1.5"
alloy-provider = { version = "1", features = ["reqwest"] }
alloy-rpc-types = "1"
alloy-sol-types = "1"
eyre = "0.6"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = "0.3"
shellexpand = "3"
```

### Implementation Details

**Deploy proxy:** `forge create` for logic, encode `abi.encode(impl, admin, initData)` in Rust, `forge create` for proxy, verify admin slot via alloy `eth_getStorageAt`.

**Upgrade:** alloy RPC to walk hierarchy (`owner()` calls), build calldata in Rust, `cast send` to execute.

**Artifacts:** Pure Rust JSON writing to `deployments/<network>/<Name>.json` + update `exported_address`.

**Init guard:** alloy `eth_getStorageAt` for `_initialized` slots, `forge inspect` for storage layout & method identifiers.

---

## Part 2: `fdk-std` Solidity Library (built later)

Separate repo, installed via `forge install axieinfinity/fdk-std`.

Works like forge-std: pure Solidity using existing `vm.*` cheatcodes.

### How It Works (no FFI to fdk binary needed)

| Operation | Cheatcode Used |
|-----------|---------------|
| Read `fdk.toml` config | `vm.readFile()` + `vm.parseToml()` |
| Write deployment artifacts | `vm.serializeJson()` + `vm.writeFile()` |
| Load registry | `vm.readDir()` + `vm.readFile()` |
| Read proxy admin/impl | `vm.load()` (EIP-1967 slots) |
| Deploy contracts | `vm.getCode()` + inline assembly CREATE |
| Broadcast transactions | `vm.startBroadcast()` / `vm.stopBroadcast()` |
| Record logs & state diffs | `vm.recordLogs()` + `vm.startStateDiffRecording()` |
| Inspect storage layout | `vm.ffi("forge inspect ...")` (only for init guard) |

### Library Structure

```
fdk-std/
├── src/
│   ├── FdkScript.sol           # Base script (extends forge-std Script)
│   ├── FdkMigration.sol        # BaseMigration equivalent
│   ├── FdkConfig.sol           # Reads fdk.toml via vm.parseToml()
│   ├── FdkDeploy.sol           # _deployImmutable, _deployLogic, _deployProxy
│   ├── FdkProxy.sol            # getProxyAdmin, getProxyImpl, findHierarchyAdmin
│   ├── FdkArtifact.sol         # JSON artifact generation via vm.writeJson
│   ├── FdkRegistry.sol         # Load/save deployments via vm.readFile/writeFile
│   ├── FdkInitGuard.sol        # Initialization validation
│   ├── FdkNetwork.sol          # Network switching (wraps vm.createFork/selectFork)
│   └── FdkWallet.sol           # Wallet management
```

### Usage in Solidity Scripts

```solidity
import {FdkMigration} from "fdk-std/FdkMigration.sol";

contract DeployMyContract is FdkMigration {
    function _sharedArguments() internal override returns (bytes memory) {
        address owner = config().getAddress("ronin-testnet", "ProxyAdmin");
        return abi.encode(owner);
    }

    function run() public override {
        address owner = abi.decode(arguments(), (address));

        address proxy = _deployProxy(
            "MyContract",
            abi.encodeCall(MyContract.initialize, (owner))
        );

        address token = _deployImmutable(
            "MyToken",
            abi.encode("MyToken", "MTK")
        );
    }
}
```

```solidity
contract UpgradeMyContract is FdkMigration {
    function run() public override {
        address proxy = registry().get("MyContract");

        _upgradeProxy(
            "MyContract",
            proxy,
            abi.encodeCall(MyContract.reinitialize, (2))
        );
    }
}
```

---

## Implementation Phases (fdk CLI first)

| Phase | Deliverable | Days |
|-------|------------|------|
| 1 | Project scaffold + `fdk init` + TOML config loading | 1-2 |
| 2 | `fdk proxy admin/impl/hierarchy` (alloy RPC) | 2-4 |
| 3 | `fdk deploy --type immutable/logic` + artifact generation | 4-7 |
| 4 | `fdk deploy --type proxy` (OZv4 first) | 7-10 |
| 5 | `fdk upgrade` (hierarchy walk + cast send) | 10-13 |
| 6 | `fdk registry list/get/set` + deployment loading | 13-15 |
| 7 | `fdk validate` (initialization guard) | 15-19 |
| 8 | `fdk migrate` (forge script wrapper + post-processing) | 19-22 |
| 9 | `fdk network list/info` + wallet support | 22-25 |
| 10 | **fdk-std** Solidity library (separate repo) | 25-32 |

**Total: ~5 weeks** (4 weeks CLI + 1 week Solidity library)
