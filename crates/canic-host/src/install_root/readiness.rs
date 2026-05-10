use super::{icp_command_in_network, icp_command_on_network};
use crate::release_set::{icp_call_on_network, icp_root};
use canic_core::protocol;
use serde::Deserialize;
use serde_json::Value;
use std::{thread, time::Duration};

///
/// BootstrapStatusSnapshot
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct BootstrapStatusSnapshot {
    pub(super) ready: bool,
    pub(super) phase: String,
    pub(super) last_error: Option<String>,
}

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
    let output = icp_call_on_network(network, root_canister, "canic_ready", None, Some("json"))?;
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(parse_root_ready_value(&data))
}

// Return the current root bootstrap diagnostic state when the query is available.
fn root_bootstrap_status(
    network: &str,
    root_canister: &str,
) -> Result<Option<BootstrapStatusSnapshot>, Box<dyn std::error::Error>> {
    let output = match icp_call_on_network(
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

fn print_bootstrap_failure_diagnostics(
    network: &str,
    root_canister: &str,
    status: &BootstrapStatusSnapshot,
    last_error: &str,
) {
    eprintln!(
        "root bootstrap reported failure during phase '{}' : {}",
        status.phase, last_error
    );
    print_root_diagnostics(network, root_canister);
}

fn print_current_bootstrap_status(
    network: &str,
    root_canister: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(status) = root_bootstrap_status(network, root_canister)? {
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
    Ok(())
}

fn print_current_registry_roles(network: &str, root_canister: &str) {
    if let Ok(registry_json) = icp_call_on_network(
        network,
        root_canister,
        "canic_subnet_registry",
        None,
        Some("json"),
    ) {
        println!("Current subnet registry roles:");
        println!("  {}", registry_roles(&registry_json));
    }
}

fn print_root_diagnostics(network: &str, root_canister: &str) {
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_bootstrap_status");
    print_raw_call(network, root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_subnet_registry");
    print_raw_call(network, root_canister, "canic_subnet_registry");
    eprintln!(
        "Diagnostic: icp canister -n {network} call {root_canister} canic_wasm_store_bootstrap_debug"
    );
    print_raw_call(network, root_canister, "canic_wasm_store_bootstrap_debug");
    eprintln!(
        "Diagnostic: icp canister -n {network} call {root_canister} canic_wasm_store_overview"
    );
    print_raw_call(network, root_canister, "canic_wasm_store_overview");
    eprintln!("Diagnostic: icp canister -n {network} call {root_canister} canic_log");
    print_recent_root_logs(network, root_canister);
}

// Accept both plain-bool and wrapped-result JSON shapes from `icp --output json`.
pub(super) fn parse_root_ready_value(data: &Value) -> bool {
    matches!(data, Value::Bool(true))
        || matches!(data.get("Ok"), Some(Value::Bool(true)))
        || data
            .get("response_candid")
            .and_then(Value::as_str)
            .is_some_and(|value| value.trim() == "(true)")
}

pub(super) fn parse_bootstrap_status_value(data: &Value) -> Option<BootstrapStatusSnapshot> {
    serde_json::from_value::<BootstrapStatusSnapshot>(data.clone())
        .ok()
        .or_else(|| {
            data.get("Ok")
                .cloned()
                .and_then(|ok| serde_json::from_value::<BootstrapStatusSnapshot>(ok).ok())
        })
        .or_else(|| {
            data.get("response_candid")
                .and_then(Value::as_str)
                .and_then(parse_bootstrap_status_candid)
        })
}

fn parse_bootstrap_status_candid(candid: &str) -> Option<BootstrapStatusSnapshot> {
    let ready = if candid.contains("3_870_990_435 = true") || candid.contains("ready = true") {
        true
    } else if candid.contains("3_870_990_435 = false") || candid.contains("ready = false") {
        false
    } else {
        return None;
    };

    let phase = extract_candid_text_field(candid, "3_253_282_875")
        .or_else(|| extract_candid_text_field(candid, "phase"))
        .unwrap_or_else(|| {
            if ready {
                "ready".to_string()
            } else {
                "unknown".to_string()
            }
        });
    let last_error = extract_candid_text_field(candid, "89_620_959")
        .or_else(|| extract_candid_text_field(candid, "last_error"));

    Some(BootstrapStatusSnapshot {
        ready,
        phase,
        last_error,
    })
}

fn extract_candid_text_field(candid: &str, label: &str) -> Option<String> {
    let (_, tail) = candid.split_once(&format!("{label} = "))?;
    let tail = tail.trim_start();
    let quoted = tail
        .strip_prefix("opt \"")
        .or_else(|| tail.strip_prefix('"'))?;
    let mut value = String::new();
    let mut escaped = false;
    for ch in quoted.chars() {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(value);
        }
        value.push(ch);
    }
    None
}

// Print recent structured root log entries without raw byte dumps.
fn print_recent_root_logs(network: &str, root_canister: &str) {
    let page_args = r"(null, null, null, record { limit = 8; offset = 0 })";
    let Ok(logs_json) = icp_call_on_network(
        network,
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

// Render the current subnet registry roles from one JSON response.
fn registry_roles(registry_json: &str) -> String {
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

// Print one raw `icp canister call` result to stderr for diagnostics.
fn print_raw_call(network: &str, root_canister: &str, method: &str) {
    let mut command = icp_root().map_or_else(
        |_| icp_command_on_network(network),
        |root| icp_command_in_network(&root, network),
    );
    let _ = command
        .arg("canister")
        .args(["call", root_canister, method, "()", "-e", network])
        .status();
}
