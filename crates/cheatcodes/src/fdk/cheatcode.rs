//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatcodesExecutor, CheatsCtxt, Fdk::*, Result, Vm::*};
use alloy_chains::Chain as AlloyChain;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use foundry_common::{block_on, fs, provider::get_http_provider};
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::{FoundryContextExt, FoundryTransaction, evm::FoundryEvmNetwork};
use revm::context::{ContextTr, JournalTr};
use std::path::PathBuf;

use super::artifact::{generate_artifact, save_artifact};

impl Cheatcode for fdkVersionCall {
    fn apply<FEN: FoundryEvmNetwork>(&self, _state: &mut Cheatcodes<FEN>) -> Result {
        Ok("0.1.0".abi_encode())
    }
}

impl Cheatcode for loadContract_0Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        let chain = get_chain_by_id(ccx.state, chain_id)?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_1Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, chainAlias } = self;
        let chain = get_chain_by_alias(ccx.state, chainAlias)?;
        load_contract(ccx.state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for loadContract_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        let chain = get_chain_by_id(state, chain_id)?;
        load_contract(state, chain, contractName).map(|address| address.abi_encode())
    }
}

impl Cheatcode for deployImmutable_0Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let chain_id = ccx.ecx.cfg().chain_id;
        let Self { artifact, constructorArgs } = self;

        // Get deployer address before deployment
        let deployer = ccx
            .state
            .get_prank(ccx.ecx.journal().depth())
            .map_or(ccx.caller, |prank| prank.new_caller);

        // Resolve artifact path from input (could be name, path, or full path:contract)
        let artifact_path = contract_name_to_artifact_path(ccx.state, artifact);

        let deploy_call = deployCode_1Call {
            artifactPath: artifact_path,
            constructorArgs: constructorArgs.clone(),
        };

        let address_bytes = deploy_call.apply_full(ccx, executor)?;
        let address = Address::abi_decode(&address_bytes)
            .map_err(|e| fmt_err!("failed to decode address: {}", e))?;

        // Extract contract name from artifact for deployment tracking
        let contract_name = extract_contract_name(artifact);
        save_deployment_address(
            ccx,
            chain_id,
            &contract_name,
            address,
            deployer,
            Some(constructorArgs),
            None,
        )?;

        Ok(address_bytes)
    }
}

impl Cheatcode for deployImmutable_1Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let chain_id = ccx.ecx.cfg().chain_id;
        let Self { artifact } = self;

        // Get deployer address before deployment
        let deployer = ccx
            .state
            .get_prank(ccx.ecx.journal().depth())
            .map_or(ccx.caller, |prank| prank.new_caller);

        // Resolve artifact path from input
        let artifact_path = contract_name_to_artifact_path(ccx.state, artifact);

        let deploy_call =
            deployCode_1Call { artifactPath: artifact_path, constructorArgs: Bytes::new() };

        let address_bytes = deploy_call.apply_full(ccx, executor)?;
        let address = Address::abi_decode(&address_bytes)
            .map_err(|e| fmt_err!("failed to decode address: {}", e))?;

        // Extract contract name from artifact for deployment tracking
        let contract_name = extract_contract_name(artifact);
        save_deployment_address(
            ccx,
            chain_id,
            &contract_name,
            address,
            deployer,
            Some(&Bytes::new()),
            None,
        )?;

        Ok(address_bytes)
    }
}

impl Cheatcode for deployLogicCall {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let chain_id = ccx.ecx.cfg().chain_id;
        let Self { artifact, constructorArgs } = self;

        let deployer = ccx
            .state
            .get_prank(ccx.ecx.journal().depth())
            .map_or(ccx.caller, |prank| prank.new_caller);

        // Resolve artifact path from input
        let artifact_path = contract_name_to_artifact_path(ccx.state, artifact);

        let deploy_call = deployCode_1Call {
            artifactPath: artifact_path,
            constructorArgs: constructorArgs.clone(),
        };

        let address_bytes = deploy_call.apply_full(ccx, executor)?;
        let address = Address::abi_decode(&address_bytes)
            .map_err(|e| fmt_err!("failed to decode address: {}", e))?;

        // Extract contract name and save as {contractName}Logic
        let contract_name = extract_contract_name(artifact);
        let logic_name = format!("{}Logic", contract_name);
        save_deployment_address(
            ccx,
            chain_id,
            &logic_name,
            address,
            deployer,
            Some(constructorArgs),
            None,
        )?;

        Ok(address_bytes)
    }
}

impl Cheatcode for deployProxy_0Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact, constructorArgs, initializationData, proxyAdmin } = self;
        deploy_proxy(
            ccx,
            executor,
            artifact,
            Some(constructorArgs),
            initializationData,
            Some(*proxyAdmin),
        )
    }
}

impl Cheatcode for deployProxy_1Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact, initializationData, proxyAdmin } = self;
        deploy_proxy(ccx, executor, artifact, None, initializationData, Some(*proxyAdmin))
    }
}

impl Cheatcode for deployProxy_2Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact, initializationData } = self;
        deploy_proxy(ccx, executor, artifact, None, initializationData, None)
    }
}

impl Cheatcode for deployProxy_3Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact } = self;
        deploy_proxy(ccx, executor, artifact, None, &Bytes::new(), None)
    }
}

impl Cheatcode for upgradeProxy_0Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact, constructorArgs, initializationData } = self;
        upgrade_proxy(ccx, executor, artifact, Some(constructorArgs), initializationData)
    }
}

impl Cheatcode for upgradeProxy_1Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact, initializationData } = self;
        upgrade_proxy(ccx, executor, artifact, None, initializationData)
    }
}

impl Cheatcode for upgradeProxy_2Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { artifact } = self;
        upgrade_proxy(ccx, executor, artifact, None, &Bytes::new())
    }
}

// ============================================================================
// Contract Configuration Management
// ============================================================================

impl Cheatcode for storeConfig_0Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, config } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        store_config(ccx.state, chain_id, contractName, config.clone())
    }
}

impl Cheatcode for storeConfig_1Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainAlias, config } = self;
        let chain_id = chain_alias_to_id(state, chainAlias)?;
        store_config(state, chain_id, contractName, config.clone())
    }
}

impl Cheatcode for storeConfig_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, state: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId, config } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        store_config(state, chain_id, contractName, config.clone())
    }
}

impl Cheatcode for loadConfig_0Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        load_config(ccx.state, chain_id, contractName)
    }
}

impl Cheatcode for loadConfig_1Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, ccx: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainAlias } = self;
        let chain_id = chain_alias_to_id(ccx, chainAlias)?;
        load_config(ccx, chain_id, contractName)
    }
}

impl Cheatcode for loadConfig_2Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, ccx: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        load_config(ccx, chain_id, contractName)
    }
}

impl Cheatcode for setContract_0Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, ccx: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainId, contractAddr } = self;
        ensure!(*chainId <= U256::from(u64::MAX), "chain ID must be less than 2^64");
        let chain_id = chainId.to::<u64>();
        set_contract(ccx, chain_id, contractName, *contractAddr)
    }
}

impl Cheatcode for setContract_1Call {
    fn apply<FEN: FoundryEvmNetwork>(&self, ccx: &mut Cheatcodes<FEN>) -> Result {
        let Self { contractName, chainAlias, contractAddr } = self;
        let chain_id = chain_alias_to_id(ccx, chainAlias)?;
        set_contract(ccx, chain_id, contractName, *contractAddr)
    }
}

impl Cheatcode for setContract_2Call {
    fn apply_stateful<FEN: FoundryEvmNetwork>(&self, ccx: &mut CheatsCtxt<'_, '_, FEN>) -> Result {
        let Self { contractName, contractAddr } = self;
        let chain_id = ccx.ecx.cfg().chain_id;
        set_contract(ccx.state, chain_id, contractName, *contractAddr)
    }
}

#[derive(serde::Deserialize)]
struct DeploymentArtifact {
    address: Address,
}

/// Extracts the logical contract name from an artifact input.
///
/// Handles all input formats:
/// - "contracts/tokens/ERC20.sol:MyToken" → "MyToken"
/// - "ERC20.sol:MyToken" → "MyToken"
/// - "contracts/tokens/ERC20.sol" → "ERC20"
/// - "ERC20.sol" → "ERC20"
/// - "MyToken" → "MyToken"
fn extract_contract_name(input: &str) -> String {
    // If input contains ':', the contract name is after the colon
    if let Some(colon_pos) = input.find(':') {
        return input[colon_pos + 1..].to_string();
    }

    // If input ends with .sol, extract the filename without extension
    if input.ends_with(".sol") {
        let path = std::path::Path::new(input);
        return path.file_stem().and_then(|s| s.to_str()).unwrap_or(input).to_string();
    }

    // Otherwise, it's just the contract name
    input.to_string()
}

/// Converts a contract name or path to a proper artifact path for use with deployCode.
///
/// This function uses the artifact metadata's `compilationTarget` to get the exact source path,
/// ensuring accuracy regardless of input format.
///
/// Input formats supported:
/// - Contract name only: "MyToken"
/// - Path with extension: "contracts/Token.sol"
/// - Path without extension: "contracts/Token"
/// - Full artifact path: "contracts/Token.sol:MyToken"
///
/// The function will look up the artifact and use its metadata.settings.compilationTarget
/// to construct the proper path, e.g.: "contracts/atia-shrine/AtiaShrine.sol:AtiaShrine"
fn contract_name_to_artifact_path<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    input: &str,
) -> String {
    // If input already contains `:` and has a path separator, it's likely already correct
    if input.contains(':') && (input.contains('/') || input.contains('\\')) {
        tracing::debug!(input, "artifact path already complete, using as-is");
        return input.to_string();
    }

    // Try to find the artifact from available artifacts using the input
    if let Ok(artifact_path) = resolve_artifact_path_from_metadata(state, input) {
        tracing::info!(input, artifact_path, "resolved artifact path from metadata");
        return artifact_path;
    }

    // Fallback to heuristic-based resolution if artifact lookup fails
    let artifact_path = fallback_artifact_path_resolution(state, input);
    tracing::warn!(
        input,
        artifact_path,
        "using fallback heuristic resolution (metadata not available)"
    );
    artifact_path
}

/// Resolves artifact path by reading the artifact JSON file from the `out` directory
/// and extracting the metadata.settings.compilationTarget
fn resolve_artifact_path_from_metadata<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    input: &str,
) -> Result<String> {
    // Extract potential contract name from input
    let contract_name = if let Some(colon_pos) = input.find(':') {
        &input[colon_pos + 1..]
    } else if input.ends_with(".sol") {
        std::path::Path::new(input).file_stem().and_then(|s| s.to_str()).unwrap_or(input)
    } else {
        // Remove path components and .sol extension if present
        std::path::Path::new(input).file_name().and_then(|s| s.to_str()).unwrap_or(input)
    };

    tracing::debug!(input, contract_name, "extracting contract name from input");

    // Get the `out` directory from foundry config
    let out_dir = &state.config.paths.artifacts;

    // Construct path to artifact JSON: out/<ContractName>.sol/<ContractName>.json
    let artifact_json_path =
        out_dir.join(format!("{}.sol", contract_name)).join(format!("{}.json", contract_name));

    tracing::debug!(
        path = %artifact_json_path.display(),
        "looking for artifact JSON file"
    );

    // Check if file exists
    if !artifact_json_path.exists() {
        tracing::debug!(
            path = %artifact_json_path.display(),
            "artifact file not found, will use fallback"
        );
        return Err(fmt_err!("artifact file not found: {}", artifact_json_path.display()));
    }

    // Read the JSON file
    let json_content = fs::read_to_string(&artifact_json_path)
        .map_err(|e| fmt_err!("failed to read artifact file: {}", e))?;

    // Parse JSON to extract metadata.settings.compilationTarget
    let artifact: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| fmt_err!("failed to parse artifact JSON: {}", e))?;

    // Navigate to metadata.settings.compilationTarget
    let compilation_target = artifact
        .get("metadata")
        .and_then(|m| m.get("settings"))
        .and_then(|s| s.get("compilationTarget"))
        .and_then(|ct| ct.as_object())
        .ok_or_else(|| fmt_err!("compilationTarget not found in artifact metadata"))?;

    tracing::debug!(?compilation_target, "found compilationTarget in metadata");

    // The compilationTarget is an object with one entry: { "path/to/file.sol": "ContractName" }
    // Extract the first (and should be only) entry
    let (source_path, target_contract_name) =
        compilation_target.iter().next().ok_or_else(|| fmt_err!("compilationTarget is empty"))?;

    let target_name = target_contract_name
        .as_str()
        .ok_or_else(|| fmt_err!("contract name in compilationTarget is not a string"))?;

    // Return the proper artifact path: "contracts/atia-shrine/AtiaShrine.sol:AtiaShrine"
    let result = format!("{}:{}", source_path, target_name);
    tracing::debug!(
        source_path,
        target_name,
        result,
        "extracted compilation target from artifact metadata"
    );
    Ok(result)
}

/// Fallback heuristic-based resolution when artifact metadata is not available
fn fallback_artifact_path_resolution<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    input: &str,
) -> String {
    let src_dir = state.config.paths.sources.file_name().and_then(|s| s.to_str()).unwrap_or("src");

    // If input contains `:`, handle path:contract format
    if let Some(colon_pos) = input.find(':') {
        let (path_part, _) = input.split_at(colon_pos);
        if path_part.contains('/') || path_part.contains('\\') {
            return input.to_string();
        }
        return format!("{}/{}", src_dir, input);
    }

    // Handle .sol files
    if input.ends_with(".sol") {
        let path = std::path::Path::new(input);
        let contract_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or(input);

        if input.contains('/') || input.contains('\\') {
            return format!("{}:{}", input, contract_name);
        }
        return format!("{}/{}:{}", src_dir, input, contract_name);
    }

    // Handle paths without .sol extension
    if input.contains('/') || input.contains('\\') {
        let path = std::path::Path::new(input);
        let contract_name = path.file_name().and_then(|s| s.to_str()).unwrap_or(input);
        return format!("{}.sol:{}", input, contract_name);
    }

    // Simple contract name
    format!("{}/{}.sol:{}", src_dir, input, input)
}

fn load_contract<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain: Chain,
    contract_name: &str,
) -> Result<Address> {
    let chain_id = chain.chainId.to::<u64>();
    if let Some(address) =
        state.fdk.address_book.get(&chain_id).and_then(|book| book.get(contract_name))
    {
        return Ok(address);
    }

    let chain_alias = chain.chainAlias.clone();

    // if it doesn't exist in address book, we need to resolve the address from the deployment files
    let address =
        resolve_deployment_address(state, &chain_alias, contract_name)?.ok_or_else(|| {
            fmt_err!("no deployment found for {contract_name} on chain {chain_alias}")
        })?;

    state.fdk.address_book.entry(chain_id).or_default().insert(contract_name, address);
    Ok(address)
}

fn save_deployment_address<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    chain_id: u64,
    contract_name: &str,
    address: Address,
    deployer: Address,
    constructor_args: Option<&Bytes>,
    value: Option<U256>,
) -> Result<()> {
    // save to address book
    ccx.state.fdk.address_book.entry(chain_id).or_default().insert(contract_name, address);

    // Get chain alias for deployment path
    let chain_alias = chain_id_to_alias(ccx.state, chain_id)?;

    // Generate full artifact with metadata
    let artifact =
        generate_artifact(ccx, contract_name, address, deployer, constructor_args, value)?;

    // Save artifact to deployments/{chain_alias}/{contract_name}.json
    save_artifact(ccx.state, &chain_alias, &artifact)?;

    Ok(())
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

pub(super) fn chain_id_to_alias<FEN: FoundryEvmNetwork>(
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

fn get_chain_by_alias<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_alias: &str,
) -> Result<Chain> {
    let chain_id = chain_alias_to_id(state, chain_alias)?;
    get_chain(state, chain_alias, chain_id)
}

fn get_chain_by_id<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
) -> Result<Chain> {
    let chain_alias = chain_id_to_alias(state, chain_id)?;

    get_chain(state, &chain_alias, chain_id)
}

fn get_chain<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_alias: &str,
    chain_id: u64,
) -> Result<Chain> {
    let chain = AlloyChain::from_id(chain_id);
    let chain_name = chain.to_string();
    let rpc_url =
        state.config.rpc_endpoint(&chain_name).ok().and_then(|e| e.url().ok()).unwrap_or_default();

    Ok(Chain {
        name: chain_name,
        chainId: U256::from(chain_id),
        chainAlias: chain_alias.to_string(),
        rpcUrl: rpc_url,
    })
}

fn resolve_deployment_address<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    chain_alias: &str,
    contract_name: &str,
) -> Result<Option<Address>> {
    let deployments_path = deployments_root(state);
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

fn set_contract<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
    contract_name: &str,
    contract_addr: Address,
) -> Result {
    state.fdk.address_book.entry(chain_id).or_default().insert(contract_name, contract_addr);
    Ok(Default::default())
}

fn deployments_root<FEN: FoundryEvmNetwork>(state: &Cheatcodes<FEN>) -> PathBuf {
    PathBuf::from(&state.config.fdk.deployments_root)
}

/// Deploy a TransparentUpgradeableProxy with a new logic contract.
fn deploy_proxy<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    artifact: &str,
    constructor_args: Option<&Bytes>,
    initialization_data: &Bytes,
    proxy_admin: Option<Address>,
) -> Result {
    let chain_id = ccx.ecx.cfg().chain_id;
    let deployer =
        ccx.state.get_prank(ccx.ecx.journal().depth()).map_or(ccx.caller, |prank| prank.new_caller);

    // Extract contract name from artifact input
    let contract_name = extract_contract_name(artifact);

    // 1. Deploy the logic contract
    // Resolve artifact path from input
    let artifact_path = contract_name_to_artifact_path(ccx.state, artifact);

    let logic_deploy = if let Some(args) = constructor_args {
        deployCode_1Call { artifactPath: artifact_path.clone(), constructorArgs: args.clone() }
    } else {
        deployCode_1Call { artifactPath: artifact_path, constructorArgs: Bytes::new() }
    };

    let logic_address_bytes = logic_deploy.apply_full(ccx, executor)?;
    let logic_address = Address::abi_decode(&logic_address_bytes)
        .map_err(|e| fmt_err!("failed to decode logic address: {}", e))?;

    // Save logic contract as {contractName}Logic
    let logic_name = format!("{}Logic", contract_name);
    save_deployment_address(
        ccx,
        chain_id,
        &logic_name,
        logic_address,
        deployer,
        constructor_args,
        None,
    )?;

    // 2. Get or deploy ProxyAdmin
    let proxy_admin_address = if let Some(admin) = proxy_admin {
        admin
    } else {
        get_or_deploy_proxy_admin(ccx, executor)?
    };

    // 3. Deploy TransparentUpgradeableProxy
    // TransparentUpgradeableProxy constructor: (address _logic, address initialOwner, bytes memory
    // _data)
    let proxy_constructor_args =
        (logic_address, proxy_admin_address, initialization_data.clone()).abi_encode();
    let proxy_constructor_bytes: Bytes = proxy_constructor_args.clone().into();

    let proxy_deploy = deployCode_1Call {
        artifactPath: ccx.state.config.fdk.transparent_proxy_path.clone(),
        constructorArgs: proxy_constructor_bytes.clone(),
    };

    let proxy_address_bytes = proxy_deploy.apply_full(ccx, executor)?;
    let proxy_address = Address::abi_decode(&proxy_address_bytes)
        .map_err(|e| fmt_err!("failed to decode proxy address: {}", e))?;

    // Save proxy as {contractName}Proxy
    let proxy_name = format!("{}Proxy", contract_name);
    save_deployment_address(
        ccx,
        chain_id,
        &proxy_name,
        proxy_address,
        deployer,
        Some(&proxy_constructor_bytes),
        None,
    )?;

    // Also save under the main contract name for easy loading
    save_deployment_address(ccx, chain_id, &contract_name, proxy_address, deployer, None, None)?;

    Ok(proxy_address_bytes)
}

/// Upgrade an existing proxy to a new logic contract.
///
/// This function deploys a new logic contract and then calls ProxyAdmin.upgradeAndCall
/// to point the existing proxy to the new logic implementation.
fn upgrade_proxy<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    artifact: &str,
    constructor_args: Option<&Bytes>,
    _initialization_data: &Bytes,
) -> Result {
    let chain_id = ccx.ecx.cfg().chain_id;
    let chain = get_chain_by_id(ccx.state, chain_id)?;
    let deployer =
        ccx.state.get_prank(ccx.ecx.journal().depth()).map_or(ccx.caller, |prank| prank.new_caller);

    // Extract contract name from artifact input
    let contract_name = extract_contract_name(artifact);

    // 1. Load the existing proxy address
    let proxy_name = format!("{}Proxy", contract_name);
    let _proxy_address = load_contract(ccx.state, chain.clone(), &proxy_name)?;

    // 2. Deploy the new logic contract
    // Resolve artifact path from input
    let artifact_path = contract_name_to_artifact_path(ccx.state, artifact);

    let logic_deploy = if let Some(args) = constructor_args {
        deployCode_1Call { artifactPath: artifact_path.clone(), constructorArgs: args.clone() }
    } else {
        deployCode_1Call { artifactPath: artifact_path, constructorArgs: Bytes::new() }
    };

    let new_logic_address_bytes = logic_deploy.apply_full(ccx, executor)?;
    let new_logic_address = Address::abi_decode(&new_logic_address_bytes)
        .map_err(|e| fmt_err!("failed to decode new logic address: {}", e))?;

    // Save new logic contract
    let logic_name = format!("{}Logic", contract_name);
    save_deployment_address(
        ccx,
        chain_id,
        &logic_name,
        new_logic_address,
        deployer,
        constructor_args,
        None,
    )?;

    // 3. Get ProxyAdmin address
    let proxy_admin_address = load_contract(ccx.state, chain, "ProxyAdmin")?;

    // 4. Execute ProxyAdmin.upgradeAndCall to upgrade the proxy
    execute_proxy_upgrade(
        ccx,
        executor,
        proxy_admin_address,
        _proxy_address,
        new_logic_address,
        _initialization_data,
    )?;

    Ok(new_logic_address_bytes)
}

/// Execute the proxy upgrade by calling ProxyAdmin.upgradeAndCall
fn execute_proxy_upgrade<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    proxy_admin: Address,
    proxy: Address,
    new_logic: Address,
    init_data: &Bytes,
) -> Result<()> {
    use super::multisig::execute_or_log_multisig;
    use alloy_primitives::keccak256;

    // Build the call to ProxyAdmin.upgradeAndCall(proxy, implementation, data)
    let selector = keccak256(b"upgradeAndCall(address,address,bytes)")[..4].to_vec();
    let params = (proxy, new_logic, init_data.clone()).abi_encode();
    let mut call_data_vec = selector;
    call_data_vec.extend_from_slice(&params);
    let call_data: Bytes = call_data_vec.into();

    let caller =
        ccx.state.get_prank(ccx.ecx.journal().depth()).map_or(ccx.caller, |prank| prank.new_caller);

    // Check if this should be handled as multisig
    let description = Some(format!(
        "Upgrade proxy {} to implementation {} via ProxyAdmin {}",
        proxy, new_logic, proxy_admin
    ));

    let is_multisig = execute_or_log_multisig(
        ccx,
        executor,
        caller,
        proxy_admin,
        call_data.clone(),
        U256::ZERO,
        description,
    )?;

    if is_multisig {
        return Ok(());
    }

    // Normal execution (non-multisig)
    use revm::primitives::TxKind;

    let mut tx_env = ccx.ecx.tx_clone();
    tx_env.set_caller(caller);
    tx_env.set_kind(TxKind::Call(proxy_admin));
    tx_env.set_data(call_data);
    tx_env.set_value(U256::ZERO);
    tx_env.set_gas_limit(ccx.gas_limit);

    // Execute the transaction using the executor
    executor
        .transact_from_tx_on_db(ccx.state, ccx.ecx, tx_env)
        .map_err(|e| fmt_err!("proxy upgrade failed: {e}"))?;

    Ok(())
}

/// Get or deploy the ProxyAdmin contract.
fn get_or_deploy_proxy_admin<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
) -> Result<Address> {
    let chain_id = ccx.ecx.cfg().chain_id;
    let chain = get_chain_by_id(ccx.state, chain_id)?;

    // Try to load existing ProxyAdmin
    if let Ok(address) = load_contract(ccx.state, chain, "ProxyAdmin") {
        return Ok(address);
    }

    // Deploy new ProxyAdmin
    let deployer =
        ccx.state.get_prank(ccx.ecx.journal().depth()).map_or(ccx.caller, |prank| prank.new_caller);

    // ProxyAdmin constructor takes: address initialOwner
    let constructor_args = deployer.abi_encode();

    let deploy_call = deployCode_1Call {
        artifactPath: ccx.state.config.fdk.proxy_admin_path.clone(),
        constructorArgs: constructor_args.clone().into(),
    };

    let address_bytes = deploy_call.apply_full(ccx, executor)?;
    let address = Address::abi_decode(&address_bytes)
        .map_err(|e| fmt_err!("failed to decode ProxyAdmin address: {}", e))?;

    // Save ProxyAdmin
    save_deployment_address(
        ccx,
        chain_id,
        "ProxyAdmin",
        address,
        deployer,
        Some(&constructor_args.into()),
        None,
    )?;

    Ok(address)
}

// ============================================================================
// Contract Configuration Helpers
// ============================================================================

/// Stores contract configuration for a given chain and contract name.
fn store_config<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
    contract_name: &str,
    config: Bytes,
) -> Result {
    state.fdk.contract_configs.entry(chain_id).or_default().insert(contract_name, config);
    Ok(Default::default())
}

/// Loads contract configuration for a given chain and contract name.
fn load_config<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    chain_id: u64,
    contract_name: &str,
) -> Result {
    let config = state
        .fdk
        .contract_configs
        .get(&chain_id)
        .and_then(|store| store.get(contract_name))
        .cloned();

    match config {
        Some(cfg) => Ok(cfg.abi_encode()),
        None => {
            let chain_alias = chain_id_to_alias(state, chain_id)?;
            Err(fmt_err!(
                "no config found for {contract_name} on chain {chain_alias} (ID: {chain_id})"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CheatsConfig;
    use foundry_compilers::ProjectPathsConfig;

    fn create_test_state() -> Cheatcodes<foundry_evm_core::evm::EthEvmNetwork> {
        let mut paths = ProjectPathsConfig::builder().build_with_root("./");
        paths.sources = std::path::PathBuf::from("src");

        let config = CheatsConfig { paths, ..Default::default() };

        Cheatcodes::new(std::sync::Arc::new(config))
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_1_full_path_with_contract() {
        let state = create_test_state();

        // Priority 1: Full path with contract - unchanged
        assert_eq!(
            contract_name_to_artifact_path(&state, "contracts/tokens/ERC20.sol:MyToken"),
            "contracts/tokens/ERC20.sol:MyToken"
        );

        assert_eq!(
            contract_name_to_artifact_path(
                &state,
                "lib/openzeppelin/contracts/token/ERC20/ERC20.sol:ERC20"
            ),
            "lib/openzeppelin/contracts/token/ERC20/ERC20.sol:ERC20"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_2_partial_path_with_contract() {
        let state = create_test_state();

        // Priority 2: Partial path with contract - prepend src dir
        assert_eq!(
            contract_name_to_artifact_path(&state, "ERC20.sol:MyToken"),
            "src/ERC20.sol:MyToken"
        );

        assert_eq!(
            contract_name_to_artifact_path(&state, "Token.sol:CustomToken"),
            "src/Token.sol:CustomToken"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_3_full_path_no_contract() {
        let state = create_test_state();

        // Priority 3: Full path without contract - append contract name from file
        assert_eq!(
            contract_name_to_artifact_path(&state, "contracts/tokens/ERC20.sol"),
            "contracts/tokens/ERC20.sol:ERC20"
        );

        assert_eq!(
            contract_name_to_artifact_path(&state, "lib/utils/SafeMath.sol"),
            "lib/utils/SafeMath.sol:SafeMath"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_4_partial_path_no_contract() {
        let state = create_test_state();

        // Priority 4: Partial path without contract - prepend src and append contract name
        assert_eq!(contract_name_to_artifact_path(&state, "ERC20.sol"), "src/ERC20.sol:ERC20");

        assert_eq!(
            contract_name_to_artifact_path(&state, "MyToken.sol"),
            "src/MyToken.sol:MyToken"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_5_contract_name_only() {
        let state = create_test_state();

        // Priority 5: Contract name only - construct full path
        assert_eq!(contract_name_to_artifact_path(&state, "MyToken"), "src/MyToken.sol:MyToken");

        assert_eq!(contract_name_to_artifact_path(&state, "ERC20"), "src/ERC20.sol:ERC20");

        assert_eq!(
            contract_name_to_artifact_path(&state, "ProxyAdmin"),
            "src/ProxyAdmin.sol:ProxyAdmin"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_with_custom_src_dir() {
        let mut state = create_test_state();
        std::sync::Arc::get_mut(&mut state.config).unwrap().paths.sources =
            PathBuf::from("contracts");

        // Should use "contracts" instead of "src"
        assert_eq!(
            contract_name_to_artifact_path(&state, "MyToken"),
            "contracts/MyToken.sol:MyToken"
        );

        assert_eq!(
            contract_name_to_artifact_path(&state, "ERC20.sol:Token"),
            "contracts/ERC20.sol:Token"
        );
    }

    #[test]
    fn test_contract_name_to_artifact_path_windows_paths() {
        let state = create_test_state();

        // Windows-style paths should also work
        assert_eq!(
            contract_name_to_artifact_path(&state, "contracts\\tokens\\ERC20.sol:MyToken"),
            "contracts\\tokens\\ERC20.sol:MyToken"
        );
    }

    #[test]
    fn test_extract_contract_name_from_full_path_with_contract() {
        assert_eq!(extract_contract_name("contracts/tokens/ERC20.sol:MyToken"), "MyToken");

        assert_eq!(extract_contract_name("lib/openzeppelin/ERC721.sol:CustomNFT"), "CustomNFT");
    }

    #[test]
    fn test_extract_contract_name_from_partial_path_with_contract() {
        assert_eq!(extract_contract_name("ERC20.sol:MyToken"), "MyToken");
    }

    #[test]
    fn test_extract_contract_name_from_full_path_no_contract() {
        assert_eq!(extract_contract_name("contracts/tokens/ERC20.sol"), "ERC20");

        assert_eq!(extract_contract_name("lib/utils/SafeMath.sol"), "SafeMath");
    }

    #[test]
    fn test_extract_contract_name_from_partial_path_no_contract() {
        assert_eq!(extract_contract_name("ERC20.sol"), "ERC20");

        assert_eq!(extract_contract_name("MyToken.sol"), "MyToken");
    }

    #[test]
    fn test_extract_contract_name_from_name_only() {
        assert_eq!(extract_contract_name("MyToken"), "MyToken");

        assert_eq!(extract_contract_name("ERC20"), "ERC20");
    }

    #[test]
    fn test_no_double_conversion_bug() {
        let state = create_test_state();

        // If user provides full path, it should NOT be modified
        let full_path = "contracts/tokens/ERC20.sol:MyToken";
        let artifact_path = contract_name_to_artifact_path(&state, full_path);
        assert_eq!(artifact_path, "contracts/tokens/ERC20.sol:MyToken");

        // Extract contract name from the ORIGINAL input (not the artifact_path)
        let contract_name = extract_contract_name(full_path);
        assert_eq!(contract_name, "MyToken");

        // If we mistakenly converted contract_name back to artifact path,
        // we'd get "src/MyToken.sol:MyToken" which is WRONG
        // This test ensures we don't do that
        let wrong_path = contract_name_to_artifact_path(&state, &contract_name);
        assert_eq!(wrong_path, "src/MyToken.sol:MyToken");
        assert_ne!(
            wrong_path, artifact_path,
            "Should NOT convert extracted name back to artifact path"
        );
    }

    #[test]
    fn test_artifact_path_with_subdirectories() {
        let state = create_test_state();

        // User provides path with subdirectories and .sol extension
        let result = contract_name_to_artifact_path(&state, "src/dex/UniswapV2.sol");
        assert_eq!(result, "src/dex/UniswapV2.sol:UniswapV2");

        // With different src directory
        let mut state2 = create_test_state();
        std::sync::Arc::get_mut(&mut state2.config).unwrap().paths.sources =
            PathBuf::from("contracts");
        let result2 = contract_name_to_artifact_path(&state2, "contracts/utils/Math.sol");
        assert_eq!(result2, "contracts/utils/Math.sol:Math");
    }

    #[test]
    fn test_contract_name_to_artifact_path_priority_5_path_without_extension() {
        let state = create_test_state();

        // Priority 5: Path without .sol extension
        assert_eq!(
            contract_name_to_artifact_path(&state, "src/dex/v1/UniswapV2"),
            "src/dex/v1/UniswapV2.sol:UniswapV2"
        );

        assert_eq!(
            contract_name_to_artifact_path(&state, "contracts/tokens/ERC20"),
            "contracts/tokens/ERC20.sol:ERC20"
        );

        assert_eq!(
            contract_name_to_artifact_path(&state, "lib/utils/SafeMath"),
            "lib/utils/SafeMath.sol:SafeMath"
        );
    }
}
