# FDK Cheatcodes — Architecture

## Overview

FDK adds a second cheatcode interface to Foundry alongside the existing `vm.*` cheatcodes. FDK cheatcodes are intercepted at a dedicated address (`FDK_CHEATCODE_ADDRESS`) by the same `Cheatcodes` inspector that handles `vm.*`, following the identical pattern: `sol!` macro → `#[derive(Cheatcode)]` → `impl Cheatcode for *Call`.

## Architecture Diagram

```
forge script / forge test
    ↓
revm EVM execution
    ↓
InspectorStack::call()
    ↓
Cheatcodes::call_with_executor()
    ├── if target == CHEATCODE_ADDRESS → apply_cheatcode() [existing vm.*]
    └── if target == FDK_CHEATCODE_ADDRESS → apply_fdk_cheatcode() [NEW]
            ↓
        Fdk::FdkCalls::abi_decode(input)
            ↓
        fdk_dispatch!(match decoded { ... })
            ↓
        Cheatcode::apply / apply_stateful / apply_full
```

## Key Components

### 1. Spec Layer (`crates/cheatcodes/spec/src/fdk.rs`)
Defines the `Fdk` Solidity interface via `sol!` macro. The `#[derive(Cheatcode)]` proc macro auto-generates:
- `FdkCalls` enum (all decoded call variants)
- `fdk_calls!` dispatch macro
- `CheatcodeDef` impl for each `*Call` struct

### 2. Implementation Layer (`crates/cheatcodes/src/fdk/`)
Module containing `impl Cheatcode for <FdkCallStruct>` for each cheatcode. Sub-modules:
- `config.rs` — fdk.toml loading, getAddress, loadContract
- `proxy.rs` — getProxyAdmin, getProxyImplementation (EIP-1967 slot reads)
- `deploy.rs` — deployImmutable, deployLogic, deployProxy
- `upgrade.rs` — upgradeProxy (hierarchy walk + execute)
- `validate.rs` — validateInit (OZv4/v5 slot reading)
- `artifact.rs` — saveArtifact, artifact JSON generation, registry I/O

### 3. Constants (`crates/evm/core/src/constants.rs`)
- `FDK_CHEATCODE_ADDRESS` — deterministic from `keccak256("fdk cheat code")`
- `FDK_CHEATCODE_CONTRACT_HASH` — for code hash checks

### 4. Inspector Integration (`crates/cheatcodes/src/inspector.rs`)
- Second address check in `call_with_executor()` after the existing VM check
- `apply_fdk_cheatcode()` method that decodes `FdkCalls` and dispatches
- FDK address added to `call_end` cheatcode_call check

### 5. Exclusion Lists (various files)
FDK address added everywhere `CHEATCODE_ADDRESS` is excluded: gas reports, fuzzing, invariants, traces, revert diagnostics, backend persistent addresses.

## Data Flow

### Deploy Proxy Flow
1. User calls `fdk.deployProxy("MyContract", initData)` in Solidity
2. Inspector intercepts call to FDK address
3. `deployProxyCall` struct decoded
4. Implementation reads compiled artifact from `out/MyContract.sol/MyContract.json`
5. Deploys logic via CREATE in EVM context
6. Reads ProxyAdmin from fdk.toml `[contracts.<chainId>]`
7. Encodes TransparentProxy constructor args (logic, admin, initData)
8. Deploys proxy via CREATE
9. Verifies EIP-1967 admin slot
10. Writes artifact to `deployments/<chainId>/MyContract.json`
11. Returns proxy address as ABI-encoded bytes

### Config Resolution
- fdk.toml loaded from project root (same dir as foundry.toml)
- Environment variables interpolated (`${VAR}` syntax)
- Network resolved by current chain ID matching `[networks.*].chain_id`
- Contract addresses looked up in `[contracts.<network>.<name>]`

## EIP-1967 Slots
- Implementation: `0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc`
- Admin: `0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103`

## OZ Initialization Slots
- OZv5 (ERC-7201): `0xf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00` (uint64 _initialized + bool _initializing)
- OZv4: slot 0 (uint8 _initialized + bool _initializing, packed)

## Invariants
- FDK cheatcodes never modify existing `vm.*` behavior
- FDK state is stored in the existing `Cheatcodes` struct (new fields added)
- All FDK file I/O respects Foundry's `fs_permissions` configuration
- Proxy deployments always verify EIP-1967 slots post-deploy
