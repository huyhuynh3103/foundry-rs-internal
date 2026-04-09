# Environment

**What belongs here:** Required env vars, external dependencies, build quirks.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## Toolchain

- Rust 1.94.0 (cargo, rustc, clippy, rustfmt)
- Foundry 1.5.1 (forge, cast, anvil) — installed system-wide at `~/.foundry/bin/`
- solc 0.8.19 at `/opt/homebrew/bin/solc`

## Build Notes

- First full build of forge takes ~5-10 minutes. Incremental builds of cheatcodes crate ~30-60 seconds.
- Building just `foundry-cheatcodes` crate is much faster than building the full `forge` binary.
- Use `cargo build -p foundry-cheatcodes` during development, `cargo build -p forge` only when you need to run forge test/script.

## Workspace Structure

The Foundry workspace has ~200 crates. Key ones for FDK:
- `foundry-cheatcodes` (`crates/cheatcodes/`) — main implementation target
- `foundry-cheatcodes-spec` (`crates/cheatcodes/spec/`) — Solidity interface definitions
- `foundry-evm-core` (`crates/evm/core/`) — constants (cheatcode addresses)
- `foundry-macros` (`crates/macros/`) — Cheatcode derive proc macro (read-only reference)
- `forge` (`crates/forge/`) — forge binary + integration tests
