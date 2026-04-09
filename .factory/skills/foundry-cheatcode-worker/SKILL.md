---
name: foundry-cheatcode-worker
description: Implements FDK cheatcodes in the Foundry codebase (Rust + Solidity tests)
---

# Foundry Cheatcode Worker

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

Features that involve:
- Defining FDK cheatcode Solidity interfaces (spec layer)
- Implementing FDK cheatcodes in Rust (impl layer)
- Modifying the inspector to dispatch FDK calls
- Adding FDK address to exclusion lists
- Writing Solidity integration tests for FDK cheatcodes
- Creating test fixtures (contracts, fdk.toml)

## Required Skills

None. All work is done via standard file editing and shell commands.

## Work Procedure

### Step 1: Understand the Feature
- Read the feature description, preconditions, expectedBehavior, and verificationSteps carefully
- Read `.factory/library/architecture.md` for the overall design
- Read `AGENTS.md` for coding conventions and key file references
- If implementing a cheatcode, read an existing similar implementation for the pattern:
  - Simple (pure): `crates/cheatcodes/src/evm.rs` → `addrCall` (uses `apply`)
  - Stateful (EVM read): `crates/cheatcodes/src/evm.rs` → `getNonce_0Call` (uses `apply_stateful`)
  - Full (executor): `crates/cheatcodes/src/evm/fork.rs` for complex operations (uses `apply_full`)
  - File I/O: `crates/cheatcodes/src/fs.rs` for reading/writing files
  - TOML parsing: `crates/cheatcodes/src/toml.rs` for TOML operations

### Step 2: Write Tests First (Red)
- **Rust unit tests**: Add `#[cfg(test)]` module in the implementation file for internal logic (config parsing, slot constants, serialization). Write tests that will FAIL before implementation.
- **Solidity integration tests**: Add test functions to `testdata/default/cheats/Fdk.t.sol`. Each cheatcode needs at least: happy path test, error case test. Use `vm.expectRevert()` for error cases. Cross-check with `vm.load()` for storage reads.
- Run tests to confirm they fail: `cargo test -p foundry-cheatcodes` and `cargo test -p forge --test it -- cheats::fdk`

### Step 3: Implement
- **Spec changes** (`crates/cheatcodes/spec/src/fdk.rs`): Add function definitions to the `sol!` interface block with `#[cheatcode(group = ..., safety = ...)]` annotations and `///` documentation.
- **Implementation** (`crates/cheatcodes/src/fdk/*.rs`): Implement `Cheatcode` trait for the generated `*Call` structs. Choose the right method:
  - `apply` — for pure operations (no EVM access needed)
  - `apply_stateful` — for EVM state reads/writes (storage, balance)
  - `apply_full` — for operations needing the executor (CREATE, nested calls)
- **Inspector wiring** (if needed): Ensure `apply_fdk_cheatcode` in inspector.rs dispatches the new call.
- Return `Ok(result.abi_encode())` on success, `Err(...)` with descriptive message on failure.

### Step 4: Make Tests Pass (Green)
- Run `cargo build -p foundry-cheatcodes` to verify compilation
- Run `cargo test -p foundry-cheatcodes` for Rust unit tests
- Run `cargo build -p forge` to build the forge binary (needed for Solidity tests)
- Run `cargo test -p forge --test it -- cheats::fdk` for Solidity integration tests
- Fix any failures until all tests pass

### Step 5: Verify Quality
- Run `cargo clippy -p foundry-cheatcodes -p foundry-cheatcodes-spec -- -D warnings`
- Run `cargo fmt --check` (fix with `cargo fmt` if needed)
- Verify error messages are user-friendly (contain contract name, file path, or config key)
- Verify no panics — all errors go through `eyre::Result`

### Step 6: Manual Verification
- For each cheatcode implemented, mentally trace through the full call path:
  1. Solidity calls `fdk.someFunction(args)`
  2. Inspector intercepts at FDK address
  3. `FdkCalls` decoded
  4. Dispatch to correct handler
  5. Handler executes logic
  6. Returns ABI-encoded result
- Check that any file I/O uses `state.config.ensure_path_allowed()`
- Check that artifacts are written to the correct path

## Example Handoff

```json
{
  "salientSummary": "Implemented fdk.deployProxy cheatcode: spec definition in fdk.rs with sol! macro, Rust impl in fdk/deploy.rs using apply_full to deploy logic via CREATE + TransparentProxy via CREATE, verify EIP-1967 admin slot, write artifact JSON. Added 6 Solidity tests (happy path, constructor args, admin verify, init exec, missing artifact, missing ProxyAdmin). All pass: cargo test -p foundry-cheatcodes (12 passing), cargo test -p forge --test it -- cheats::fdk (6 passing), clippy clean.",
  "whatWasImplemented": "Added deployProxy(string,bytes) and deployProxy(string,address,bytes,bytes) cheatcodes. Spec in crates/cheatcodes/spec/src/fdk.rs. Impl in crates/cheatcodes/src/fdk/deploy.rs. Reads compiled artifacts from out/ dir, deploys logic + TransparentProxyOZv4_9_5, verifies admin slot, writes artifact to deployments/<chainId>/<Name>.json. 6 Solidity tests in testdata/default/cheats/Fdk.t.sol covering happy path, error cases, and artifact verification.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo build -p foundry-cheatcodes", "exitCode": 0, "observation": "Compiles cleanly" },
      { "command": "cargo test -p foundry-cheatcodes", "exitCode": 0, "observation": "12 tests passing including 4 new FDK unit tests" },
      { "command": "cargo build -p forge", "exitCode": 0, "observation": "Forge binary built with FDK support" },
      { "command": "cargo test -p forge --test it -- cheats::fdk", "exitCode": 0, "observation": "6 Solidity integration tests passing" },
      { "command": "cargo clippy -p foundry-cheatcodes -- -D warnings", "exitCode": 0, "observation": "No warnings" }
    ],
    "interactiveChecks": [
      { "action": "Traced deployProxy call path: Solidity → inspector → FdkCalls decode → deployProxy_0Call → apply_full → read artifact → CREATE logic → CREATE proxy → verify admin slot → write artifact → return proxy address", "observed": "All steps execute correctly, proxy delegates to logic, artifact written with correct fields" }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "crates/cheatcodes/src/fdk/deploy.rs",
        "cases": [
          { "name": "test_artifact_json_serialization", "verifies": "Artifact JSON has correct fields" },
          { "name": "test_proxy_constructor_encoding", "verifies": "TransparentProxy constructor args encoded correctly" }
        ]
      },
      {
        "file": "testdata/default/cheats/Fdk.t.sol",
        "cases": [
          { "name": "test_fdkDeployProxy", "verifies": "Happy path proxy deployment and delegation" },
          { "name": "test_fdkDeployProxyAdmin", "verifies": "Admin slot matches fdk.toml ProxyAdmin" },
          { "name": "test_fdkDeployProxyInit", "verifies": "Initializer executed during deploy" },
          { "name": "test_fdkDeployProxyMissingArtifact", "verifies": "Reverts with contract name for missing artifact" }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The Foundry codebase has breaking compilation errors unrelated to FDK changes
- The `Cheatcode` derive macro behaves unexpectedly (e.g., doesn't generate expected dispatch code)
- Existing Foundry tests fail due to FDK changes (should never happen if exclusion lists are correct)
- A cheatcode needs access to data not available in the `CheatsCtxt` or executor
- Feature depends on another FDK feature that hasn't been implemented yet
- Build time exceeds 15 minutes for an incremental build (environment issue)
