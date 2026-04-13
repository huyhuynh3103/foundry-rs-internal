use alloy_primitives::Address;
use std::collections::HashMap;

pub mod artifact;
pub mod cheatcode;
pub mod multisig;

#[derive(Default, Clone, Debug)]
pub struct FdkState {
    pub address_book: HashMap<u64, HashMap<String, Address>>,
    pub chain_id_to_alias: HashMap<u64, String>,
    pub alias_to_chain_id: HashMap<String, u64>,
}
