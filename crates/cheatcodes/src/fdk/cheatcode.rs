//! Implementations of FDK cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatcodesExecutor, CheatsCtxt, Fdk::*, Result, Vm::*};
use alloy_chains::Chain as AlloyChain;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolValue;
use foundry_common::{block_on, fs, provider::get_http_provider};
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::{evm::FoundryEvmNetwork, FoundryContextExt, FoundryTransaction};
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

impl Cheatcode for deployImmutableCall {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let chain_id = ccx.ecx.cfg().chain_id;
        let Self { contractName, constructorArgs } = self;
        
        // Get deployer address before deployment
        let deployer = ccx.state
            .get_prank(ccx.ecx.journal().depth())
            .map_or(ccx.caller, |prank| prank.new_caller);

        let deploy_call = deployCode_1Call {
            artifactPath: contractName.clone(),
            constructorArgs: constructorArgs.clone(),
        };

        let address_bytes = deploy_call.apply_full(ccx, executor)?;
        let address = Address::from_slice(&address_bytes);
        
        save_deployment_address(
            ccx,
            chain_id,
            contractName,
            address,
            deployer,
            Some(constructorArgs),
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
        let Self { contractName, constructorArgs } = self;
        
        let deployer = ccx.state
            .get_prank(ccx.ecx.journal().depth())
            .map_or(ccx.caller, |prank| prank.new_caller);

        let deploy_call = deployCode_1Call {
            artifactPath: contractName.clone(),
            constructorArgs: constructorArgs.clone(),
        };

        let address_bytes = deploy_call.apply_full(ccx, executor)?;
        let address = Address::from_slice(&address_bytes);
        
        // Save as {contractName}Logic
        let logic_name = format!("{}Logic", contractName);
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
        let Self { contractName, constructorArgs, initializationData, proxyAdmin } = self;
        deploy_proxy(
            ccx,
            executor,
            contractName,
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
        let Self { contractName, initializationData, proxyAdmin } = self;
        deploy_proxy(
            ccx,
            executor,
            contractName,
            None,
            initializationData,
            Some(*proxyAdmin),
        )
    }
}

impl Cheatcode for deployProxy_2Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { contractName, initializationData } = self;
        deploy_proxy(ccx, executor, contractName, None, initializationData, None)
    }
}

impl Cheatcode for deployProxy_3Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { contractName } = self;
        deploy_proxy(ccx, executor, contractName, None, &Bytes::new(), None)
    }
}

impl Cheatcode for upgradeProxy_0Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { contractName, constructorArgs, initializationData } = self;
        upgrade_proxy(
            ccx,
            executor,
            contractName,
            Some(constructorArgs),
            initializationData,
        )
    }
}

impl Cheatcode for upgradeProxy_1Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { contractName, initializationData } = self;
        upgrade_proxy(ccx, executor, contractName, None, initializationData)
    }
}

impl Cheatcode for upgradeProxy_2Call {
    fn apply_full<FEN: FoundryEvmNetwork>(
        &self,
        ccx: &mut CheatsCtxt<'_, '_, FEN>,
        executor: &mut dyn CheatcodesExecutor<FEN>,
    ) -> Result {
        let Self { contractName } = self;
        upgrade_proxy(ccx, executor, contractName, None, &Bytes::new())
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
    ccx.state.fdk.address_book.entry(chain_id).or_default().insert(contract_name.to_string(), address);

    // Get chain alias for deployment path
    let chain_alias = chain_id_to_alias(ccx.state, chain_id)?;

    // Generate full artifact with metadata
    let artifact = generate_artifact(
        ccx,
        contract_name,
        address,
        deployer,
        constructor_args,
        value,
    )?;

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

fn deployments_root<FEN: FoundryEvmNetwork>(state: &Cheatcodes<FEN>) -> PathBuf {
    PathBuf::from(&state.config.fdk.deployments_root)
}

/// Deploy a TransparentUpgradeableProxy with a new logic contract.
fn deploy_proxy<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    contract_name: &str,
    constructor_args: Option<&Bytes>,
    initialization_data: &Bytes,
    proxy_admin: Option<Address>,
) -> Result {
    let chain_id = ccx.ecx.cfg().chain_id;
    let deployer = ccx.state
        .get_prank(ccx.ecx.journal().depth())
        .map_or(ccx.caller, |prank| prank.new_caller);

    // 1. Deploy the logic contract
    let logic_deploy = if let Some(args) = constructor_args {
        deployCode_1Call {
            artifactPath: contract_name.to_string(),
            constructorArgs: args.clone(),
        }
    } else {
        deployCode_1Call {
            artifactPath: contract_name.to_string(),
            constructorArgs: Bytes::new(),
        }
    };

    let logic_address_bytes = logic_deploy.apply_full(ccx, executor)?;
    let logic_address = Address::from_slice(&logic_address_bytes);

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
    // TransparentUpgradeableProxy constructor: (address _logic, address initialOwner, bytes memory _data)
    let proxy_constructor_args = (logic_address, proxy_admin_address, initialization_data.clone()).abi_encode();
    let proxy_constructor_bytes: Bytes = proxy_constructor_args.clone().into();

    let proxy_deploy = deployCode_1Call {
        artifactPath: ccx.state.config.fdk.transparent_proxy_path.clone(),
        constructorArgs: proxy_constructor_bytes.clone(),
    };

    let proxy_address_bytes = proxy_deploy.apply_full(ccx, executor)?;
    let proxy_address = Address::from_slice(&proxy_address_bytes);

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
    save_deployment_address(
        ccx,
        chain_id,
        contract_name,
        proxy_address,
        deployer,
        None,
        None,
    )?;

    Ok(proxy_address_bytes)
}

/// Upgrade an existing proxy to a new logic contract.
///
/// This function deploys a new logic contract and then calls ProxyAdmin.upgradeAndCall
/// to point the existing proxy to the new logic implementation.
fn upgrade_proxy<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    contract_name: &str,
    constructor_args: Option<&Bytes>,
    _initialization_data: &Bytes,
) -> Result {
    let chain_id = ccx.ecx.cfg().chain_id;
    let chain = get_chain_by_id(ccx.state, chain_id)?;
    let deployer = ccx.state
        .get_prank(ccx.ecx.journal().depth())
        .map_or(ccx.caller, |prank| prank.new_caller);

    // 1. Load the existing proxy address
    let proxy_name = format!("{}Proxy", contract_name);
    let _proxy_address = load_contract(ccx.state, chain.clone(), &proxy_name)?;

    // 2. Deploy the new logic contract
    let logic_deploy = if let Some(args) = constructor_args {
        deployCode_1Call {
            artifactPath: contract_name.to_string(),
            constructorArgs: args.clone(),
        }
    } else {
        deployCode_1Call {
            artifactPath: contract_name.to_string(),
            constructorArgs: Bytes::new(),
        }
    };

    let new_logic_address_bytes = logic_deploy.apply_full(ccx, executor)?;
    let new_logic_address = Address::from_slice(&new_logic_address_bytes);

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
    use alloy_primitives::keccak256;
    use super::multisig::execute_or_log_multisig;
    
    // Build the call to ProxyAdmin.upgradeAndCall(proxy, implementation, data)
    let selector = keccak256(b"upgradeAndCall(address,address,bytes)")[..4].to_vec();
    let params = (proxy, new_logic, init_data.clone()).abi_encode();
    let mut call_data_vec = selector;
    call_data_vec.extend_from_slice(&params);
    let call_data: Bytes = call_data_vec.into();

    let caller = ccx.state
        .get_prank(ccx.ecx.journal().depth())
        .map_or(ccx.caller, |prank| prank.new_caller);

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
    executor.transact_from_tx_on_db(ccx.state, ccx.ecx, tx_env)
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
    let deployer = ccx.state
        .get_prank(ccx.ecx.journal().depth())
        .map_or(ccx.caller, |prank| prank.new_caller);

    // ProxyAdmin constructor takes: address initialOwner
    let constructor_args = deployer.abi_encode();

    let deploy_call = deployCode_1Call {
        artifactPath: ccx.state.config.fdk.proxy_admin_path.clone(),
        constructorArgs: constructor_args.clone().into(),
    };

    let address_bytes = deploy_call.apply_full(ccx, executor)?;
    let address = Address::from_slice(&address_bytes);

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
