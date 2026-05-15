use super::SnapshotCommandError;
use crate::support::path_stamp::{current_backup_directory_stamp, file_safe_component};
use crate::{
    cli::clap::{flag_arg, parse_matches, path_option, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
};
use canic_backup::{
    registry::RegistryEntry as BackupRegistryEntry,
    snapshot::{
        SnapshotDownloadConfig, SnapshotDownloadResult, SnapshotDriver, SnapshotDriverError,
        SnapshotLifecycleMode,
    },
    timestamp::current_timestamp_marker,
};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    install_root::InstallState,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, resolve_installed_fleet_from_root,
    },
    registry::{RegistryEntry as HostRegistryEntry, parse_registry_entries},
    replica_query,
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

///
/// SnapshotDownloadOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadOptions {
    pub canister: Option<String>,
    pub out: Option<PathBuf>,
    pub fleet: String,
    pub root: Option<String>,
    pub include_children: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub lifecycle: SnapshotLifecycleMode,
    pub network: Option<String>,
    pub icp: String,
}

impl SnapshotDownloadOptions {
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
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            root: string_option(&matches, "root"),
            include_children,
            recursive,
            dry_run: matches.get_flag("dry-run"),
            lifecycle: SnapshotLifecycleMode::from_resume_flag(
                matches.get_flag("resume-after-snapshot"),
            ),
            network: string_option(&matches, "network"),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

pub(super) fn snapshot_download_command() -> ClapCommand {
    ClapCommand::new("download")
        .bin_name("canic snapshot download")
        .about("Download canister snapshots for one canister or subtree")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to snapshot"),
        )
        .arg(value_arg("canister").long("canister").value_name("id"))
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
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

pub(super) fn download_usage() -> String {
    let mut command = snapshot_download_command();
    command.render_help().to_string()
}

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
    let mut driver = IcpSnapshotDriver { request: &request };
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
    icp: String,
    icp_root: PathBuf,
    registry_entries: Option<Vec<HostRegistryEntry>>,
}

// Resolve the named fleet into the explicit backup contract used downstream.
fn resolve_snapshot_download_request(
    options: &SnapshotDownloadOptions,
) -> Result<ResolvedSnapshotDownload, SnapshotCommandError> {
    let network = state_network(options.network.as_deref());
    let icp_root = resolve_current_canic_icp_root()
        .map_err(|err| SnapshotCommandError::InstallState(err.to_string()))?;
    let installed = match resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            network,
            icp: options.icp.clone(),
            detect_lost_local_root: false,
        },
        &icp_root,
    ) {
        Ok(installed) => Some(installed),
        Err(InstalledFleetError::NoInstalledFleet { .. }) => None,
        Err(err) => return Err(snapshot_installed_fleet_error(err)),
    };
    let state = installed.as_ref().map(|installed| &installed.state);
    let explicit_canister = options.canister.is_some();
    let canister = options
        .canister
        .clone()
        .or_else(|| state.map(|state| state.root_canister_id.clone()))
        .ok_or(SnapshotCommandError::MissingSnapshotSource)?;
    let fleet = state.map_or_else(|| options.fleet.clone(), |state| state.fleet.clone());
    let root = resolved_snapshot_root(options, state)?;
    let recursive = if !explicit_canister && state.is_some() {
        true
    } else {
        options.recursive
    };
    let include_children = options.include_children || recursive;
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| default_snapshot_output_path(&fleet));

    Ok(ResolvedSnapshotDownload {
        canister,
        out,
        fleet: Some(fleet),
        explicit_canister,
        root,
        include_children,
        recursive,
        dry_run: options.dry_run,
        lifecycle: options.lifecycle,
        network: options.network.clone(),
        icp: options.icp.clone(),
        icp_root,
        registry_entries: installed.map(|installed| installed.registry.entries),
    })
}

fn resolved_snapshot_root(
    options: &SnapshotDownloadOptions,
    state: Option<&InstallState>,
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

    if let Some(entries) = &request.registry_entries {
        return validate_fleet_membership_entries(fleet, &request.canister, entries);
    }

    let registry_json = call_subnet_registry(request, root)?;
    validate_fleet_membership_json(fleet, &request.canister, &registry_json)
}

fn validate_fleet_membership_json(
    fleet: &str,
    canister: &str,
    registry_json: &str,
) -> Result<(), SnapshotCommandError> {
    let entries = parse_registry_entries(registry_json).map_err(SnapshotCommandError::Registry)?;
    if entries.iter().any(|entry| entry.pid == canister) {
        return Ok(());
    }

    Err(SnapshotCommandError::CanisterNotInFleet {
        fleet: fleet.to_string(),
        canister: canister.to_string(),
    })
}

fn validate_fleet_membership_entries(
    fleet: &str,
    canister: &str,
    entries: &[HostRegistryEntry],
) -> Result<(), SnapshotCommandError> {
    if entries.iter().any(|entry| entry.pid == canister) {
        return Ok(());
    }

    Err(SnapshotCommandError::CanisterNotInFleet {
        fleet: fleet.to_string(),
        canister: canister.to_string(),
    })
}

fn default_snapshot_output_path(label: &str) -> PathBuf {
    let marker = current_backup_directory_stamp();

    PathBuf::from("backups").join(format!("fleet-{}-{marker}", file_safe_component(label)))
}

fn state_network(network: Option<&str>) -> String {
    network.map_or_else(local_network, str::to_string)
}

///
/// IcpSnapshotDriver
///

struct IcpSnapshotDriver<'a> {
    request: &'a ResolvedSnapshotDownload,
}

impl SnapshotDriver for IcpSnapshotDriver<'_> {
    fn registry_entries(
        &mut self,
        root: &str,
    ) -> Result<Vec<BackupRegistryEntry>, SnapshotDriverError> {
        if self.request.root.as_deref() == Some(root)
            && let Some(entries) = &self.request.registry_entries
        {
            return Ok(backup_registry_entries(entries));
        }

        let registry_json = call_subnet_registry(self.request, root).map_err(driver_error)?;
        let entries = parse_registry_entries(&registry_json)
            .map_err(|err| driver_error(SnapshotCommandError::Registry(err)))?;
        Ok(backup_registry_entries(&entries))
    }

    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError> {
        create_snapshot(self.request, canister_id).map_err(driver_error)
    }

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        stop_canister(self.request, canister_id).map_err(driver_error)
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError> {
        start_canister(self.request, canister_id).map_err(driver_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError> {
        download_snapshot(self.request, canister_id, snapshot_id, artifact_path)
            .map_err(driver_error)
    }

    fn create_snapshot_command(&self, canister_id: &str) -> String {
        create_snapshot_command_display(self.request, canister_id)
    }

    fn stop_canister_command(&self, canister_id: &str) -> String {
        stop_canister_command_display(self.request, canister_id)
    }

    fn start_canister_command(&self, canister_id: &str) -> String {
        start_canister_command_display(self.request, canister_id)
    }

    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String {
        download_snapshot_command_display(self.request, canister_id, snapshot_id, artifact_path)
    }
}

fn driver_error(error: SnapshotCommandError) -> SnapshotDriverError {
    Box::new(error)
}

fn icp(request: &ResolvedSnapshotDownload) -> IcpCli {
    IcpCli::new(&request.icp, None, request.network.clone()).with_cwd(&request.icp_root)
}

fn snapshot_icp_error(error: IcpCommandError) -> SnapshotCommandError {
    match error {
        IcpCommandError::Io(err) => SnapshotCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            SnapshotCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => SnapshotCommandError::IcpFailed {
            command,
            stderr: output,
        },
        IcpCommandError::SnapshotIdUnavailable { output } => {
            SnapshotCommandError::SnapshotIdUnavailable(output)
        }
    }
}

fn snapshot_installed_fleet_error(error: InstalledFleetError) -> SnapshotCommandError {
    match error {
        InstalledFleetError::NoInstalledFleet { .. }
        | InstalledFleetError::InstallState(_)
        | InstalledFleetError::ReplicaQuery(_)
        | InstalledFleetError::LostLocalFleet { .. } => {
            SnapshotCommandError::InstallState(error.to_string())
        }
        InstalledFleetError::IcpFailed { command, stderr } => {
            SnapshotCommandError::IcpFailed { command, stderr }
        }
        InstalledFleetError::Registry(err) => SnapshotCommandError::Registry(err),
        InstalledFleetError::Io(err) => SnapshotCommandError::Io(err),
    }
}

fn call_subnet_registry(
    request: &ResolvedSnapshotDownload,
    root: &str,
) -> Result<String, SnapshotCommandError> {
    if replica_query::should_use_local_replica_query(request.network.as_deref()) {
        return replica_query::query_subnet_registry_json_from_root(
            request.network.as_deref(),
            root,
            &request.icp_root,
        )
        .map_err(SnapshotCommandError::from);
    }

    icp(request)
        .canister_query_output(root, "canic_subnet_registry", Some("json"))
        .map_err(snapshot_icp_error)
}

fn create_snapshot(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<String, SnapshotCommandError> {
    icp(request)
        .snapshot_create_id(canister_id)
        .map_err(snapshot_icp_error)
}

fn stop_canister(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    icp(request)
        .stop_canister(canister_id)
        .map_err(snapshot_icp_error)
}

fn start_canister(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> Result<(), SnapshotCommandError> {
    icp(request)
        .start_canister(canister_id)
        .map_err(snapshot_icp_error)
}

fn download_snapshot(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> Result<(), SnapshotCommandError> {
    icp(request)
        .snapshot_download(canister_id, snapshot_id, artifact_path)
        .map_err(snapshot_icp_error)
}

fn create_snapshot_command_display(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
) -> String {
    icp(request).snapshot_create_display(canister_id)
}

fn download_snapshot_command_display(
    request: &ResolvedSnapshotDownload,
    canister_id: &str,
    snapshot_id: &str,
    artifact_path: &Path,
) -> String {
    icp(request).snapshot_download_display(canister_id, snapshot_id, artifact_path)
}

fn stop_canister_command_display(request: &ResolvedSnapshotDownload, canister_id: &str) -> String {
    icp(request).stop_canister_display(canister_id)
}

fn start_canister_command_display(request: &ResolvedSnapshotDownload, canister_id: &str) -> String {
    icp(request).start_canister_display(canister_id)
}

fn backup_registry_entries(entries: &[HostRegistryEntry]) -> Vec<BackupRegistryEntry> {
    entries
        .iter()
        .map(|entry| BackupRegistryEntry {
            pid: entry.pid.clone(),
            role: entry.role.clone(),
            kind: entry.kind.clone(),
            parent_pid: entry.parent_pid.clone(),
            module_hash: entry.module_hash.clone(),
        })
        .collect()
}

fn backup_id(request: &ResolvedSnapshotDownload) -> String {
    request
        .out
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "snapshot-download".to_string(), str::to_string)
}

#[cfg(test)]
mod tests;
