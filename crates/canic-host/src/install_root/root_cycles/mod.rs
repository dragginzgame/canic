use super::commands::{add_icp_environment_target, icp_canister_command_in_network, run_command};
use crate::format::cycles_tc;
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, configured_local_root_create_cycles, icp_query_on_network,
};
use crate::response_parse::parse_cycle_balance_response;
use canic_core::protocol;
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
) -> Result<(), Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(());
    }

    let current = query_root_cycle_balance(network, root_canister)?;
    if current >= LOCAL_ROOT_MIN_READY_CYCLES {
        return Ok(());
    }

    let amount = LOCAL_ROOT_MIN_READY_CYCLES.saturating_sub(current);
    let mut command = icp_canister_command_in_network(icp_root);
    command
        .args(["top-up", "--amount"])
        .arg(amount.to_string())
        .arg(root_canister);
    add_icp_environment_target(&mut command, network);
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
    network: &str,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let output = icp_query_on_network(
        network,
        root_canister,
        protocol::CANIC_CYCLE_BALANCE,
        None,
        Some("json"),
    )?;
    parse_cycle_balance_response(&output).ok_or_else(|| {
        format!(
            "could not parse {root_canister} {} response: {output}",
            protocol::CANIC_CYCLE_BALANCE
        )
        .into()
    })
}
