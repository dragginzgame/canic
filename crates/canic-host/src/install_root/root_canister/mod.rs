use super::commands::{
    add_create_root_target, add_icp_environment_target, icp_canister_command,
    parse_created_canister_id, run_command_stdout,
};
use super::root_cycles::add_local_root_create_cycles_arg;
use crate::icp::{IcpCommandError, IcpDiagnostic, LocalReplicaTarget};
use canic_core::cdk::types::Principal;
use std::path::Path;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum RootCanisterIdError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("could not parse root canister id from ICP status JSON output: {output}")]
    InvalidOutput { output: String },
}

pub(super) fn ensure_root_canister_id(
    icp_root: &Path,
    environment: &str,
    root_canister: &str,
    config_path: &Path,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    match resolve_root_canister_id(icp_root, environment, root_canister, local_replica) {
        Ok(canister_id) => return Ok(canister_id),
        Err(RootCanisterIdError::Icp(err))
            if err.diagnostic() == Some(IcpDiagnostic::CanisterIdMissing) => {}
        Err(err) => return Err(err.into()),
    }

    let mut create = icp_canister_command(icp_root);
    add_create_root_target(&mut create, root_canister, local_replica);
    add_local_root_create_cycles_arg(&mut create, config_path, environment)?;
    add_icp_environment_target(&mut create, environment, local_replica);
    let output = run_command_stdout(&mut create)?;
    if let Some(canister_id) = parse_created_canister_id(&output) {
        return Ok(canister_id);
    }

    resolve_root_canister_id(icp_root, environment, root_canister, local_replica).map_err(|_| {
        format!(
            "created root canister target '{root_canister}', but ICP CLI still has no canister ID for environment '{environment}' under ICP root {}\nExpected project-local state under {}/.icp/{environment}. If another foreground replica is reachable, stop it and restart with `canic replica start --background` from this Canic project.",
            icp_root.display(),
            icp_root.display(),
        )
        .into()
    })
}

// Resolve the installed root id, accepting principal targets without a icp lookup.
fn resolve_root_canister_id(
    icp_root: &Path,
    environment: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<String, RootCanisterIdError> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    let mut command = icp_canister_command(icp_root);
    command.args(["status", root_canister, "--json"]);
    add_icp_environment_target(&mut command, environment, local_replica);
    let output = run_command_stdout(&mut command)?;
    parse_created_canister_id(&output).ok_or(RootCanisterIdError::InvalidOutput { output })
}
