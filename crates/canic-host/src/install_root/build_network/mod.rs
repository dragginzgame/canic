use crate::{
    canister_build::{CanisterBuildProfile, WorkspaceBuildContext},
    icp::{self, LocalReplicaTarget},
    icp_config::resolve_icp_build_network_from_root,
    replica_query,
};
use canic_core::ids::BuildNetwork;
use std::path::Path;

use super::icp_context::InstallIcpContext;

pub(super) fn resolve_install_build_context(
    workspace_root: &Path,
    config_path: &Path,
    icp: &InstallIcpContext,
    role: &str,
    build_profile: Option<CanisterBuildProfile>,
) -> Result<WorkspaceBuildContext, Box<dyn std::error::Error>> {
    let profile = build_profile.unwrap_or(CanisterBuildProfile::Release);
    let icp_root = icp.root();
    let environment = icp.environment();
    let build_network = resolve_icp_build_network_from_root(icp_root, environment)?;

    Ok(WorkspaceBuildContext {
        role: role.to_string(),
        profile,
        environment: environment.to_string(),
        build_network,
        workspace_root: workspace_root.to_path_buf(),
        icp_root: icp_root.to_path_buf(),
        config_path: config_path.to_path_buf(),
        local_replica: local_replica_icp_target(icp, build_network),
        refresh_canonical_wasm_store_did: false,
        release_build_id: None,
    })
}

fn local_replica_icp_target(
    icp: &InstallIcpContext,
    build_network: BuildNetwork,
) -> Option<LocalReplicaTarget> {
    if build_network != BuildNetwork::Local {
        return None;
    }
    if icp_ping(icp).unwrap_or(false) {
        return None;
    }
    let icp_root = icp.root();
    let environment = icp.environment();
    let root_key = replica_query::local_replica_root_key_from_root(Some(environment), icp_root)
        .ok()
        .flatten()?;
    Some(LocalReplicaTarget {
        url: replica_query::local_replica_endpoint_from_root(Some(environment), icp_root),
        root_key,
    })
}

pub(super) fn ensure_icp_environment_ready(
    icp: &InstallIcpContext,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(icp)? {
        return Ok(());
    }
    let icp_root = icp.root();
    let environment = icp.environment();
    if resolve_icp_build_network_from_root(icp_root, environment)? == BuildNetwork::Local
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

fn icp_ping(icp: &InstallIcpContext) -> Result<bool, Box<dyn std::error::Error>> {
    let mut command = icp_ping_command(icp);
    Ok(icp::run_success(&mut command)?)
}

fn icp_ping_command(icp: &InstallIcpContext) -> std::process::Command {
    let mut command = icp.cli().command();
    command.args(["network", "ping"]);
    icp.add_target_args(&mut command);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icp_ping_selects_named_environment_without_treating_it_as_a_network() {
        let icp = InstallIcpContext::new("/opt/icp", Path::new("/workspace/app"), "staging");
        let command = icp_ping_command(&icp);

        assert_eq!(
            icp::command_display(&command),
            "/opt/icp --project-root-override /workspace/app network ping -e staging"
        );
    }
}
