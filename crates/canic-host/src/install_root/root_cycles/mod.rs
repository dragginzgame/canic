use super::commands::{add_icp_environment_target, icp_canister_command_in_network, run_command};
use crate::cycle_balance::query_cycle_balance;
use crate::format::cycles_tc;
use crate::icp::{IcpCli, LocalReplicaTarget};
use crate::release_set::{LOCAL_ROOT_MIN_READY_CYCLES, configured_local_root_create_cycles};
use std::{path::Path, process::Command};

pub(super) fn add_local_root_create_cycles_arg(
    command: &mut Command,
    config_path: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(());
    }

    let cycles = configured_local_root_create_cycles(config_path)?;
    command.args(["--cycles", &cycles.to_string()]);
    Ok(())
}

pub(super) fn ensure_local_root_min_cycles(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    phase: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(());
    }

    let current = query_root_cycle_balance(icp_root, network, root_canister, local_replica)?;
    if current >= LOCAL_ROOT_MIN_READY_CYCLES {
        return Ok(());
    }

    let amount = LOCAL_ROOT_MIN_READY_CYCLES.saturating_sub(current);
    let mut command = icp_canister_command_in_network(icp_root);
    command
        .args(["top-up", "--amount"])
        .arg(amount.to_string())
        .arg(root_canister);
    add_icp_environment_target(&mut command, network, local_replica);
    run_command(&mut command)?;
    println!(
        "Local root cycles ({phase}): topped up {} ({} -> {} target)",
        cycles_tc(amount),
        cycles_tc(current),
        cycles_tc(LOCAL_ROOT_MIN_READY_CYCLES)
    );
    Ok(())
}

fn query_root_cycle_balance(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<u128, Box<dyn std::error::Error>> {
    let icp = IcpCli::new("icp", Some(network.to_string()), None)
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned());
    query_cycle_balance(&icp, root_canister, network, Some(icp_root), None).map_err(Into::into)
}
