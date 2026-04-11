//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result};
use alloy_chains::Chain as AlloyChain;
use alloy_primitives::{Address, U256};
use alloy_sol_types::SolValue;
use foundry_common::fs;
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::evm::FoundryEvmNetwork;
use revm::context::ContextTr;
use std::{path::PathBuf, str::FromStr};

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}

impl Cheatcode for loadContract_0Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        load_contract_by_chain_id(ccx.state, chain_id, contractName).map(|address| {
            address.abi_encode()
        })
    }
}

impl Cheatcode for loadContract_1Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, networkName } = self;
        let alloy_chain = AlloyChain::from_str(networkName)
            .map_err(|_| fmt_err!("invalid chain alias: {networkName}"))?;
        let chain_name = alloy_chain.to_string();
        let chain_id = alloy_chain.id();
        if chain_name == chain_id.to_string() {
            return Err(fmt_err!("invalid chain alias: {networkName}"));
        }
        load_contract_by_chain_id(ccx.state, chain_id, contractName).map(|address| {
            address.abi_encode()
        })
    }
}

impl Cheatcode for loadContract_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        load_contract_by_chain_id(state, chain_id, contractName).map(|address| {
            address.abi_encode()
        })
    }
}
fn deployments_root() -> PathBuf {
    "deployments".into()
}

#[derive(serde::Deserialize)]
struct DeploymentArtifact {
    address: Address,
}

fn resolve_deployment_address<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    network_name: &str,
    contract_name: &str,
) -> Result<Option<Address>> {
    let deployments_path = deployments_root();
    let deployment_file = deployments_path.join(network_name).join(format!("{contract_name}.json"));
    let deployment_file = state.config.ensure_path_allowed(deployment_file, FsAccessKind::Read)?;
    if !deployment_file.exists() {
        let proxy_file =
            deployments_path.join(network_name).join(format!("{contract_name}Proxy.json"));
        let logic_file =
            deployments_path.join(network_name).join(format!("{contract_name}Logic.json"));
        let proxy_file = state.config.ensure_path_allowed(proxy_file, FsAccessKind::Read)?;
        let logic_file = state.config.ensure_path_allowed(logic_file, FsAccessKind::Read)?;
        if !proxy_file.exists() || !logic_file.exists() {
            return Ok(None);
        }
        let proxy_contents = fs::locked_read_to_string(&proxy_file)?;
        let proxy_artifact: DeploymentArtifact = serde_json::from_str(&proxy_contents)?;
        return Ok(Some(proxy_artifact.address));
    }
    let contents = fs::locked_read_to_string(&deployment_file)?;
    let artifact: DeploymentArtifact =
        serde_json::from_str(&contents).map_err(|e| fmt_err!("failed parsing deployment: {e}"))?;
    Ok(Some(artifact.address))
}

fn load_contract_by_chain_id<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
    contract_name: &str,
) -> Result<Address> {
    if let Some(address) = state
        .fdk_address_book
        .get(&chain_id)
        .and_then(|entries| entries.get(contract_name))
        .copied()
    {
        return Ok(address);
    }

    let network_name = AlloyChain::from_id(chain_id).to_string();
    let mut address: Option<Address> = resolve_deployment_address(state, &network_name, contract_name)?;
    if address.is_none() {
        let chain_folder = network_name.to_string();
        if chain_folder != network_name {
            address = resolve_deployment_address(state, &chain_folder, contract_name)?;
        }
    }
    let address = address.ok_or_else(|| {
        fmt_err!("no deployment found for {contract_name} on chain {network_name}")
    })?;

    state
        .fdk_address_book
        .entry(chain_id)
        .or_default()
        .insert(contract_name.to_string(), address);
    Ok(address)
}
