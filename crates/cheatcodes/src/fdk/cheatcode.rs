//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result, Vm::*};
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
        let chain = get_chain(ccx.state, &chain_id.to_string())
            .map_err(|_| fmt_err!("invalid chain ID: {chain_id}"))?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_1Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, chainAlias } = self;
        let chain = get_chain(ccx.state, chainAlias)?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        let chain = get_chain(state, &chain_id.to_string())
            .map_err(|_| fmt_err!("invalid chain ID: {chain_id}"))?;
        load_contract(state, chain, contractName).map(|address| address.abi_encode())
    }
}
#[derive(serde::Deserialize)]
struct DeploymentArtifact {
    address: Address,
}

fn load_contract<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain: Chain,
    contract_name: &str,
) -> Result<Address> {
    let chain_id = chain.chainId.to::<u64>();
    if let Some(address) = state
        .fdk
        .address_book
        .get(&chain_id)
        .and_then(|entries| entries.get(contract_name))
        .copied()
    {
        return Ok(address);
    }

    let chain_alias = chain.chainAlias.clone();

    // if it doesn't exist in address book, we need to resolve the address from the deployment files
    let address =
        resolve_deployment_address(state, &chain_alias, contract_name)?.ok_or_else(|| {
            fmt_err!("no deployment found for {contract_name} on chain {chain_alias}")
        })?;

    state.fdk.address_book.entry(chain_id).or_default().insert(contract_name.to_string(), address);
    Ok(address)
}

fn get_chain<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_alias: &str,
) -> Result<Chain> {
    // Parse the chain alias - works for both chain names and IDs
    let alloy_chain = AlloyChain::from_str(chain_alias)
        .map_err(|_| fmt_err!("invalid chain alias: {chain_alias}"))?;
    let chain_name = alloy_chain.to_string();
    let chain_id = alloy_chain.id();

    // Check if this is an unknown chain ID by comparing the name to the chain ID
    // When a numeric ID is passed for an unknown chain, alloy_chain.to_string() will return the ID
    // So if they match, it's likely an unknown chain ID
    if chain_name == chain_id.to_string() {
        return Err(fmt_err!("invalid chain alias: {chain_alias}"));
    }

    // Try to retrieve RPC URL and chain alias from user's config in foundry.toml.
    let (rpc_url, chain_alias) = if let Some(rpc_url) =
        state.config.rpc_endpoint(&chain_name).ok().and_then(|e| e.url().ok())
    {
        (rpc_url, chain_name.clone())
    } else {
        (String::new(), chain_alias.to_string())
    };

    let chain_struct = Chain {
        name: chain_name,
        chainId: U256::from(chain_id),
        chainAlias: chain_alias,
        rpcUrl: rpc_url,
    };

    Ok(chain_struct)
}

fn resolve_deployment_address<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    chain_alias: &str,
    contract_name: &str,
) -> Result<Option<Address>> {
    let deployments_path = deployments_root();
    let deployment_file = deployments_path.join(chain_alias).join(format!("{contract_name}.json"));
    let deployment_file = state.config.ensure_path_allowed(deployment_file, FsAccessKind::Read)?;
    if !deployment_file.exists() {
        let proxy_file =
            deployments_path.join(chain_alias).join(format!("{contract_name}Proxy.json"));
        let logic_file =
            deployments_path.join(chain_alias).join(format!("{contract_name}Logic.json"));
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


fn deployments_root() -> PathBuf {
    PathBuf::from("deployments")
}