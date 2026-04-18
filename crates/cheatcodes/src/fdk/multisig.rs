//! Multisig wallet transaction handling

use crate::{Cheatcodes, CheatcodesExecutor, CheatsCtxt, Result};
use alloy_primitives::{Address, Bytes, U256};
use foundry_common::fs;
use foundry_config::fs_permissions::FsAccessKind;
use foundry_evm_core::{FoundryContextExt, FoundryTransaction, evm::FoundryEvmNetwork};
use revm::{context::ContextTr, primitives::TxKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A multisig transaction that needs to be proposed/approved/executed separately
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultisigTransaction {
    /// The multisig wallet address (from)
    pub from: Address,
    /// The target contract address (to)
    pub to: Address,
    /// The calldata
    #[serde(with = "hex_bytes")]
    pub data: Bytes,
    /// The value to send (in wei)
    pub value: String,
    /// Chain ID
    pub chain_id: u64,
    /// Description/context for this transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

mod hex_bytes {
    use alloy_primitives::Bytes;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", alloy_primitives::hex::encode(bytes)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix("0x").unwrap_or(&s);
        alloy_primitives::hex::decode(s).map(Into::into).map_err(serde::de::Error::custom)
    }
}

/// Check if the current sender is a multisig wallet
pub fn is_multisig_sender<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    sender: Address,
) -> bool {
    let chain_id = ccx.ecx.cfg().chain_id;
    let chain_alias = match super::cheatcode::chain_id_to_alias(ccx.state, chain_id) {
        Ok(alias) => alias,
        Err(_) => return false,
    };

    ccx.state
        .config
        .fdk
        .multisig_wallets
        .get(&chain_alias)
        .map(|&multisig_addr| multisig_addr == sender)
        .unwrap_or(false)
}

/// Log a multisig transaction and simulate it
pub fn handle_multisig_transaction<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    from: Address,
    to: Address,
    data: Bytes,
    value: alloy_primitives::U256,
    description: Option<String>,
) -> Result<()> {
    let chain_id = ccx.ecx.cfg().chain_id;

    // Create multisig transaction record
    let multisig_tx = MultisigTransaction {
        from,
        to,
        data: data.clone(),
        value: value.to_string(),
        chain_id,
        description,
    };

    // Log to file
    log_multisig_transaction(ccx.state, &multisig_tx)?;

    // Simulate the transaction using prank
    simulate_multisig_transaction(ccx, executor, from, to, data, value)?;

    Ok(())
}

/// Log multisig transaction to a JSON file
fn log_multisig_transaction<FEN: FoundryEvmNetwork>(
    state: &mut Cheatcodes<FEN>,
    tx: &MultisigTransaction,
) -> Result<()> {
    let chain_alias = super::cheatcode::chain_id_to_alias(state, tx.chain_id)?;

    // Create multisig directory
    let deployments_root = PathBuf::from(&state.config.fdk.deployments_root);
    let multisig_dir = deployments_root.join(&chain_alias).join("multisig");

    let multisig_dir_allowed =
        state.config.ensure_path_allowed(&multisig_dir, FsAccessKind::Write)?;
    std::fs::create_dir_all(&multisig_dir_allowed)
        .map_err(|e| fmt_err!("failed to create multisig directory: {e}"))?;

    // Generate filename with timestamp
    let timestamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let filename = format!("tx_{}.json", timestamp);
    let tx_file = multisig_dir.join(&filename);
    let tx_file_allowed = state.config.ensure_path_allowed(tx_file, FsAccessKind::Write)?;

    // Write transaction
    let json = serde_json::to_string_pretty(tx)
        .map_err(|e| fmt_err!("failed to serialize multisig tx: {e}"))?;

    fs::write(&tx_file_allowed, json).map_err(|e| fmt_err!("failed to write multisig tx: {e}"))?;

    // Log to console
    println!("\n══════════════════════════════════════════════════════════");
    println!("🔐 MULTISIG TRANSACTION DETECTED");
    println!("══════════════════════════════════════════════════════════");
    println!("From (Multisig): {}", tx.from);
    println!("To:              {}", tx.to);
    println!("Value:           {} wei", tx.value);
    println!("Calldata:        0x{}", alloy_primitives::hex::encode(&tx.data));
    if let Some(desc) = &tx.description {
        println!("Description:     {}", desc);
    }
    println!("Chain ID:        {}", tx.chain_id);
    println!("Saved to:        {}", tx_file_allowed.display());
    println!("══════════════════════════════════════════════════════════");
    println!("⚠️  This transaction was NOT broadcast.");
    println!("   You need to:");
    println!("   1. Propose this transaction to the multisig");
    println!("   2. Gather required approvals");
    println!("   3. Execute through the multisig wallet");
    println!("══════════════════════════════════════════════════════════\n");

    Ok(())
}

/// Simulate the multisig transaction using prank to verify it works
fn simulate_multisig_transaction<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    from: Address,
    to: Address,
    data: Bytes,
    value: U256,
) -> Result<()> {
    println!("🔍 Simulating transaction with prank...");

    // Build transaction for simulation
    let mut tx_env = ccx.ecx.tx_clone();
    tx_env.set_caller(from);
    tx_env.set_kind(TxKind::Call(to));
    tx_env.set_data(data);
    tx_env.set_value(value);
    tx_env.set_gas_limit(ccx.gas_limit);

    // Execute the transaction using the executor
    match executor.transact_from_tx_on_db(ccx.state, ccx.ecx, tx_env) {
        Ok(_) => {
            println!(
                "✅ Simulation successful! Transaction will work when executed from multisig."
            );
            Ok(())
        }
        Err(e) => {
            println!("❌ Simulation failed! Fix the transaction before proposing to multisig.");
            Err(fmt_err!("multisig transaction simulation failed: {e}"))
        }
    }
}

/// Execute a transaction or log it for multisig if sender is a multisig wallet.
/// Returns true if handled as multisig, false if executed normally.
pub fn execute_or_log_multisig<FEN: FoundryEvmNetwork>(
    ccx: &mut CheatsCtxt<'_, '_, FEN>,
    executor: &mut dyn CheatcodesExecutor<FEN>,
    caller: Address,
    target: Address,
    call_data: Bytes,
    value: U256,
    description: Option<String>,
) -> Result<bool> {
    if is_multisig_sender(ccx, caller) {
        handle_multisig_transaction(ccx, executor, caller, target, call_data, value, description)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
