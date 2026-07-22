use super::commands::{add_icp_environment_target, icp_canister_command, run_command};
use crate::cycle_balance::query_cycle_balance;
use crate::format::cycles_tc;
use crate::icp::{IcpCli, LocalReplicaTarget};
use crate::release_set::{FleetConfigSnapshot, LOCAL_ROOT_MIN_READY_CYCLES};
use std::{path::Path, process::Command};

pub(super) fn add_local_root_create_cycles_arg(
    command: &mut Command,
    config_path: &Path,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if environment != "local" {
        return Ok(());
    }

    let cycles = FleetConfigSnapshot::load(config_path)?.local_root_create_cycles();
    command.args(["--cycles", &cycles.to_string()]);
    Ok(())
}

pub(super) fn ensure_local_root_min_cycles(
    icp_root: &Path,
    environment: &str,
    root_canister: &str,
    phase: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    if environment != "local" {
        return Ok(());
    }

    let current = query_root_cycle_balance(icp_root, environment, root_canister, local_replica)?;
    if current >= LOCAL_ROOT_MIN_READY_CYCLES {
        return Ok(());
    }

    let amount = LOCAL_ROOT_MIN_READY_CYCLES.saturating_sub(current);
    let mut command = icp_canister_command(icp_root);
    command
        .args(["top-up", "--amount"])
        .arg(amount.to_string())
        .arg(root_canister);
    add_icp_environment_target(&mut command, environment, local_replica);
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
    environment: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<u128, Box<dyn std::error::Error>> {
    let icp = IcpCli::new("icp", Some(environment.to_string()))
        .with_cwd(icp_root)
        .with_local_replica(local_replica.cloned());
    query_cycle_balance(&icp, root_canister, environment, Some(icp_root), None).map_err(Into::into)
}
