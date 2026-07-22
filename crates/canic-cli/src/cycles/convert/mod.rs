mod operation;
mod options;
mod pending;
mod request;
mod response;

use crate::{
    cycles::{
        CyclesCommandError,
        wallet::{ResolvedCanisterTarget, resolve_deployment},
    },
    support::candid::role_candid_path,
};
use canic_core::cdk::utils::hash::hex_bytes;
use canic_host::{icp::IcpCli, icp_config::resolve_current_canic_icp_root};
use operation::{
    OperationIdSource, current_unix_nanos, mark_pending_operation_completed,
    pending_operation_input, resolve_operation_id, write_generated_operation_id_notice,
};
use options::ConvertOptions;
use request::icp_refill_request_arg;
use response::decode_icp_refill_response;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

const ICP_REFILL_METHOD: &str = "canic_icp_refill";

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = ConvertOptions::parse(args)?;
    run_options(&options)
}

pub(super) fn usage() -> String {
    options::usage()
}

fn run_options(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let root_target = ResolvedCanisterTarget {
        canister_id: installed.state.root_canister_id,
        role: Some("root".to_string()),
    };
    let icp = IcpCli::new(
        &options.target.icp,
        Some(options.target.environment.clone()),
    )
    .with_cwd(&root);

    let now_nanos = current_unix_nanos();
    let pending_input = pending_operation_input(&root, options, &root_target, now_nanos);
    let (operation_id, operation_id_source, pending_operation_key) = resolve_operation_id(
        options.operation_id,
        &pending_input,
        options.dry_run,
        now_nanos,
    )?;
    let request_arg = icp_refill_request_arg(
        operation_id,
        options.source_subaccount,
        options.amount_e8s,
        options.dry_run,
    );
    let root_candid_path =
        canister_target_candid_path(&root, &options.target.environment, &root_target);
    if options.dry_run {
        let command = icp.canister_call_arg_output_display_with_candid(
            &root_target.canister_id,
            ICP_REFILL_METHOD,
            &request_arg,
            Some("hex"),
            root_candid_path.as_deref(),
        );
        write_dry_run(
            options,
            &root_target,
            operation_id,
            operation_id_source,
            &command,
        );
        return Ok(());
    }

    write_generated_operation_id_notice(options.json, operation_id, operation_id_source);

    let output = icp
        .canister_call_arg_output_with_candid(
            &root_target.canister_id,
            ICP_REFILL_METHOD,
            &request_arg,
            Some("hex"),
            root_candid_path.as_deref(),
        )
        .map_err(CyclesCommandError::from)?;
    let response = decode_icp_refill_response(&output, operation_id)?;
    if !response.is_resumable() {
        mark_pending_operation_completed(&root, pending_operation_key.as_deref(), operation_id)?;
    }
    let output = response.render(options.json);
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn canister_target_candid_path(
    root: &Path,
    environment: &str,
    target: &ResolvedCanisterTarget,
) -> Option<PathBuf> {
    role_candid_path(Some(root), environment, target.role.as_deref()?)
}

fn write_dry_run(
    options: &ConvertOptions,
    root: &ResolvedCanisterTarget,
    operation_id: [u8; 32],
    operation_id_source: OperationIdSource,
    command: &str,
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "deployment": options.deployment,
                "root_canister_id": root.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "amount_e8s": options.amount_e8s,
                "operation_id": hex_bytes(operation_id),
                "dry_run": true,
                "command": command,
            })
        );
    } else {
        write_generated_operation_id_notice(options.json, operation_id, operation_id_source);
        println!("{command}");
    }
}
