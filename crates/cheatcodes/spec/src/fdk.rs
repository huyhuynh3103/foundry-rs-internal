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
}
}
