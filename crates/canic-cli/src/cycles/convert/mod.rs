mod operation;
mod options;
mod pending;
mod response;

use crate::cycles::{CyclesCommandError, wallet::ResolvedCanisterTarget};
use canic_contracts::protocol::cycle_refill as wire;
use canic_core::cdk::utils::hash::hex_bytes;
use canic_host::{
    fleet_ensure::resolve_current_fleet, icp_config::resolve_current_canic_icp_root,
    protocol_binding::resolve_current_registry_wire_schema,
};
use operation::{
    OperationIdSource, current_unix_nanos, mark_pending_operation_completed,
    pending_operation_input, resolve_operation_id, write_generated_operation_id_notice,
};
use options::ConvertOptions;
use response::{validate_icp_refill_command_response, validate_icp_refill_status_response};
use std::ffi::OsString;

fn extend_hash_part(bytes: &mut Vec<u8>, part: &[u8]) {
    bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
    bytes.extend_from_slice(part);
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = ConvertOptions::parse(args)?;
    run_options(&options)
}

pub(super) fn usage() -> String {
    options::usage()
}

fn run_options(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&root, &options.target.environment, &options.fleet)?;
    let selected_root = options.root_principal.to_text();
    if !current
        .topology
        .fleet_subnet_root_canister_ids
        .contains(&selected_root)
    {
        return Err(CyclesCommandError::UnknownTarget {
            fleet: options.fleet.clone(),
            target: selected_root,
        });
    }
    let root_entry = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == selected_root)
        .ok_or_else(|| CyclesCommandError::UnknownTarget {
            fleet: options.fleet.clone(),
            target: selected_root.clone(),
        })?;
    let root_target = ResolvedCanisterTarget {
        canister_id: selected_root,
        role: Some("root".to_string()),
    };
    let icp = options.target.icp_cli(&root);

    resolve_current_registry_wire_schema(&root, &current, root_entry)
        .map_err(|error| CyclesCommandError::Usage(error.to_string()))?;

    let now_nanos = current_unix_nanos();
    let pending_input = pending_operation_input(&root, options, &root_target, now_nanos);
    let (operation_id, operation_id_source, pending_operation_key) = resolve_operation_id(
        options.operation_id,
        &pending_input,
        options.dry_run,
        now_nanos,
    )?;
    let request = canic_contracts::dto::icp_refill::CycleRefillInput {
        operation_id,
        source_subaccount: options.source_subaccount,
        amount_e8s: options.amount_e8s,
    };
    if options.dry_run {
        let request_bytes = ic_wire::native::encode_request(
            wire::execute::METHOD_ID,
            &request,
            wire::REQUEST_LIMITS,
        )
        .map_err(canic_host::icp::IcpWireQueryError::from)?;
        write_dry_run(
            options,
            &root_target,
            operation_id,
            operation_id_source,
            &request_bytes,
        );
        return Ok(());
    }

    write_generated_operation_id_notice(options.json, operation_id, operation_id_source);

    let response = icp.canister_update_wire(
        &root_target.canister_id,
        wire::execute::METHOD,
        wire::execute::METHOD_ID,
        &request,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    validate_icp_refill_command_response(response, operation_id)?;
    let status = icp.canister_query_wire(
        &root_target.canister_id,
        wire::status::METHOD,
        wire::status::METHOD_ID,
        &operation_id,
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    let response = validate_icp_refill_status_response(status, operation_id)?;
    if !response.is_resumable() {
        mark_pending_operation_completed(&root, pending_operation_key.as_deref(), operation_id)?;
    }
    let output = response.render(options.json);
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn write_dry_run(
    options: &ConvertOptions,
    root: &ResolvedCanisterTarget,
    operation_id: [u8; 32],
    operation_id_source: OperationIdSource,
    request_bytes: &[u8],
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "fleet": options.fleet,
                "selected_root_principal": options.root_principal,
                "root_canister_id": root.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "amount_e8s": options.amount_e8s,
                "operation_id": hex_bytes(operation_id),
                "dry_run": true,
                "method": canic_contracts::protocol::cycle_refill::execute::METHOD,
                "request_hex": hex_bytes(request_bytes),
            })
        );
    } else {
        write_generated_operation_id_notice(options.json, operation_id, operation_id_source);
        println!(
            "canister={}\nmethod={}\nrequest_hex={}",
            root.canister_id,
            canic_contracts::protocol::cycle_refill::execute::METHOD,
            hex_bytes(request_bytes)
        );
    }
}
