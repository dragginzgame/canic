use crate::version_text;
use canic_backup::snapshot::{
    SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDownloadResult, SnapshotDriver,
    SnapshotDriverError, SnapshotLifecycleMode,
};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error as ThisError;

///
/// SnapshotCommandError
///

#[derive(Debug, ThisError)]
pub enum SnapshotCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error("dfx command failed: {command}\n{stderr}")]
    DfxFailed { command: String, stderr: String },

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
    pub canister: String,
    pub out: PathBuf,
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
        let mut canister = None;
        let mut out = None;
        let mut root = None;
        let mut include_children = false;
        let mut recursive = false;
        let mut dry_run = false;
        let mut stop_before_snapshot = false;
        let mut resume_after_snapshot = false;
        let mut network = None;
        let mut dfx = "dfx".to_string();

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| SnapshotCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--canister" => canister = Some(next_value(&mut args, "--canister")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--root" => root = Some(next_value(&mut args, "--root")?),
                "--include-children" => include_children = true,
                "--recursive" => {
                    recursive = true;
                    include_children = true;
                }
                "--dry-run" => dry_run = true,
                "--stop-before-snapshot" => stop_before_snapshot = true,
                "--resume-after-snapshot" => resume_after_snapshot = true,
                "--network" => network = Some(next_value(&mut args, "--network")?),
                "--dfx" => dfx = next_value(&mut args, "--dfx")?,
                "--help" | "-h" => return Err(SnapshotCommandError::Usage(usage())),
                _ => return Err(SnapshotCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            canister: canister.ok_or(SnapshotCommandError::MissingOption("--canister"))?,
            out: out.ok_or(SnapshotCommandError::MissingOption("--out"))?,
            root,
            include_children,
            recursive,
            dry_run,
            lifecycle: SnapshotLifecycleMode::from_flags(
                stop_before_snapshot,
                resume_after_snapshot,
            ),
            network,
            dfx,
        })
    }
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
    let config = SnapshotDownloadConfig {
        canister: options.canister.clone(),
        out: options.out.clone(),
        root: options.root.clone(),
        include_children: options.include_children,
        recursive: options.recursive,
        dry_run: options.dry_run,
        lifecycle: options.lifecycle,
        backup_id: backup_id(options),
        created_at: timestamp_placeholder(),
        tool_name: "canic-cli".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        environment: options
            .network
            .clone()
            .unwrap_or_else(|| "local".to_string()),
    };
    let mut driver = DfxSnapshotDriver { options };
    canic_backup::snapshot::download_snapshots(&config, &mut driver)
        .map_err(SnapshotCommandError::from)
}

///
/// DfxSnapshotDriver
///

struct DfxSnapshotDriver<'a> {
    options: &'a SnapshotDownloadOptions,
}

impl SnapshotDriver for DfxSnapshotDriver<'_> {
    /// Load the root registry JSON via `dfx canister call`.
    fn registry_json(&mut self, root: &str) -> Result<String, SnapshotDriverError> {
        call_subnet_registry(self.options, root).map_err(driver_error)
    }

    /// Create a canister snapshot via DFX.
    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError> {
        create_snapshot(self.options, canister_id).map_err(driver_error)
    }

    /// Stop a canister via DFX.
    fn stop_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        stop_canister(self.options, canister_id).map_err(driver_error)
    }

    /// Start a canister via DFX.
    fn start_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        start_canister(self.options, canister_id).map_err(driver_error)
    }

    /// Download a canister snapshot via DFX.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError> {
        download_snapshot(self.options, canister_id, snapshot_id, artifact_path)
            .map_err(driver_error)
    }

    /// Render the planned create command for dry runs.
    fn create_snapshot_command(&self, canister_id: &str) -> String {
        create_snapshot_command_display(self.options, canister_id)
    }

    /// Render the planned stop command for dry runs.
    fn stop_canister_command(&self, canister_id: &str) -> String {
        stop_canister_command_display(self.options, canister_id)
    }

    /// Render the planned start command for dry runs.
    fn start_canister_command(&self, canister_id: &str) -> String {
        start_canister_command_display(self.options, canister_id)
    }

    /// Render the planned download command for dry runs.
    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String {
        download_snapshot_command_display(self.options, canister_id, snapshot_id, artifact_path)
    }
}

// Box a CLI command error for the backup snapshot driver boundary.
fn driver_error(error: SnapshotCommandError) -> SnapshotDriverError {
    Box::new(error)
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(
    options: &SnapshotDownloadOptions,
    root: &str,
) -> Result<String, SnapshotCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["call", root, "canic_subnet_registry", "--output", "json"]);
    run_output(&mut command)
}

// Create one canister snapshot and parse the snapshot id from dfx output.
fn create_snapshot(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<String, SnapshotCommandError> {
    let before = list_snapshot_ids(options, canister_id)?;
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "create", canister_id]);
    let output = run_output_with_stderr(&mut command)?;
    if let Some(snapshot_id) = parse_snapshot_id(&output) {
        return Ok(snapshot_id);
    }

    let before = before.into_iter().collect::<BTreeSet<_>>();
    let mut new_ids = list_snapshot_ids(options, canister_id)?
        .into_iter()
        .filter(|snapshot_id| !before.contains(snapshot_id))
        .collect::<Vec<_>>();
    if new_ids.len() == 1 {
        Ok(new_ids.remove(0))
    } else {
        Err(SnapshotCommandError::SnapshotIdUnavailable(output))
    }
}

// List the existing snapshot ids for one canister.
fn list_snapshot_ids(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<Vec<String>, SnapshotCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "list", canister_id]);
    let output = run_output(&mut command)?;
    Ok(parse_snapshot_list_ids(&output))
}

// Stop a canister before taking a snapshot when explicitly requested.
fn stop_canister(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["stop", canister_id]);
    run_status(&mut command)
}

// Start a canister after snapshot capture when explicitly requested.
fn start_canister(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["start", canister_id]);
    run_status(&mut command)
}

// Download one canister snapshot into the target artifact directory.
fn download_snapshot(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> Result<(), SnapshotCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "download", canister_id, snapshot_id, "--dir"]);
    command.arg(artifact_path);
    run_status(&mut command)
}

// Add optional `dfx canister` network arguments.
fn add_canister_network_args(command: &mut Command, options: &SnapshotDownloadOptions) {
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
}

// Execute a command and capture stdout.
fn run_output(command: &mut Command) -> Result<String, SnapshotCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(SnapshotCommandError::DfxFailed {
            command: display,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

// Execute a command and capture stdout plus stderr on success.
fn run_output_with_stderr(command: &mut Command) -> Result<String, SnapshotCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(text.trim().to_string())
    } else {
        Err(SnapshotCommandError::DfxFailed {
            command: display,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

// Execute a command and require a successful status.
fn run_status(command: &mut Command) -> Result<(), SnapshotCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SnapshotCommandError::DfxFailed {
            command: display,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

// Render a command for diagnostics.
fn command_display(command: &Command) -> String {
    let mut parts = vec![command.get_program().to_string_lossy().to_string()];
    parts.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string()),
    );
    parts.join(" ")
}

// Render one dry-run create command.
fn create_snapshot_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "create", canister_id]);
    command_display(&command)
}

// Render one dry-run download command.
fn download_snapshot_command_display(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> String {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "download", canister_id, snapshot_id, "--dir"]);
    command.arg(artifact_path);
    command_display(&command)
}

// Render one dry-run stop command.
fn stop_canister_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["stop", canister_id]);
    command_display(&command)
}

// Render one dry-run start command.
fn start_canister_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["start", canister_id]);
    command_display(&command)
}

// Parse a likely snapshot id from dfx output.
fn parse_snapshot_id(output: &str) -> Option<String> {
    output
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ':' | ','))
        .filter(|part| !part.is_empty())
        .rev()
        .find(|part| {
            part.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
        .map(str::to_string)
}

// Parse dfx snapshot list output into snapshot ids.
fn parse_snapshot_list_ids(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            line.split_once(':')
                .map(|(snapshot_id, _)| snapshot_id.trim())
        })
        .filter(|snapshot_id| !snapshot_id.is_empty())
        .map(str::to_string)
        .collect()
}

// Build a stable backup id for this command's output directory.
fn backup_id(options: &SnapshotDownloadOptions) -> String {
    options
        .out
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "snapshot-download".to_string(), str::to_string)
}

// Return a placeholder timestamp until the CLI owns a clock abstraction.
fn timestamp_placeholder() -> String {
    "unknown".to_string()
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, SnapshotCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(SnapshotCommandError::MissingValue(option))
}

// Return snapshot command usage text.
const fn usage() -> &'static str {
    "usage: canic snapshot download --canister <id> --out <dir> [--root <id>] [--include-children] [--recursive] [--dry-run] [--stop-before-snapshot] [--resume-after-snapshot] [--network <name>]"
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "aaaaa-aa";

    // Ensure snapshot ids can be extracted from common command output.
    #[test]
    fn parses_snapshot_id_from_output() {
        let snapshot_id = parse_snapshot_id("Created snapshot: snap_abc-123\n");

        assert_eq!(snapshot_id.as_deref(), Some("snap_abc-123"));
    }

    // Ensure dfx snapshot list output can be used when create is quiet.
    #[test]
    fn parses_snapshot_ids_from_list_output() {
        let snapshot_ids = parse_snapshot_list_ids(
            "0000000000000000ffffffffff9000050101: 213.76 MiB, taken at 2026-05-03 12:20:53 UTC\n",
        );

        assert_eq!(snapshot_ids, vec!["0000000000000000ffffffffff9000050101"]);
    }

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
            OsString::from("--stop-before-snapshot"),
            OsString::from("--resume-after-snapshot"),
        ])
        .expect("parse options");

        assert_eq!(options.canister, ROOT);
        assert!(options.include_children);
        assert!(options.recursive);
        assert!(options.dry_run);
        assert_eq!(options.root.as_deref(), Some(ROOT));
        assert_eq!(options.lifecycle, SnapshotLifecycleMode::StopAndResume);
    }
}
