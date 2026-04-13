//! Configuration for FDK (Foundry Deployment Kit)

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the Foundry Deployment Kit
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FdkConfig {
    /// Path to the TransparentUpgradeableProxy contract.
    /// 
    /// Default: `@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol:TransparentUpgradeableProxy`
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
}

impl Default for FdkConfig {
    fn default() -> Self {
        Self {
            transparent_proxy_path: default_transparent_proxy_path(),
            proxy_admin_path: default_proxy_admin_path(),
            deployments_root: default_deployments_root(),
            multisig_wallets: HashMap::new(),
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
        assert_eq!(config.transparent_proxy_path, "custom/path/TransparentProxy.sol:TransparentProxy");
        assert_eq!(config.proxy_admin_path, "custom/path/ProxyAdmin.sol:ProxyAdmin");
        assert_eq!(config.deployments_root, "deploy");
        assert_eq!(config.multisig_wallets.len(), 2);
        assert_eq!(
            config.multisig_wallets.get("mainnet").unwrap(),
            &"0x1234567890123456789012345678901234567890".parse::<Address>().unwrap()
        );
    }
}
