use super::InstallTimingSummary;
use crate::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
    current_workspace_build_context_once,
};
use crate::format::{cycles_tc, wasm_size_label};
use crate::icp::{self, CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV};
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, configured_local_root_create_cycles, icp_query_on_network,
};
use crate::replica_query;
use crate::response_parse::parse_cycle_balance_response;
use crate::table::{ColumnAlign, render_separator, render_table, render_table_row, table_widths};
use canic_core::{
    cdk::{types::Principal, utils::hash::wasm_hash},
    protocol,
};
use serde_json::Value as JsonValue;
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

pub(super) fn parse_created_canister_id(output: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<JsonValue>(output) {
        return parse_canister_id_json(&value);
    }

    output
        .lines()
        .map(str::trim)
        .find(|line| Principal::from_text(*line).is_ok())
        .map(ToString::to_string)
}

pub(super) fn parse_canister_id_json(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(text) if Principal::from_text(text).is_ok() => Some(text.clone()),
        JsonValue::Array(values) => values.iter().find_map(parse_canister_id_json),
        JsonValue::Object(object) => ["canister_id", "id", "principal"]
            .iter()
            .filter_map(|key| object.get(*key))
            .find_map(parse_canister_id_json),
        _ => None,
    }
}

pub(super) fn add_create_root_target(command: &mut Command, root_canister: &str) {
    if env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV).is_some() {
        command.args(["create", "--detached", "--json"]);
    } else {
        command.args(["create", root_canister, "--json"]);
    }
}

pub(super) fn is_missing_canister_id_error(message: &str) -> bool {
    message.contains("failed to lookup canister ID")
        || message.contains("could not find ID for canister")
        || message.contains("Canister ID is missing")
}

pub(super) fn root_init_args(root_wasm: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let wasm = fs::read(root_wasm)?;
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

pub(super) fn run_canic_build_targets(
    network: &str,
    targets: &[String],
    build_profile: Option<CanisterBuildProfile>,
    config_path: &Path,
    icp_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let _env = BuildEnvGuard::apply(network, config_path, icp_root);
    let profile = build_profile.unwrap_or_else(CanisterBuildProfile::current);
    if let Some(context) = current_workspace_build_context_once(profile)? {
        for line in context.lines() {
            println!("{line}");
        }
        println!("config: {}", config_path.display());
        println!(
            "artifacts: {}",
            planned_build_artifact_root(icp_root).display()
        );
        println!();
    }

    fs::create_dir_all(planned_build_artifact_root(icp_root))?;
    println!("Building {} canisters", targets.len());
    println!();
    let headers = ["CANISTER", "PROGRESS", "WASM", "ELAPSED"];
    let planned_rows = targets
        .iter()
        .map(|target| {
            [
                target.clone(),
                progress_bar(targets.len(), targets.len(), 10),
                "000.00 MiB (gz 000.00 MiB)".to_string(),
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
        let artifact_size = wasm_artifact_size(&output.wasm_path, &output.wasm_gz_path)?;

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

pub(super) fn planned_build_artifact_root(icp_root: &Path) -> PathBuf {
    icp_root.join(".icp/local/canisters")
}

fn wasm_artifact_size(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let wasm_bytes = Some(fs::metadata(wasm_path)?.len());
    let gzip_bytes = fs::metadata(wasm_gz_path)
        .ok()
        .map(|metadata| metadata.len());
    Ok(wasm_size_label(wasm_bytes, gzip_bytes))
}

pub(super) struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_config_path: Option<OsString>,
    previous_icp_root: Option<OsString>,
    previous_local_network_url: Option<OsString>,
    previous_local_root_key: Option<OsString>,
}

impl BuildEnvGuard {
    pub(super) fn apply(network: &str, config_path: &Path, icp_root: &Path) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_config_path: env::var_os("CANIC_CONFIG_PATH"),
            previous_icp_root: env::var_os("CANIC_ICP_ROOT"),
            previous_local_network_url: env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV),
            previous_local_root_key: env::var_os(CANIC_ICP_LOCAL_ROOT_KEY_ENV),
        };
        set_env("ICP_ENVIRONMENT", network);
        set_env("CANIC_CONFIG_PATH", config_path);
        set_env("CANIC_ICP_ROOT", icp_root);
        if let Some(target) = local_replica_icp_target(network, icp_root) {
            set_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV, target.url);
            set_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV, target.root_key);
        } else {
            remove_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
            remove_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV);
        }
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
        restore_env("CANIC_CONFIG_PATH", self.previous_config_path.take());
        restore_env("CANIC_ICP_ROOT", self.previous_icp_root.take());
        restore_env(
            CANIC_ICP_LOCAL_NETWORK_URL_ENV,
            self.previous_local_network_url.take(),
        );
        restore_env(
            CANIC_ICP_LOCAL_ROOT_KEY_ENV,
            self.previous_local_root_key.take(),
        );
    }
}

struct LocalReplicaIcpTarget {
    url: String,
    root_key: String,
}

fn local_replica_icp_target(network: &str, icp_root: &Path) -> Option<LocalReplicaIcpTarget> {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return None;
    }
    if icp_ping(icp_root, network).unwrap_or(false) {
        return None;
    }
    let root_key = replica_query::local_replica_root_key_from_root(Some(network), icp_root)
        .ok()
        .flatten()?;
    Some(LocalReplicaIcpTarget {
        url: replica_query::local_replica_endpoint_from_root(Some(network), icp_root),
        root_key,
    })
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

fn remove_env<K>(key: K)
where
    K: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::remove_var(key);
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    // See set_env: this restores the single-threaded install build context.
    if let Some(value) = value {
        set_env(key, value);
    } else {
        remove_env(key);
    }
}

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

pub(super) fn ensure_icp_environment_ready(
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(icp_root, network)? {
        return Ok(());
    }
    if replica_query::should_use_local_replica_query(Some(network))
        && replica_query::local_replica_status_reachable_from_root(Some(network), icp_root)
    {
        println!(
            "Replica reachable via HTTP status endpoint even though ICP CLI reports network '{network}' stopped; continuing from ICP root {}.",
            icp_root.display()
        );
        return Ok(());
    }

    Err(format!(
        "icp environment is not running for network '{network}'\nStart the target replica in another terminal with `canic replica start` and rerun."
    )
    .into())
}

fn icp_ping(icp_root: &Path, network: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut command = icp::default_command_in(icp_root);
    command.args(["network", "ping", network]);
    Ok(icp::run_success(&mut command)?)
}

pub(super) fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{}", render_install_timing_summary(timings, total));
}

pub(super) fn render_install_timing_summary(
    timings: &InstallTimingSummary,
    total: Duration,
) -> String {
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

pub(super) fn print_install_result_summary(
    network: &str,
    deployment: &str,
    fleet_template: &str,
    state_path: &Path,
) {
    println!("Install result:");
    println!("{:<14} success", "status");
    println!("{:<14} {}", "deployment", deployment);
    println!("{:<14} {}", "fleet_template", fleet_template);
    println!("{:<14} {}", "install_state", state_path.display());
    println!(
        "{:<14} canic list {} --network {}",
        "smoke_check", deployment, network
    );
}

pub(super) fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    icp::run_status(command).map_err(Into::into)
}

pub(super) fn run_command_stdout(
    command: &mut Command,
) -> Result<String, Box<dyn std::error::Error>> {
    icp::run_output(command).map_err(Into::into)
}

pub(super) fn icp_command_on_network(network: &str) -> Command {
    let mut command = icp::default_command();
    command.env("ICP_ENVIRONMENT", network);
    command
}

pub(super) fn icp_command_in_network(icp_root: &Path, network: &str) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.env("ICP_ENVIRONMENT", network);
    command
}

pub(super) fn icp_canister_command_in_network(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

pub(super) fn add_icp_environment_target(command: &mut Command, network: &str) {
    icp::add_target_args(command, Some(network), None);
}
