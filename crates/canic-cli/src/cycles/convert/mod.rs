mod operation;
mod options;
mod pending;
mod request;

use crate::{
    cycles::{
        CyclesCommandError,
        wallet::{
            ResolvedCanisterTarget, cycles_icp_error, resolve_canister_target, resolve_deployment,
            target_label,
        },
    },
    support::candid::role_candid_path,
};
use canic_core::cdk::utils::hash::hex_bytes;
use canic_host::{format::cycles_tc, icp::IcpCli, icp_config::resolve_current_canic_icp_root};
use operation::{
    OperationIdSource, current_unix_nanos, mark_pending_operation_completed,
    pending_operation_input, resolve_operation_id, write_generated_operation_id_notice,
};
use options::ConvertOptions;
use request::{
    FABRICATE_MODE_MESSAGE, icp_refill_request_arg, json_output_arg, provisional_top_up_arg,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

const ICP_REFILL_METHOD: &str = "canic_icp_refill";
const MANAGEMENT_CANISTER_ID: &str = "aaaaa-aa";
const PROVISIONAL_TOP_UP_METHOD: &str = "provisional_top_up_canister";

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = ConvertOptions::parse(args)?;
    run_options(&options)
}

pub(super) fn usage() -> String {
    options::usage()
}

fn run_options(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let target = resolve_canister_target(
        &options.deployment,
        &options.canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let icp = IcpCli::new(
        &options.target.icp,
        None,
        Some(options.target.network.clone()),
    )
    .with_cwd(&root);

    if options.fabricate {
        return run_fabricate(options, &icp, &target);
    }

    let source_selector = required_source_selector(options)?;
    let source = resolve_canister_target(
        &options.deployment,
        source_selector,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let amount_e8s = required_amount_e8s(options)?;
    let now_nanos = current_unix_nanos();
    let pending_input =
        pending_operation_input(&root, options, &source, &target, amount_e8s, now_nanos);
    let (operation_id, operation_id_source, pending_operation_key) = resolve_operation_id(
        options.operation_id,
        &pending_input,
        options.dry_run,
        now_nanos,
    )?;
    let request_arg = icp_refill_request_arg(
        operation_id,
        &source.canister_id,
        options.source_subaccount,
        &target.canister_id,
        amount_e8s,
        options.dry_run,
    );
    let source_candid_path = canister_target_candid_path(&root, &options.target.network, &source);
    let command = icp.canister_call_arg_output_display_with_candid(
        &source.canister_id,
        ICP_REFILL_METHOD,
        &request_arg,
        json_output_arg(options.json),
        source_candid_path.as_deref(),
    );

    if options.dry_run {
        write_canister_dry_run(
            options,
            &source,
            &target,
            operation_id,
            operation_id_source,
            amount_e8s,
            &command,
        );
        return Ok(());
    }

    write_generated_operation_id_notice(options.json, operation_id, operation_id_source);

    let output = icp
        .canister_call_arg_output_with_candid(
            &source.canister_id,
            ICP_REFILL_METHOD,
            &request_arg,
            json_output_arg(options.json),
            source_candid_path.as_deref(),
        )
        .map_err(cycles_icp_error)?;
    mark_pending_operation_completed(&root, pending_operation_key.as_deref(), operation_id);
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "canister",
                "deployment": options.deployment,
                "source": source.role.as_deref(),
                "source_canister_id": source.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_e8s": amount_e8s,
                "operation_id": hex_bytes(operation_id),
                "dry_run": false,
                "command": command,
                "icp_output": output,
            })
        );
    } else if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn canister_target_candid_path(
    root: &Path,
    network: &str,
    target: &ResolvedCanisterTarget,
) -> Option<PathBuf> {
    role_candid_path(Some(root), network, target.role.as_deref()?)
}

fn run_fabricate(
    options: &ConvertOptions,
    icp: &IcpCli,
    target: &ResolvedCanisterTarget,
) -> Result<(), CyclesCommandError> {
    ensure_fabricate_local_network(&options.target.network)?;
    let amount_cycles = required_cycles_amount(options)?;
    let request_arg = provisional_top_up_arg(&target.canister_id, amount_cycles);
    let command = icp.canister_call_arg_output_display(
        MANAGEMENT_CANISTER_ID,
        PROVISIONAL_TOP_UP_METHOD,
        &request_arg,
        json_output_arg(options.json),
    );

    if options.dry_run {
        write_fabricate_dry_run(options, target, amount_cycles, &command);
        return Ok(());
    }

    let output = icp
        .canister_call_arg_output(
            MANAGEMENT_CANISTER_ID,
            PROVISIONAL_TOP_UP_METHOD,
            &request_arg,
            json_output_arg(options.json),
        )
        .map_err(cycles_icp_error)?;
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "fabricate",
                "message": FABRICATE_MODE_MESSAGE,
                "deployment": options.deployment,
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_cycles": amount_cycles.to_string(),
                "amount_display": cycles_tc(amount_cycles),
                "dry_run": false,
                "command": command,
                "icp_output": output,
            })
        );
    } else {
        println!(
            "Fabricated {} for {}.",
            cycles_tc(amount_cycles),
            target_label(target.role.as_deref(), &target.canister_id)
        );
    }
    Ok(())
}

fn ensure_fabricate_local_network(network: &str) -> Result<(), CyclesCommandError> {
    if network == "local" {
        Ok(())
    } else {
        Err(CyclesCommandError::FabricationRequiresLocal {
            network: network.to_string(),
        })
    }
}

fn required_source_selector(options: &ConvertOptions) -> Result<&str, CyclesCommandError> {
    options
        .source_canister_or_role
        .as_deref()
        .ok_or_else(|| CyclesCommandError::Usage(usage()))
}

fn required_amount_e8s(options: &ConvertOptions) -> Result<u64, CyclesCommandError> {
    options
        .amount_e8s
        .ok_or_else(|| CyclesCommandError::Usage(usage()))
}

fn required_cycles_amount(options: &ConvertOptions) -> Result<u128, CyclesCommandError> {
    options
        .cycles_amount
        .ok_or_else(|| CyclesCommandError::Usage(usage()))
}

fn write_canister_dry_run(
    options: &ConvertOptions,
    source: &ResolvedCanisterTarget,
    target: &ResolvedCanisterTarget,
    operation_id: [u8; 32],
    operation_id_source: OperationIdSource,
    amount_e8s: u64,
    command: &str,
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "canister",
                "deployment": options.deployment,
                "source": source.role.as_deref(),
                "source_canister_id": source.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_e8s": amount_e8s,
                "operation_id": hex_bytes(operation_id),
                "dry_run": true,
                "command": command,
            })
        );
    } else {
        println!("mode=canister");
        write_generated_operation_id_notice(options.json, operation_id, operation_id_source);
        println!("{command}");
    }
}

fn write_fabricate_dry_run(
    options: &ConvertOptions,
    target: &ResolvedCanisterTarget,
    amount_cycles: u128,
    command: &str,
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "fabricate",
                "message": FABRICATE_MODE_MESSAGE,
                "deployment": options.deployment,
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_cycles": amount_cycles.to_string(),
                "amount_display": cycles_tc(amount_cycles),
                "dry_run": true,
                "command": command,
            })
        );
    } else {
        println!("{FABRICATE_MODE_MESSAGE}");
        println!("{command}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabricate_requires_local_network() {
        std::assert_matches!(
            ensure_fabricate_local_network("ic"),
            Err(CyclesCommandError::FabricationRequiresLocal { .. })
        );
        assert!(ensure_fabricate_local_network("local").is_ok());
    }
}
