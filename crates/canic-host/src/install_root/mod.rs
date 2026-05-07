use crate::dfx;
use crate::release_set::{
    configured_fleet_name, configured_install_targets, dfx_call, dfx_root,
    emit_root_release_set_manifest_with_config, load_root_release_set_manifest,
    resume_root_bootstrap, stage_root_release_set, workspace_root,
};
use canic_core::{cdk::types::Principal, protocol};
use config_selection::resolve_install_config_path;
use serde::Deserialize;
use serde_json::Value;
use std::{
    env,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod config_selection;
mod state;

pub use state::{
    FleetSummary, InstallState, list_current_fleets, read_current_install_state,
    read_current_or_fleet_install_state, select_current_fleet,
};
use state::{INSTALL_STATE_SCHEMA_VERSION, validate_fleet_name, write_install_state};

#[cfg(test)]
mod tests;

#[cfg(test)]
use config_selection::{config_selection_error, discover_canic_config_choices};
#[cfg(test)]
use state::{
    current_fleet_path, fleet_install_state_path, list_fleets, read_fleet_install_state,
    read_install_state,
};

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
    fabricate_cycles: Duration,
    install_root: Duration,
    stage_release_set: Duration,
    resume_bootstrap: Duration,
    wait_ready: Duration,
}

const LOCAL_ROOT_TARGET_CYCLES: u128 = 9_000_000_000_000_000;
const LOCAL_DFX_READY_TIMEOUT_SECONDS: u64 = 30;

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    let config_path = resolve_install_config_path(
        &workspace_root,
        options.config_path.as_deref(),
        options.interactive_config_selection,
    )?;
    let fleet_name = configured_fleet_name(&config_path)?;
    validate_fleet_name(&fleet_name)?;
    let total_started_at = Instant::now();
    let mut timings = InstallTimingSummary::default();

    println!(
        "Installing fleet {} against DFX_NETWORK={}",
        fleet_name, options.network
    );
    ensure_dfx_running(&dfx_root, &options.network)?;
    let mut create = Command::new("dfx");
    create
        .current_dir(&dfx_root)
        .args(["canister", "create", "--all", "-qq"]);
    let create_started_at = Instant::now();
    run_command(&mut create)?;
    timings.create_canisters = create_started_at.elapsed();

    let build_targets = configured_install_targets(&config_path, &options.root_build_target)?;
    let build_session_id = install_build_session_id();
    let build_started_at = Instant::now();
    run_dfx_build_targets(&dfx_root, &build_targets, &build_session_id, &config_path)?;
    timings.build_all = build_started_at.elapsed();

    let emit_manifest_started_at = Instant::now();
    let manifest_path = emit_root_release_set_manifest_with_config(
        &workspace_root,
        &dfx_root,
        &options.network,
        &config_path,
    )?;
    timings.emit_manifest = emit_manifest_started_at.elapsed();

    timings.fabricate_cycles =
        maybe_fabricate_local_cycles(&dfx_root, &options.root_canister, &options.network)?;

    let mut install = Command::new("dfx");
    install.current_dir(&dfx_root).args([
        "canister",
        "install",
        &options.root_canister,
        "--mode=reinstall",
        "-y",
        "--argument",
        "(variant { Prime })",
    ]);
    let install_started_at = Instant::now();
    run_command(&mut install)?;
    timings.install_root = install_started_at.elapsed();

    let manifest = load_root_release_set_manifest(&manifest_path)?;
    let stage_started_at = Instant::now();
    stage_root_release_set(&dfx_root, &options.root_canister, &manifest)?;
    timings.stage_release_set = stage_started_at.elapsed();
    let resume_started_at = Instant::now();
    resume_root_bootstrap(&options.root_canister)?;
    timings.resume_bootstrap = resume_started_at.elapsed();
    let ready_started_at = Instant::now();
    let ready_result = wait_for_root_ready(&options.root_canister, options.ready_timeout_seconds);
    timings.wait_ready = ready_started_at.elapsed();
    if let Err(err) = ready_result {
        print_install_timing_summary(&timings, total_started_at.elapsed());
        return Err(err);
    }

    print_install_timing_summary(&timings, total_started_at.elapsed());
    let state = build_install_state(
        &options,
        &workspace_root,
        &dfx_root,
        &config_path,
        &manifest_path,
        &fleet_name,
    )?;
    let state_path = write_install_state(&dfx_root, &options.network, &state)?;
    print_install_result_summary(&options.network, &state.fleet, &state_path);
    Ok(())
}

// Build the persisted project-local install state from a completed install.
fn build_install_state(
    options: &InstallRootOptions,
    workspace_root: &Path,
    dfx_root: &Path,
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
        root_canister_id: resolve_root_canister_id(dfx_root, &options.root_canister)?,
        root_build_target: options.root_build_target.clone(),
        workspace_root: workspace_root.display().to_string(),
        dfx_root: dfx_root.display().to_string(),
        config_path: config_path.display().to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    })
}

// Resolve the installed root id, accepting principal targets without a dfx lookup.
fn resolve_root_canister_id(
    dfx_root: &Path,
    root_canister: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if Principal::from_text(root_canister).is_ok() {
        return Ok(root_canister.to_string());
    }

    let mut command = Command::new("dfx");
    command
        .current_dir(dfx_root)
        .args(["canister", "id", root_canister]);
    Ok(run_command_stdout(&mut command)?.trim().to_string())
}

// Read the current host clock as a unix timestamp for install state.
fn current_unix_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

// Run one `dfx build <canister>` call per configured local install target.
fn run_dfx_build_targets(
    dfx_root: &Path,
    targets: &[String],
    build_session_id: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Build artifacts:");
    println!("{:<16} {:<18} {:>10}", "CANISTER", "PROGRESS", "ELAPSED");

    for (index, target) in targets.iter().enumerate() {
        let mut command = dfx_build_target_command(dfx_root, target, build_session_id);
        command.env("CANIC_CONFIG_PATH", config_path);
        let started_at = Instant::now();
        let output = command.output()?;
        let elapsed = started_at.elapsed();

        if !output.status.success() {
            return Err(format!(
                "dfx build failed for {target}: {}\nstdout:\n{}\nstderr:\n{}",
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

// Spawn one local `dfx build <canister>` step without overriding the caller's
// selected build profile environment.
fn dfx_build_target_command(dfx_root: &Path, target: &str, build_session_id: &str) -> Command {
    let mut command = Command::new("dfx");
    command
        .current_dir(dfx_root)
        .env("CANIC_BUILD_CONTEXT_SESSION", build_session_id)
        .args(["build", "-qq", target]);
    command
}

fn install_build_session_id() -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("install-root-{}-{unique}", std::process::id())
}

// Top up local root cycles only when the current balance is below the target floor.
fn maybe_fabricate_local_cycles(
    dfx_root: &Path,
    root_canister: &str,
    network: &str,
) -> Result<Duration, Box<dyn std::error::Error>> {
    if network != "local" {
        return Ok(Duration::ZERO);
    }

    let current_balance = root_cycle_balance(dfx_root, root_canister)?;
    let Some(fabricate_cycles) = required_local_cycle_topup(current_balance) else {
        println!(
            "Skipping local cycle fabrication for {root_canister}; balance {} already meets target {}",
            format_cycles(current_balance),
            format_cycles(LOCAL_ROOT_TARGET_CYCLES)
        );
        return Ok(Duration::ZERO);
    };

    let mut fabricate = Command::new("dfx");
    fabricate.current_dir(dfx_root);
    fabricate.args([
        "ledger",
        "fabricate-cycles",
        "--canister",
        root_canister,
        "--cycles",
        &fabricate_cycles.to_string(),
    ]);
    let fabricate_started_at = Instant::now();
    let output = fabricate.output()?;
    print_local_cycle_topup_summary(root_canister, current_balance, fabricate_cycles, &output);

    Ok(fabricate_started_at.elapsed())
}

// Print a compact, separated summary for the noisy local dfx cycle top-up.
fn print_local_cycle_topup_summary(
    root_canister: &str,
    current_balance: u128,
    fabricate_cycles: u128,
    output: &std::process::Output,
) {
    let status = if output.status.success() {
        "topped up"
    } else {
        "top-up requested"
    };
    println!(
        "\n\x1b[33mcycles: {status} local root {root_canister} by {} toward target {} (was {})\x1b[0m\n",
        format_cycles(fabricate_cycles),
        format_cycles(LOCAL_ROOT_TARGET_CYCLES),
        format_cycles(current_balance)
    );
}

// Read the current root canister cycle balance from `dfx canister status`.
fn root_cycle_balance(
    dfx_root: &Path,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let mut command = Command::new("dfx");
    command
        .current_dir(dfx_root)
        .args(["canister", "status", root_canister]);
    let stdout = dfx::run_output(&mut command)?;
    parse_canister_status_cycles(&stdout)
        .ok_or_else(|| "could not parse cycle balance from `dfx canister status` output".into())
}

// Parse the cycle balance from the human-readable `dfx canister status` output.
fn parse_canister_status_cycles(status_output: &str) -> Option<u128> {
    status_output
        .lines()
        .find_map(parse_canister_status_balance_line)
}

fn parse_canister_status_balance_line(line: &str) -> Option<u128> {
    let (label, value) = line.trim().split_once(':')?;
    let label = label.trim().to_ascii_lowercase();
    if label != "balance" && label != "cycle balance" {
        return None;
    }

    let digits = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }

    digits.parse::<u128>().ok()
}

// Return the local top-up delta needed to bring root up to the target cycle floor.
fn required_local_cycle_topup(current_balance: u128) -> Option<u128> {
    (current_balance < LOCAL_ROOT_TARGET_CYCLES)
        .then_some(LOCAL_ROOT_TARGET_CYCLES.saturating_sub(current_balance))
        .filter(|cycles| *cycles > 0)
}

fn format_cycles(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + (digits.len().saturating_sub(1) / 3));
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('_');
        }
        out.push(ch);
    }
    out
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
fn ensure_dfx_running(dfx_root: &Path, network: &str) -> Result<(), Box<dyn std::error::Error>> {
    if dfx_ping(network)? {
        return Ok(());
    }

    if network == "local" && local_dfx_autostart_enabled() {
        println!("Local dfx replica is not reachable; starting a clean local replica");
        let mut stop = dfx_stop_command(dfx_root);
        let _ = run_command_allow_failure(&mut stop)?;

        let mut start = dfx_start_local_command(dfx_root);
        run_command(&mut start)?;
        wait_for_dfx_ping(
            network,
            Duration::from_secs(LOCAL_DFX_READY_TIMEOUT_SECONDS),
        )?;
        return Ok(());
    }

    Err(format!(
        "dfx replica is not running for network '{network}'\nStart the target replica externally and rerun."
    )
    .into())
}

// Check whether `dfx ping <network>` currently succeeds.
fn dfx_ping(network: &str) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(Command::new("dfx")
        .args(["ping", network])
        .output()?
        .status
        .success())
}

// Return true when the local install flow should auto-start a clean local replica.
fn local_dfx_autostart_enabled() -> bool {
    parse_local_dfx_autostart(env::var("CANIC_AUTO_START_LOCAL_DFX").ok().as_deref())
}

fn parse_local_dfx_autostart(value: Option<&str>) -> bool {
    !matches!(
        value.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("0" | "false" | "no" | "off")
    )
}

// Spawn one local `dfx stop` command for cleanup before a clean restart.
fn dfx_stop_command(dfx_root: &Path) -> Command {
    let mut command = Command::new("dfx");
    command.current_dir(dfx_root).arg("stop");
    command
}

// Spawn one clean background `dfx start` command for local install/test flows.
fn dfx_start_local_command(dfx_root: &Path) -> Command {
    let mut command = Command::new("dfx");
    command
        .current_dir(dfx_root)
        .args(["start", "--background", "--clean", "--system-canisters"]);
    command
}

// Poll `dfx ping` until the requested network responds or the timeout expires.
fn wait_for_dfx_ping(network: &str, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if dfx_ping(network)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }

    Err(format!(
        "dfx replica did not become ready for network '{network}' within {}s",
        timeout.as_secs()
    )
    .into())
}

// Wait until root reports ready, printing periodic progress and diagnostics.
fn wait_for_root_ready(
    root_canister: &str,
    timeout_seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let mut next_report = 0_u64;

    println!("Waiting for {root_canister} to report canic_ready (timeout {timeout_seconds}s)");

    loop {
        if root_ready(root_canister)? {
            println!(
                "{root_canister} reported canic_ready after {}s",
                start.elapsed().as_secs()
            );
            return Ok(());
        }

        if let Some(status) = root_bootstrap_status(root_canister)?
            && let Some(last_error) = status.last_error.as_deref()
        {
            eprintln!(
                "root bootstrap reported failure during phase '{}' : {}",
                status.phase, last_error
            );
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_bootstrap_status");
            print_raw_call(root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_subnet_registry");
            print_raw_call(root_canister, "canic_subnet_registry");
            eprintln!(
                "Diagnostic: dfx canister call {root_canister} canic_wasm_store_bootstrap_debug"
            );
            print_raw_call(root_canister, "canic_wasm_store_bootstrap_debug");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_wasm_store_overview");
            print_raw_call(root_canister, "canic_wasm_store_overview");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_log");
            print_recent_root_logs(root_canister);
            return Err(format!(
                "root bootstrap failed during phase '{}' : {}",
                status.phase, last_error
            )
            .into());
        }

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_seconds {
            eprintln!("root did not report canic_ready within {timeout_seconds}s");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_bootstrap_status");
            print_raw_call(root_canister, protocol::CANIC_BOOTSTRAP_STATUS);
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_subnet_registry");
            print_raw_call(root_canister, "canic_subnet_registry");
            eprintln!(
                "Diagnostic: dfx canister call {root_canister} canic_wasm_store_bootstrap_debug"
            );
            print_raw_call(root_canister, "canic_wasm_store_bootstrap_debug");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_wasm_store_overview");
            print_raw_call(root_canister, "canic_wasm_store_overview");
            eprintln!("Diagnostic: dfx canister call {root_canister} canic_log");
            print_recent_root_logs(root_canister);
            return Err("root did not become ready".into());
        }

        if elapsed >= next_report {
            println!("Still waiting for {root_canister} canic_ready ({elapsed}s elapsed)");
            if let Some(status) = root_bootstrap_status(root_canister)? {
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
            if let Ok(registry_json) =
                dfx_call(root_canister, "canic_subnet_registry", None, Some("json"))
            {
                println!("Current subnet registry roles:");
                println!("  {}", registry_roles(&registry_json));
            }
            next_report = elapsed + 5;
        }

        thread::sleep(Duration::from_secs(1));
    }
}

// Return true once root reports `canic_ready == true`.
fn root_ready(root_canister: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = dfx_call(root_canister, "canic_ready", None, Some("json"))?;
    let data = serde_json::from_str::<Value>(&output)?;
    Ok(parse_root_ready_value(&data))
}

// Return the current root bootstrap diagnostic state when the query is available.
fn root_bootstrap_status(
    root_canister: &str,
) -> Result<Option<BootstrapStatusSnapshot>, Box<dyn std::error::Error>> {
    let output = match dfx_call(
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

// Accept both plain-bool and wrapped-result JSON shapes from `dfx --output json`.
fn parse_root_ready_value(data: &Value) -> bool {
    matches!(data, Value::Bool(true)) || matches!(data.get("Ok"), Some(Value::Bool(true)))
}

fn parse_bootstrap_status_value(data: &Value) -> Option<BootstrapStatusSnapshot> {
    serde_json::from_value::<BootstrapStatusSnapshot>(data.clone())
        .ok()
        .or_else(|| {
            data.get("Ok")
                .cloned()
                .and_then(|ok| serde_json::from_value::<BootstrapStatusSnapshot>(ok).ok())
        })
}

fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{:<20} {:>10}", "phase", "elapsed");
    println!("{:<20} {:>10}", "--------------------", "----------");
    print_timing_row("create_canisters", timings.create_canisters);
    print_timing_row("build_all", timings.build_all);
    print_timing_row("emit_manifest", timings.emit_manifest);
    print_timing_row("fabricate_cycles", timings.fabricate_cycles);
    print_timing_row("install_root", timings.install_root);
    print_timing_row("stage_release_set", timings.stage_release_set);
    print_timing_row("resume_bootstrap", timings.resume_bootstrap);
    print_timing_row("wait_ready", timings.wait_ready);
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
    println!("{:<14} canic list --network {}", "smoke_check", network);
}

// Print recent structured root log entries without raw byte dumps.
fn print_recent_root_logs(root_canister: &str) {
    let page_args = r"(null, null, null, record { limit = 8; offset = 0 })";
    let Ok(logs_json) = dfx_call(root_canister, "canic_log", Some(page_args), Some("json")) else {
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
    dfx::run_status(command).map_err(Into::into)
}

// Run one command, require success, and return stdout.
fn run_command_stdout(command: &mut Command) -> Result<String, Box<dyn std::error::Error>> {
    dfx::run_output(command).map_err(Into::into)
}

// Run one command and return its status without failing the caller on non-zero exit.
fn run_command_allow_failure(
    command: &mut Command,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    Ok(command.status()?)
}

// Print one raw fallback `dfx canister call` result to stderr for diagnostics.
fn print_raw_call(root_canister: &str, method: &str) {
    let mut command = Command::new("dfx");
    if let Ok(root) = dfx_root() {
        command.current_dir(root);
    }
    let _ = command
        .args(["canister", "call", root_canister, method])
        .status();
}
