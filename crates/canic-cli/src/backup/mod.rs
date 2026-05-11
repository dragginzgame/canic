use crate::{
    args::{parse_subcommand, passthrough_subcommand, print_help_or_version},
    output,
    path_stamp::{backup_list_timestamp, current_backup_directory_stamp, file_safe_component},
    version_text,
};
use candid::Principal;
use canic_backup::{
    discovery::{DiscoveryError, RegistryEntry, parse_registry_entries},
    execution::{BackupExecutionJournal, BackupExecutionJournalError},
    journal::JournalResumeReport,
    manifest::IdentityMode,
    persistence::{BackupIntegrityReport, BackupLayout, PersistenceError},
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        BackupPlanBuildInput, BackupPlanError, BackupScopeKind, ControlAuthority,
        ControlAuthorityReceipt, QuiescencePreflightReceipt, QuiescencePreflightTarget,
        SnapshotReadAuthority, SnapshotReadAuthorityReceipt, TopologyPreflightReceipt,
        TopologyPreflightTarget, build_backup_plan, resolve_backup_selector,
    },
    runner::{
        BackupRunResponse, BackupRunnerCommandError, BackupRunnerConfig, BackupRunnerError,
        BackupRunnerExecutor, backup_run_execute_with_executor,
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
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

mod options;

pub use options::{
    BackupCreateOptions, BackupInspectOptions, BackupListOptions, BackupStatusOptions,
    BackupVerifyOptions,
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

    #[error("backup plan {plan_id} is a dry-run layout, not a complete backup")]
    DryRunNotComplete { plan_id: String },

    #[error("backup reference {reference} was not found under backups; run `canic backup list`")]
    BackupReferenceNotFound { reference: String },

    #[error("backup reference {reference} is ambiguous under backups; use `--dir <dir>`")]
    BackupReferenceAmbiguous { reference: String },

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

    #[error(transparent)]
    BackupRunner(#[from] BackupRunnerError),
}

///
/// BackupCreateReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCreateReport {
    pub fleet: String,
    pub network: String,
    pub out: PathBuf,
    pub plan_id: String,
    pub run_id: String,
    pub mode: String,
    pub status: String,
    pub scope: String,
    pub targets: usize,
    pub operations: usize,
    pub executed_operations: usize,
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

///
/// BackupStatusReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupStatusReport {
    Download(JournalResumeReport),
    DryRun(BackupDryRunStatusReport),
}

///
/// BackupDryRunStatusReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupDryRunStatusReport {
    pub layout_status: String,
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub targets: usize,
    pub operations: usize,
    pub execution: canic_backup::execution::BackupExecutionResumeSummary,
}

///
/// BackupInspectReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectReport {
    pub layout_status: String,
    pub plan_id: String,
    pub run_id: String,
    pub fleet: String,
    pub network: String,
    pub scope: String,
    pub targets: Vec<BackupInspectTarget>,
    pub operations: Vec<BackupInspectOperation>,
    pub execution: canic_backup::execution::BackupExecutionResumeSummary,
}

///
/// BackupInspectTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectTarget {
    pub role: String,
    pub canister_id: String,
    pub parent_canister_id: String,
    pub expected_module_hash: String,
    pub depth: u32,
    pub control_authority: String,
    pub snapshot_read_authority: String,
}

///
/// BackupInspectOperation
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupInspectOperation {
    pub sequence: usize,
    pub kind: String,
    pub target_canister_id: String,
    pub state: String,
    pub blocking_reasons: Vec<String>,
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
        "inspect" => {
            if print_help_or_version(&args, inspect_usage, version_text()) {
                return Ok(());
            }
            let options = BackupInspectOptions::parse(args)?;
            let report = backup_inspect(&options)?;
            write_inspect_report(&options, &report)?;
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
) -> Result<BackupCreateReport, BackupCommandError> {
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
        control_authority: backup_control_authority(options.dry_run),
        snapshot_read_authority: backup_snapshot_read_authority(options.dry_run),
        quiescence_policy: backup_quiescence_policy(options.dry_run),
        identity_mode: IdentityMode::Relocatable,
    })?;
    persist_backup_create_layout(&out, &plan)?;

    let run = if options.dry_run {
        None
    } else {
        let mut executor = BackupIcpRunnerExecutor::new(options);
        Some(backup_run_execute_with_executor(
            &BackupRunnerConfig {
                out: out.clone(),
                max_steps: None,
                updated_at: None,
                tool_name: "canic".to_string(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            &mut executor,
        )?)
    };

    Ok(BackupCreateReport {
        fleet: plan.fleet.clone(),
        network: plan.network.clone(),
        out,
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        mode: if options.dry_run {
            "dry-run"
        } else {
            "execute"
        }
        .to_string(),
        status: run
            .as_ref()
            .map_or_else(|| "planned".to_string(), backup_run_status),
        scope: backup_scope_label(&plan),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        executed_operations: run.as_ref().map_or(0, |run| run.executed_operation_count),
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
) -> Result<BackupStatusReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    if layout.backup_plan_path().is_file() {
        let plan = layout.read_backup_plan()?;
        let journal = layout.read_execution_journal()?;
        layout.verify_execution_integrity()?;
        return Ok(BackupStatusReport::DryRun(BackupDryRunStatusReport {
            layout_status: execution_layout_status(&journal, layout.manifest_path().is_file()),
            plan_id: plan.plan_id.clone(),
            run_id: plan.run_id.clone(),
            fleet: plan.fleet,
            network: plan.network,
            targets: plan.targets.len(),
            operations: plan.phases.len(),
            execution: journal.resume_summary(),
        }));
    }
    if layout.journal_path().is_file() {
        let journal = layout.read_journal()?;
        return Ok(BackupStatusReport::Download(journal.resume_report()));
    }

    let journal = layout.read_journal()?;
    Ok(BackupStatusReport::Download(journal.resume_report()))
}

pub fn backup_inspect(
    options: &BackupInspectOptions,
) -> Result<BackupInspectReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    let plan = layout.read_backup_plan()?;
    let journal = layout.read_execution_journal()?;
    layout.verify_execution_integrity()?;

    Ok(BackupInspectReport {
        layout_status: execution_layout_status(&journal, layout.manifest_path().is_file()),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        fleet: plan.fleet.clone(),
        network: plan.network.clone(),
        scope: backup_scope_label(&plan),
        targets: plan.targets.iter().map(inspect_target).collect(),
        operations: journal.operations.iter().map(inspect_operation).collect(),
        execution: journal.resume_summary(),
    })
}

pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(resolve_backup_dir(
        options.dir.as_deref(),
        options.backup_ref.as_deref(),
    )?);
    if !layout.manifest_path().is_file() && layout.backup_plan_path().is_file() {
        let plan = layout.read_backup_plan()?;
        return Err(BackupCommandError::DryRunNotComplete {
            plan_id: plan.plan_id,
        });
    }

    layout.verify_integrity().map_err(BackupCommandError::from)
}

fn resolve_backup_dir(
    dir: Option<&Path>,
    backup_ref: Option<&str>,
) -> Result<PathBuf, BackupCommandError> {
    if let Some(dir) = dir {
        return Ok(dir.to_path_buf());
    }
    if let Some(backup_ref) = backup_ref {
        return resolve_backup_reference(backup_ref);
    }

    Err(BackupCommandError::Usage(
        "backup target required; pass <backup-ref> or --dir <dir>".to_string(),
    ))
}

fn resolve_backup_reference(reference: &str) -> Result<PathBuf, BackupCommandError> {
    resolve_backup_reference_in(Path::new("backups"), reference)
}

fn resolve_backup_reference_in(
    root: &Path,
    reference: &str,
) -> Result<PathBuf, BackupCommandError> {
    let entries = backup_list(&BackupListOptions {
        dir: root.to_path_buf(),
        out: None,
    })?;

    if reference.bytes().all(|byte| byte.is_ascii_digit()) {
        let index = reference.parse::<usize>().unwrap_or(0);
        return entries
            .get(index.saturating_sub(1))
            .map(|entry| entry.dir.clone())
            .ok_or_else(|| BackupCommandError::BackupReferenceNotFound {
                reference: reference.to_string(),
            });
    }

    let mut matches = entries
        .into_iter()
        .filter(|entry| entry.backup_id == reference)
        .map(|entry| entry.dir)
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(BackupCommandError::BackupReferenceNotFound {
            reference: reference.to_string(),
        }),
        1 => Ok(matches.remove(0)),
        _ => Err(BackupCommandError::BackupReferenceAmbiguous {
            reference: reference.to_string(),
        }),
    }
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
        "invalid-plan-journal".to_string()
    } else if let Ok(journal) = layout.read_execution_journal() {
        execution_layout_status(&journal, layout.manifest_path().is_file())
    } else {
        "dry-run".to_string()
    };

    BackupListEntry {
        dir,
        backup_id: plan.plan_id,
        created_at: planned_backup_created_at(&plan.run_id),
        members: plan.targets.len(),
        status,
    }
}

fn execution_layout_status(journal: &BackupExecutionJournal, has_manifest: bool) -> String {
    let summary = journal.resume_summary();
    if has_manifest && execution_is_complete(&summary) {
        "complete".to_string()
    } else if summary.failed_operations > 0 {
        "failed".to_string()
    } else if journal.preflight_accepted || summary.completed_operations > 0 {
        "running".to_string()
    } else {
        "dry-run".to_string()
    }
}

fn ensure_complete_status(report: &BackupStatusReport) -> Result<(), BackupCommandError> {
    match report {
        BackupStatusReport::Download(report) if report.is_complete => Ok(()),
        BackupStatusReport::Download(report) => Err(BackupCommandError::IncompleteJournal {
            backup_id: report.backup_id.clone(),
            total_artifacts: report.total_artifacts,
            pending_artifacts: report.pending_artifacts,
        }),
        BackupStatusReport::DryRun(report) if execution_is_complete(&report.execution) => Ok(()),
        BackupStatusReport::DryRun(report) => Err(BackupCommandError::DryRunNotComplete {
            plan_id: report.plan_id.clone(),
        }),
    }
}

fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &BackupStatusReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

fn write_status_report(
    options: &BackupStatusOptions,
    report: &BackupStatusReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

fn write_inspect_report(
    options: &BackupInspectOptions,
    report: &BackupInspectReport,
) -> Result<(), BackupCommandError> {
    if options.json {
        return output::write_pretty_json(options.out.as_ref(), report);
    }

    output::write_text::<BackupCommandError>(options.out.as_ref(), &render_inspect_report(report))
}

// Write the backup-create dry-run summary as a compact table.
fn write_create_report(report: &BackupCreateReport) {
    let rows = [[
        report.fleet.clone(),
        report.network.clone(),
        report.mode.clone(),
        report.status.clone(),
        report.scope.clone(),
        report.targets.to_string(),
        report.operations.to_string(),
        report.executed_operations.to_string(),
        report.out.display().to_string(),
    ]];
    println!(
        "{}",
        render_table(
            &[
                "FLEET",
                "NETWORK",
                "MODE",
                "STATUS",
                "SCOPE",
                "TARGETS",
                "OPERATIONS",
                "EXECUTED",
                "OUT",
            ],
            &rows,
            &[ColumnAlign::Left; 9],
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
    output::write_text::<BackupCommandError>(options.out.as_ref(), &render_backup_list(entries))
}

fn render_backup_list(entries: &[BackupListEntry]) -> String {
    let rows = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            [
                (index + 1).to_string(),
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                display_created_at(&entry.created_at),
                entry.members.to_string(),
                entry.status.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["#", "DIR", "BACKUP_ID", "CREATED_AT", "MEMBERS", "STATUS"],
        &rows,
        &[
            ColumnAlign::Right,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
        ],
    )
}

fn render_inspect_report(report: &BackupInspectReport) -> String {
    let summary_rows = [[
        report.layout_status.clone(),
        report.fleet.clone(),
        report.network.clone(),
        report.scope.clone(),
        report.targets.len().to_string(),
        report.operations.len().to_string(),
        report.execution.next_operation.as_ref().map_or_else(
            || "-".to_string(),
            |operation| operation.operation_id.clone(),
        ),
    ]];
    let target_rows = report
        .targets
        .iter()
        .map(|target| {
            [
                target.role.clone(),
                target.canister_id.clone(),
                target.parent_canister_id.clone(),
                target.expected_module_hash.clone(),
                target.depth.to_string(),
                target.control_authority.clone(),
                target.snapshot_read_authority.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let operation_rows = report
        .operations
        .iter()
        .map(|operation| {
            [
                operation.sequence.to_string(),
                operation.kind.clone(),
                operation.target_canister_id.clone(),
                operation.state.clone(),
                operation.blocking_reasons.join("; "),
            ]
        })
        .collect::<Vec<_>>();

    [
        format!("Plan: {}", report.plan_id),
        format!("Run:  {}", report.run_id),
        String::new(),
        render_table(
            &[
                "STATUS",
                "FLEET",
                "NETWORK",
                "SCOPE",
                "TARGETS",
                "OPERATIONS",
                "NEXT",
            ],
            &summary_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Targets".to_string(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "PARENT",
                "MODULE_HASH",
                "DEPTH",
                "CONTROL",
                "SNAPSHOT_READ",
            ],
            &target_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Operations".to_string(),
        render_table(
            &["SEQ", "KIND", "TARGET", "STATE", "REASONS"],
            &operation_rows,
            &[
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
            ],
        ),
    ]
    .join("\n")
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

#[cfg(test)]
fn persist_backup_create_dry_run(out: &Path, plan: &BackupPlan) -> Result<(), BackupCommandError> {
    persist_backup_create_layout(out, plan)
}

fn persist_backup_create_layout(out: &Path, plan: &BackupPlan) -> Result<(), BackupCommandError> {
    let journal = BackupExecutionJournal::from_plan(plan)?;
    let layout = BackupLayout::new(out.to_path_buf());
    layout.write_backup_plan(plan)?;
    layout.write_execution_journal(&journal)?;
    layout.verify_execution_integrity()?;
    Ok(())
}

const fn backup_control_authority(dry_run: bool) -> ControlAuthority {
    if dry_run {
        ControlAuthority::root_controller(AuthorityEvidence::Declared)
    } else {
        ControlAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

const fn backup_snapshot_read_authority(dry_run: bool) -> SnapshotReadAuthority {
    if dry_run {
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared)
    } else {
        SnapshotReadAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

const fn backup_quiescence_policy(dry_run: bool) -> canic_backup::plan::QuiescencePolicy {
    if dry_run {
        canic_backup::plan::QuiescencePolicy::RootCoordinated
    } else {
        canic_backup::plan::QuiescencePolicy::CrashConsistent
    }
}

fn backup_run_status(run: &BackupRunResponse) -> String {
    if run.complete {
        "complete"
    } else if run.max_steps_reached {
        "paused"
    } else {
        "running"
    }
    .to_string()
}

const fn execution_is_complete(
    execution: &canic_backup::execution::BackupExecutionResumeSummary,
) -> bool {
    execution.completed_operations + execution.skipped_operations == execution.total_operations
}

///
/// BackupIcpRunnerExecutor
///

struct BackupIcpRunnerExecutor {
    options: BackupCreateOptions,
    icp: IcpCli,
}

impl BackupIcpRunnerExecutor {
    fn new(options: &BackupCreateOptions) -> Self {
        Self {
            options: options.clone(),
            icp: IcpCli::new(&options.icp, None, Some(options.network.clone())),
        }
    }
}

impl BackupRunnerExecutor for BackupIcpRunnerExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        let registry_json =
            call_subnet_registry(&self.options, &plan.root_canister_id).map_err(preflight_error)?;
        let registry = parse_registry_entries(&registry_json).map_err(preflight_error)?;
        let topology_hash = registry_topology_hash(&registry).map_err(preflight_error)?;
        for target in &plan.targets {
            self.icp
                .canister_status(&target.canister_id)
                .map_err(runner_icp_error)?;
        }

        Ok(BackupExecutionPreflightReceipts {
            plan_id: plan.plan_id.clone(),
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            expires_at: expires_at.to_string(),
            topology: TopologyPreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
                topology_hash_at_preflight: topology_hash,
                targets: plan
                    .targets
                    .iter()
                    .map(TopologyPreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("root registry matched planned topology".to_string()),
            },
            control_authority: plan
                .targets
                .iter()
                .map(|target| ControlAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some("icp canister status succeeded".to_string()),
                })
                .collect(),
            snapshot_read_authority: plan
                .targets
                .iter()
                .map(|target| SnapshotReadAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: SnapshotReadAuthority::operator_controller(
                        AuthorityEvidence::Proven,
                    ),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: Some("operator control permits snapshot read".to_string()),
                })
                .collect(),
            quiescence: QuiescencePreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                quiescence_policy: plan.quiescence_policy.clone(),
                accepted: true,
                targets: plan
                    .targets
                    .iter()
                    .map(QuiescencePreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: Some("crash-consistent operator backup accepted".to_string()),
            },
        })
    }

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .stop_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .start_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, BackupRunnerCommandError> {
        self.icp
            .snapshot_create_id(canister_id)
            .map_err(runner_icp_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .snapshot_download(canister_id, snapshot_id, artifact_path)
            .map_err(runner_icp_error)
    }
}

fn preflight_error(error: impl std::error::Error) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("preflight", error.to_string())
}

fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}

fn inspect_target(target: &canic_backup::plan::BackupTarget) -> BackupInspectTarget {
    BackupInspectTarget {
        role: target.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target
            .parent_canister_id
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        expected_module_hash: target
            .expected_module_hash
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        depth: target.depth,
        control_authority: format_authority(
            control_authority_source_label(&target.control_authority.source),
            &target.control_authority.evidence,
        ),
        snapshot_read_authority: format_authority(
            snapshot_read_authority_source_label(&target.snapshot_read_authority.source),
            &target.snapshot_read_authority.evidence,
        ),
    }
}

fn inspect_operation(
    operation: &canic_backup::execution::BackupExecutionJournalOperation,
) -> BackupInspectOperation {
    BackupInspectOperation {
        sequence: operation.sequence,
        kind: operation_kind_label(&operation.kind).to_string(),
        target_canister_id: operation
            .target_canister_id
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        state: operation_state_label(&operation.state).to_string(),
        blocking_reasons: operation.blocking_reasons.clone(),
    }
}

fn format_authority(source: &str, evidence: &canic_backup::plan::AuthorityEvidence) -> String {
    format!("{source}/{}", authority_evidence_label(evidence))
}

const fn control_authority_source_label(
    source: &canic_backup::plan::ControlAuthoritySource,
) -> &str {
    match source {
        canic_backup::plan::ControlAuthoritySource::Unknown => "unknown",
        canic_backup::plan::ControlAuthoritySource::RootController => "root-controller",
        canic_backup::plan::ControlAuthoritySource::OperatorController => "operator-controller",
        canic_backup::plan::ControlAuthoritySource::AlternateController { .. } => {
            "alternate-controller"
        }
    }
}

const fn snapshot_read_authority_source_label(
    source: &canic_backup::plan::SnapshotReadAuthoritySource,
) -> &str {
    match source {
        canic_backup::plan::SnapshotReadAuthoritySource::Unknown => "unknown",
        canic_backup::plan::SnapshotReadAuthoritySource::OperatorController => {
            "operator-controller"
        }
        canic_backup::plan::SnapshotReadAuthoritySource::SnapshotVisibility => {
            "snapshot-visibility"
        }
        canic_backup::plan::SnapshotReadAuthoritySource::RootConfiguredRead => {
            "root-configured-read"
        }
        canic_backup::plan::SnapshotReadAuthoritySource::RootMediatedTransfer => {
            "root-mediated-transfer"
        }
    }
}

const fn authority_evidence_label(evidence: &canic_backup::plan::AuthorityEvidence) -> &str {
    match evidence {
        canic_backup::plan::AuthorityEvidence::Proven => "proven",
        canic_backup::plan::AuthorityEvidence::Declared => "declared",
        canic_backup::plan::AuthorityEvidence::Unknown => "unknown",
    }
}

const fn operation_kind_label(kind: &canic_backup::plan::BackupOperationKind) -> &str {
    match kind {
        canic_backup::plan::BackupOperationKind::ValidateTopology => "validate-topology",
        canic_backup::plan::BackupOperationKind::ValidateControlAuthority => {
            "validate-control-authority"
        }
        canic_backup::plan::BackupOperationKind::ValidateSnapshotReadAuthority => {
            "validate-snapshot-read-authority"
        }
        canic_backup::plan::BackupOperationKind::ValidateQuiescencePolicy => {
            "validate-quiescence-policy"
        }
        canic_backup::plan::BackupOperationKind::Stop => "stop",
        canic_backup::plan::BackupOperationKind::CreateSnapshot => "create-snapshot",
        canic_backup::plan::BackupOperationKind::Start => "start",
        canic_backup::plan::BackupOperationKind::DownloadSnapshot => "download-snapshot",
        canic_backup::plan::BackupOperationKind::VerifyArtifact => "verify-artifact",
        canic_backup::plan::BackupOperationKind::FinalizeManifest => "finalize-manifest",
    }
}

const fn operation_state_label(
    state: &canic_backup::execution::BackupExecutionOperationState,
) -> &str {
    match state {
        canic_backup::execution::BackupExecutionOperationState::Ready => "ready",
        canic_backup::execution::BackupExecutionOperationState::Pending => "pending",
        canic_backup::execution::BackupExecutionOperationState::Blocked => "blocked",
        canic_backup::execution::BackupExecutionOperationState::Completed => "completed",
        canic_backup::execution::BackupExecutionOperationState::Failed => "failed",
        canic_backup::execution::BackupExecutionOperationState::Skipped => "skipped",
    }
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
                module_hash: entry.module_hash.clone(),
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

fn inspect_usage() -> String {
    let mut command = options::backup_inspect_command();
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
            ClapCommand::new("inspect")
                .about("Inspect a backup or dry-run plan layout")
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
