//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result};
use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use foundry_common::fs;
use foundry_config::{fs_permissions::FsAccessKind, resolve::interpolate};
use foundry_evm_core::evm::FoundryEvmNetwork;
use revm::context::ContextTr;
use std::{env, path::PathBuf, str::FromStr};
use toml::Value as TomlValue;

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}

impl Cheatcode for loadContractCall {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        if let Some(address) = ccx
            .state
            .fdk_address_book
            .get(&chain_id)
            .and_then(|entries| entries.get(contractName))
            .copied()
        {
            return Ok(address.abi_encode());
        }

        let toml = load_fdk_toml(ccx.state)?;
        let chain_id_i64 = chain_id as i64;
        let network_name = resolve_network_name(&toml, chain_id_i64)?;
        let contract_address = if let Some(address) =
            resolve_deployment_address(ccx.state, &toml, &network_name, contractName)?
        {
            address
        } else {
            resolve_contract_address(&toml, &network_name, contractName)?
        };
        ccx.state
            .fdk_address_book
            .entry(chain_id)
            .or_default()
            .insert(contractName.clone(), contract_address);
        Ok(contract_address.abi_encode())
    }
}

fn load_fdk_toml<FEN: FoundryEvmNetwork>(state: &Cheatcodes<FEN>) -> Result<TomlValue> {
    let path = state.config.ensure_path_allowed(fdk_toml_path(), FsAccessKind::Read)?;
    let contents = fs::locked_read_to_string(&path)?;
    let resolved =
        interpolate(&contents).map_err(|e| fmt_err!("failed to resolve env var: {e}"))?;
    toml::from_str(&resolved).map_err(|e| fmt_err!("failed parsing TOML: {e}"))
}

fn fdk_toml_path() -> PathBuf {
    env::var("FDK_TOML_PATH").map(PathBuf::from).unwrap_or_else(|_| "fdk.toml".into())
}

fn resolve_network_name(toml: &TomlValue, chain_id: i64) -> Result<String> {
    let networks = toml
        .get("networks")
        .and_then(|value| value.as_table())
        .ok_or_else(|| fmt_err!("missing [networks] section in fdk.toml"))?;
    for (name, value) in networks {
        let Some(id) = value.get("chain_id").and_then(|v| v.as_integer()) else { continue };
        if id == chain_id {
            return Ok(name.clone());
        }
    }
    Err(fmt_err!("no network found for chain_id {chain_id} in fdk.toml"))
}

fn resolve_contract_address(
    toml: &TomlValue,
    network_name: &str,
    contract_name: &str,
) -> Result<Address> {
    let contracts = toml
        .get("contracts")
        .and_then(|value| value.as_table())
        .ok_or_else(|| fmt_err!("missing [contracts] section in fdk.toml"))?;
    let network_contracts = contracts
        .get(network_name)
        .and_then(|value| value.as_table())
        .ok_or_else(|| fmt_err!("missing [contracts.{network_name}] section in fdk.toml"))?;
    let address_str = network_contracts
        .get(contract_name)
        .and_then(|value| value.as_str())
        .ok_or_else(|| fmt_err!("missing contracts.{network_name}.{contract_name} in fdk.toml"))?;
    Address::from_str(address_str)
        .map_err(|e| fmt_err!("invalid address for {contract_name} in {network_name}: {e}"))
}

fn resolve_deployments_path(toml: &TomlValue) -> Result<PathBuf> {
    let project = toml
        .get("project")
        .and_then(|value| value.as_table())
        .ok_or_else(|| fmt_err!("missing [project] section in fdk.toml"))?;
    let path =
        project.get("deployments_path").and_then(|value| value.as_str()).unwrap_or("deployments");
    Ok(PathBuf::from(path))
}

#[derive(serde::Deserialize)]
struct DeploymentArtifact {
    address: Address,
}

fn resolve_deployment_address<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    toml: &TomlValue,
    network_name: &str,
    contract_name: &str,
) -> Result<Option<Address>> {
    let deployments_path = resolve_deployments_path(toml)?;
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
