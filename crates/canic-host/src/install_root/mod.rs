use crate::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
    current_workspace_build_context_once,
};
use crate::format::byte_size;
use crate::icp;
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, configured_fleet_name, configured_install_targets,
    configured_local_root_create_cycles, emit_root_release_set_manifest_with_config,
    icp_call_on_network, icp_root, load_root_release_set_manifest, resolve_artifact_root,
    resume_root_bootstrap, stage_root_release_set, workspace_root,
};
use crate::response_parse::parse_cycle_balance_response;
use crate::table::{ColumnAlign, render_separator, render_table, render_table_row, table_widths};
use canic_core::{
    cdk::{types::Principal, utils::hash::wasm_hash},
    protocol,
};
use config_selection::resolve_install_config_path;
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

mod config_selection;
mod readiness;
mod state;

pub use config_selection::{
    CANIC_FLEETS_ROOT_ENV, discover_canic_config_choices, discover_project_canic_config_choices,
    discover_project_canic_config_choices_with_root, project_fleet_roots,
    project_fleet_roots_with_override,
};
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
    pub build_profile: Option<CanisterBuildProfile>,
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
    let network = options.network.as_str();
    let root_canister = options.root_canister.as_str();

    println!("Installing fleet {fleet_name}");
    println!();
    ensure_icp_environment_ready(&options.network)?;
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
    let build_started_at = Instant::now();
    run_canic_build_targets(
        &options.network,
        &build_targets,
        options.build_profile,
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
    ensure_local_root_min_cycles(&icp_root, network, root_canister, "pre-bootstrap")?;
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
    ensure_local_root_min_cycles(&icp_root, network, root_canister, "post-ready")?;
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
    build_profile: Option<CanisterBuildProfile>,
    config_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let _env = BuildEnvGuard::apply(network, config_path);
    let profile = build_profile.unwrap_or_else(CanisterBuildProfile::current);
    if let Some(context) = current_workspace_build_context_once(profile)? {
        for line in context.lines() {
            println!("{line}");
        }
        println!("config: {}", config_path.display());
        println!("artifacts: {}", planned_build_artifact_root()?.display());
        println!();
    }

    fs::create_dir_all(planned_build_artifact_root()?)?;
    println!("Building {} canisters", targets.len());
    println!();
    let headers = ["CANISTER", "PROGRESS", "WASM_GZ", "ELAPSED"];
    let planned_rows = targets
        .iter()
        .map(|target| {
            [
                target.clone(),
                progress_bar(targets.len(), targets.len(), 10),
                "000.00 MiB".to_string(),
                "0.00s".to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ];
    let widths = table_widths(&headers, &planned_rows);
    println!("{}", render_table_row(&headers, &widths, &alignments));
    println!("{}", render_separator(&widths));

    for (index, target) in targets.iter().enumerate() {
        let started_at = Instant::now();
        let output = build_current_workspace_canister_artifact(target, profile)
            .map_err(|err| format!("artifact build failed for {target}: {err}"))?;
        let elapsed = started_at.elapsed();
        let artifact_size = wasm_gz_size(&output.wasm_gz_path)?;

        let row = [
            target.clone(),
            progress_bar(index + 1, targets.len(), 10),
            artifact_size,
            format!("{:.2}s", elapsed.as_secs_f64()),
        ];
        println!("{}", render_table_row(&row, &widths, &alignments));
    }

    println!();
    Ok(())
}

fn planned_build_artifact_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(icp_root()?.join(".icp/local/canisters"))
}

fn wasm_gz_size(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(byte_size(std::fs::metadata(path)?.len()))
}

struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_config_path: Option<OsString>,
}

impl BuildEnvGuard {
    fn apply(network: &str, config_path: &Path) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_config_path: env::var_os("CANIC_CONFIG_PATH"),
        };
        set_env("ICP_ENVIRONMENT", network);
        set_env("CANIC_CONFIG_PATH", config_path);
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
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
        crate::format::cycles_tc(amount),
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
fn ensure_icp_environment_ready(network: &str) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(network)? {
        return Ok(());
    }

    Err(format!(
        "icp environment is not running for network '{network}'\nStart the target replica in another terminal with `canic replica start` and rerun."
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

fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{}", render_install_timing_summary(timings, total));
}

fn render_install_timing_summary(timings: &InstallTimingSummary, total: Duration) -> String {
    let rows = [
        timing_row("create_canisters", timings.create_canisters),
        timing_row("build_all", timings.build_all),
        timing_row("emit_manifest", timings.emit_manifest),
        timing_row("install_root", timings.install_root),
        timing_row("fund_root", timings.fund_root),
        timing_row("stage_release_set", timings.stage_release_set),
        timing_row("resume_bootstrap", timings.resume_bootstrap),
        timing_row("wait_ready", timings.wait_ready),
        timing_row("finalize_root_funding", timings.finalize_root_funding),
        timing_row("total", total),
    ];
    render_table(
        &["PHASE", "ELAPSED"],
        &rows,
        &[ColumnAlign::Left, ColumnAlign::Right],
    )
}

fn timing_row(label: &str, duration: Duration) -> [String; 2] {
    [label.to_string(), format!("{:.2}s", duration.as_secs_f64())]
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
