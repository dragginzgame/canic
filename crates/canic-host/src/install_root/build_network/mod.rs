use crate::{
    canister_build::{CanisterBuildProfile, WorkspaceBuildContext},
    icp::{self, LocalReplicaTarget},
    icp_config::resolve_icp_build_network_from_root,
    replica_query,
};
use std::path::Path;

pub(super) fn resolve_install_build_context(
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    environment: &str,
    role: &str,
    build_profile: Option<CanisterBuildProfile>,
) -> Result<WorkspaceBuildContext, Box<dyn std::error::Error>> {
    let profile = build_profile.unwrap_or(CanisterBuildProfile::Release);
    let build_network = resolve_icp_build_network_from_root(icp_root, environment)?;

    Ok(WorkspaceBuildContext {
        role: role.to_string(),
        profile,
        environment: environment.to_string(),
        build_network,
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: config_path.to_path_buf(),
        local_replica: local_replica_icp_target(environment, icp_root),
        refresh_canonical_wasm_store_did: false,
    })
}

pub(super) fn local_replica_icp_target(
    environment: &str,
    icp_root: &Path,
) -> Option<LocalReplicaTarget> {
    if !replica_query::should_use_local_replica_query(Some(environment)) {
        return None;
    }
    if icp_ping(icp_root, environment).unwrap_or(false) {
        return None;
    }
    let root_key = replica_query::local_replica_root_key_from_root(Some(environment), icp_root)
        .ok()
        .flatten()?;
    Some(LocalReplicaTarget {
        url: replica_query::local_replica_endpoint_from_root(Some(environment), icp_root),
        root_key,
    })
}

pub(super) fn ensure_icp_environment_ready(
    icp_root: &Path,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(icp_root, environment)? {
        return Ok(());
    }
    if replica_query::should_use_local_replica_query(Some(environment))
        && replica_query::local_replica_status_reachable_from_root(Some(environment), icp_root)
    {
        println!(
            "Replica reachable via HTTP status endpoint even though ICP CLI reports environment '{environment}' stopped; continuing from ICP root {}.",
            icp_root.display()
        );
        return Ok(());
    }

    Err(format!(
        "ICP environment '{environment}' is not running\nStart the target replica in another terminal with `canic replica start` and rerun."
    )
    .into())
}

fn icp_ping(icp_root: &Path, environment: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut command = icp_ping_command(icp_root, environment);
    Ok(icp::run_success(&mut command)?)
}

fn icp_ping_command(icp_root: &Path, environment: &str) -> std::process::Command {
    let mut command = icp::default_command_in(icp_root);
    command.args(["network", "ping"]);
    icp::add_target_args(&mut command, Some(environment), None);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icp_ping_selects_named_environment_without_treating_it_as_a_network() {
        let command = icp_ping_command(Path::new("/workspace/app"), "staging");

        assert_eq!(
            icp::command_display(&command),
            "icp --project-root-override /workspace/app network ping -e staging"
        );
    }
}
