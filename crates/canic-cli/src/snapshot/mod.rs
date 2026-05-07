use crate::{
    args::{
        default_dfx, default_network, flag_arg, parse_matches, path_option, string_option,
        value_arg,
    },
    version_text,
};
use canic_backup::{
    discovery::parse_registry_entries,
    snapshot::{
        SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDownloadResult, SnapshotDriver,
        SnapshotDriverError, SnapshotLifecycleMode,
    },
    timestamp::current_timestamp_marker,
};
use canic_host::{
    dfx::{Dfx, DfxCommandError},
    install_root::read_current_or_fleet_install_state,
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

///
/// SnapshotCommandError
///

#[derive(Debug, ThisError)]
pub enum SnapshotCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("snapshot download needs --canister, --fleet, or a selected current fleet")]
    MissingSnapshotSource,

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("cannot combine --fleet root {fleet_root} with --root {root}")]
    ConflictingFleetRoot { fleet_root: String, root: String },

    #[error("canister {canister} is not a member of fleet {fleet}")]
    CanisterNotInFleet { fleet: String, canister: String },

    #[error("dfx command failed: {command}\n{stderr}")]
    DfxFailed { command: String, stderr: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("could not parse snapshot id from dfx output: {0}")]
    SnapshotIdUnavailable(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SnapshotDownload(#[from] SnapshotDownloadError),
}

///
/// SnapshotDownloadOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadOptions {
    pub canister: Option<String>,
    pub out: Option<PathBuf>,
    pub fleet: Option<String>,
    pub root: Option<String>,
    pub include_children: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub lifecycle: SnapshotLifecycleMode,
    pub network: Option<String>,
    pub dfx: String,
}

impl SnapshotDownloadOptions {
    /// Parse snapshot download options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, SnapshotCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(snapshot_download_command(), args)
            .map_err(|_| SnapshotCommandError::Usage(download_usage()))?;
        let recursive = matches.get_flag("recursive");
        let include_children = matches.get_flag("include-children") || recursive;

        Ok(Self {
            canister: string_option(&matches, "canister"),
            out: path_option(&matches, "out"),
            fleet: string_option(&matches, "fleet"),
            root: string_option(&matches, "root"),
            include_children,
            recursive,
            dry_run: matches.get_flag("dry-run"),
            lifecycle: SnapshotLifecycleMode::from_resume_flag(
                matches.get_flag("resume-after-snapshot"),
            ),
            network: string_option(&matches, "network"),
            dfx: string_option(&matches, "dfx").unwrap_or_else(default_dfx),
        })
    }
}

// Build the snapshot download parser.
fn snapshot_download_command() -> ClapCommand {
    ClapCommand::new("download")
        .bin_name("canic snapshot download")
        .about("Download canister snapshots for one canister or subtree")
        .disable_help_flag(true)
        .arg(value_arg("canister").long("canister").value_name("id"))
        .arg(
            value_arg("fleet")
                .long("fleet")
                .value_name("name")
                .help("Backup a named installed fleet; omit to use the current fleet"),
        )
        .arg(
            value_arg("out")
                .long("out")
                .value_name("dir")
                .help("Backup output directory; defaults to backups/fleet-<name>-YYYYMMDD-HHMMSS"),
        )
        .arg(value_arg("root").long("root").value_name("id"))
        .arg(flag_arg("include-children").long("include-children"))
        .arg(flag_arg("recursive").long("recursive"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("resume-after-snapshot").long("resume-after-snapshot"))
        .arg(value_arg("network").long("network").value_name("name"))
        .arg(value_arg("dfx").long("dfx").value_name("path"))
}

/// Run a snapshot subcommand.
pub fn run<I>(args: I) -> Result<(), SnapshotCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(SnapshotCommandError::Usage(usage()));
    };

    match command.as_str() {
        "download" => {
            let options = SnapshotDownloadOptions::parse(args)?;
            let result = download_snapshots(&options)?;
            for command in result.planned_commands {
                println!("{command}");
            }
            for artifact in result.artifacts {
                println!(
                    "{} {} {}",
                    artifact.canister_id,
                    artifact.snapshot_id,
                    artifact.path.display()
                );
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
        _ => Err(SnapshotCommandError::UnknownOption(command)),
    }
}

/// Create and download snapshots for the selected canister set.
pub fn download_snapshots(
    options: &SnapshotDownloadOptions,
) -> Result<SnapshotDownloadResult, SnapshotCommandError> {
    let request = resolve_snapshot_download_request(options)?;
    validate_fleet_selection_if_needed(&request)?;

    let config = SnapshotDownloadConfig {
        canister: request.canister.clone(),
        out: request.out.clone(),
        root: request.root.clone(),
        include_children: request.include_children,
        recursive: request.recursive,
        dry_run: request.dry_run,
        lifecycle: request.lifecycle,
        backup_id: backup_id(&request),
        created_at: current_timestamp_marker(),
        tool_name: "canic-cli".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        environment: request
            .network
            .clone()
            .unwrap_or_else(|| "local".to_string()),
    };
    let mut driver = DfxSnapshotDriver { request: &request };
    canic_backup::snapshot::download_snapshots(&config, &mut driver)
        .map_err(SnapshotCommandError::from)
}

///
/// ResolvedSnapshotDownload
///

#[expect(
    clippy::struct_excessive_bools,
    reason = "resolved CLI request mirrors snapshot flags before passing them to backup config"
)]
struct ResolvedSnapshotDownload {
    canister: String,
    out: PathBuf,
    fleet: Option<String>,
    explicit_canister: bool,
    root: Option<String>,
    include_children: bool,
    recursive: bool,
    dry_run: bool,
    lifecycle: SnapshotLifecycleMode,
    network: Option<String>,
    dfx: String,
}

// Resolve fleet-aware defaults into the explicit backup contract used downstream.
fn resolve_snapshot_download_request(
    options: &SnapshotDownloadOptions,
) -> Result<ResolvedSnapshotDownload, SnapshotCommandError> {
    let network = state_network(options.network.as_deref());
    let state = if options.fleet.is_some() || options.canister.is_none() {
        read_current_or_fleet_install_state(&network, options.fleet.as_deref())
            .map_err(|err| SnapshotCommandError::InstallState(err.to_string()))?
    } else {
        None
    };
    let explicit_canister = options.canister.is_some();
    let canister = options
        .canister
        .clone()
        .or_else(|| state.as_ref().map(|state| state.root_canister_id.clone()))
        .ok_or(SnapshotCommandError::MissingSnapshotSource)?;
    let fleet = state
        .as_ref()
        .map(|state| state.fleet.clone())
        .or_else(|| options.fleet.clone());
    let root = resolved_snapshot_root(options, state.as_ref())?;
    let recursive = if !explicit_canister && state.is_some() {
        true
    } else {
        options.recursive
    };
    let include_children = options.include_children || recursive;
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| default_snapshot_output_path(fleet.as_deref().unwrap_or(&canister)));

    Ok(ResolvedSnapshotDownload {
        canister,
        out,
        fleet,
        explicit_canister,
        root,
        include_children,
        recursive,
        dry_run: options.dry_run,
        lifecycle: options.lifecycle,
        network: options.network.clone(),
        dfx: options.dfx.clone(),
    })
}

// Resolve the registry root, making fleet state authoritative when selected.
fn resolved_snapshot_root(
    options: &SnapshotDownloadOptions,
    state: Option<&canic_host::install_root::InstallState>,
) -> Result<Option<String>, SnapshotCommandError> {
    let Some(state) = state else {
        return Ok(options.root.clone());
    };

    if let Some(root) = &options.root
        && root != &state.root_canister_id
    {
        return Err(SnapshotCommandError::ConflictingFleetRoot {
            fleet_root: state.root_canister_id.clone(),
            root: root.clone(),
        });
    }

    Ok(Some(state.root_canister_id.clone()))
}

// Validate explicit --canister selections against the selected fleet before mutation.
fn validate_fleet_selection_if_needed(
    request: &ResolvedSnapshotDownload,
) -> Result<(), SnapshotCommandError> {
    if !request.explicit_canister {
        return Ok(());
    }
    let Some(fleet) = &request.fleet else {
        return Ok(());
    };
    let Some(root) = &request.root else {
        return Ok(());
    };

    let registry_json = call_subnet_registry(request, root)?;
    validate_fleet_membership_json(fleet, &request.canister, &registry_json)
}

// Reject explicit canister selections that are not present in a fleet registry.
fn validate_fleet_membership_json(
    fleet: &str,
    canister: &str,
    registry_json: &str,
) -> Result<(), SnapshotCommandError> {
    let entries = parse_registry_entries(registry_json)
        .map_err(|err| SnapshotCommandError::SnapshotDownload(err.into()))?;
    if entries.iter().any(|entry| entry.pid == canister) {
        return Ok(());
    }

    Err(SnapshotCommandError::CanisterNotInFleet {
        fleet: fleet.to_string(),
        canister: canister.to_string(),
    })
}

// Build the default snapshot output path from current fleet state or the selected canister.
fn default_snapshot_output_path(label: &str) -> PathBuf {
    let marker = current_backup_directory_stamp();

    PathBuf::from("backups").join(format!("fleet-{}-{marker}", file_safe_component(label)))
}

// Resolve the selected state network consistently with other fleet commands.
fn state_network(network: Option<&str>) -> String {
    network.map_or_else(default_network, str::to_string)
}

// Return a compact UTC timestamp for human-readable backup directories.
fn current_backup_directory_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());

    backup_directory_stamp_from_unix(seconds)
}

// Format unix seconds as a compact UTC timestamp without leaking unix terminology.
fn backup_directory_stamp_from_unix(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}

// Convert days since 1970-01-01 into a proleptic Gregorian UTC date.
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + (month <= 2) as i64;

    (year, month, day)
}

// Keep generated path components portable across shells and filesystems.
fn file_safe_component(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned.to_string()
    }
}

///
/// DfxSnapshotDriver
///

struct DfxSnapshotDriver<'a> {
    request: &'a ResolvedSnapshotDownload,
}

impl SnapshotDriver for DfxSnapshotDriver<'_> {
    /// Load the root registry JSON via `dfx canister call`.
    fn registry_json(&mut self, root: &str) -> Result<String, SnapshotDriverError> {
        call_subnet_registry(self.request, root).map_err(driver_error)
    }

    /// Create a canister snapshot via DFX.
    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError> {
        create_snapshot(self.request, canister_id).map_err(driver_error)
    }

    /// Stop a canister via DFX.
    fn stop_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        stop_canister(self.request, canister_id).map_err(driver_error)
    }

    /// Start a canister via DFX.
    fn start_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        start_canister(self.request, canister_id).map_err(driver_error)
    }

    /// Download a canister snapshot via DFX.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError> {
        download_snapshot(self.request, canister_id, snapshot_id, artifact_path)
            .map_err(driver_error)
    }

    /// Render the planned create command for dry runs.
    fn create_snapshot_command(&self, canister_id: &str) -> String {
        create_snapshot_command_display(self.request, canister_id)
    }

    /// Render the planned stop command for dry runs.
    fn stop_canister_command(&self, canister_id: &str) -> String {
        stop_canister_command_display(self.request, canister_id)
    }

    /// Render the planned start command for dry runs.
    fn start_canister_command(&self, canister_id: &str) -> String {
        start_canister_command_display(self.request, canister_id)
    }

    /// Render the planned download command for dry runs.
    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String {
        download_snapshot_command_display(self.request, canister_id, snapshot_id, artifact_path)
    }
}

// Box a CLI command error for the backup snapshot driver boundary.
fn driver_error(error: SnapshotCommandError) -> SnapshotDriverError {
    Box::new(error)
}

// Build the shared host dfx context for snapshot commands.
fn dfx(request: &ResolvedSnapshotDownload) -> Dfx {
    Dfx::new(&request.dfx, request.network.clone())
}

// Convert host dfx failures into the snapshot command's public error surface.
fn snapshot_dfx_error(error: DfxCommandError) -> SnapshotCommandError {
    match error {
        DfxCommandError::Io(err) => SnapshotCommandError::Io(err),
        DfxCommandError::Failed { command, stderr } => {
            SnapshotCommandError::DfxFailed { command, stderr }
        }
        DfxCommandError::SnapshotIdUnavailable { output } => {
            SnapshotCommandError::SnapshotIdUnavailable(output)
        }
    }
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(
    request: &ResolvedSnapshotDownload,
    root: &str,
) -> Result<String, SnapshotCommandError> {
    dfx(request)
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(snapshot_dfx_error)
}

// Create one canister snapshot and return the host-resolved snapshot id.
fn create_snapshot(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<String, SnapshotCommandError> {
    dfx(request)
        .snapshot_create_id(canister_id)
        .map_err(snapshot_dfx_error)
}

// Stop a canister before taking a snapshot when explicitly requested.
fn stop_canister(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    dfx(request)
        .stop_canister(canister_id)
        .map_err(snapshot_dfx_error)
}

// Start a canister after snapshot capture when explicitly requested.
fn start_canister(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    dfx(request)
        .start_canister(canister_id)
        .map_err(snapshot_dfx_error)
}

// Download one canister snapshot into the target artifact directory.
fn download_snapshot(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> Result<(), SnapshotCommandError> {
    dfx(request)
        .snapshot_download(canister_id, snapshot_id, artifact_path)
        .map_err(snapshot_dfx_error)
}

// Render one dry-run create command.
fn create_snapshot_command_display(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> String {
    dfx(request).snapshot_create_display(canister_id)
}

// Render one dry-run download command.
fn download_snapshot_command_display(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> String {
    dfx(request).snapshot_download_display(canister_id, snapshot_id, artifact_path)
}

// Render one dry-run stop command.
fn stop_canister_command_display(request: &ResolvedSnapshotDownload, canister_id: &str) -> String {
    dfx(request).stop_canister_display(canister_id)
}

// Render one dry-run start command.
fn start_canister_command_display(request: &ResolvedSnapshotDownload, canister_id: &str) -> String {
    dfx(request).start_canister_display(canister_id)
}

// Build a stable backup id for this command's output directory.
fn backup_id(request: &ResolvedSnapshotDownload) -> String {
    request
        .out
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "snapshot-download".to_string(), str::to_string)
}

// Return snapshot command usage text.
fn usage() -> String {
    let mut command = snapshot_command();
    command.render_help().to_string()
}

// Return snapshot download usage text.
fn download_usage() -> String {
    let mut command = snapshot_download_command();
    command.render_help().to_string()
}

// Build the snapshot command-family parser for help rendering.
fn snapshot_command() -> ClapCommand {
    ClapCommand::new("snapshot")
        .bin_name("canic snapshot")
        .about("Capture and download canister snapshots")
        .disable_help_flag(true)
        .subcommand(
            ClapCommand::new("download")
                .about("Download canister snapshots for one canister or subtree")
                .disable_help_flag(true),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "aaaaa-aa";

    // Ensure option parsing covers the intended dry-run command.
    #[test]
    fn parses_download_options() {
        let options = SnapshotDownloadOptions::parse([
            OsString::from("--canister"),
            OsString::from(ROOT),
            OsString::from("--out"),
            OsString::from("backups/test"),
            OsString::from("--root"),
            OsString::from(ROOT),
            OsString::from("--recursive"),
            OsString::from("--dry-run"),
            OsString::from("--resume-after-snapshot"),
        ])
        .expect("parse options");

        assert_eq!(options.canister.as_deref(), Some(ROOT));
        assert_eq!(options.out.as_deref(), Some(Path::new("backups/test")));
        assert!(options.include_children);
        assert!(options.recursive);
        assert!(options.dry_run);
        assert_eq!(options.root.as_deref(), Some(ROOT));
        assert_eq!(options.lifecycle, SnapshotLifecycleMode::StopAndResume);
    }

    // Ensure --out can be omitted for the common current-fleet backup flow.
    #[test]
    fn download_options_default_output_directory() {
        let options = SnapshotDownloadOptions::parse([
            OsString::from("--canister"),
            OsString::from(ROOT),
            OsString::from("--recursive"),
        ])
        .expect("parse options");
        let request = resolve_snapshot_download_request(&options).expect("resolve request");
        let out = request.out.to_string_lossy();

        assert!(out.starts_with("backups/fleet-"));
        assert!(out.chars().last().is_some_and(|last| last.is_ascii_digit()));
    }

    // Ensure a named fleet can be selected without spelling out its root canister.
    #[test]
    fn parses_download_fleet_options_without_canister() {
        let options = SnapshotDownloadOptions::parse([
            OsString::from("--fleet"),
            OsString::from("demo"),
            OsString::from("--dry-run"),
        ])
        .expect("parse options");

        assert_eq!(options.fleet.as_deref(), Some("demo"));
        assert_eq!(options.canister, None);
        assert!(options.dry_run);
    }

    // Ensure explicit fleet/canister selections fail when the registry omits the canister.
    #[test]
    fn fleet_membership_rejects_unknown_canister() {
        let registry = serde_json::json!({
            "Ok": [
                {
                    "pid": ROOT,
                    "role": "root",
                    "record": { "parent_pid": null }
                }
            ]
        })
        .to_string();
        let err = validate_fleet_membership_json("demo", "missing-cai", &registry)
            .expect_err("missing canister should reject");

        assert!(matches!(
            err,
            SnapshotCommandError::CanisterNotInFleet { fleet, canister }
                if fleet == "demo" && canister == "missing-cai"
        ));
    }

    // Ensure generated default path labels are filesystem-friendly.
    #[test]
    fn snapshot_default_path_sanitizes_labels() {
        assert_eq!(file_safe_component("demo fleet/root"), "demo-fleet-root");
    }

    // Ensure default backup directory timestamps are compact calendar labels.
    #[test]
    fn backup_directory_stamp_uses_calendar_time() {
        assert_eq!(backup_directory_stamp_from_unix(0), "19700101-000000");
        assert_eq!(
            backup_directory_stamp_from_unix(1_715_090_400),
            "20240507-140000"
        );
    }
}
