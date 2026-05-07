use crate::{
    args::{flag_arg, parse_matches, path_option, string_option, value_arg},
    version_text,
};
use canic_backup::{
    snapshot::{
        SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDownloadResult, SnapshotDriver,
        SnapshotDriverError, SnapshotLifecycleMode,
    },
    timestamp::current_timestamp_marker,
};
use canic_host::dfx::{Dfx, DfxCommandError};
use clap::Command as ClapCommand;
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
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
        let matches = parse_matches(snapshot_download_command(), args)
            .map_err(|_| SnapshotCommandError::Usage(usage()))?;
        let recursive = matches.get_flag("recursive");
        let include_children = matches.get_flag("include-children") || recursive;

        Ok(Self {
            canister: string_option(&matches, "canister")
                .ok_or(SnapshotCommandError::MissingOption("--canister"))?,
            out: path_option(&matches, "out")
                .ok_or(SnapshotCommandError::MissingOption("--out"))?,
            root: string_option(&matches, "root"),
            include_children,
            recursive,
            dry_run: matches.get_flag("dry-run"),
            lifecycle: SnapshotLifecycleMode::from_resume_flag(
                matches.get_flag("resume-after-snapshot"),
            ),
            network: string_option(&matches, "network"),
            dfx: string_option(&matches, "dfx").unwrap_or_else(|| "dfx".to_string()),
        })
    }
}

// Build the snapshot download parser.
fn snapshot_download_command() -> ClapCommand {
    ClapCommand::new("snapshot-download")
        .disable_help_flag(true)
        .arg(value_arg("canister").long("canister"))
        .arg(value_arg("out").long("out"))
        .arg(value_arg("root").long("root"))
        .arg(flag_arg("include-children").long("include-children"))
        .arg(flag_arg("recursive").long("recursive"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("resume-after-snapshot").long("resume-after-snapshot"))
        .arg(value_arg("network").long("network"))
        .arg(value_arg("dfx").long("dfx"))
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
        created_at: current_timestamp_marker(),
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

// Build the shared host dfx context for snapshot commands.
fn dfx(options: &SnapshotDownloadOptions) -> Dfx {
    Dfx::new(&options.dfx, options.network.clone())
}

// Convert host dfx failures into the snapshot command's public error surface.
fn snapshot_dfx_error(error: DfxCommandError) -> SnapshotCommandError {
    match error {
        DfxCommandError::Io(err) => SnapshotCommandError::Io(err),
        DfxCommandError::Failed { command, stderr } => {
            SnapshotCommandError::DfxFailed { command, stderr }
        }
    }
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(
    options: &SnapshotDownloadOptions,
    root: &str,
) -> Result<String, SnapshotCommandError> {
    dfx(options)
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(snapshot_dfx_error)
}

// Create one canister snapshot and parse the snapshot id from dfx output.
fn create_snapshot(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<String, SnapshotCommandError> {
    let before = list_snapshot_ids(options, canister_id)?;
    let output = dfx(options)
        .snapshot_create(canister_id)
        .map_err(snapshot_dfx_error)?;
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
    let output = dfx(options)
        .snapshot_list(canister_id)
        .map_err(snapshot_dfx_error)?;
    Ok(parse_snapshot_list_ids(&output))
}

// Stop a canister before taking a snapshot when explicitly requested.
fn stop_canister(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    dfx(options)
        .stop_canister(canister_id)
        .map_err(snapshot_dfx_error)
}

// Start a canister after snapshot capture when explicitly requested.
fn start_canister(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    dfx(options)
        .start_canister(canister_id)
        .map_err(snapshot_dfx_error)
}

// Download one canister snapshot into the target artifact directory.
fn download_snapshot(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> Result<(), SnapshotCommandError> {
    dfx(options)
        .snapshot_download(canister_id, snapshot_id, artifact_path)
        .map_err(snapshot_dfx_error)
}

// Render one dry-run create command.
fn create_snapshot_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    dfx(options).snapshot_create_display(canister_id)
}

// Render one dry-run download command.
fn download_snapshot_command_display(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> String {
    dfx(options).snapshot_download_display(canister_id, snapshot_id, artifact_path)
}

// Render one dry-run stop command.
fn stop_canister_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    dfx(options).stop_canister_display(canister_id)
}

// Render one dry-run start command.
fn start_canister_command_display(options: &SnapshotDownloadOptions, canister_id: &str) -> String {
    dfx(options).start_canister_display(canister_id)
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

// Return snapshot command usage text.
const fn usage() -> &'static str {
    "usage: canic snapshot download --canister <id> --out <dir> [--root <id>] [--include-children] [--recursive] [--dry-run] [--resume-after-snapshot] [--network <name>]"
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
