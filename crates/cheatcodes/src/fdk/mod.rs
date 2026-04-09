//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, Fdk::*, Result};
use alloy_sol_types::SolValue;
use foundry_evm_core::evm::FoundryEvmNetwork;

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}
