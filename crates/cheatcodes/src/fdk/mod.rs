use alloy_primitives::{Address, Bytes};
use std::collections::HashMap;

pub mod artifact;
pub mod cheatcode;
pub mod multisig;

#[derive(Default, Clone, Debug)]
pub struct FdkState {
    /// Contract address book: chainId => contractName => address
    pub address_book: HashMap<u64, HashMap<String, Address>>,
    /// Chain ID to alias mapping (cached)
    pub chain_id_to_alias: HashMap<u64, String>,
    /// Alias to chain ID mapping (cached)
    pub alias_to_chain_id: HashMap<String, u64>,
    /// Contract configuration storage: chainId => contractName => config (abi-encoded bytes)
    /// Allows storing arbitrary configuration data for contracts across chains
    pub contract_configs: HashMap<u64, HashMap<String, Bytes>>,
}
