use alloy_primitives::Address;
use std::collections::HashMap;

pub mod cheatcode;

#[derive(Default, Clone, Debug)]
pub struct FdkState {
    pub address_book: HashMap<u64, HashMap<String, Address>>,
}
