use self::diagnostics::{
    print_bootstrap_failure_diagnostics, print_bootstrap_status, print_current_registry_roles,
    print_root_diagnostics,
};
pub(super) use self::parsing::parse_bootstrap_status_value;
use crate::{
    canister_ready::query_canister_ready,
    icp::IcpCli,
    release_set::{icp_query_on_network, icp_root},
    replica_query,
};
use canic_core::{dto::state::BootstrapStatusResponse, protocol};
use serde_json::Value;
use std::{thread, time::Duration};

mod diagnostics;
mod parsing;

// Wait until root reports ready, printing periodic progress and diagnostics.
pub(super) fn wait_for_root_ready(
    network: &str,
    root_canister: &str,
    timeout_seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let mut next_report = 0_u64;

    println!("Waiting for {root_canister} to report canic_ready (timeout {timeout_seconds}s)");

    loop {
        if root_ready(network, root_canister)? {
            println!(
                "{root_canister} reported canic_ready after {}s",
                start.elapsed().as_secs()
            );
            return Ok(());
        }

        if let Some(status) = root_bootstrap_status(network, root_canister)?
            && let Some(last_error) = status.last_error.as_deref()
        {
            print_bootstrap_failure_diagnostics(network, root_canister, &status, last_error);
            return Err(format!(
                "root bootstrap failed during phase '{}' : {}",
                status.phase, last_error
            )
            .into());
        }

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
            print_root_diagnostics(network, root_canister);
            return Err("root did not become ready".into());
        }

        if elapsed >= next_report {
            println!("Still waiting for {root_canister} canic_ready ({elapsed}s elapsed)");
            print_current_bootstrap_status(network, root_canister)?;
            print_current_registry_roles(network, root_canister);
            next_report = elapsed + 5;
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// Return true once root reports `canic_ready == true`.
fn root_ready(network: &str, root_canister: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let root = icp_root()?;
    let icp = IcpCli::new("icp", Some(network.to_string()), None).with_cwd(&root);
    query_canister_ready(&icp, root_canister, network, Some(&root), None).map_err(Into::into)
}

// Return the current root bootstrap diagnostic state when the query is available.
fn root_bootstrap_status(
    network: &str,
    root_canister: &str,
) -> Result<Option<BootstrapStatusResponse>, Box<dyn std::error::Error>> {
    if let Some(status) = local_bootstrap_status(network, root_canister) {
        return Ok(Some(status));
    }

    let output = match icp_query_on_network(
        network,
        root_canister,
        protocol::CANIC_BOOTSTRAP_STATUS,
        None,
        Some("json"),
    ) {
        Ok(output) => output,
        Err(err) => {
            let message = err.to_string();
            if message.contains("has no query method")
                || message.contains("method not found")
                || message.contains("Canister has no query method")
            {
                return Ok(None);
            }
            return Err(err);
        }
    };
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(parse_bootstrap_status_value(&data))
}

fn local_bootstrap_status(network: &str, root_canister: &str) -> Option<BootstrapStatusResponse> {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return None;
    }
    let root = icp_root().ok()?;
    replica_query::query_bootstrap_status_from_root(Some(network), root_canister, &root).ok()
}

fn print_current_bootstrap_status(
    network: &str,
    root_canister: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(status) = root_bootstrap_status(network, root_canister)? {
        print_bootstrap_status(&status);
    }
    Ok(())
}
