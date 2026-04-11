//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result};
use alloy_chains::Chain as AlloyChain;
use alloy_network::AnyNetwork;
use alloy_primitives::{Address, U256};
use alloy_sol_types::SolValue;
use foundry_common::fs;
use foundry_common::provider::ProviderBuilder;
use alloy_provider::Provider;
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::evm::FoundryEvmNetwork;
use revm::context::ContextTr;
use std::path::PathBuf;

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}

impl Cheatcode for loadContract_0Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        load_contract_by_chain_id(ccx.state, chain_id, contractName)
            .map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_1Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, networkName } = self;
        load_contract_by_network_name(ccx.state, networkName, contractName)
            .map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        load_contract_by_chain_id(state, chain_id, contractName).map(|address| address.abi_encode())
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

    let network_name = to_network_name(state, chain_id)?;
    let mut address: Option<Address> =
        resolve_deployment_address(state, &network_name, contract_name)?;
    if address.is_none() {
        let chain_folder = chain_id.to_string();
        if chain_folder != network_name {
            address = resolve_deployment_address(state, &chain_folder, contract_name)?;
        }
    }
    let address = address.ok_or_else(|| {
        fmt_err!("no deployment found for {contract_name} on chain {network_name}")
    })?;

    state.fdk_address_book.entry(chain_id).or_default().insert(contract_name.to_string(), address);
    Ok(address)
}

fn load_contract_by_network_name<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    network_name: &str,
    contract_name: &str,
) -> Result<Address> {
    let address = resolve_deployment_address(state, network_name, contract_name)?
        .ok_or_else(|| fmt_err!("no deployment found for {contract_name} on chain {network_name}"))?;

    if let Ok(chain_id) = resolve_chain_id_for_alias(state, network_name) {
        state
            .fdk_address_book
            .entry(chain_id)
            .or_default()
            .insert(contract_name.to_string(), address);
        state
            .fdk_network_aliases
            .entry(chain_id)
            .or_insert_with(|| network_name.to_string());
    }

    Ok(address)
}

fn resolve_chain_id_for_alias<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    alias: &str,
) -> Result<u64> {
    let url = state.config.rpc_endpoint(alias)?.url()?;
    let provider = ProviderBuilder::<AnyNetwork>::new(&url).build()?;
    foundry_common::block_on(provider.get_chain_id())
        .map_err(|e| fmt_err!("failed to get chain id for {alias}: {e}"))
}

fn to_network_name<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
) -> Result<String> {
    if let Some(alias) = state.fdk_network_aliases.get(&chain_id) {
        return Ok(alias.clone());
    }

    for alias in state.config.rpc_endpoints.keys() {
        let Ok(alias_chain_id) = resolve_chain_id_for_alias(state, alias) else { continue };
        if alias_chain_id == chain_id {
            state.fdk_network_aliases.insert(chain_id, alias.clone());
            return Ok(alias.clone());
        }
    }

    Ok(AlloyChain::from_id(chain_id).to_string())
}
