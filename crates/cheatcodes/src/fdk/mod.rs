//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, Fdk::*, Result};
use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use foundry_common::fs;
use foundry_config::{fs_permissions::FsAccessKind, resolve::interpolate};
use foundry_evm_core::evm::FoundryEvmNetwork;
use revm::context::ContextTr;
use std::str::FromStr;
use toml::Value as TomlValue;

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}

impl Cheatcode for loadContractCall {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let toml = load_fdk_toml(ccx.state)?;
        let chain_id = ccx.ecx.cfg().chain_id as i64;
        let network_name = resolve_network_name(&toml, chain_id)?;
        let contract_address = resolve_contract_address(&toml, &network_name, contractName)?;
        Ok(contract_address.abi_encode())
    }
}

fn load_fdk_toml<FEN: FoundryEvmNetwork>(state: &Cheatcodes<FEN>) -> Result<TomlValue> {
    let path = state.config.ensure_path_allowed("fdk.toml", FsAccessKind::Read)?;
    let contents = fs::locked_read_to_string(&path)?;
    let resolved =
        interpolate(&contents).map_err(|e| fmt_err!("failed to resolve env var: {e}"))?;
    toml::from_str(&resolved).map_err(|e| fmt_err!("failed parsing TOML: {e}"))
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
    Address::from_str(address_str).map_err(|e| {
        fmt_err!("invalid address for {contract_name} in {network_name}: {e}")
    })
}