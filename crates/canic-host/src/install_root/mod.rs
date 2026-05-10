use crate::icp;
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, configured_fleet_name, configured_install_targets,
    configured_local_root_create_cycles, emit_root_release_set_manifest_with_config,
    icp_call_on_network, icp_root, load_root_release_set_manifest, resolve_artifact_root,
    resume_root_bootstrap, stage_root_release_set, workspace_root,
};
use canic_core::{cdk::types::Principal, protocol};
use config_selection::resolve_install_config_path;
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod config_selection;
mod state;

pub use config_selection::discover_canic_config_choices;
use state::{INSTALL_STATE_SCHEMA_VERSION, validate_fleet_name, write_install_state};
pub use state::{InstallState, read_named_fleet_install_state};

#[cfg(test)]
mod tests;

#[cfg(test)]
use config_selection::config_selection_error;
#[cfg(test)]
use state::{fleet_install_state_path, read_fleet_install_state};

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub root_canister: String,
    pub root_build_target: String,
    pub network: String,
    pub ready_timeout_seconds: u64,
    pub config_path: Option<String>,
    pub expected_fleet: Option<String>,
    pub interactive_config_selection: bool,
}

///
/// BootstrapStatusSnapshot
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct BootstrapStatusSnapshot {
    ready: bool,
    phase: String,
    last_error: Option<String>,
}

///
/// InstallTimingSummary
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InstallTimingSummary {
    create_canisters: Duration,
    build_all: Duration,
    emit_manifest: Duration,
    install_root: Duration,
    fund_root: Duration,
    stage_release_set: Duration,
    resume_bootstrap: Duration,
    wait_ready: Duration,
    finalize_root_funding: Duration,
}

const LOCAL_ICP_READY_TIMEOUT_SECONDS: u64 = 30;

/// Discover installable Canic config choices under the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    config_selection::discover_workspace_canic_config_choices(&workspace_root)
}

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    let config_path = resolve_install_config_path(
        &workspace_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )?;
    let fleet_name = configured_fleet_name(&config_path)?;
    validate_expected_fleet_name(options.expected_fleet.as_deref(), &fleet_name, &config_path)?;
    validate_fleet_name(&fleet_name)?;
    let total_started_at = Instant::now();
    let mut timings = InstallTimingSummary::default();

    println!(
        "Installing fleet {} against ICP_ENVIRONMENT={}",
        fleet_name, options.network
    );
    ensure_icp_environment_ready(&icp_root, &options.network)?;
    let create_started_at = Instant::now();
    if Principal::from_text(&options.root_canister).is_err() {
        let mut create = icp_canister_command_in_network(&icp_root);
        create.args(["create", &options.root_canister, "-q"]);
        add_local_root_create_cycles_arg(&mut create, &config_path, &options.network)?;
        add_icp_environment_target(&mut create, &options.network);
        run_command(&mut create)?;
    }
    timings.create_canisters = create_started_at.elapsed();

    let build_targets = configured_install_targets(&config_path, &options.root_build_target)?;
    let build_session_id = install_build_session_id();
    let build_started_at = Instant::now();
    run_canic_build_targets(
        &icp_root,
        &options.network,
        &build_targets,
        &build_session_id,
        &config_path,
    )?;
    timings.build_all = build_started_at.elapsed();

    let emit_manifest_started_at = Instant::now();
    let manifest_path = emit_root_release_set_manifest_with_config(
        &workspace_root,
        &icp_root,
        &options.network,
        &config_path,
    )?;
    timings.emit_manifest = emit_manifest_started_at.elapsed();

    let root_wasm = resolve_artifact_root(&icp_root, &options.network)?
        .join(&options.root_build_target)
        .join(format!("{}.wasm", options.root_build_target));
    let install_started_at = Instant::now();
    reinstall_root_wasm(
        &icp_root,
        &options.network,
        &options.root_canister,
        &root_wasm,
    )?;
    timings.install_root = install_started_at.elapsed();
    let fund_root_started_at = Instant::now();
    ensure_local_root_min_cycles(&icp_root, &options.network, &options.root_canister)?;
    timings.fund_root = fund_root_started_at.elapsed();

    let manifest = load_root_release_set_manifest(&manifest_path)?;
    let stage_started_at = Instant::now();
    stage_root_release_set(
        &icp_root,
        &options.network,
        &options.root_canister,
        &manifest,
    )?;
    timings.stage_release_set = stage_started_at.elapsed();
    let resume_started_at = Instant::now();
    resume_root_bootstrap(&options.network, &options.root_canister)?;
    timings.resume_bootstrap = resume_started_at.elapsed();
    let ready_started_at = Instant::now();
    let ready_result = wait_for_root_ready(
        &options.network,
        &options.root_canister,
        options.ready_timeout_seconds,
    );
    timings.wait_ready = ready_started_at.elapsed();
    if let Err(err) = ready_result {
        print_install_timing_summary(&timings, total_started_at.elapsed());
        return Err(err);
    }
    let finalize_funding_started_at = Instant::now();
    ensure_local_root_min_cycles(&icp_root, &options.network, &options.root_canister)?;
    timings.finalize_root_funding = finalize_funding_started_at.elapsed();

    print_install_timing_summary(&timings, total_started_at.elapsed());
    let state = build_install_state(
        &options,
        &workspace_root,
        &icp_root,
        &config_path,
        &manifest_path,
        &fleet_name,
    )?;
    let state_path = write_install_state(&icp_root, &options.network, &state)?;
    print_install_result_summary(&options.network, &state.fleet, &state_path);
    Ok(())
}

fn validate_expected_fleet_name(
    expected: Option<&str>,
    actual: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if expected == actual {
        return Ok(());
    }
    Err(format!(
        "install requested fleet {expected}, but {} declares [fleet].name = {actual:?}",
        config_path.display()
    )
    .into())
}

fn reinstall_root_wasm(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    root_wasm: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command_in_network(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", "(variant { Prime })"]);
    add_icp_environment_target(&mut install, network);
    run_command(&mut install)
}

// Build the persisted project-local install state from a completed install.
fn build_install_state(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    release_set_manifest_path: &Path,
    fleet_name: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    Ok(InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        fleet: fleet_name.to_string(),
        installed_at_unix_secs: current_unix_secs()?,
        network: options.network.clone(),
        root_target: options.root_canister.clone(),
        root_canister_id: resolve_root_canister_id(
            icp_root,
            &options.network,
            &options.root_canister,
        )?,
        root_build_target: options.root_build_target.clone(),
        workspace_root: workspace_root.display().to_string(),
        icp_root: icp_root.display().to_string(),
        config_path: config_path.display().to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    })
}

// Resolve the installed root id, accepting principal targets without a icp lookup.
fn resolve_root_canister_id(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    let mut command = icp_canister_command_in_network(icp_root);
    command.args(["status", root_canister, "-i"]);
    add_icp_environment_target(&mut command, network);
    Ok(run_command_stdout(&mut command)?.trim().to_string())
}

// Read the current host clock as a unix timestamp for install state.
fn current_unix_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

// Run one `canic build <canister>` call per configured local install target.
fn run_canic_build_targets(
    icp_root: &Path,
    network: &str,
    targets: &[String],
    build_session_id: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Build artifacts:");
    println!("{:<16} {:<18} {:>10}", "CANISTER", "PROGRESS", "ELAPSED");

    for (index, target) in targets.iter().enumerate() {
        let mut command = canic_build_target_command(icp_root, network, target, build_session_id);
        command.env("CANIC_CONFIG_PATH", config_path);
        let started_at = Instant::now();
        let output = command.output()?;
        let elapsed = started_at.elapsed();

        if !output.status.success() {
            return Err(format!(
                "canic build failed for {target}: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }

        println!(
            "{:<16} {:<18} {:>9.2}s",
            target,
            progress_bar(index + 1, targets.len(), 10),
            elapsed.as_secs_f64()
        );
    }

    println!();
    Ok(())
}

// Spawn one local `canic build <canister>` step without overriding the caller's
// selected build profile environment.
fn canic_build_target_command(
    _icp_root: &Path,
    network: &str,
    target: &str,
    build_session_id: &str,
) -> Command {
    let mut command = canic_command();
    command
        .env("CANIC_BUILD_CONTEXT_SESSION", build_session_id)
        .env("ICP_ENVIRONMENT", network)
        .args(["build", target]);
    command
}

// Re-enter the current Canic CLI binary so install builds use the same public
// build path operators can run directly.
fn canic_command() -> Command {
    std::env::current_exe().map_or_else(|_| Command::new("canic"), Command::new)
}

fn install_build_session_id() -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("install-root-{}-{unique}", std::process::id())
}

fn add_local_root_create_cycles_arg(
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

fn ensure_local_root_min_cycles(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
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
        "Topped up local root from {} to at least {}",
        crate::format::cycles_tc(current),
        crate::format::cycles_tc(LOCAL_ROOT_MIN_READY_CYCLES)
    );
    Ok(())
}

fn query_root_cycle_balance(
    network: &str,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let output = icp_call_on_network(
        network,
        root_canister,
        protocol::CANIC_CYCLE_BALANCE,
        None,
        None,
    )?;
    parse_cycle_balance_response(&output).ok_or_else(|| {
        format!(
            "could not parse {root_canister} {} response: {output}",
            protocol::CANIC_CYCLE_BALANCE
        )
        .into()
    })
}

fn parse_cycle_balance_response(output: &str) -> Option<u128> {
    output
        .split_once('=')
        .map_or(output, |(_, cycles)| cycles)
        .lines()
        .find_map(parse_leading_integer)
}

fn parse_leading_integer(line: &str) -> Option<u128> {
    let digits = line
        .trim_start_matches(|ch: char| ch == '(' || ch.is_whitespace())
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|digits| digits.parse::<u128>().ok())
}

fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return "[] 0/0".to_string();
    }

    let filled = current.saturating_mul(width).div_ceil(total);
    let filled = filled.min(width);
    format!(
        "[{}{}] {current}/{total}",
        "#".repeat(filled),
        " ".repeat(width - filled)
    )
}

// Ensure the requested replica is reachable before the local install flow begins.
fn ensure_icp_environment_ready(
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(network)? {
        return Ok(());
    }

    if network == "local" && local_icp_autostart_enabled() {
        println!("Local icp environment is not reachable; starting a clean local replica");
        let mut stop = icp_stop_command(icp_root);
        let _ = run_command_allow_failure(&mut stop)?;

        let mut start = icp_start_local_command(icp_root);
        run_command(&mut start)?;
        wait_for_icp_ping(
            network,
            Duration::from_secs(LOCAL_ICP_READY_TIMEOUT_SECONDS),
        )?;
        return Ok(());
    }

    Err(format!(
        "icp environment is not running for network '{network}'\nStart the target replica externally and rerun."
    )
    .into())
}

// Check whether `icp network ping <network>` currently succeeds.
fn icp_ping(network: &str) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(icp::default_command()
        .args(["network", "ping", network])
        .output()?
        .status
        .success())
}

// Return true when the local install flow should auto-start a clean local replica.
fn local_icp_autostart_enabled() -> bool {
    parse_local_icp_autostart(env::var("CANIC_AUTO_START_LOCAL_ICP").ok().as_deref())
}

fn parse_local_icp_autostart(value: Option<&str>) -> bool {
    !matches!(
        value.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("0" | "false" | "no" | "off")
    )
}

// Spawn one local `icp network stop` command for cleanup before a restart.
fn icp_stop_command(icp_root: &Path) -> Command {
    let mut command = icp_command_in_network(icp_root, "local");
    command.args(["network", "stop", "local"]);
    command
}

// Spawn one background `icp network start` command for local install/test flows.
fn icp_start_local_command(icp_root: &Path) -> Command {
    let mut command = icp_command_in_network(icp_root, "local");
    command.args(["network", "start", "local", "--background"]);
    command
}

// Poll `icp network ping` until the requested network responds or the timeout expires.
fn wait_for_icp_ping(network: &str, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if icp_ping(network)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }

    Err(format!(
        "icp environment did not become ready for network '{network}' within {}s",
        timeout.as_secs()
    )
    .into())
}

// Wait until root reports ready, printing periodic progress and diagnostics.
fn wait_for_root_ready(
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
            eprintln!(
                "root bootstrap reported failure during phase '{}' : {}",
                status.phase, last_error
            );
            eprintln!(
                "Diagnostic: icp canister -n {network} call {root_canister} canic_bootstrap_status"
            );
            print_raw_call(network, root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!(
                "Diagnostic: icp canister -n {network} call {root_canister} canic_subnet_registry"
            );
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
            return Err(format!(
                "root bootstrap failed during phase '{}' : {}",
                status.phase, last_error
            )
            .into());
        }

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
            eprintln!(
                "Diagnostic: icp canister -n {network} call {root_canister} canic_bootstrap_status"
            );
            print_raw_call(network, root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!(
                "Diagnostic: icp canister -n {network} call {root_canister} canic_subnet_registry"
            );
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
            return Err("root did not become ready".into());
        }

        if elapsed >= next_report {
            println!("Still waiting for {root_canister} canic_ready ({elapsed}s elapsed)");
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

// Accept both plain-bool and wrapped-result JSON shapes from `icp --output json`.
fn parse_root_ready_value(data: &Value) -> bool {
    matches!(data, Value::Bool(true))
        || matches!(data.get("Ok"), Some(Value::Bool(true)))
        || data
            .get("response_candid")
            .and_then(Value::as_str)
            .is_some_and(|value| value.trim() == "(true)")
}

fn parse_bootstrap_status_value(data: &Value) -> Option<BootstrapStatusSnapshot> {
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

fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{:<20} {:>10}", "phase", "elapsed");
    println!("{:<20} {:>10}", "--------------------", "----------");
    print_timing_row("create_canisters", timings.create_canisters);
    print_timing_row("build_all", timings.build_all);
    print_timing_row("emit_manifest", timings.emit_manifest);
    print_timing_row("install_root", timings.install_root);
    print_timing_row("fund_root", timings.fund_root);
    print_timing_row("stage_release_set", timings.stage_release_set);
    print_timing_row("resume_bootstrap", timings.resume_bootstrap);
    print_timing_row("wait_ready", timings.wait_ready);
    print_timing_row("finalize_root_funding", timings.finalize_root_funding);
    print_timing_row("total", total);
}

fn print_timing_row(label: &str, duration: Duration) {
    println!("{label:<20} {:>9.2}s", duration.as_secs_f64());
}

// Print the final install result as a compact whitespace table.
fn print_install_result_summary(network: &str, fleet: &str, state_path: &Path) {
    println!("Install result:");
    println!("{:<14} success", "status");
    println!("{:<14} {}", "fleet", fleet);
    println!("{:<14} {}", "install_state", state_path.display());
    println!(
        "{:<14} canic list {} --network {}",
        "smoke_check", fleet, network
    );
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

// Run one command and require a zero exit status.
fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    icp::run_status(command).map_err(Into::into)
}

// Run one command, require success, and return stdout.
fn run_command_stdout(command: &mut Command) -> Result<String, Box<dyn std::error::Error>> {
    icp::run_output(command).map_err(Into::into)
}

// Run one command and return its status without failing the caller on non-zero exit.
fn run_command_allow_failure(
    command: &mut Command,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    Ok(command.status()?)
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

// Build an icp command with the selected install environment exported
// for Rust build scripts that inspect ICP_ENVIRONMENT at compile time.
fn icp_command_on_network(network: &str) -> Command {
    let mut command = icp::default_command();
    command.env("ICP_ENVIRONMENT", network);
    command
}

// Build an icp command in one project directory with ICP_ENVIRONMENT applied.
fn icp_command_in_network(icp_root: &Path, network: &str) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.env("ICP_ENVIRONMENT", network);
    command
}

// Build an icp canister command in one project directory.
fn icp_canister_command_in_network(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

fn add_icp_environment_target(command: &mut Command, network: &str) {
    icp::add_target_args(command, Some(network), None);
}
