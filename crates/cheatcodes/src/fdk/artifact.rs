//! Artifact generation for contract deployments.

use crate::{Cheatcodes, CheatsCtxt, Result};
use alloy_primitives::{Address, Bytes, U256, hex};
use foundry_common::{contracts::ContractData, fs};
use foundry_compilers::artifacts::StorageLayout;
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::evm::FoundryEvmNetwork;
use revm::context::{Block, ContextTr, JournalTr};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::Command, sync::Arc};

/// Full deployment artifact with all metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentArtifact {
    pub name: String,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub nonce: String,
    pub deployer: Address,
    pub chainid: String,
    pub block_number: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abi: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devdoc: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userdoc: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_layout: Option<Arc<StorageLayout>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytecode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployed_bytecode: Option<String>,
}

/// Generate a full deployment artifact with contract metadata.
pub fn generate_artifact<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    contract_name: &str,
    address: Address,
    deployer: Address,
    constructor_args: Option<&Bytes>,
    value: Option<U256>,
) -> Result<DeploymentArtifact> {
    let chain_id = ccx.ecx.cfg().chain_id;
    let block_number = ccx.ecx.block().number().to_string();
    let timestamp = ccx.ecx.block().timestamp().to_string();

    // Get nonce from deployer account
    let account = ccx.ecx.journal_mut().load_account(deployer)?;
    let nonce = account.data.info.nonce;

    // Try to load contract data from artifacts
    let contract_data = load_contract_data(ccx.state, contract_name).ok();

    // Extract source name for forge inspect (remove path and extension)
    let source_name = extract_source_name(contract_name);

    // Use forge inspect to get additional metadata
    let devdoc = forge_inspect(&source_name, "devdoc").ok();
    let userdoc = forge_inspect(&source_name, "userdoc").ok();
    let metadata = forge_inspect(&source_name, "metadata").ok();

    let artifact = DeploymentArtifact {
        name: contract_name.to_string(),
        address,
        args: constructor_args.map(|args| format!("0x{}", hex::encode(args))),
        value: value.map(|v| v.to_string()),
        nonce: nonce.to_string(),
        deployer,
        chainid: chain_id.to_string(),
        block_number,
        timestamp,
        abi: contract_data.as_ref().and_then(|data| serde_json::to_value(&data.abi).ok()),
        devdoc,
        userdoc,
        metadata,
        storage_layout: contract_data.as_ref().and_then(|data| data.storage_layout.clone()),
        bytecode: contract_data
            .as_ref()
            .and_then(|data| data.bytecode().map(|b| format!("0x{}", hex::encode(b)))),
        deployed_bytecode: contract_data
            .as_ref()
            .and_then(|data| data.deployed_bytecode().map(|b| format!("0x{}", hex::encode(b)))),
    };

    Ok(artifact)
}

/// Save deployment artifact to the deployments directory.
pub fn save_artifact<FEN: FoundryEvmNetwork>(
    state: &Cheatcodes<FEN>,
    chain_alias: &str,
    artifact: &DeploymentArtifact,
) -> Result<PathBuf> {
    let deployments_path = PathBuf::from(&state.config.fdk.deployments_root);
    let chain_path = deployments_path.join(chain_alias);

    // Ensure directory exists
    let chain_path_allowed = state.config.ensure_path_allowed(&chain_path, FsAccessKind::Write)?;
    std::fs::create_dir_all(&chain_path_allowed)
        .map_err(|e| fmt_err!("failed to create deployments directory: {e}"))?;

    // Write artifact file
    let artifact_file = chain_path.join(format!("{}.json", artifact.name));
    let artifact_file_allowed =
        state.config.ensure_path_allowed(artifact_file, FsAccessKind::Write)?;

    let json = serde_json::to_string_pretty(&artifact)
        .map_err(|e| fmt_err!("failed to serialize artifact: {e}"))?;

    fs::write(&artifact_file_allowed, json)
        .map_err(|e| fmt_err!("failed to write artifact file: {e}"))?;

    Ok(artifact_file_allowed)
}

/// Extract source name from contract path/name for forge inspect.
/// Mimics the bash script: basename + remove .json/.sol extension
fn extract_source_name(contract_name: &str) -> String {
    let path = std::path::Path::new(contract_name);
    let basename = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy();
    
    // Remove .json or .sol extension
    basename
        .strip_suffix(".json")
        .or_else(|| basename.strip_suffix(".sol"))
        .unwrap_or(&basename)
        .to_string()
}

/// Run forge inspect to get contract metadata.
/// Returns sanitized JSON value or None if the command fails.
fn forge_inspect(source_name: &str, field: &str) -> Result<serde_json::Value> {
    let output = Command::new("forge")
        .args(["inspect", source_name, field, "--json"])
        .output()
        .map_err(|e| fmt_err!("failed to execute forge inspect: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(fmt_err!("forge inspect {field} failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Sanitize: check if output is valid JSON, default to null if not
    serde_json::from_str(&stdout)
        .map_err(|e| fmt_err!("invalid JSON from forge inspect {field}: {e}"))
}

/// Load contract data from the artifact cache.
fn load_contract_data<'a, FEN: FoundryEvmNetwork>(
    state: &'a Cheatcodes<FEN>,
    contract_name: &str,
) -> Result<&'a ContractData> {
    // Parse contract name to handle various formats
    let mut parts = contract_name.split(':');
    let path_or_name = parts.next().unwrap();

    let contract_name_only = if path_or_name.contains('.') {
        // It's a path like "Counter.sol"
        parts.next().or(Some(
            std::path::Path::new(path_or_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path_or_name),
        ))
    } else {
        Some(path_or_name)
    };

    let Some(artifacts) = &state.config.available_artifacts else {
        return Err(fmt_err!("no artifacts available"));
    };

    // Find matching artifact
    let filtered: Vec<_> = artifacts
        .iter()
        .filter(|(id, _)| {
            if let Some(name) = contract_name_only {
                let id_name = id.name.split('.').next().unwrap();
                id_name == name
            } else {
                false
            }
        })
        .collect();

    match &filtered[..] {
        [] => Err(fmt_err!("no artifact found for {contract_name}")),
        [(_, artifact)] => Ok(*artifact),
        multiple => {
            // If multiple matches, try to find the best one
            // Prefer the one matching the running artifact version
            if let Some(running) = &state.config.running_artifact {
                if let Some((_, artifact)) =
                    multiple.iter().find(|(id, _)| id.version == running.version)
                {
                    return Ok(*artifact);
                }
            }
            // Otherwise just take the first one
            Ok(multiple[0].1)
        }
    }
}
