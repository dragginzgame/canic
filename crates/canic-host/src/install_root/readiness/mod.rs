//! Module: install_root::readiness
//!
//! Responsibility: wait for a newly installed root to complete bootstrap.
//! Does not own: bootstrap state, endpoint DTOs, or diagnostic rendering.
//! Boundary: polls typed readiness and bootstrap responses, then delegates diagnostics.

mod diagnostics;

use self::diagnostics::{
    print_bootstrap_failure_diagnostics, print_bootstrap_status, print_current_registry_roles,
    print_root_diagnostics,
};
use crate::{
    canister_ready::query_canister_ready,
    icp::{IcpCli, LocalReplicaTarget, decode_json_response},
    release_set::icp_query_on_network,
    replica_query,
};
use std::{path::Path, thread, time::Duration};

use canic_core::{dto::state::BootstrapStatusResponse, protocol};

// Wait until root reports ready, printing periodic progress and diagnostics.
pub(super) fn wait_for_root_ready(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    timeout_seconds: u64,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let mut next_report = 0_u64;

    println!("Waiting for {root_canister} to report canic_ready (timeout {timeout_seconds}s)");

    loop {
        if root_ready(icp_root, network, root_canister, local_replica)? {
            println!(
                "{root_canister} reported canic_ready after {}s",
                start.elapsed().as_secs()
            );
            return Ok(());
        }

        let status = root_bootstrap_status(icp_root, network, root_canister, local_replica)?;
        if let Some(last_error) = status.last_error.as_deref() {
            print_bootstrap_failure_diagnostics(
                icp_root,
                network,
                root_canister,
                local_replica,
                &status,
                last_error,
            );
            return Err(format!(
                "root bootstrap failed during phase '{}' : {}",
                status.phase, last_error
            )
            .into());
        }

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
            print_root_diagnostics(icp_root, network, root_canister, local_replica);
            return Err("root did not become ready".into());
        }

        if elapsed >= next_report {
            println!("Still waiting for {root_canister} canic_ready ({elapsed}s elapsed)");
            print_current_bootstrap_status(icp_root, network, root_canister, local_replica)?;
            print_current_registry_roles(icp_root, network, root_canister, local_replica);
            next_report = elapsed + 5;
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// Return true once root reports `canic_ready == true`.
fn root_ready(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let icp = IcpCli::new("icp", Some(network.to_string()), None)
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned());
    query_canister_ready(&icp, root_canister, network, Some(icp_root), None).map_err(Into::into)
}

// Return the current root bootstrap diagnostic state when the query is available.
fn root_bootstrap_status(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<BootstrapStatusResponse, Box<dyn std::error::Error>> {
    if let Some(status) = local_bootstrap_status(icp_root, network, root_canister) {
        return Ok(status);
    }

    let output = icp_query_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        protocol::CANIC_BOOTSTRAP_STATUS,
        None,
        Some("json"),
    )?;
    decode_json_response(&output).map_err(Into::into)
}

fn local_bootstrap_status(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
) -> Option<BootstrapStatusResponse> {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return None;
    }
    replica_query::query_bootstrap_status_from_root(Some(network), root_canister, icp_root).ok()
}

fn print_current_bootstrap_status(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = root_bootstrap_status(icp_root, network, root_canister, local_replica)?;
    print_bootstrap_status(&status);
    Ok(())
}
