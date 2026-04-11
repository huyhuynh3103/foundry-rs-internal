use std::collections::HashMap;
use alloy_primitives::Address;

pub mod cheatcode;

#[derive(Default, Clone, Debug)]
pub struct FdkState {
    pub address_book: HashMap<u64, HashMap<String, Address>>,
}
