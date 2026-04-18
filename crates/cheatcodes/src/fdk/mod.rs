use alloy_primitives::{Address, B256, Bytes, keccak256};
use std::collections::HashMap;

pub mod artifact;
pub mod cheatcode;
pub mod multisig;

/// Converts a string to a bytes32 key using keccak256 hash.
/// This provides efficient storage while maintaining string-based API for users.
#[inline]
pub fn string_to_key(s: &str) -> B256 {
    keccak256(s.as_bytes())
}

/// Contract address book that uses bytes32 keys internally but provides string-based API.
///
/// This is more efficient than using String keys directly because:
/// - Hashing ensures O(1) lookup regardless of string length
/// - Fixed-size keys (32 bytes) are more cache-friendly
/// - Follows Solidity best practices (mapping(bytes32 => address))
#[derive(Default, Clone, Debug)]
pub struct ContractAddressBook {
    /// Internal storage: contractNameHash => address
    inner: HashMap<B256, Address>,
    /// Reverse lookup for debugging: hash => original name
    /// Only populated when inserting, not strictly necessary for operation
    #[cfg(debug_assertions)]
    reverse_lookup: HashMap<B256, String>,
}

impl ContractAddressBook {
    /// Creates a new empty address book
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a contract address with a string name (converted to bytes32 key internally)
    pub fn insert(&mut self, contract_name: &str, address: Address) {
        let key = string_to_key(contract_name);
        self.inner.insert(key, address);

        #[cfg(debug_assertions)]
        {
            self.reverse_lookup.insert(key, contract_name.to_string());
        }
    }

    /// Gets a contract address by string name
    pub fn get(&self, contract_name: &str) -> Option<Address> {
        let key = string_to_key(contract_name);
        self.inner.get(&key).copied()
    }

    /// Checks if a contract exists in the address book
    pub fn contains(&self, contract_name: &str) -> bool {
        let key = string_to_key(contract_name);
        self.inner.contains_key(&key)
    }

    /// Returns the number of contracts in the address book
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks if the address book is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Creates an address book from a HashMap of string names to addresses
    pub fn from_string_map(map: HashMap<String, Address>) -> Self {
        let mut book = Self::new();
        for (name, address) in map {
            book.insert(&name, address);
        }
        book
    }
}

/// Contract configuration storage that uses bytes32 keys internally.
#[derive(Default, Clone, Debug)]
pub struct ContractConfigStore {
    /// Internal storage: contractNameHash => config
    inner: HashMap<B256, Bytes>,
    /// Reverse lookup for debugging
    #[cfg(debug_assertions)]
    reverse_lookup: HashMap<B256, String>,
}

impl ContractConfigStore {
    /// Creates a new empty config store
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a contract configuration
    pub fn insert(&mut self, contract_name: &str, config: Bytes) {
        let key = string_to_key(contract_name);
        self.inner.insert(key, config);

        #[cfg(debug_assertions)]
        {
            self.reverse_lookup.insert(key, contract_name.to_string());
        }
    }

    /// Gets a contract configuration by name
    pub fn get(&self, contract_name: &str) -> Option<&Bytes> {
        let key = string_to_key(contract_name);
        self.inner.get(&key)
    }
}

#[derive(Default, Clone, Debug)]
pub struct FdkState {
    /// Contract address book: chainId => contractName (as bytes32) => address
    /// Uses bytes32 keys internally for efficiency, but provides string-based API
    pub address_book: HashMap<u64, ContractAddressBook>,
    /// Chain ID to alias mapping (cached)
    pub chain_id_to_alias: HashMap<u64, String>,
    /// Alias to chain ID mapping (cached)
    pub alias_to_chain_id: HashMap<String, u64>,
    /// Contract configuration storage: chainId => contractName (as bytes32) => config
    /// Allows storing arbitrary configuration data for contracts across chains
    pub contract_configs: HashMap<u64, ContractConfigStore>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_to_key_deterministic() {
        // Same string should always produce same key
        let key1 = string_to_key("MyContract");
        let key2 = string_to_key("MyContract");
        assert_eq!(key1, key2);

        // Different strings should produce different keys
        let key3 = string_to_key("OtherContract");
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_contract_address_book_basic() {
        let mut book = ContractAddressBook::new();
        let addr = Address::random();

        // Test insert and get
        book.insert("MyToken", addr);
        assert_eq!(book.get("MyToken"), Some(addr));

        // Test get non-existent
        assert_eq!(book.get("NonExistent"), None);

        // Test contains
        assert!(book.contains("MyToken"));
        assert!(!book.contains("NonExistent"));

        // Test len
        assert_eq!(book.len(), 1);
        assert!(!book.is_empty());
    }

    #[test]
    fn test_contract_address_book_multiple_inserts() {
        let mut book = ContractAddressBook::new();
        let addr1 = Address::random();
        let addr2 = Address::random();
        let addr3 = Address::random();

        book.insert("Token1", addr1);
        book.insert("Token2", addr2);
        book.insert("NFT", addr3);

        assert_eq!(book.get("Token1"), Some(addr1));
        assert_eq!(book.get("Token2"), Some(addr2));
        assert_eq!(book.get("NFT"), Some(addr3));
        assert_eq!(book.len(), 3);
    }

    #[test]
    fn test_contract_address_book_overwrite() {
        let mut book = ContractAddressBook::new();
        let addr1 = Address::random();
        let addr2 = Address::random();

        // Insert initial value
        book.insert("MyToken", addr1);
        assert_eq!(book.get("MyToken"), Some(addr1));

        // Overwrite with new value
        book.insert("MyToken", addr2);
        assert_eq!(book.get("MyToken"), Some(addr2));
        assert_eq!(book.len(), 1); // Still only 1 entry
    }

    #[test]
    fn test_contract_address_book_from_string_map() {
        let mut map = HashMap::new();
        let addr1 = Address::random();
        let addr2 = Address::random();

        map.insert("Token1".to_string(), addr1);
        map.insert("Token2".to_string(), addr2);

        let book = ContractAddressBook::from_string_map(map);

        assert_eq!(book.get("Token1"), Some(addr1));
        assert_eq!(book.get("Token2"), Some(addr2));
        assert_eq!(book.len(), 2);
    }

    #[test]
    fn test_contract_config_store_basic() {
        let mut store = ContractConfigStore::new();
        let config = Bytes::from(vec![1, 2, 3, 4]);

        // Test insert and get
        store.insert("MyContract", config.clone());
        assert_eq!(store.get("MyContract"), Some(&config));

        // Test get non-existent
        assert_eq!(store.get("NonExistent"), None);
    }

    #[test]
    fn test_contract_config_store_multiple() {
        let mut store = ContractConfigStore::new();
        let config1 = Bytes::from(vec![1, 2, 3]);
        let config2 = Bytes::from(vec![4, 5, 6]);

        store.insert("Contract1", config1.clone());
        store.insert("Contract2", config2.clone());

        assert_eq!(store.get("Contract1"), Some(&config1));
        assert_eq!(store.get("Contract2"), Some(&config2));
    }

    #[test]
    fn test_contract_name_special_characters() {
        let mut book = ContractAddressBook::new();
        let addr = Address::random();

        // Test with path-like names
        book.insert("src/contracts/MyToken.sol", addr);
        assert_eq!(book.get("src/contracts/MyToken.sol"), Some(addr));

        // Test with colons (common in Foundry artifact paths)
        book.insert("contracts/Token.sol:Token", addr);
        assert_eq!(book.get("contracts/Token.sol:Token"), Some(addr));
    }

    #[test]
    fn test_bytes32_key_efficiency() {
        // Verify that short and long strings both hash to fixed-size B256
        let short_key = string_to_key("A");
        let long_key = string_to_key("VeryLongContractNameWithManyCharacters");

        // Both should be B256 (32 bytes)
        assert_eq!(short_key.len(), 32);
        assert_eq!(long_key.len(), 32);
    }
}
