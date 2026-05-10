use crate::{
    args::{parse_subcommand, passthrough_subcommand, print_help_or_version},
    output, version_text,
};
use candid::Principal;
use canic_backup::{
    discovery::{DiscoveryError, RegistryEntry, parse_registry_entries},
    execution::{BackupExecutionJournal, BackupExecutionJournalError},
    journal::JournalResumeReport,
    manifest::IdentityMode,
    persistence::{BackupIntegrityReport, BackupLayout, PersistenceError},
    plan::{
        AuthorityEvidence, BackupPlan, BackupPlanBuildInput, BackupPlanError, BackupScopeKind,
        ControlAuthority, SnapshotReadAuthority, build_backup_plan, resolve_backup_selector,
    },
    topology::{TopologyHasher, TopologyRecord},
};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    install_root::read_named_fleet_install_state,
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

mod options;

pub use options::{
    BackupCreateOptions, BackupListOptions, BackupStatusOptions, BackupVerifyOptions,
};

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error("backup create currently supports planning only; pass --dry-run")]
    CreateRequiresDryRun,

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` before planning a backup"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("registry entry {canister_id} is not a valid principal")]
    InvalidRegistryPrincipal { canister_id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    BackupPlan(#[from] BackupPlanError),

    #[error(transparent)]
    BackupExecutionJournal(#[from] BackupExecutionJournalError),
}

///
/// BackupCreateDryRunReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCreateDryRunReport {
    pub fleet: String,
    pub network: String,
    pub out: PathBuf,
    pub plan_id: String,
    pub run_id: String,
    pub scope: String,
    pub targets: usize,
    pub operations: usize,
}

///
/// BackupListEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupListEntry {
    pub dir: PathBuf,
    pub backup_id: String,
    pub created_at: String,
    pub members: usize,
    pub status: String,
}

pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) =
        parse_subcommand(backup_command(), args).map_err(|_| BackupCommandError::Usage(usage()))?
    else {
        return Err(BackupCommandError::Usage(usage()));
    };

    match command.as_str() {
        "create" => {
            if print_help_or_version(&args, create_usage, version_text()) {
                return Ok(());
            }
            let options = BackupCreateOptions::parse(args)?;
            let report = backup_create(&options)?;
            write_create_report(&report);
            Ok(())
        }
        "list" => {
            if print_help_or_version(&args, list_usage, version_text()) {
                return Ok(());
            }
            let options = BackupListOptions::parse(args)?;
            let entries = backup_list(&options)?;
            write_list_report(&options, &entries)?;
            Ok(())
        }
        "status" => {
            if print_help_or_version(&args, status_usage, version_text()) {
                return Ok(());
            }
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            if print_help_or_version(&args, verify_usage, version_text()) {
                return Ok(());
            }
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_report(&options, &report)?;
            Ok(())
        }
        _ => unreachable!("backup dispatch command only defines known commands"),
    }
}

pub fn backup_create(
    options: &BackupCreateOptions,
) -> Result<BackupCreateDryRunReport, BackupCommandError> {
    if !options.dry_run {
        return Err(BackupCommandError::CreateRequiresDryRun);
    }

    let state = read_named_fleet_install_state(&options.network, &options.fleet)
        .map_err(|err| BackupCommandError::InstallState(err.to_string()))?
        .ok_or_else(|| BackupCommandError::NoInstalledFleet {
            network: options.network.clone(),
            fleet: options.fleet.clone(),
        })?;
    let registry_json = call_subnet_registry(options, &state.root_canister_id)?;
    let registry = parse_registry_entries(&registry_json)?;
    let topology_hash = registry_topology_hash(&registry)?;
    let plan_id = backup_plan_id(&options.fleet);
    let run_id = plan_id.replace("plan-", "run-");
    let out = options
        .out
        .clone()
        .unwrap_or_else(|| default_backup_output_path(&options.fleet));
    let selected_canister_id = options
        .subtree
        .as_deref()
        .map(|selector| resolve_backup_selector(&registry, selector))
        .transpose()?;
    let selected_scope_kind = if selected_canister_id.is_some() {
        BackupScopeKind::Subtree
    } else {
        BackupScopeKind::NonRootFleet
    };
    let plan = build_backup_plan(BackupPlanBuildInput {
        plan_id,
        run_id,
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        root_canister_id: state.root_canister_id,
        selected_canister_id,
        selected_scope_kind,
        include_descendants: true,
        topology_hash_before_quiesce: topology_hash,
        registry: &registry,
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Declared),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Declared,
        ),
        quiescence_policy: canic_backup::plan::QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })?;
    persist_backup_create_dry_run(&out, &plan)?;

    Ok(BackupCreateDryRunReport {
        fleet: plan.fleet.clone(),
        network: plan.network.clone(),
        out,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        scope: backup_scope_label(&plan),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
    })
}

pub fn backup_list(
    options: &BackupListOptions,
) -> Result<Vec<BackupListEntry>, BackupCommandError> {
    if !options.dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&options.dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(backup_list_entry)
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.dir.cmp(&left.dir))
    });
    Ok(entries)
}

pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.verify_integrity().map_err(BackupCommandError::from)
}

fn backup_list_entry(dir: PathBuf) -> Option<BackupListEntry> {
    let layout = BackupLayout::new(dir.clone());
    if layout.manifest_path().is_file() {
        return Some(manifest_backup_list_entry(dir, &layout));
    }
    if layout.backup_plan_path().is_file() {
        return Some(planned_backup_list_entry(dir, &layout));
    }

    None
}

fn manifest_backup_list_entry(dir: PathBuf, layout: &BackupLayout) -> BackupListEntry {
    let Ok(manifest) = layout.read_manifest() else {
        return BackupListEntry {
            dir,
            backup_id: "-".to_string(),
            created_at: "-".to_string(),
            members: 0,
            status: "invalid-manifest".to_string(),
        };
    };

    BackupListEntry {
        dir,
        backup_id: manifest.backup_id,
        created_at: manifest.created_at,
        members: manifest.fleet.members.len(),
        status: "ok".to_string(),
    }
}

fn planned_backup_list_entry(dir: PathBuf, layout: &BackupLayout) -> BackupListEntry {
    let Ok(plan) = layout.read_backup_plan() else {
        return BackupListEntry {
            dir,
            backup_id: "-".to_string(),
            created_at: "-".to_string(),
            members: 0,
            status: "invalid-plan".to_string(),
        };
    };
    let status = if layout.execution_journal_path().is_file()
        && layout.verify_execution_integrity().is_err()
    {
        "invalid-plan-journal"
    } else {
        "dry-run"
    };

    BackupListEntry {
        dir,
        backup_id: plan.plan_id,
        created_at: planned_backup_created_at(&plan.run_id),
        members: plan.targets.len(),
        status: status.to_string(),
    }
}

fn ensure_complete_status(report: &JournalResumeReport) -> Result<(), BackupCommandError> {
    if report.is_complete {
        return Ok(());
    }

    Err(BackupCommandError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

fn write_status_report(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the backup-create dry-run summary as a compact table.
fn write_create_report(report: &BackupCreateDryRunReport) {
    let rows = [[
        report.fleet.clone(),
        report.network.clone(),
        report.scope.clone(),
        report.targets.to_string(),
        report.operations.to_string(),
        report.out.display().to_string(),
    ]];
    println!(
        "{}",
        render_table(
            &["FLEET", "NETWORK", "SCOPE", "TARGETS", "OPERATIONS", "OUT"],
            &rows,
            &[ColumnAlign::Left; 6],
        )
    );
}

// Write the integrity report to stdout or a requested output file.
fn write_report(
    options: &BackupVerifyOptions,
    report: &BackupIntegrityReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the backup directory list as a compact whitespace table.
fn write_list_report(
    options: &BackupListOptions,
    entries: &[BackupListEntry],
) -> Result<(), BackupCommandError> {
    let text = render_backup_list(entries);
    if let Some(path) = &options.out {
        fs::write(path, text)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{text}")?;
    Ok(())
}

fn render_backup_list(entries: &[BackupListEntry]) -> String {
    let rows = entries
        .iter()
        .map(|entry| {
            [
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                display_created_at(&entry.created_at),
                entry.members.to_string(),
                entry.status.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["DIR", "BACKUP_ID", "CREATED_AT", "MEMBERS", "STATUS"],
        &rows,
        &[ColumnAlign::Left; 5],
    )
}

fn display_created_at(created_at: &str) -> String {
    created_at
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map_or_else(|| created_at.to_string(), backup_list_timestamp)
}

fn planned_backup_created_at(run_id: &str) -> String {
    let mut parts = run_id.rsplit('-');
    let Some(time) = parts.next() else {
        return "-".to_string();
    };
    let Some(date) = parts.next() else {
        return "-".to_string();
    };
    let valid = date.len() == 8
        && time.len() == 6
        && date.bytes().all(|byte| byte.is_ascii_digit())
        && time.bytes().all(|byte| byte.is_ascii_digit());
    if valid {
        format!("{date}-{time}")
    } else {
        "-".to_string()
    }
}

fn backup_list_timestamp(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;

    format!("{day:02}/{month:02}/{year:04} {hour:02}:{minute:02}")
}

fn persist_backup_create_dry_run(out: &Path, plan: &BackupPlan) -> Result<(), BackupCommandError> {
    let journal = BackupExecutionJournal::from_plan(plan)?;
    let layout = BackupLayout::new(out.to_path_buf());
    layout.write_backup_plan(plan)?;
    layout.write_execution_journal(&journal)?;
    layout.verify_execution_integrity()?;
    Ok(())
}

fn call_subnet_registry(
    options: &BackupCreateOptions,
    root: &str,
) -> Result<String, BackupCommandError> {
    if replica_query::should_use_local_replica_query(Some(&options.network)) {
        return replica_query::query_subnet_registry_json(Some(&options.network), root)
            .map_err(|err| BackupCommandError::ReplicaQuery(err.to_string()));
    }

    IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(backup_icp_error)
}

fn backup_icp_error(error: IcpCommandError) -> BackupCommandError {
    match error {
        IcpCommandError::Io(err) => BackupCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => BackupCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

fn registry_topology_hash(registry: &[RegistryEntry]) -> Result<String, BackupCommandError> {
    let records = registry
        .iter()
        .map(|entry| {
            Ok(TopologyRecord {
                pid: Principal::from_text(&entry.pid).map_err(|_| {
                    BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.pid.clone(),
                    }
                })?,
                parent_pid: entry
                    .parent_pid
                    .as_deref()
                    .map(Principal::from_text)
                    .transpose()
                    .map_err(|_| BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.parent_pid.clone().unwrap_or_default(),
                    })?,
                role: entry.role.clone().unwrap_or_default(),
                module_hash: None,
            })
        })
        .collect::<Result<Vec<_>, BackupCommandError>>()?;

    Ok(TopologyHasher::hash(&records).hash)
}

fn backup_scope_label(plan: &BackupPlan) -> String {
    match plan.selected_scope_kind {
        BackupScopeKind::NonRootFleet => "non-root-fleet".to_string(),
        BackupScopeKind::Subtree => plan
            .selected_subtree_root
            .as_ref()
            .map_or_else(|| "subtree".to_string(), |root| format!("subtree:{root}")),
        BackupScopeKind::Member => plan
            .selected_subtree_root
            .as_ref()
            .map_or_else(|| "member".to_string(), |root| format!("member:{root}")),
        BackupScopeKind::MaintenanceRoot => "maintenance-root".to_string(),
    }
}

fn backup_plan_id(fleet: &str) -> String {
    format!(
        "plan-{}-{}",
        file_safe_component(fleet),
        current_backup_directory_stamp()
    )
}

fn default_backup_output_path(fleet: &str) -> PathBuf {
    PathBuf::from("backups").join(format!(
        "fleet-{}-{}",
        file_safe_component(fleet),
        current_backup_directory_stamp()
    ))
}

fn current_backup_directory_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());

    backup_directory_stamp_from_unix(seconds)
}

fn backup_directory_stamp_from_unix(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}

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

fn usage() -> String {
    let mut command = backup_command();
    command.render_help().to_string()
}

fn status_usage() -> String {
    let mut command = options::backup_status_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = options::backup_list_command();
    command.render_help().to_string()
}

fn create_usage() -> String {
    let mut command = options::backup_create_command();
    command.render_help().to_string()
}

fn verify_usage() -> String {
    let mut command = options::backup_verify_command();
    command.render_help().to_string()
}

fn backup_command() -> ClapCommand {
    ClapCommand::new("backup")
        .bin_name("canic backup")
        .about("Plan, inspect, and verify backup artifacts")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Plan a topology-aware fleet backup")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List backup directories under a backup root")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verify")
                .about("Verify layout, journal agreement, and durable artifact checksums")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("status")
                .about("Summarize resumable download journal state")
                .disable_help_flag(true),
        ))
}

#[cfg(test)]
mod tests;
