//! Configuration for FDK (Foundry Deployment Kit)

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the Foundry Deployment Kit
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FdkConfig {
    /// Path to the TransparentUpgradeableProxy contract.
    ///
    /// Default: `@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:
    /// TransparentUpgradeableProxy`
    #[serde(default = "default_transparent_proxy_path")]
    pub transparent_proxy_path: String,

    /// Path to the ProxyAdmin contract.
    ///
    /// Default: `@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin`
    #[serde(default = "default_proxy_admin_path")]
    pub proxy_admin_path: String,

    /// Root directory for deployment artifacts.
    ///
    /// Default: `deployments`
    #[serde(default = "default_deployments_root")]
    pub deployments_root: String,

    /// Multisig wallet addresses by chain alias.
    ///
    /// When the sender is one of these multisig wallets, transactions will be:
    /// 1. Logged to a file (from, to, calldata) instead of broadcast
    /// 2. Simulated using prank to verify correctness
    ///
    /// Format: `{ "mainnet": "0x...", "optimism": "0x..." }`
    #[serde(default)]
    pub multisig_wallets: HashMap<String, Address>,

    /// Default contracts to pre-load into the registry by chain.
    ///
    /// These contracts will be automatically loaded at initialization,
    /// making them available via `fdk.loadContract()` without needing
    /// to deploy them first.
    ///
    /// Format:
    /// ```toml
    /// [fdk.default_contracts]
    /// mainnet = { ProxyAdmin = "0x1234...", USDC = "0x5678..." }
    /// sepolia = { ProxyAdmin = "0xabcd..." }
    /// ```
    #[serde(default)]
    pub default_contracts: HashMap<String, HashMap<String, Address>>,
}

impl Default for FdkConfig {
    fn default() -> Self {
        Self {
            transparent_proxy_path: default_transparent_proxy_path(),
            proxy_admin_path: default_proxy_admin_path(),
            deployments_root: default_deployments_root(),
            multisig_wallets: HashMap::new(),
            default_contracts: HashMap::new(),
        }
    }
}

fn default_transparent_proxy_path() -> String {
    "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy".to_string()
}

fn default_proxy_admin_path() -> String {
    "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin".to_string()
}

fn default_deployments_root() -> String {
    "deployments".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fdk_config() {
        let config = FdkConfig::default();
        assert_eq!(
            config.transparent_proxy_path,
            "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy"
        );
        assert_eq!(
            config.proxy_admin_path,
            "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin"
        );
        assert_eq!(config.deployments_root, "deployments");
    }

    #[test]
    fn test_fdk_config_serde() {
        let toml_str = r#"
transparent_proxy_path = "custom/path/TransparentProxy.sol:TransparentProxy"
proxy_admin_path = "custom/path/ProxyAdmin.sol:ProxyAdmin"
deployments_root = "deploy"

[multisig_wallets]
mainnet = "0x1234567890123456789012345678901234567890"
optimism = "0x0987654321098765432109876543210987654321"
"#;

        let config: FdkConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.transparent_proxy_path,
            "custom/path/TransparentProxy.sol:TransparentProxy"
        );
        assert_eq!(config.proxy_admin_path, "custom/path/ProxyAdmin.sol:ProxyAdmin");
        assert_eq!(config.deployments_root, "deploy");
        assert_eq!(config.multisig_wallets.len(), 2);
        assert_eq!(
            config.multisig_wallets.get("mainnet").unwrap(),
            &"0x1234567890123456789012345678901234567890".parse::<Address>().unwrap()
        );
    }

    #[test]
    fn test_fdk_config_with_default_contracts() {
        let toml_str = r#"
transparent_proxy_path = "custom/TransparentProxy.sol:TransparentProxy"
proxy_admin_path = "custom/ProxyAdmin.sol:ProxyAdmin"
deployments_root = "deployments"

[default_contracts.mainnet]
ProxyAdmin = "0x1234567890123456789012345678901234567890"
USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"

[default_contracts.sepolia]
ProxyAdmin = "0xabcdef0123456789abcdef0123456789abcdef01"
USDC = "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238"
"#;

        let config: FdkConfig = toml::from_str(toml_str).unwrap();

        // Check default contracts
        assert_eq!(config.default_contracts.len(), 2);

        let mainnet_contracts = config.default_contracts.get("mainnet").unwrap();
        assert_eq!(mainnet_contracts.len(), 3);
        assert_eq!(
            mainnet_contracts.get("ProxyAdmin").unwrap(),
            &"0x1234567890123456789012345678901234567890".parse::<Address>().unwrap()
        );
        assert_eq!(
            mainnet_contracts.get("USDC").unwrap(),
            &"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>().unwrap()
        );

        let sepolia_contracts = config.default_contracts.get("sepolia").unwrap();
        assert_eq!(sepolia_contracts.len(), 2);
        assert_eq!(
            sepolia_contracts.get("ProxyAdmin").unwrap(),
            &"0xabcdef0123456789abcdef0123456789abcdef01".parse::<Address>().unwrap()
        );
    }
}
