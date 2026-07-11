use super::commands::{
    add_create_root_target, add_icp_environment_target, icp_canister_command_in_network,
    is_missing_canister_id_error, parse_created_canister_id, run_command_stdout,
};
use super::root_cycles::add_local_root_create_cycles_arg;
use crate::icp::LocalReplicaTarget;
use canic_core::cdk::types::Principal;
use std::path::Path;

pub(super) fn ensure_root_canister_id(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    config_path: &Path,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    match resolve_root_canister_id(icp_root, network, root_canister, local_replica) {
        Ok(canister_id) => return Ok(canister_id),
        Err(err) if !is_missing_canister_id_error(&err.to_string()) => return Err(err),
        Err(_) => {}
    }

    let mut create = icp_canister_command_in_network(icp_root);
    add_create_root_target(&mut create, root_canister, local_replica);
    add_local_root_create_cycles_arg(&mut create, config_path, network)?;
    add_icp_environment_target(&mut create, network, local_replica);
    let output = run_command_stdout(&mut create)?;
    if let Some(canister_id) = parse_created_canister_id(&output) {
        return Ok(canister_id);
    }

    resolve_root_canister_id(icp_root, network, root_canister, local_replica).map_err(|_| {
        format!(
            "created root canister target '{root_canister}', but ICP CLI still has no canister ID for environment '{network}' under ICP root {}\nExpected project-local state under {}/.icp/{network}. If another foreground replica is reachable, stop it and restart with `canic replica start --background` from this Canic project.",
            icp_root.display(),
            icp_root.display(),
        )
        .into()
    })
}

// Resolve the installed root id, accepting principal targets without a icp lookup.
fn resolve_root_canister_id(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    let mut command = icp_canister_command_in_network(icp_root);
    command.args(["status", root_canister, "--json"]);
    add_icp_environment_target(&mut command, network, local_replica);
    let output = run_command_stdout(&mut command)?;
    parse_created_canister_id(&output).ok_or_else(|| {
        format!("could not parse root canister id from ICP status JSON output: {output}").into()
    })
}
