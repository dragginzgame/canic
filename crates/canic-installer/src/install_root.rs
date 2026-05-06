use crate::release_set::{
    configured_install_targets, configured_release_roles, dfx_call, dfx_root,
    emit_root_release_set_manifest_with_config, load_root_release_set_manifest,
    resolve_artifact_root, resume_root_bootstrap, root_release_set_manifest_path,
    stage_root_release_set, workspace_root,
};
use crate::workspace_discovery::normalize_workspace_path;
use canic::cdk::types::Principal;
use canic_core::protocol;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub fleet_name: String,
    pub root_canister: String,
    pub root_build_target: String,
    pub network: String,
    pub ready_timeout_seconds: u64,
    pub config_path: Option<String>,
}

///
/// InstallState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallState {
    pub schema_version: u32,
    #[serde(default = "default_fleet_name")]
    pub fleet: String,
    pub installed_at_unix_secs: u64,
    pub network: String,
    pub root_target: String,
    pub root_canister_id: String,
    pub root_build_target: String,
    pub workspace_root: String,
    pub dfx_root: String,
    pub config_path: String,
    pub release_set_manifest_path: String,
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

///
/// ConfigChoiceRow
///

struct ConfigChoiceRow {
    option: String,
    config: String,
    canisters: String,
}

const LOCAL_ROOT_TARGET_CYCLES: u128 = 9_000_000_000_000_000;
const LOCAL_DFX_READY_TIMEOUT_SECONDS: u64 = 30;
const INSTALL_STATE_SCHEMA_VERSION: u32 = 1;
const INSTALL_STATE_FILE: &str = "install-state.json";
pub const DEFAULT_FLEET_NAME: &str = "default";
const CURRENT_FLEET_FILE: &str = "current-fleet";
const CONFIG_CHOICE_ROLE_PREVIEW_LIMIT: usize = 5;

impl InstallRootOptions {
    // Resolve the current local-root install options from args and environment.
    #[must_use]
    pub fn from_env_and_args() -> Self {
        let root_canister = env::args()
            .nth(1)
            .or_else(|| env::var("ROOT_CANISTER").ok())
            .unwrap_or_else(|| "root".to_string());

        Self {
            fleet_name: env::var("CANIC_FLEET").unwrap_or_else(|_| DEFAULT_FLEET_NAME.to_string()),
            root_build_target: env::var("ROOT_BUILD_TARGET")
                .ok()
                .unwrap_or_else(|| root_canister.clone()),
            root_canister,
            network: env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string()),
            ready_timeout_seconds: env::var("READY_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(120),
            config_path: None,
        }
    }
}

// Execute the local thin-root install flow against an already running replica.
pub fn install_root(options: InstallRootOptions) -> Result<(), Box<dyn std::error::Error>> {
    validate_fleet_name(&options.fleet_name)?;
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    let config_path = resolve_install_config_path(&workspace_root, options.config_path.as_deref())?;
    let total_started_at = Instant::now();
    let mut timings = InstallTimingSummary::default();

    println!(
        "Installing fleet {} against DFX_NETWORK={}",
        options.fleet_name, options.network
    );
    ensure_dfx_running(&dfx_root, &options.network)?;
    let mut create = Command::new("dfx");
    create
        .current_dir(&dfx_root)
        .args(["canister", "create", "--all", "-qq"]);
    let create_started_at = Instant::now();
    run_command(&mut create)?;
    timings.create_canisters = create_started_at.elapsed();

    let build_targets = local_install_build_targets(&config_path, &options.root_build_target)?;
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

    let artifact_root = resolve_artifact_root(&dfx_root, &options.network)?;
    let manifest =
        load_root_release_set_manifest(&root_release_set_manifest_path(&artifact_root)?)?;
    assert_eq!(
        manifest_path,
        root_release_set_manifest_path(&artifact_root)?
    );
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
    )?;
    let state_path = write_install_state(&dfx_root, &options.network, &state)?;
    print_install_result_summary(&options.network, &state.fleet, &state_path);
    Ok(())
}

/// Read the persisted install state for one project/network when present.
pub fn read_install_state(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    if let Some(fleet) = read_selected_fleet_name(dfx_root, network)? {
        return read_fleet_install_state(dfx_root, network, &fleet);
    }

    read_legacy_install_state(dfx_root, network)
}

/// Read a named fleet install state for one project/network when present.
pub fn read_fleet_install_state(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    validate_fleet_name(fleet)?;
    let path = fleet_install_state_path(dfx_root, network, fleet);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let mut state: InstallState = serde_json::from_slice(&bytes)?;
    if state.fleet.is_empty() {
        state.fleet = fleet.to_string();
    }
    Ok(Some(state))
}

/// Read the install state for the discovered current project/network.
pub fn read_current_install_state(
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    read_install_state(&dfx_root, network)
}

/// Read either a named fleet state or the selected current fleet state.
pub fn read_current_or_fleet_install_state(
    network: &str,
    fleet: Option<&str>,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    match fleet {
        Some(fleet) => read_fleet_install_state(&dfx_root, network, fleet),
        None => read_install_state(&dfx_root, network),
    }
}

///
/// FleetSummary
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetSummary {
    pub name: String,
    pub current: bool,
    pub state: InstallState,
}

/// List installed fleets for the current project/network.
pub fn list_current_fleets(network: &str) -> Result<Vec<FleetSummary>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    list_fleets(&dfx_root, network)
}

/// List installed fleets for one project/network.
pub fn list_fleets(
    dfx_root: &Path,
    network: &str,
) -> Result<Vec<FleetSummary>, Box<dyn std::error::Error>> {
    let current = read_selected_fleet_name(dfx_root, network)?;
    let mut fleets = Vec::new();
    let dir = fleets_dir(dfx_root, network);
    if dir.is_dir() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if let Some(state) = read_fleet_install_state(dfx_root, network, name)? {
                fleets.push(FleetSummary {
                    name: name.to_string(),
                    current: current.as_deref() == Some(name),
                    state,
                });
            }
        }
    }

    if fleets.is_empty()
        && let Some(state) = read_legacy_install_state(dfx_root, network)?
    {
        fleets.push(FleetSummary {
            name: state.fleet.clone(),
            current: true,
            state,
        });
    }

    fleets.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fleets)
}

/// Select one installed fleet as the current project/network default.
pub fn select_current_fleet(
    network: &str,
    fleet: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    select_fleet(&dfx_root, network, fleet)
}

/// Select one installed fleet for one project/network.
pub fn select_fleet(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let Some(state) = read_fleet_install_state(dfx_root, network, fleet)?.or_else(|| {
        matching_legacy_fleet_state(dfx_root, network, fleet)
            .ok()
            .flatten()
    }) else {
        return Err(format!("unknown fleet {fleet} on network {network}").into());
    };
    if fleet_install_state_path(dfx_root, network, fleet).is_file() {
        write_current_fleet_name(dfx_root, network, fleet)?;
    } else {
        write_install_state(dfx_root, network, &state)?;
    }
    Ok(state)
}

/// Return the legacy project-local install state path for one network.
#[must_use]
pub fn install_state_path(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root
        .join(".canic")
        .join(network)
        .join(INSTALL_STATE_FILE)
}

/// Return the project-local state path for one named fleet.
#[must_use]
pub fn fleet_install_state_path(dfx_root: &Path, network: &str, fleet: &str) -> PathBuf {
    fleets_dir(dfx_root, network).join(format!("{fleet}.json"))
}

/// Return the project-local current-fleet pointer path for one network.
#[must_use]
pub fn current_fleet_path(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root
        .join(".canic")
        .join(network)
        .join(CURRENT_FLEET_FILE)
}

// Return the directory that owns named fleet state files.
fn fleets_dir(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root.join(".canic").join(network).join("fleets")
}

// Resolve install config selection without silently choosing among demo/test configs.
fn resolve_install_config_path(
    workspace_root: &Path,
    explicit_config_path: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = explicit_config_path {
        return Ok(normalize_workspace_path(
            workspace_root,
            PathBuf::from(path),
        ));
    }

    if let Some(path) = env::var_os("CANIC_CONFIG_PATH") {
        return Ok(normalize_workspace_path(
            workspace_root,
            PathBuf::from(path),
        ));
    }

    let default = workspace_root.join("canisters/canic.toml");
    if default.is_file() {
        return Ok(default);
    }

    let choices = discover_canic_config_choices(&workspace_root.join("canisters"))?;
    if let Some(path) = prompt_install_config_choice(workspace_root, &default, &choices)? {
        return Ok(path);
    }

    Err(config_selection_error(workspace_root, &default, &choices).into())
}

// Discover candidate `canic.toml` files under the conventional canisters tree.
fn discover_canic_config_choices(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut choices = Vec::new();
    collect_canic_config_choices(root, &mut choices)?;
    choices.sort();
    Ok(choices)
}

// Recursively collect candidate config paths.
fn collect_canic_config_choices(
    root: &Path,
    choices: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_canic_config_choices(&path, choices)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("canic.toml")
            && is_install_project_config(&path)
        {
            choices.push(path);
        }
    }

    Ok(())
}

// Treat only configs next to a root canister directory as installable choices.
fn is_install_project_config(path: &Path) -> bool {
    path.parent()
        .is_some_and(|parent| parent.join("root/Cargo.toml").is_file())
}

// Format an actionable config-selection error with whitespace-aligned choices.
fn config_selection_error(workspace_root: &Path, default: &Path, choices: &[PathBuf]) -> String {
    let mut lines = vec![format!(
        "missing default Canic config at {}",
        display_workspace_path(workspace_root, default)
    )];

    if choices.is_empty() {
        lines.push("create canisters/canic.toml or run canic install --config <path>".to_string());
        return lines.join("\n");
    }

    if choices.len() == 1 {
        let choice = display_workspace_path(workspace_root, &choices[0]);
        lines.push(String::new());
        lines.extend(config_choice_table(workspace_root, choices));
        lines.push(String::new());
        lines.push(format!("run: canic install --config {choice}"));
        return lines.join("\n");
    }

    lines.push("choose a config path explicitly:".to_string());
    lines.push(String::new());
    lines.extend(config_choice_table(workspace_root, choices));
    lines.push(String::new());
    lines.push("run: canic install --config <path>".to_string());
    lines.join("\n")
}

// Prompt interactively for one discovered config when running in a terminal.
fn prompt_install_config_choice(
    workspace_root: &Path,
    default: &Path,
    choices: &[PathBuf],
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if choices.is_empty() || !io::stdin().is_terminal() {
        return Ok(None);
    }

    eprintln!(
        "missing default Canic config at {}",
        display_workspace_path(workspace_root, default)
    );
    eprintln!();
    for line in config_choice_table(workspace_root, choices) {
        eprintln!("{line}");
    }
    eprintln!();

    loop {
        eprint!("enter config number (ctrl-c to quit): ");
        io::stderr().flush()?;

        let mut answer = String::new();
        if io::stdin().read_line(&mut answer)? == 0 {
            return Ok(None);
        }

        let trimmed = answer.trim();
        let Ok(index) = trimmed.parse::<usize>() else {
            eprintln!("invalid selection: {trimmed}");
            continue;
        };
        let Some(path) = choices.get(index.saturating_sub(1)) else {
            eprintln!("selection out of range: {index}");
            continue;
        };

        return Ok(Some(path.clone()));
    }
}

// Render config choices with enough metadata to choose the intended topology.
fn config_choice_table(workspace_root: &Path, choices: &[PathBuf]) -> Vec<String> {
    let rows = choices
        .iter()
        .enumerate()
        .map(|(index, path)| config_choice_row(workspace_root, index + 1, path))
        .collect::<Vec<_>>();
    let option_width = rows
        .iter()
        .map(|row| row.option.len())
        .chain(["#".len()])
        .max()
        .expect("option width");
    let config_width = rows
        .iter()
        .map(|row| row.config.len())
        .chain(["CONFIG".len()])
        .max()
        .expect("config width");
    let mut lines = vec![format!(
        "{:<option_width$}  {:<config_width$}  CANISTERS",
        "#", "CONFIG"
    )];

    for row in rows {
        lines.push(format!(
            "{:<option_width$}  {:<config_width$}  {}",
            row.option, row.config, row.canisters
        ));
    }

    lines
}

// Summarize the root-subnet release roles for one install config choice.
fn config_choice_row(workspace_root: &Path, option: usize, path: &Path) -> ConfigChoiceRow {
    let config = display_workspace_path(workspace_root, path);
    match configured_release_roles(path) {
        Ok(roles) => ConfigChoiceRow {
            option: option.to_string(),
            config,
            canisters: format_canister_summary(&roles),
        },
        Err(_) => ConfigChoiceRow {
            option: option.to_string(),
            config,
            canisters: "invalid config".to_string(),
        },
    }
}

// Format the root-subnet canister count with a bounded role preview.
fn format_canister_summary(roles: &[String]) -> String {
    if roles.is_empty() {
        return "0".to_string();
    }

    let preview = roles
        .iter()
        .take(CONFIG_CHOICE_ROLE_PREVIEW_LIMIT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if roles.len() > CONFIG_CHOICE_ROLE_PREVIEW_LIMIT {
        ", ..."
    } else {
        ""
    };

    format!("{} ({preview}{suffix})", roles.len())
}

// Render a workspace-relative path where possible for concise diagnostics.
fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// Resolve the local install build set from the root canister plus the
// configured ordinary roles owned by the root subnet.
fn local_install_build_targets(
    config_path: &Path,
    root_canister: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    configured_install_targets(config_path, root_canister)
}

// Build the persisted project-local install state from a completed install.
fn build_install_state(
    options: &InstallRootOptions,
    workspace_root: &Path,
    dfx_root: &Path,
    config_path: &Path,
    release_set_manifest_path: &Path,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    Ok(InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        fleet: options.fleet_name.clone(),
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

// Persist the completed install state under the project-local `.canic` directory.
fn write_install_state(
    dfx_root: &Path,
    network: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_fleet_name(&state.fleet)?;
    let path = fleet_install_state_path(dfx_root, network, &state.fleet);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_vec_pretty(state)?)?;
    write_current_fleet_name(dfx_root, network, &state.fleet)?;
    Ok(path)
}

// Read a legacy single-slot install state when no named fleet pointer exists.
fn read_legacy_install_state(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let path = install_state_path(dfx_root, network);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let mut state: InstallState = serde_json::from_slice(&bytes)?;
    if state.fleet.is_empty() {
        state.fleet = DEFAULT_FLEET_NAME.to_string();
    }
    Ok(Some(state))
}

// Return the legacy single-slot state only when it matches the requested fleet.
fn matching_legacy_fleet_state(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    Ok(read_legacy_install_state(dfx_root, network)?.filter(|state| state.fleet == fleet))
}

// Read the selected fleet name for one project/network.
fn read_selected_fleet_name(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let path = current_fleet_path(dfx_root, network);
    if !path.is_file() {
        return Ok(None);
    }

    let name = fs::read_to_string(path)?.trim().to_string();
    validate_fleet_name(&name)?;
    Ok(Some(name))
}

// Write the selected fleet name for one project/network.
fn write_current_fleet_name(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_fleet_name(fleet)?;
    let path = current_fleet_path(dfx_root, network);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{fleet}\n"))?;
    Ok(())
}

// Return the serde default for legacy install-state records.
fn default_fleet_name() -> String {
    DEFAULT_FLEET_NAME.to_string()
}

// Keep fleet names filesystem-safe and easy to type in commands.
fn validate_fleet_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!("invalid fleet name {name:?}; use letters, numbers, '-' or '_'").into())
    }
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

    println!(
        "Fabricating {} cycles locally for {root_canister} to reach target {} (current balance {})",
        format_cycles(fabricate_cycles),
        format_cycles(LOCAL_ROOT_TARGET_CYCLES),
        format_cycles(current_balance)
    );

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
    let _ = run_command_allow_failure(&mut fabricate)?;

    Ok(fabricate_started_at.elapsed())
}

// Read the current root canister cycle balance from `dfx canister status`.
fn root_cycle_balance(
    dfx_root: &Path,
    root_canister: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let output = Command::new("dfx")
        .current_dir(dfx_root)
        .args(["canister", "status", root_canister])
        .output()?;
    if !output.status.success() {
        return Err(format!("dfx canister status failed: {}", output.status).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
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
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed: {status}").into())
    }
}

// Run one command, require success, and return stdout.
fn run_command_stdout(command: &mut Command) -> Result<String, Box<dyn std::error::Error>> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    Err(format!(
        "command failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
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

#[cfg(test)]
mod tests {
    use super::{
        INSTALL_STATE_SCHEMA_VERSION, InstallState, LOCAL_ROOT_TARGET_CYCLES,
        config_selection_error, current_fleet_path, dfx_build_target_command,
        dfx_start_local_command, dfx_stop_command, discover_canic_config_choices,
        fleet_install_state_path, install_build_session_id, list_fleets,
        local_install_build_targets, parse_bootstrap_status_value, parse_canister_status_cycles,
        parse_local_dfx_autostart, parse_root_ready_value, read_fleet_install_state,
        read_install_state, required_local_cycle_topup, resolve_install_config_path,
        write_install_state,
    };
    use serde_json::json;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn parse_root_ready_accepts_plain_true() {
        assert!(parse_root_ready_value(&json!(true)));
    }

    #[test]
    fn parse_root_ready_accepts_wrapped_ok_true() {
        assert!(parse_root_ready_value(&json!({ "Ok": true })));
    }

    #[test]
    fn parse_root_ready_rejects_false_shapes() {
        assert!(!parse_root_ready_value(&json!(false)));
        assert!(!parse_root_ready_value(&json!({ "Ok": false })));
        assert!(!parse_root_ready_value(&json!({ "Err": "nope" })));
    }

    #[test]
    fn parse_bootstrap_status_accepts_plain_record() {
        let status = parse_bootstrap_status_value(&json!({
            "ready": false,
            "phase": "root:init:create_canisters",
            "last_error": null
        }))
        .expect("plain bootstrap status must parse");

        assert!(!status.ready);
        assert_eq!(status.phase, "root:init:create_canisters");
        assert_eq!(status.last_error, None);
    }

    #[test]
    fn parse_bootstrap_status_accepts_wrapped_ok_record() {
        let status = parse_bootstrap_status_value(&json!({
            "Ok": {
                "ready": false,
                "phase": "failed",
                "last_error": "registry phase failed"
            }
        }))
        .expect("wrapped bootstrap status must parse");

        assert!(!status.ready);
        assert_eq!(status.phase, "failed");
        assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
    }

    #[test]
    fn parse_canister_status_cycles_accepts_balance_line() {
        let output = "\
Canister status call result for root.
Status: Running
Balance: 9_002_999_998_056_000 Cycles
Memory Size: 1_234_567 Bytes
";

        assert_eq!(
            parse_canister_status_cycles(output),
            Some(9_002_999_998_056_000)
        );
    }

    #[test]
    fn parse_canister_status_cycles_accepts_cycle_balance_line() {
        let output = "\
Canister status call result for root.
Cycle balance: 12_345 Cycles
";

        assert_eq!(parse_canister_status_cycles(output), Some(12_345));
    }

    #[test]
    fn required_local_cycle_topup_skips_when_balance_already_meets_target() {
        assert_eq!(required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES), None);
        assert_eq!(
            required_local_cycle_topup(LOCAL_ROOT_TARGET_CYCLES + 1_000),
            None
        );
    }

    #[test]
    fn required_local_cycle_topup_returns_missing_delta_only() {
        assert_eq!(
            required_local_cycle_topup(3_000_000_000_000),
            Some(8_997_000_000_000_000)
        );
    }

    #[test]
    fn dfx_build_command_targets_one_canister_per_call() {
        let command = dfx_build_target_command(
            Path::new("/tmp/canic-dfx-root"),
            "user_hub",
            "install-root-test",
        );

        assert_eq!(command.get_program(), "dfx");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["build", "-qq", "user_hub"]
        );
        assert_eq!(
            command
                .get_current_dir()
                .map(|path| path.to_string_lossy().into_owned()),
            Some("/tmp/canic-dfx-root".to_string())
        );
        assert!(
            command
                .get_envs()
                .any(|(key, value)| key == "CANIC_BUILD_CONTEXT_SESSION" && value.is_some()),
            "dfx build must carry the shared build-session marker"
        );
    }

    #[test]
    fn install_build_session_id_is_prefixed_for_logs() {
        let session_id = install_build_session_id();
        assert!(session_id.starts_with("install-root-"));
    }

    #[test]
    fn local_dfx_autostart_defaults_to_enabled() {
        assert!(parse_local_dfx_autostart(None));
        assert!(parse_local_dfx_autostart(Some("")));
        assert!(parse_local_dfx_autostart(Some("1")));
        assert!(parse_local_dfx_autostart(Some("true")));
    }

    #[test]
    fn local_dfx_autostart_accepts_explicit_disable_values() {
        assert!(!parse_local_dfx_autostart(Some("0")));
        assert!(!parse_local_dfx_autostart(Some("false")));
        assert!(!parse_local_dfx_autostart(Some("no")));
        assert!(!parse_local_dfx_autostart(Some("off")));
    }

    #[test]
    fn local_dfx_start_command_uses_clean_background_mode() {
        let command = dfx_start_local_command(Path::new("/tmp/canic-dfx-root"));

        assert_eq!(command.get_program(), "dfx");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["start", "--background", "--clean", "--system-canisters"]
        );
        assert_eq!(
            command
                .get_current_dir()
                .map(|path| path.to_string_lossy().into_owned()),
            Some("/tmp/canic-dfx-root".to_string())
        );
    }

    #[test]
    fn local_dfx_stop_command_targets_project_root() {
        let command = dfx_stop_command(Path::new("/tmp/canic-dfx-root"));

        assert_eq!(command.get_program(), "dfx");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["stop"]
        );
        assert_eq!(
            command
                .get_current_dir()
                .map(|path| path.to_string_lossy().into_owned()),
            Some("/tmp/canic-dfx-root".to_string())
        );
    }

    #[test]
    fn local_install_build_targets_use_root_subnet_release_roles_only() {
        let workspace_root = write_temp_workspace_config(
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.project_registry]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.extra.canisters.oracle_pokemon]
kind = "singleton"
"#,
        );

        let targets =
            local_install_build_targets(&workspace_root.join("canisters/canic.toml"), "root")
                .expect("targets must resolve");

        assert_eq!(
            targets,
            vec![
                "root".to_string(),
                "project_registry".to_string(),
                "user_hub".to_string()
            ]
        );
    }

    #[test]
    fn install_config_defaults_to_project_config_when_present() {
        with_guarded_env(|| {
            let root = unique_temp_dir("canic-install-config-default");
            let config = root.join("canisters/canic.toml");
            fs::create_dir_all(config.parent().expect("config parent")).expect("create parent");
            fs::write(&config, "").expect("write config");
            let previous = env::var_os("CANIC_CONFIG_PATH");
            unsafe {
                env::remove_var("CANIC_CONFIG_PATH");
            }

            let resolved = resolve_install_config_path(&root, None).expect("resolve config");

            assert_eq!(resolved, config);
            restore_env_var("CANIC_CONFIG_PATH", previous);
            fs::remove_dir_all(root).expect("clean temp dir");
        });
    }

    #[test]
    fn install_config_accepts_explicit_path() {
        let root = unique_temp_dir("canic-install-config-explicit");
        let resolved = resolve_install_config_path(&root, Some("canisters/demo/canic.toml"))
            .expect("resolve config");

        assert_eq!(resolved, root.join("canisters/demo/canic.toml"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_config_error_lists_choices_when_project_default_missing() {
        with_guarded_env(|| {
            let root = unique_temp_dir("canic-install-config-choices");
            let demo = root.join("canisters/demo/canic.toml");
            let test = root.join("canisters/test/runtime_probe/canic.toml");
            fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
            fs::create_dir_all(test.parent().expect("test parent")).expect("create test parent");
            fs::create_dir_all(root.join("canisters/demo/root")).expect("create demo root");
            fs::write(
                &demo,
                r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#,
            )
            .expect("write demo config");
            fs::write(&test, "").expect("write test config");
            fs::write(root.join("canisters/demo/root/Cargo.toml"), "")
                .expect("write demo root manifest");
            let previous = env::var_os("CANIC_CONFIG_PATH");
            unsafe {
                env::remove_var("CANIC_CONFIG_PATH");
            }

            let err = resolve_install_config_path(&root, None).expect_err("selection must fail");
            let message = err.to_string();

            assert!(message.contains("missing default Canic config at canisters/canic.toml"));
            assert!(!message.contains("found one install config:"));
            assert!(message.contains("canisters/demo/canic.toml"));
            assert!(message.contains("2 (app, user_hub)"));
            assert!(message.contains("canisters/canic.toml\n\n#"));
            assert!(message.contains("2 (app, user_hub)\n\nrun:"));
            assert!(!message.contains("canisters/test/runtime_probe/canic.toml"));
            assert!(message.contains("run: canic install --config canisters/demo/canic.toml"));

            restore_env_var("CANIC_CONFIG_PATH", previous);
            fs::remove_dir_all(root).expect("clean temp dir");
        });
    }

    #[test]
    fn config_selection_error_is_whitespace_table() {
        let root = unique_temp_dir("canic-install-config-single-table");
        let config = root.join("canisters/demo/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(
            &config,
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
        )
        .expect("write config");
        let message = config_selection_error(
            &root,
            &root.join("canisters/canic.toml"),
            std::slice::from_ref(&config),
        );

        assert!(message.contains('#'));
        assert!(message.contains("CONFIG"));
        assert!(message.contains("CANISTERS"));
        assert!(message.contains("canisters/demo/canic.toml"));
        assert!(message.contains("1 (app)"));
        assert!(message.contains("canisters/canic.toml\n\n#"));
        assert!(message.contains("1 (app)\n\nrun:"));
        assert!(message.contains("run: canic install --config canisters/demo/canic.toml"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn config_selection_error_lists_multiple_paths_with_numbered_options() {
        let root = unique_temp_dir("canic-install-config-multiple-table");
        let demo = root.join("canisters/demo/canic.toml");
        let example = root.join("canisters/example/canic.toml");
        fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
        fs::create_dir_all(example.parent().expect("example parent"))
            .expect("create example parent");
        fs::write(
            &demo,
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
        )
        .expect("write demo config");
        fs::write(
            &example,
            r#"
[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_shard]
kind = "singleton"

[subnets.prime.canisters.scale]
kind = "singleton"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
"#,
        )
        .expect("write example config");
        let message =
            config_selection_error(&root, &root.join("canisters/canic.toml"), &[demo, example]);

        assert!(message.contains("choose a config path explicitly:"));
        assert!(message.contains("choose a config path explicitly:\n\n#"));
        assert!(message.contains('#'));
        assert!(message.contains("CONFIG"));
        assert!(message.contains("CANISTERS"));
        assert!(message.contains("1  canisters/demo/canic.toml"));
        assert!(message.contains("2  canisters/example/canic.toml"));
        assert!(message.contains("canisters/demo/canic.toml"));
        assert!(message.contains("1 (app)"));
        assert!(message.contains("canisters/example/canic.toml"));
        assert!(message.contains("4 (scale, scale_hub, user_hub, user_shard)"));
        assert!(message.contains("4 (scale, scale_hub, user_hub, user_shard)\n\nrun:"));
        assert!(message.contains("run: canic install --config <path>"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn discovered_install_config_choices_are_path_sorted() {
        let root = unique_temp_dir("canic-install-config-sorted");
        let alpha = root.join("alpha/canic.toml");
        let zeta = root.join("zeta/canic.toml");
        fs::create_dir_all(alpha.parent().expect("alpha parent").join("root"))
            .expect("create alpha root");
        fs::create_dir_all(zeta.parent().expect("zeta parent").join("root"))
            .expect("create zeta root");
        fs::write(&zeta, "").expect("write zeta config");
        fs::write(&alpha, "").expect("write alpha config");
        fs::write(
            alpha
                .parent()
                .expect("alpha parent")
                .join("root/Cargo.toml"),
            "",
        )
        .expect("write alpha root manifest");
        fs::write(
            zeta.parent().expect("zeta parent").join("root/Cargo.toml"),
            "",
        )
        .expect("write zeta root manifest");

        let choices = discover_canic_config_choices(&root).expect("discover choices");

        assert_eq!(choices, vec![alpha, zeta]);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn install_state_path_is_scoped_by_network() {
        assert_eq!(
            fleet_install_state_path(Path::new("/tmp/canic-project"), "local", "demo"),
            PathBuf::from("/tmp/canic-project/.canic/local/fleets/demo.json")
        );
        assert_eq!(
            current_fleet_path(Path::new("/tmp/canic-project"), "local"),
            PathBuf::from("/tmp/canic-project/.canic/local/current-fleet")
        );
    }

    #[test]
    fn install_state_round_trips_from_project_state_dir() {
        let root = unique_temp_dir("canic-install-state");
        let state = InstallState {
            schema_version: INSTALL_STATE_SCHEMA_VERSION,
            fleet: "demo".to_string(),
            installed_at_unix_secs: 42,
            network: "local".to_string(),
            root_target: "root".to_string(),
            root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
            root_build_target: "root".to_string(),
            workspace_root: root.display().to_string(),
            dfx_root: root.display().to_string(),
            config_path: root.join("canisters/canic.toml").display().to_string(),
            release_set_manifest_path: root
                .join(".dfx/local/canisters/root/root.release-set.json")
                .display()
                .to_string(),
        };

        let path = write_install_state(&root, "local", &state).expect("write state");
        let read_back = read_install_state(&root, "local")
            .expect("read state")
            .expect("state exists");
        let named = read_fleet_install_state(&root, "local", "demo")
            .expect("read named fleet")
            .expect("named fleet exists");
        let fleets = list_fleets(&root, "local").expect("list fleets");

        assert_eq!(path, root.join(".canic/local/fleets/demo.json"));
        assert_eq!(read_back, state);
        assert_eq!(named, state);
        assert_eq!(fleets.len(), 1);
        assert_eq!(fleets[0].name, "demo");
        assert!(fleets[0].current);

        fs::remove_dir_all(root).expect("clean temp dir");
    }

    fn write_temp_workspace_config(config_source: &str) -> PathBuf {
        let root = unique_temp_dir("canic-install-root-test");
        fs::create_dir_all(root.join("canisters")).expect("temp canisters dir must be created");
        fs::write(root.join("canisters/canic.toml"), config_source)
            .expect("temp canic.toml must be written");
        root
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic enough for test temp dir")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
    }

    fn with_guarded_env(run: impl FnOnce()) {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().expect("env lock poisoned");
        run();
    }

    fn restore_env_var(key: &str, previous: Option<std::ffi::OsString>) {
        unsafe {
            if let Some(value) = previous {
                env::set_var(key, value);
            } else {
                env::remove_var(key);
            }
        }
    }
}
