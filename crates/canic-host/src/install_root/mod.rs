use crate::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
    print_current_workspace_build_context_once,
};
use crate::icp;
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, configured_fleet_name, configured_install_targets,
    configured_local_root_create_cycles, emit_root_release_set_manifest_with_config,
    icp_call_on_network, icp_root, load_root_release_set_manifest, resolve_artifact_root,
    resume_root_bootstrap, stage_root_release_set, workspace_root,
};
use canic_core::{
    cdk::{types::Principal, utils::hash::wasm_hash},
    protocol,
};
use config_selection::resolve_install_config_path;
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod config_selection;
mod readiness;
mod state;

pub use config_selection::discover_canic_config_choices;
use readiness::wait_for_root_ready;
use state::{INSTALL_STATE_SCHEMA_VERSION, validate_fleet_name, write_install_state};
pub use state::{InstallState, read_named_fleet_install_state};

#[cfg(test)]
mod tests;

#[cfg(test)]
use config_selection::config_selection_error;
#[cfg(test)]
use readiness::{parse_bootstrap_status_value, parse_root_ready_value};
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
    install.args(["--args", &root_init_args(root_wasm)?]);
    add_icp_environment_target(&mut install, network);
    run_command(&mut install)
}

fn root_init_args(root_wasm: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let wasm = std::fs::read(root_wasm)?;
    Ok(format!(
        "(variant {{ PrimeWithModuleHash = {} }})",
        idl_blob(&wasm_hash(&wasm))
    ))
}

fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "\\{byte:02X}");
    }
    encoded.push('"');
    encoded
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

// Build each configured local install target through the host builder.
fn run_canic_build_targets(
    network: &str,
    targets: &[String],
    build_session_id: &str,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Build artifacts:");
    println!("{:<16} {:<18} {:>10}", "CANISTER", "PROGRESS", "ELAPSED");

    let _env = BuildEnvGuard::apply(network, build_session_id, config_path);
    let profile = CanisterBuildProfile::current();
    print_current_workspace_build_context_once(profile)?;
    for (index, target) in targets.iter().enumerate() {
        let started_at = Instant::now();
        build_current_workspace_canister_artifact(target, profile)
            .map_err(|err| format!("artifact build failed for {target}: {err}"))?;
        let elapsed = started_at.elapsed();

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

struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_session: Option<OsString>,
    previous_config_path: Option<OsString>,
}

impl BuildEnvGuard {
    fn apply(network: &str, build_session_id: &str, config_path: &Path) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_session: env::var_os("CANIC_BUILD_CONTEXT_SESSION"),
            previous_config_path: env::var_os("CANIC_CONFIG_PATH"),
        };
        set_env("ICP_ENVIRONMENT", network);
        set_env("CANIC_BUILD_CONTEXT_SESSION", build_session_id);
        set_env("CANIC_CONFIG_PATH", config_path);
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
        restore_env("CANIC_BUILD_CONTEXT_SESSION", self.previous_session.take());
        restore_env("CANIC_CONFIG_PATH", self.previous_config_path.take());
    }
}

fn set_env<K, V>(key: K, value: V)
where
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::set_var(key, value);
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    // See set_env: this restores the single-threaded install build context.
    unsafe {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
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
