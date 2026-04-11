//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result, Vm::*};
use alloy_chains::Chain as AlloyChain;
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use foundry_common::{block_on, fs, provider::get_http_provider};
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
        let chain = get_chain(ccx.state, &chain_id.to_string())?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_1Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, chainAlias } = self;
        let chain_id = chain_alias_to_id(ccx.state, chainAlias)?;
        let chain = get_chain(ccx.state, &chain_id.to_string())?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        let chain = get_chain(state, &chain_id.to_string())?;
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

fn chain_alias_to_id<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_alias: &str,
) -> Result<u64> {
    let chain_id = state.fdk.alias_to_chain_id.get(chain_alias).copied();
    match chain_id {
        Some(chain_id) => Ok(chain_id),
        None => {
            match state
                .config
                .rpc_endpoint(&chain_alias)
                .ok()
                .and_then(|e| e.url().ok())
                .and_then(|rpc_url| block_on(get_http_provider(&rpc_url).get_chain_id()).ok())
            {
                Some(chain_id) => {
                    state.fdk.alias_to_chain_id.insert(chain_alias.to_string(), chain_id);
                    state.fdk.chain_id_to_alias.insert(chain_id, chain_alias.to_string());
                    Ok(chain_id)
                }
                None => Err(fmt_err!("chain alias not found: {chain_alias}")),
            }
        }
    }
}

fn chain_id_to_alias<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
) -> Result<String> {
    let chain_alias =
        state.fdk.chain_id_to_alias.get(&chain_id).and_then(|alias| Some(alias.clone()));

    match chain_alias {
        Some(chain_alias) => Ok(chain_alias),
        None => {
            // find chain alias from rpc urls configured in foundry.toml
            let rpc_urls = state.config.rpc_urls()?;
            let chain_alias = rpc_urls.iter().find_map(|rpc| {
                let provider = get_http_provider(&rpc.url);
                let fetched_chain_id = block_on(provider.get_chain_id()).ok()?;
                if fetched_chain_id == chain_id { Some(rpc.key.clone()) } else { None }
            });
            match chain_alias {
                Some(chain_alias) => {
                    state.fdk.chain_id_to_alias.insert(chain_id, chain_alias.clone());
                    state.fdk.alias_to_chain_id.insert(chain_alias.clone(), chain_id);
                    Ok(chain_alias)
                }
                None => {
                    // fallback to name defined in alloy_chains
                    let chain = AlloyChain::from_id(chain_id);
                    let chain_alias = chain.to_string();

                    state.fdk.chain_id_to_alias.insert(chain_id, chain_alias.to_string());
                    state.fdk.alias_to_chain_id.insert(chain_alias.to_string(), chain_id);

                    Ok(chain_alias)
                }
            }
        }
    }
}

fn get_chain<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_alias: &str,
) -> Result<Chain> {
    let chain_id = chain_alias_to_id(state, chain_alias)?;
    let chain_alias = chain_id_to_alias(state, chain_id)?;

    let chain = AlloyChain::from_id(chain_id);
    let chain_name = chain.to_string();
    let rpc_url =
        state.config.rpc_endpoint(&chain_name).ok().and_then(|e| e.url().ok()).unwrap_or_default();
    Ok(Chain {
        name: chain_name,
        chainId: U256::from(chain_id),
        chainAlias: chain_alias,
        rpcUrl: rpc_url,
    })
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
