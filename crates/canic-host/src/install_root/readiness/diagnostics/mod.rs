//! Module: install_root::readiness::diagnostics
//!
//! Responsibility: render bounded diagnostics for root bootstrap readiness failures.
//! Does not own: bootstrap state, registry state, or readiness polling.
//! Boundary: queries maintained diagnostic endpoints without mutating canister state.

#[cfg(test)]
mod tests;

use crate::{
    icp::{self, LocalReplicaTarget, decode_json_result_response},
    install_root::commands::add_icp_network_target,
    registry::parse_registry_entries,
    release_set::icp_query_on_network,
    replica_query,
};
use std::path::Path;

use canic_core::{
    dto::{log::LogEntry, page::Page, state::BootstrapStatusResponse},
    protocol,
};

pub(super) fn print_bootstrap_failure_diagnostics(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
    status: &BootstrapStatusResponse,
    last_error: &str,
) {
    eprintln!(
        "root bootstrap reported failure during phase '{}' : {}",
        status.phase, last_error
    );
    print_root_diagnostics(icp_root, network, root_canister, local_replica);
}

pub(super) fn print_bootstrap_status(status: &BootstrapStatusResponse) {
    match status.last_error.as_deref() {
        Some(last_error) => println!(
            "Current bootstrap status: phase={} ready={} error={}",
            status.phase, status.ready, last_error
        ),
        None => println!(
            "Current bootstrap status: phase={} ready={}",
            status.phase, status.ready
        ),
    }
}

pub(super) fn print_current_registry_roles(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    if let Some(registry_roles) =
        current_registry_roles(icp_root, network, root_canister, local_replica)
    {
        println!("Current subnet registry roles:");
        println!("  {registry_roles}");
    }
}

pub(super) fn print_root_diagnostics(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_bootstrap_status");
    print_raw_call(
        icp_root,
        network,
        root_canister,
        local_replica,
        protocol::CANIC_BOOTSTRAP_STATUS,
    );
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_subnet_registry");
    print_raw_call(
        icp_root,
        network,
        root_canister,
        local_replica,
        "canic_subnet_registry",
    );
    eprintln!(
        "Diagnostic: icp canister -n {network} call {root_canister} canic_wasm_store_bootstrap_debug"
    );
    print_raw_call(
        icp_root,
        network,
        root_canister,
        local_replica,
        "canic_wasm_store_bootstrap_debug",
    );
    eprintln!(
        "Diagnostic: icp canister -n {network} call {root_canister} canic_wasm_store_overview"
    );
    print_raw_call(
        icp_root,
        network,
        root_canister,
        local_replica,
        "canic_wasm_store_overview",
    );
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_log");
    print_recent_root_logs(icp_root, network, root_canister, local_replica);
}

// Print recent structured root log entries without raw byte dumps.
fn print_recent_root_logs(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    let page_args = r"(null, null, null, record { limit = 8; offset = 0 })";
    let Ok(output) = icp_query_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        "canic_log",
        Some(page_args),
        Some("json"),
    ) else {
        return;
    };
    let Ok(page) = decode_json_result_response::<Page<LogEntry>>(&output) else {
        return;
    };

    if page.entries.is_empty() {
        println!("  <no runtime log entries>");
        return;
    }

    for entry in page.entries.iter().rev() {
        let topic_prefix = entry
            .topic
            .as_deref()
            .map_or_else(String::new, |topic| format!("[{topic}] "));
        println!(
            "  {:?} {topic_prefix}{}",
            entry.level,
            entry.message.replace('\n', "\\n")
        );
    }
}

fn current_registry_roles(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) -> Option<String> {
    if replica_query::should_use_local_replica_query(Some(network))
        && let Ok(roles) = replica_query::query_subnet_registry_roles_from_root(
            Some(network),
            root_canister,
            icp_root,
        )
    {
        return Some(render_registry_roles(&roles));
    }

    let output = icp_query_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        "canic_subnet_registry",
        None,
        Some("json"),
    )
    .ok()?;
    let entries = parse_registry_entries(&output).ok()?;
    let roles = entries
        .into_iter()
        .filter_map(|entry| entry.role)
        .collect::<Vec<_>>();
    Some(render_registry_roles(&roles))
}

fn render_registry_roles(roles: &[String]) -> String {
    if roles.is_empty() {
        "<empty>".to_string()
    } else {
        roles.join(", ")
    }
}

// Print one raw `icp canister call` result to stderr for diagnostics.
fn print_raw_call(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
    method: &str,
) {
    let mut command = icp::default_command_in(icp_root);
    command
        .arg("canister")
        .args(["call", root_canister, method, "()"]);
    add_icp_network_target(&mut command, network, local_replica);
    let _ = command.status();
}
