mod types;

pub use types::*;

use crate::{
    artifacts::ArtifactChecksum,
    execution::{
        BackupExecutionJournal, BackupExecutionJournalOperation, BackupExecutionOperationReceipt,
        BackupExecutionOperationState,
    },
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    },
    persistence::BackupLayout,
    plan::{BackupOperationKind, BackupPlan, ControlAuthoritySource},
    timestamp::current_timestamp_marker,
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const PREFLIGHT_TTL_SECONDS: u64 = 300;

/// Execute a persisted backup plan through an injected host executor.
pub fn backup_run_execute_with_executor(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
) -> Result<BackupRunResponse, BackupRunnerError> {
    let layout = BackupLayout::new(config.out.clone());
    let _lock = BackupRunLock::acquire(&layout.execution_journal_path())?;
    let mut plan = layout.read_backup_plan()?;
    let mut journal = if layout.execution_journal_path().is_file() {
        layout.read_execution_journal()?
    } else {
        let journal = BackupExecutionJournal::from_plan(&plan)?;
        layout.write_execution_journal(&journal)?;
        journal
    };
    layout.verify_execution_integrity()?;

    accept_preflight_if_needed(config, executor, &layout, &mut plan, &mut journal)?;
    execute_ready_operations(config, executor, &layout, &plan, &mut journal)
}

fn accept_preflight_if_needed(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &mut BackupPlan,
    journal: &mut BackupExecutionJournal,
) -> Result<(), BackupRunnerError> {
    if journal.preflight_accepted {
        return Ok(());
    }

    let validated_at = state_updated_at(config.updated_at.as_ref());
    let expires_at = timestamp_marker(timestamp_seconds(&validated_at) + PREFLIGHT_TTL_SECONDS);
    let preflight_id = format!("preflight-{}", plan.run_id);
    let receipts = executor
        .preflight_receipts(plan, &preflight_id, &validated_at, &expires_at)
        .map_err(|error| BackupRunnerError::PreflightFailed {
            status: error.status,
            message: error.message,
        })?;
    plan.apply_execution_preflight_receipts(&receipts, &validated_at)?;
    layout.write_backup_plan(plan)?;
    journal.accept_preflight_receipts_at(&receipts, Some(validated_at))?;
    layout.write_execution_journal(journal)?;
    Ok(())
}

fn execute_ready_operations(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
) -> Result<BackupRunResponse, BackupRunnerError> {
    let mut executed = Vec::new();

    loop {
        let summary = journal.resume_summary();
        if summary.completed_operations + summary.skipped_operations == summary.total_operations {
            return Ok(run_response(plan, journal, executed, false));
        }
        if config
            .max_steps
            .is_some_and(|max_steps| executed.len() >= max_steps)
        {
            return Ok(run_response(plan, journal, executed, true));
        }

        let operation = journal
            .next_ready_operation()
            .cloned()
            .ok_or(BackupRunnerError::NoReadyOperation)?;
        if operation.state == BackupExecutionOperationState::Blocked {
            return Err(BackupRunnerError::Blocked {
                reasons: operation.blocking_reasons,
            });
        }

        if operation.state != BackupExecutionOperationState::Pending {
            journal.mark_operation_pending_at(
                operation.sequence,
                Some(state_updated_at(config.updated_at.as_ref())),
            )?;
            layout.write_execution_journal(journal)?;
        }

        match execute_operation_receipt(config, executor, layout, plan, journal, &operation) {
            Ok(receipt) => {
                journal.record_operation_receipt(receipt)?;
                layout.write_execution_journal(journal)?;
                executed.push(BackupRunExecutedOperation::completed(&operation));
            }
            Err(error) => {
                let receipt = BackupExecutionOperationReceipt::failed(
                    journal,
                    &operation,
                    Some(state_updated_at(config.updated_at.as_ref())),
                    error.to_string(),
                );
                journal.record_operation_receipt(receipt)?;
                layout.write_execution_journal(journal)?;
                executed.push(BackupRunExecutedOperation::failed(&operation));
                return Err(error);
            }
        }
    }
}

fn execute_operation_receipt(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    match operation.kind {
        BackupOperationKind::Stop => execute_stop(executor, journal, operation),
        BackupOperationKind::CreateSnapshot => {
            execute_create_snapshot(executor, layout, plan, journal, operation)
        }
        BackupOperationKind::Start => execute_start(executor, journal, operation),
        BackupOperationKind::DownloadSnapshot => {
            execute_download_snapshot(executor, layout, journal, operation)
        }
        BackupOperationKind::VerifyArtifact => execute_verify_artifact(layout, journal, operation),
        BackupOperationKind::FinalizeManifest => {
            execute_finalize_manifest(config, layout, plan, journal, operation)
        }
        BackupOperationKind::ValidateTopology
        | BackupOperationKind::ValidateControlAuthority
        | BackupOperationKind::ValidateSnapshotReadAuthority
        | BackupOperationKind::ValidateQuiescencePolicy => {
            Ok(BackupExecutionOperationReceipt::completed(
                journal,
                operation,
                Some(state_updated_at(config.updated_at.as_ref())),
            ))
        }
    }
}

fn execute_stop(
    executor: &mut impl BackupRunnerExecutor,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .stop_canister(&target)
        .map_err(|error| command_failed(operation.sequence, error))?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn execute_start(
    executor: &mut impl BackupRunnerExecutor,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .start_canister(&target)
        .map_err(|error| command_failed(operation.sequence, error))?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn execute_create_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let snapshot_id = executor
        .create_snapshot(&target)
        .map_err(|error| command_failed(operation.sequence, error))?;
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.snapshot_id = Some(snapshot_id.clone());

    let mut download_journal = read_or_new_download_journal(layout, plan, journal)?;
    upsert_artifact_entry(
        &mut download_journal,
        ArtifactJournalEntry {
            canister_id: target.clone(),
            snapshot_id,
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: artifact_relative_path(&target),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: current_timestamp_marker(),
        },
    );
    layout.write_journal(&download_journal)?;
    Ok(receipt)
}

fn execute_download_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let snapshot_id = snapshot_id_for_target(journal, operation.sequence, &target)?;
    let temp_path = artifact_temp_path(layout.root(), &target);
    if temp_path.exists() {
        fs::remove_dir_all(&temp_path)?;
    }
    fs::create_dir_all(&temp_path)?;
    executor
        .download_snapshot(&target, &snapshot_id, &temp_path)
        .map_err(|error| command_failed(operation.sequence, error))?;

    let mut download_journal = layout.read_journal()?;
    let entry = artifact_entry_mut(&mut download_journal, operation.sequence, &target)?;
    entry.temp_path = Some(temp_path.display().to_string());
    entry.advance_to(ArtifactState::Downloaded, current_timestamp_marker())?;
    layout.write_journal(&download_journal)?;

    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.artifact_path = Some(artifact_relative_path(&target));
    Ok(receipt)
}

fn execute_verify_artifact(
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let mut download_journal = layout.read_journal()?;
    let entry = artifact_entry_mut(&mut download_journal, operation.sequence, &target)?;
    let temp_path =
        entry
            .temp_path
            .as_deref()
            .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
                sequence: operation.sequence,
                target_canister_id: target.clone(),
            })?;
    let checksum = ArtifactChecksum::from_path(Path::new(temp_path))?;
    entry.checksum = Some(checksum.hash.clone());
    entry.advance_to(ArtifactState::ChecksumVerified, current_timestamp_marker())?;
    layout.write_journal(&download_journal)?;

    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.checksum = Some(checksum.hash);
    Ok(receipt)
}

fn execute_finalize_manifest(
    config: &BackupRunnerConfig,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let mut download_journal = layout.read_journal()?;
    for index in 0..download_journal.artifacts.len() {
        if download_journal.artifacts[index].state == ArtifactState::Durable {
            continue;
        }
        let canister_id = download_journal.artifacts[index].canister_id.clone();
        let temp_path = download_journal.artifacts[index].temp_path.clone().ok_or(
            BackupRunnerError::MissingArtifactEntry {
                sequence: operation.sequence,
                target_canister_id: canister_id,
            },
        )?;
        let artifact_path = layout
            .root()
            .join(&download_journal.artifacts[index].artifact_path);
        if artifact_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("artifact path already exists: {}", artifact_path.display()),
            )
            .into());
        }
        fs::rename(&temp_path, artifact_path)?;
        download_journal.artifacts[index].temp_path = None;
        download_journal.artifacts[index]
            .advance_to(ArtifactState::Durable, current_timestamp_marker())?;
        layout.write_journal(&download_journal)?;
    }

    let manifest = build_manifest(config, plan, &download_journal)?;
    layout.write_manifest(&manifest)?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn build_manifest(
    config: &BackupRunnerConfig,
    plan: &BackupPlan,
    journal: &DownloadJournal,
) -> Result<FleetBackupManifest, BackupRunnerError> {
    let roles = plan
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_role(index, target.role.as_deref()))
        .collect::<Vec<_>>();
    let manifest = FleetBackupManifest {
        manifest_version: 1,
        backup_id: plan.run_id.clone(),
        created_at: state_updated_at(config.updated_at.as_ref()),
        tool: ToolMetadata {
            name: config.tool_name.clone(),
            version: config.tool_version.clone(),
        },
        source: SourceMetadata {
            environment: plan.network.clone(),
            root_canister: plan.root_canister_id.clone(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "backup-selection".to_string(),
                kind: if plan.targets.len() == 1 {
                    BackupUnitKind::Single
                } else {
                    BackupUnitKind::Subtree
                },
                roles,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: format!("canic-backup-plan:{}", plan.plan_id),
            discovery_topology_hash: plan.topology_hash_before_quiesce.clone(),
            pre_snapshot_topology_hash: plan.topology_hash_before_quiesce.clone(),
            topology_hash: plan.topology_hash_before_quiesce.clone(),
            members: plan
                .targets
                .iter()
                .enumerate()
                .map(|(index, target)| {
                    let role = target_role(index, target.role.as_deref());
                    let entry = journal
                        .artifacts
                        .iter()
                        .find(|entry| {
                            entry.canister_id == target.canister_id
                                && entry.state == ArtifactState::Durable
                        })
                        .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
                            sequence: usize::MAX,
                            target_canister_id: target.canister_id.clone(),
                        })?;
                    Ok(FleetMember {
                        role: role.clone(),
                        canister_id: target.canister_id.clone(),
                        parent_canister_id: target.parent_canister_id.clone(),
                        subnet_canister_id: None,
                        controller_hint: controller_hint(plan, target),
                        identity_mode: target.identity_mode.clone(),
                        verification_checks: vec![VerificationCheck {
                            kind: "status".to_string(),
                            roles: vec![role],
                        }],
                        source_snapshot: SourceSnapshot {
                            snapshot_id: entry.snapshot_id.clone(),
                            module_hash: target.expected_module_hash.clone(),
                            code_version: None,
                            artifact_path: entry.artifact_path.clone(),
                            checksum_algorithm: entry.checksum_algorithm.clone(),
                            checksum: entry.checksum.clone(),
                        },
                    })
                })
                .collect::<Result<Vec<_>, BackupRunnerError>>()?,
        },
        verification: VerificationPlan::default(),
    };
    manifest.validate()?;
    Ok(manifest)
}

fn controller_hint(plan: &BackupPlan, target: &crate::plan::BackupTarget) -> Option<String> {
    if matches!(
        target.control_authority.source,
        ControlAuthoritySource::RootController
    ) {
        Some(plan.root_canister_id.clone())
    } else {
        None
    }
}

fn run_response(
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    executed: Vec<BackupRunExecutedOperation>,
    max_steps_reached: bool,
) -> BackupRunResponse {
    let execution = journal.resume_summary();
    BackupRunResponse {
        run_id: plan.run_id.clone(),
        plan_id: plan.plan_id.clone(),
        backup_id: plan.run_id.clone(),
        complete: execution.completed_operations + execution.skipped_operations
            == execution.total_operations,
        max_steps_reached,
        executed_operation_count: executed.len(),
        executed_operations: executed,
        execution,
    }
}

fn read_or_new_download_journal(
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
) -> Result<DownloadJournal, BackupRunnerError> {
    if layout.journal_path().is_file() {
        let mut journal = layout.read_journal()?;
        journal.discovery_topology_hash = Some(plan.topology_hash_before_quiesce.clone());
        journal.pre_snapshot_topology_hash = Some(plan.topology_hash_before_quiesce.clone());
        return Ok(journal);
    }

    Ok(DownloadJournal {
        journal_version: 1,
        backup_id: journal.run_id.clone(),
        discovery_topology_hash: Some(plan.topology_hash_before_quiesce.clone()),
        pre_snapshot_topology_hash: Some(plan.topology_hash_before_quiesce.clone()),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: Vec::new(),
    })
}

fn upsert_artifact_entry(journal: &mut DownloadJournal, entry: ArtifactJournalEntry) {
    if let Some(existing) = journal
        .artifacts
        .iter_mut()
        .find(|existing| existing.canister_id == entry.canister_id)
    {
        *existing = entry;
    } else {
        journal.operation_metrics.target_count = journal.artifacts.len() + 1;
        journal.artifacts.push(entry);
    }
}

fn artifact_entry_mut<'a>(
    journal: &'a mut DownloadJournal,
    sequence: usize,
    target: &str,
) -> Result<&'a mut ArtifactJournalEntry, BackupRunnerError> {
    journal
        .artifacts
        .iter_mut()
        .find(|entry| entry.canister_id == target)
        .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
            sequence,
            target_canister_id: target.to_string(),
        })
}

fn snapshot_id_for_target(
    journal: &BackupExecutionJournal,
    sequence: usize,
    target: &str,
) -> Result<String, BackupRunnerError> {
    journal
        .operation_receipts
        .iter()
        .rev()
        .find(|receipt| {
            receipt.kind == BackupOperationKind::CreateSnapshot
                && receipt.target_canister_id.as_deref() == Some(target)
                && receipt.snapshot_id.is_some()
        })
        .and_then(|receipt| receipt.snapshot_id.clone())
        .ok_or_else(|| BackupRunnerError::MissingSnapshotId {
            sequence,
            target_canister_id: target.to_string(),
        })
}

fn operation_target(
    operation: &BackupExecutionJournalOperation,
) -> Result<String, BackupRunnerError> {
    operation
        .target_canister_id
        .clone()
        .ok_or(BackupRunnerError::MissingOperationTarget {
            sequence: operation.sequence,
        })
}

fn command_failed(sequence: usize, error: BackupRunnerCommandError) -> BackupRunnerError {
    BackupRunnerError::CommandFailed {
        sequence,
        status: error.status,
        message: error.message,
    }
}

fn artifact_relative_path(canister_id: &str) -> String {
    safe_path_segment(canister_id)
}

fn artifact_temp_path(root: &Path, canister_id: &str) -> PathBuf {
    root.join(format!("{}.tmp", safe_path_segment(canister_id)))
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn target_role(index: usize, role: Option<&str>) -> String {
    role.map_or_else(|| format!("member-{index}"), str::to_string)
}

fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(current_timestamp_marker)
}

fn timestamp_seconds(marker: &str) -> u64 {
    marker
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .unwrap_or_else(current_unix_seconds)
}

fn timestamp_marker(seconds: u64) -> String {
    format!("unix:{seconds}")
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

struct BackupRunLock {
    path: PathBuf,
}

impl BackupRunLock {
    fn acquire(journal_path: &Path) -> Result<Self, BackupRunnerError> {
        let path = journal_lock_path(journal_path);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                writeln!(file, "pid={}", std::process::id())?;
                Ok(Self { path })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(BackupRunnerError::JournalLocked {
                    lock_path: path.to_string_lossy().to_string(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for BackupRunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

#[cfg(test)]
mod tests;
