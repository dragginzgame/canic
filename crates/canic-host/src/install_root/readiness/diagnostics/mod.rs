use super::super::commands::{add_icp_environment_target, icp_command_in_network};
use crate::{icp::LocalReplicaTarget, release_set::icp_query_on_network, replica_query};
use canic_core::{dto::state::BootstrapStatusResponse, protocol};
use serde_json::Value;
use std::path::Path;

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
    let Ok(logs_json) = icp_query_on_network(
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
    let Ok(data) = serde_json::from_str::<Value>(&logs_json) else {
        return;
    };
    let entries = data
        .get("Ok")
        .and_then(|ok| ok.get("entries"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if entries.is_empty() {
        println!("  <no runtime log entries>");
        return;
    }

    for entry in entries.iter().rev() {
        let level = entry.get("level").and_then(Value::as_str).unwrap_or("Info");
        let topic = entry.get("topic").and_then(Value::as_str).unwrap_or("");
        let message = entry
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace('\n', "\\n");
        let topic_prefix = if topic.is_empty() {
            String::new()
        } else {
            format!("[{topic}] ")
        };
        println!("  {level} {topic_prefix}{message}");
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

    let registry_json = icp_query_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        "canic_subnet_registry",
        None,
        Some("json"),
    )
    .ok()?;
    Some(registry_roles_from_json(&registry_json))
}

// Render the current subnet registry roles from one JSON response.
fn registry_roles_from_json(registry_json: &str) -> String {
    serde_json::from_str::<Value>(registry_json)
        .ok()
        .and_then(|data| {
            data.get("Ok").and_then(Value::as_array).map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .get("role")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .map_or_else(
            || "<unavailable>".to_string(),
            |roles| {
                if roles.is_empty() {
                    "<empty>".to_string()
                } else {
                    roles.join(", ")
                }
            },
        )
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
    let mut command = icp_command_in_network(icp_root, network);
    command
        .arg("canister")
        .args(["call", root_canister, method, "()"]);
    add_icp_environment_target(&mut command, network, local_replica);
    let _ = command.status();
}

#[cfg(test)]
mod tests;
