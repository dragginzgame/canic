use crate::version_text;
use candid::Principal;
use canic_backup::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    journal::{
        ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics,
        JournalValidationError,
    },
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetBackupManifest,
        FleetMember, FleetSection, IdentityMode, ManifestValidationError, SourceMetadata,
        SourceSnapshot, ToolMetadata, VerificationCheck, VerificationPlan,
    },
    persistence::{BackupLayout, PersistenceError},
    topology::{TopologyHash, TopologyHasher, TopologyRecord},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsString,
    fs,
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

    #[error("cannot combine --root and --registry-json")]
    ConflictingRegistrySources,

    #[error("registry JSON did not contain the requested canister {0}")]
    CanisterNotInRegistry(String),

    #[error("dfx command failed: {command}\n{stderr}")]
    DfxFailed { command: String, stderr: String },

    #[error("could not parse snapshot id from dfx output: {0}")]
    SnapshotIdUnavailable(String),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error(
        "topology changed before snapshot start: discovery={discovery}, pre_snapshot={pre_snapshot}"
    )]
    TopologyChanged {
        discovery: String,
        pre_snapshot: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Checksum(#[from] ArtifactChecksumError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Journal(#[from] JournalValidationError),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),
}

///
/// SnapshotDownloadOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadOptions {
    pub canister: String,
    pub out: PathBuf,
    pub root: Option<String>,
    pub registry_json: Option<PathBuf>,
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
        let mut registry_json = None;
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
                "--registry-json" => {
                    registry_json = Some(PathBuf::from(next_value(&mut args, "--registry-json")?));
                }
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

        if root.is_some() && registry_json.is_some() {
            return Err(SnapshotCommandError::ConflictingRegistrySources);
        }

        Ok(Self {
            canister: canister.ok_or(SnapshotCommandError::MissingOption("--canister"))?,
            out: out.ok_or(SnapshotCommandError::MissingOption("--out"))?,
            root,
            registry_json,
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

///
/// SnapshotLifecycleMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotLifecycleMode {
    SnapshotOnly,
    StopBeforeSnapshot,
    ResumeAfterSnapshot,
    StopAndResume,
}

impl SnapshotLifecycleMode {
    /// Build the lifecycle mode from CLI stop/resume flags.
    #[must_use]
    pub const fn from_flags(stop_before_snapshot: bool, resume_after_snapshot: bool) -> Self {
        match (stop_before_snapshot, resume_after_snapshot) {
            (false, false) => Self::SnapshotOnly,
            (true, false) => Self::StopBeforeSnapshot,
            (false, true) => Self::ResumeAfterSnapshot,
            (true, true) => Self::StopAndResume,
        }
    }

    /// Return whether the CLI should stop before snapshot creation.
    #[must_use]
    pub const fn stop_before_snapshot(self) -> bool {
        matches!(self, Self::StopBeforeSnapshot | Self::StopAndResume)
    }

    /// Return whether the CLI should start after snapshot capture.
    #[must_use]
    pub const fn resume_after_snapshot(self) -> bool {
        matches!(self, Self::ResumeAfterSnapshot | Self::StopAndResume)
    }
}

///
/// SnapshotTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
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

///
/// SnapshotDownloadResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadResult {
    pub artifacts: Vec<SnapshotArtifact>,
}

///
/// SnapshotArtifact
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotArtifact {
    pub canister_id: String,
    pub snapshot_id: String,
    pub path: PathBuf,
    pub checksum: String,
}

/// Create and download snapshots for the selected canister set.
pub fn download_snapshots(
    options: &SnapshotDownloadOptions,
) -> Result<SnapshotDownloadResult, SnapshotCommandError> {
    let targets = resolve_targets(options)?;
    let discovery_topology_hash = topology_hash_for_targets(options, &targets)?;
    let pre_snapshot_topology_hash =
        accepted_pre_snapshot_topology_hash(options, &discovery_topology_hash)?;
    let mut artifacts = Vec::with_capacity(targets.len());
    let mut journal = DownloadJournal {
        journal_version: 1,
        backup_id: backup_id(options),
        discovery_topology_hash: Some(discovery_topology_hash.hash.clone()),
        pre_snapshot_topology_hash: Some(pre_snapshot_topology_hash.hash.clone()),
        operation_metrics: DownloadOperationMetrics {
            target_count: targets.len(),
            ..DownloadOperationMetrics::default()
        },
        artifacts: Vec::new(),
    };
    let layout = BackupLayout::new(options.out.clone());

    for target in &targets {
        let artifact_relative_path = PathBuf::from(safe_path_segment(&target.canister_id));
        let artifact_path = options.out.join(&artifact_relative_path);
        let temp_path = options
            .out
            .join(format!("{}.tmp", safe_path_segment(&target.canister_id)));

        if options.dry_run {
            artifacts.push(dry_run_artifact(options, target, artifact_path));
            continue;
        }

        artifacts.push(capture_snapshot_artifact(
            options,
            &layout,
            &mut journal,
            target,
            &artifact_relative_path,
            artifact_path,
            temp_path,
        )?);
    }

    if !options.dry_run {
        let manifest = build_manifest(
            options,
            &targets,
            &artifacts,
            discovery_topology_hash,
            pre_snapshot_topology_hash,
        )?;
        layout.write_manifest(&manifest)?;
    }

    Ok(SnapshotDownloadResult { artifacts })
}

// Resolve and verify the pre-snapshot topology hash before any mutation.
fn accepted_pre_snapshot_topology_hash(
    options: &SnapshotDownloadOptions,
    discovery_topology_hash: &TopologyHash,
) -> Result<TopologyHash, SnapshotCommandError> {
    if options.dry_run {
        return Ok(discovery_topology_hash.clone());
    }

    let pre_snapshot_targets = resolve_targets(options)?;
    let pre_snapshot_topology_hash = topology_hash_for_targets(options, &pre_snapshot_targets)?;
    ensure_topology_stable(discovery_topology_hash, &pre_snapshot_topology_hash)?;
    Ok(pre_snapshot_topology_hash)
}

// Print the planned commands and return a placeholder artifact for dry runs.
fn dry_run_artifact(
    options: &SnapshotDownloadOptions,
    target: &SnapshotTarget,
    artifact_path: PathBuf,
) -> SnapshotArtifact {
    if options.lifecycle.stop_before_snapshot() {
        println!(
            "{}",
            stop_canister_command_display(options, &target.canister_id)
        );
    }
    println!(
        "{}",
        create_snapshot_command_display(options, &target.canister_id)
    );
    println!(
        "{}",
        download_snapshot_command_display(options, &target.canister_id, "<snapshot-id>")
    );
    if options.lifecycle.resume_after_snapshot() {
        println!(
            "{}",
            start_canister_command_display(options, &target.canister_id)
        );
    }

    SnapshotArtifact {
        canister_id: target.canister_id.clone(),
        snapshot_id: "<snapshot-id>".to_string(),
        path: artifact_path,
        checksum: "<sha256>".to_string(),
    }
}

// Create, download, checksum, and finalize one durable snapshot artifact.
fn capture_snapshot_artifact(
    options: &SnapshotDownloadOptions,
    layout: &BackupLayout,
    journal: &mut DownloadJournal,
    target: &SnapshotTarget,
    artifact_relative_path: &Path,
    artifact_path: PathBuf,
    temp_path: PathBuf,
) -> Result<SnapshotArtifact, SnapshotCommandError> {
    with_optional_stop(options, &target.canister_id, || {
        journal.operation_metrics.snapshot_create_started += 1;
        let snapshot_id = create_snapshot(options, &target.canister_id)?;
        journal.operation_metrics.snapshot_create_completed += 1;
        let mut entry = ArtifactJournalEntry {
            canister_id: target.canister_id.clone(),
            snapshot_id: snapshot_id.clone(),
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: artifact_relative_path.display().to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: timestamp_placeholder(),
        };
        journal.artifacts.push(entry.clone());
        layout.write_journal(journal)?;

        if temp_path.exists() {
            fs::remove_dir_all(&temp_path)?;
        }
        fs::create_dir_all(&temp_path)?;
        journal.operation_metrics.snapshot_download_started += 1;
        layout.write_journal(journal)?;
        download_snapshot(options, &target.canister_id, &snapshot_id, &temp_path)?;
        journal.operation_metrics.snapshot_download_completed += 1;
        entry.advance_to(ArtifactState::Downloaded, timestamp_placeholder())?;
        entry.temp_path = Some(temp_path.display().to_string());
        update_journal_entry(journal, &entry);
        layout.write_journal(journal)?;

        journal.operation_metrics.checksum_verify_started += 1;
        layout.write_journal(journal)?;
        let checksum = ArtifactChecksum::from_path(&temp_path)?;
        journal.operation_metrics.checksum_verify_completed += 1;
        entry.checksum = Some(checksum.hash.clone());
        entry.advance_to(ArtifactState::ChecksumVerified, timestamp_placeholder())?;
        update_journal_entry(journal, &entry);
        layout.write_journal(journal)?;

        journal.operation_metrics.artifact_finalize_started += 1;
        layout.write_journal(journal)?;
        if artifact_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("artifact path already exists: {}", artifact_path.display()),
            )
            .into());
        }
        fs::rename(&temp_path, &artifact_path)?;
        journal.operation_metrics.artifact_finalize_completed += 1;
        entry.temp_path = None;
        entry.advance_to(ArtifactState::Durable, timestamp_placeholder())?;
        update_journal_entry(journal, &entry);
        layout.write_journal(journal)?;

        Ok(SnapshotArtifact {
            canister_id: target.canister_id.clone(),
            snapshot_id,
            path: artifact_path,
            checksum: checksum.hash,
        })
    })
}

// Replace one artifact row in the mutable journal.
fn update_journal_entry(journal: &mut DownloadJournal, entry: &ArtifactJournalEntry) {
    if let Some(existing) = journal.artifacts.iter_mut().find(|existing| {
        existing.canister_id == entry.canister_id && existing.snapshot_id == entry.snapshot_id
    }) {
        *existing = entry.clone();
    }
}

/// Resolve the selected canister plus optional direct/recursive children.
pub fn resolve_targets(
    options: &SnapshotDownloadOptions,
) -> Result<Vec<SnapshotTarget>, SnapshotCommandError> {
    if !options.include_children {
        return Ok(vec![SnapshotTarget {
            canister_id: options.canister.clone(),
            role: None,
            parent_canister_id: None,
        }]);
    }

    let registry = load_registry_entries(options)?;
    targets_from_registry(&registry, &options.canister, options.recursive)
}

// Load registry entries from a file or live root query.
fn load_registry_entries(
    options: &SnapshotDownloadOptions,
) -> Result<Vec<RegistryEntry>, SnapshotCommandError> {
    let registry_json = if let Some(path) = &options.registry_json {
        fs::read_to_string(path)?
    } else if let Some(root) = &options.root {
        call_subnet_registry(options, root)?
    } else {
        return Err(SnapshotCommandError::MissingOption(
            "--root or --registry-json when using --include-children",
        ));
    };

    parse_registry_entries(&registry_json)
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

// Run one snapshot operation with optional stop/start lifecycle commands.
fn with_optional_stop<T>(
    options: &SnapshotDownloadOptions,
    canister_id: &str,
    operation: impl FnOnce() -> Result<T, SnapshotCommandError>,
) -> Result<T, SnapshotCommandError> {
    if options.lifecycle.stop_before_snapshot() {
        stop_canister(options, canister_id)?;
    }

    let result = operation();

    if options.lifecycle.resume_after_snapshot() {
        match result {
            Ok(value) => {
                start_canister(options, canister_id)?;
                Ok(value)
            }
            Err(error) => {
                let _ = start_canister(options, canister_id);
                Err(error)
            }
        }
    } else {
        result
    }
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
) -> String {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    add_canister_network_args(&mut command, options);
    command.args(["snapshot", "download", canister_id, snapshot_id, "--dir"]);
    command.arg(options.out.join(safe_path_segment(canister_id)));
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

///
/// RegistryEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub parent_pid: Option<String>,
}

/// Parse the `dfx --output json` subnet registry shape.
pub fn parse_registry_entries(
    registry_json: &str,
) -> Result<Vec<RegistryEntry>, SnapshotCommandError> {
    let data = serde_json::from_str::<Value>(registry_json)?;
    let entries = data
        .get("Ok")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())
        .ok_or(SnapshotCommandError::Usage(
            "registry JSON must be an array or {\"Ok\": [...]}",
        ))?;

    Ok(entries.iter().filter_map(parse_registry_entry).collect())
}

// Parse one registry entry from dfx JSON.
fn parse_registry_entry(value: &Value) -> Option<RegistryEntry> {
    let pid = value.get("pid").and_then(Value::as_str)?.to_string();
    let role = value
        .get("role")
        .and_then(Value::as_str)
        .map(str::to_string);
    let parent_pid = value
        .get("record")
        .and_then(|record| record.get("parent_pid"))
        .and_then(parse_optional_principal);

    Some(RegistryEntry {
        pid,
        role,
        parent_pid,
    })
}

// Parse optional principal JSON emitted as null, string, or optional vector form.
fn parse_optional_principal(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Resolve selected target and children from registry entries.
pub fn targets_from_registry(
    registry: &[RegistryEntry],
    canister_id: &str,
    recursive: bool,
) -> Result<Vec<SnapshotTarget>, SnapshotCommandError> {
    let by_pid = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry))
        .collect::<BTreeMap<_, _>>();

    let root = by_pid
        .get(canister_id)
        .ok_or_else(|| SnapshotCommandError::CanisterNotInRegistry(canister_id.to_string()))?;

    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    targets.push(SnapshotTarget {
        canister_id: root.pid.clone(),
        role: root.role.clone(),
        parent_canister_id: root.parent_pid.clone(),
    });
    seen.insert(root.pid.clone());

    let mut queue = VecDeque::from([root.pid.clone()]);
    while let Some(parent) = queue.pop_front() {
        for child in registry
            .iter()
            .filter(|entry| entry.parent_pid.as_deref() == Some(parent.as_str()))
        {
            if seen.insert(child.pid.clone()) {
                targets.push(SnapshotTarget {
                    canister_id: child.pid.clone(),
                    role: child.role.clone(),
                    parent_canister_id: child.parent_pid.clone(),
                });
                if recursive {
                    queue.push_back(child.pid.clone());
                }
            }
        }
    }

    Ok(targets)
}

// Build a validated manifest for one successful snapshot download run.
fn build_manifest(
    options: &SnapshotDownloadOptions,
    targets: &[SnapshotTarget],
    artifacts: &[SnapshotArtifact],
    discovery_topology_hash: TopologyHash,
    pre_snapshot_topology_hash: TopologyHash,
) -> Result<FleetBackupManifest, SnapshotCommandError> {
    let roles = targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_role(options, index, target))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let manifest = FleetBackupManifest {
        manifest_version: 1,
        backup_id: backup_id(options),
        created_at: timestamp_placeholder(),
        tool: ToolMetadata {
            name: "canic-cli".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        source: SourceMetadata {
            environment: options
                .network
                .clone()
                .unwrap_or_else(|| "local".to_string()),
            root_canister: options
                .root
                .clone()
                .unwrap_or_else(|| options.canister.clone()),
        },
        consistency: ConsistencySection {
            mode: ConsistencyMode::CrashConsistent,
            backup_units: vec![BackupUnit {
                unit_id: "snapshot-selection".to_string(),
                kind: if options.include_children {
                    BackupUnitKind::SubtreeRooted
                } else {
                    BackupUnitKind::Flat
                },
                roles,
                consistency_reason: if options.include_children {
                    None
                } else {
                    Some("explicit single-canister snapshot selection".to_string())
                },
                dependency_closure: Vec::new(),
                topology_validation: if options.include_children {
                    "registry-subtree-selection".to_string()
                } else {
                    "explicit-selection".to_string()
                },
                quiescence_strategy: None,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: discovery_topology_hash.algorithm,
            topology_hash_input: discovery_topology_hash.input,
            discovery_topology_hash: discovery_topology_hash.hash.clone(),
            pre_snapshot_topology_hash: pre_snapshot_topology_hash.hash,
            topology_hash: discovery_topology_hash.hash,
            members: targets
                .iter()
                .enumerate()
                .map(|(index, target)| fleet_member(options, index, target, artifacts))
                .collect::<Result<Vec<_>, _>>()?,
        },
        verification: VerificationPlan::default(),
    };

    manifest.validate()?;
    Ok(manifest)
}

// Compute the canonical topology hash for one resolved target set.
fn topology_hash_for_targets(
    options: &SnapshotDownloadOptions,
    targets: &[SnapshotTarget],
) -> Result<TopologyHash, SnapshotCommandError> {
    let topology_records = targets
        .iter()
        .enumerate()
        .map(|(index, target)| topology_record(options, index, target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TopologyHasher::hash(&topology_records))
}

// Fail closed if topology changes after discovery but before snapshot creation.
fn ensure_topology_stable(
    discovery: &TopologyHash,
    pre_snapshot: &TopologyHash,
) -> Result<(), SnapshotCommandError> {
    if discovery.hash == pre_snapshot.hash {
        return Ok(());
    }

    Err(SnapshotCommandError::TopologyChanged {
        discovery: discovery.hash.clone(),
        pre_snapshot: pre_snapshot.hash.clone(),
    })
}

// Build one canonical topology record for manifest hashing.
fn topology_record(
    options: &SnapshotDownloadOptions,
    index: usize,
    target: &SnapshotTarget,
) -> Result<TopologyRecord, SnapshotCommandError> {
    Ok(TopologyRecord {
        pid: parse_principal("fleet.members[].canister_id", &target.canister_id)?,
        parent_pid: target
            .parent_canister_id
            .as_deref()
            .map(|parent| parse_principal("fleet.members[].parent_canister_id", parent))
            .transpose()?,
        role: target_role(options, index, target),
        module_hash: None,
    })
}

// Build one manifest member from a captured durable artifact.
fn fleet_member(
    options: &SnapshotDownloadOptions,
    index: usize,
    target: &SnapshotTarget,
    artifacts: &[SnapshotArtifact],
) -> Result<FleetMember, SnapshotCommandError> {
    let Some(artifact) = artifacts
        .iter()
        .find(|artifact| artifact.canister_id == target.canister_id)
    else {
        return Err(SnapshotCommandError::SnapshotIdUnavailable(format!(
            "missing artifact for {}",
            target.canister_id
        )));
    };
    let role = target_role(options, index, target);

    Ok(FleetMember {
        role: role.clone(),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target.parent_canister_id.clone(),
        subnet_canister_id: options.root.clone(),
        controller_hint: None,
        identity_mode: if target.canister_id == options.canister {
            IdentityMode::Fixed
        } else {
            IdentityMode::Relocatable
        },
        restore_group: if target.canister_id == options.canister {
            1
        } else {
            2
        },
        verification_class: "basic".to_string(),
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            method: None,
            roles: vec![role],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: artifact.snapshot_id.clone(),
            module_hash: None,
            wasm_hash: None,
            code_version: None,
            artifact_path: safe_path_segment(&target.canister_id),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(artifact.checksum.clone()),
        },
    })
}

// Return the manifest role for one selected snapshot target.
fn target_role(options: &SnapshotDownloadOptions, index: usize, target: &SnapshotTarget) -> String {
    target.role.clone().unwrap_or_else(|| {
        if target.canister_id == options.canister {
            "root".to_string()
        } else {
            format!("member-{index}")
        }
    })
}

// Parse one principal used by generated topology manifest metadata.
fn parse_principal(field: &'static str, value: &str) -> Result<Principal, SnapshotCommandError> {
    Principal::from_text(value).map_err(|_| SnapshotCommandError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
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

// Convert a principal into a conservative filesystem path segment.
fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
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
    "usage: canic snapshot download --canister <id> --out <dir> [--root <id> | --registry-json <file>] [--include-children] [--recursive] [--dry-run] [--stop-before-snapshot] [--resume-after-snapshot] [--network <name>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::persistence::BackupLayout;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const GRANDCHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure dfx registry JSON parses in the wrapped Ok shape.
    #[test]
    fn parses_wrapped_registry_json() {
        let json = registry_json();

        let entries = parse_registry_entries(&json).expect("parse registry");

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].parent_pid.as_deref(), Some(ROOT));
    }

    // Ensure direct-child resolution includes only one level.
    #[test]
    fn targets_include_direct_children() {
        let entries = parse_registry_entries(&registry_json()).expect("parse registry");

        let targets = targets_from_registry(&entries, ROOT, false).expect("resolve targets");

        assert_eq!(
            targets
                .iter()
                .map(|target| target.canister_id.as_str())
                .collect::<Vec<_>>(),
            vec![ROOT, CHILD]
        );
    }

    // Ensure recursive resolution walks descendants.
    #[test]
    fn targets_include_recursive_children() {
        let entries = parse_registry_entries(&registry_json()).expect("parse registry");

        let targets = targets_from_registry(&entries, ROOT, true).expect("resolve targets");

        assert_eq!(
            targets
                .iter()
                .map(|target| target.canister_id.as_str())
                .collect::<Vec<_>>(),
            vec![ROOT, CHILD, GRANDCHILD]
        );
    }

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
            OsString::from("--registry-json"),
            OsString::from("registry.json"),
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
        assert_eq!(options.lifecycle, SnapshotLifecycleMode::StopAndResume);
    }

    // Ensure snapshot capture fails closed when topology changes before creation.
    #[test]
    fn topology_stability_rejects_pre_snapshot_drift() {
        let discovery = topology_hash(HASH);
        let pre_snapshot =
            topology_hash("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");

        let err = ensure_topology_stable(&discovery, &pre_snapshot)
            .expect_err("topology drift should fail");

        assert!(matches!(err, SnapshotCommandError::TopologyChanged { .. }));
    }

    // Ensure the actual command path writes a manifest and durable journal.
    #[cfg(unix)]
    #[test]
    fn download_snapshots_writes_manifest_and_durable_journal() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("canic-cli-download");
        let fake_dfx = root.join("fake-dfx.sh");
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(
            &fake_dfx,
            r#"#!/bin/sh
set -eu
if [ "$1" = "canister" ] && [ "$2" = "snapshot" ] && [ "$3" = "create" ]; then
  echo "snapshot-$4"
  exit 0
fi
if [ "$1" = "canister" ] && [ "$2" = "snapshot" ] && [ "$3" = "list" ]; then
  exit 0
fi
if [ "$1" = "canister" ] && [ "$2" = "snapshot" ] && [ "$3" = "download" ]; then
  mkdir -p "$7"
  printf "%s:%s\n" "$4" "$5" > "$7/snapshot.txt"
  exit 0
fi
echo "unexpected args: $*" >&2
exit 1
"#,
        )
        .expect("write fake dfx");
        let mut permissions = fs::metadata(&fake_dfx)
            .expect("stat fake dfx")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_dfx, permissions).expect("chmod fake dfx");

        let out = root.join("backup");
        let options = SnapshotDownloadOptions {
            canister: ROOT.to_string(),
            out: out.clone(),
            root: None,
            registry_json: None,
            include_children: false,
            recursive: false,
            dry_run: false,
            lifecycle: SnapshotLifecycleMode::SnapshotOnly,
            network: None,
            dfx: fake_dfx.display().to_string(),
        };

        let result = download_snapshots(&options).expect("download snapshots");
        let layout = BackupLayout::new(out);
        let journal = layout.read_journal().expect("read journal");
        let manifest = layout.read_manifest().expect("read manifest");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(journal.artifacts.len(), 1);
        assert_eq!(journal.operation_metrics.target_count, 1);
        assert_eq!(journal.operation_metrics.snapshot_create_started, 1);
        assert_eq!(journal.operation_metrics.snapshot_create_completed, 1);
        assert_eq!(journal.operation_metrics.snapshot_download_started, 1);
        assert_eq!(journal.operation_metrics.snapshot_download_completed, 1);
        assert_eq!(journal.operation_metrics.checksum_verify_started, 1);
        assert_eq!(journal.operation_metrics.checksum_verify_completed, 1);
        assert_eq!(journal.operation_metrics.artifact_finalize_started, 1);
        assert_eq!(journal.operation_metrics.artifact_finalize_completed, 1);
        assert_eq!(journal.artifacts[0].state, ArtifactState::Durable);
        assert!(journal.artifacts[0].checksum.is_some());
        assert_eq!(manifest.backup_id, journal.backup_id);
        assert_eq!(manifest.fleet.members.len(), 1);
        assert_eq!(manifest.fleet.members[0].canister_id, ROOT);
        assert_eq!(
            manifest.fleet.members[0].source_snapshot.snapshot_id,
            "snapshot-aaaaa-aa"
        );
        assert_eq!(
            manifest.fleet.members[0]
                .source_snapshot
                .checksum
                .as_deref(),
            journal.artifacts[0].checksum.as_deref()
        );
    }

    // Build representative subnet registry JSON.
    fn registry_json() -> String {
        json!({
            "Ok": [
                {
                    "pid": ROOT,
                    "role": "root",
                    "record": {
                        "pid": ROOT,
                        "role": "root",
                        "parent_pid": null
                    }
                },
                {
                    "pid": CHILD,
                    "role": "app",
                    "record": {
                        "pid": CHILD,
                        "role": "app",
                        "parent_pid": ROOT
                    }
                },
                {
                    "pid": GRANDCHILD,
                    "role": "worker",
                    "record": {
                        "pid": GRANDCHILD,
                        "role": "worker",
                        "parent_pid": [CHILD]
                    }
                }
            ]
        })
        .to_string()
    }

    // Build one topology hash for stability tests.
    fn topology_hash(hash: &str) -> TopologyHash {
        TopologyHash {
            algorithm: "sha256".to_string(),
            input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            hash: hash.to_string(),
        }
    }

    // Build a unique temporary directory.
    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
