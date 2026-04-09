# User Testing

## Validation Surface

**Primary surface:** CLI — custom-built `forge` binary executing Solidity tests and scripts that use FDK cheatcodes.

**Testing approach:**
1. Build the modified forge: `cargo build -p forge`
2. Run Solidity tests: `./target/debug/forge test --match-path testdata/default/cheats/Fdk.t.sol -vvvv`
3. Verify file system artifacts: check `deployments/` directory for generated JSONs
4. Verify on-chain state: use `vm.load()` cross-checks in Solidity tests

**Tools:** Shell commands via `Execute` tool. No browser or TUI testing needed.

**Testing prerequisites:**
- Modified forge binary must be built (`cargo build -p forge`)
- Test contracts must be compiled (`forge build` in test project context)
- fdk.toml fixture must exist in the test project root

## Validation Concurrency

**Max concurrent validators:** 2

**Rationale:** Each validator needs to build forge (~2-3 GB RAM for compilation) and run tests. On 16GB RAM / 10 CPU machine with ~6GB baseline usage, 2 concurrent validators = ~6GB additional (2x ~3GB each) = ~12GB total, within the 70% headroom of 10GB available.

## Test Fixtures Needed

- `testdata/default/cheats/Fdk.t.sol` — Solidity test contract for FDK cheatcodes
- `testdata/default/cheats/fdk.toml` — FDK config fixture
- Simple Solidity contracts for deployment tests (Counter, Token, upgradeable variants)
