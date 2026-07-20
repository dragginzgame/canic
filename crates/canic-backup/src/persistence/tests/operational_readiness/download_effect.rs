//! Module: persistence::tests::operational_readiness::download_effect
//!
//! Responsibility: prove private snapshot-download staging recovery across process death.
//! Does not own: checksum verification, artifact publication, or manifest recovery.
//! Boundary: replaces only uncommitted exact staging and preserves stronger journal evidence.

use super::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier};
use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::{BackupLayout, CommandLifetimeHandle},
    plan::{BackupExecutionPreflightReceipts, BackupOperationKind, BackupPlan},
    runner::{
        BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerConfig,
        BackupRunnerExecutor, BackupRunnerSnapshot, backup_run_execute_with_executor,
    },
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

const DOWNLOAD_EFFECT_CHILD_ROOT_ENV: &str = "CANIC_TEST_DOWNLOAD_EFFECT_ROOT";
const DOWNLOAD_EFFECT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_DOWNLOAD_EFFECT_HANDSHAKE";
pub(super) const COMPLETE_BYTES: &[u8] = b"complete snapshot download";
const INTERRUPTED_BYTES: &[u8] = b"uncommitted snapshot download";

#[test]
fn interrupted_download_staging_is_replaced_before_resume() {
    let Some(root) = std::env::var_os(DOWNLOAD_EFFECT_CHILD_ROOT_ENV) else {
        prove_interrupted_download_recovery();
        return;
    };

    let root = PathBuf::from(root);
    let handshake_root = PathBuf::from(
        std::env::var_os(DOWNLOAD_EFFECT_CHILD_HANDSHAKE_ENV)
            .expect("download effect handshake root"),
    );
    let mut executor = DownloadEffectExecutor::new(Some(handshake_root), false);
    backup_run_execute_with_executor(&runner_config(root, Some(1)), &mut executor)
        .expect("download effect child remains at armed barrier");
    panic!("download effect child passed its armed barrier");
}

#[test]
fn pending_download_rejects_unsafe_staging_entry_without_following_it() {
    let (root, layout) = prepared_download_operation("unsafe-download-stage");
    let pending = mark_download_pending(&layout);
    let download = pending
        .next_ready_operation()
        .cloned()
        .expect("pending download operation");
    let temp_path = artifact_temp_path(&layout);
    let outside = temp_dir("canic-backup-download-stage-outside");
    fs::create_dir_all(&outside).expect("create outside directory");
    let sentinel = outside.join("sentinel");
    fs::write(&sentinel, b"must survive").expect("write outside sentinel");
    symlink(&outside, &temp_path).expect("create staging symlink");

    let mut executor = DownloadEffectExecutor::new(None, false);
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("unsafe staging entry must reject");
    let persisted = layout
        .read_execution_journal()
        .expect("read rejected execution journal");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::ArtifactTempPathUnsafeEntry {
            sequence,
            target_canister_id,
            kind,
            ..
        } if sequence == download.sequence
            && target_canister_id == target(&download)
            && kind == "Symlink"
    );
    assert_eq!(
        persisted.operations[download.sequence].state,
        BackupExecutionOperationState::Failed
    );
    assert_eq!(
        persisted
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == download.sequence)
            .count(),
        1
    );
    assert!(executor.commands.is_empty());
    assert_eq!(
        fs::read(&sentinel).expect("read outside sentinel"),
        b"must survive"
    );

    fs::remove_file(temp_path).expect("remove staging symlink");
    fs::remove_dir_all(outside).expect("remove outside directory");
    fs::remove_dir_all(root).expect("remove unsafe staging layout");
}

#[test]
fn pending_download_adopts_downloaded_journal_evidence_without_redownload() {
    let (root, layout) = prepared_download_operation("downloaded-evidence");
    let pending = mark_download_pending(&layout);
    let download = pending
        .next_ready_operation()
        .cloned()
        .expect("pending download operation");
    let temp_path = artifact_temp_path(&layout);
    fs::create_dir_all(&temp_path).expect("create completed staging directory");
    fs::write(temp_path.join("snapshot.bin"), COMPLETE_BYTES)
        .expect("write completed staged snapshot");
    let mut artifact_journal = layout.read_journal().expect("read artifact journal");
    artifact_journal.artifacts[0].temp_path = Some(temp_path.display().to_string());
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::Downloaded, "unix:30".to_string())
        .expect("advance artifact to downloaded");
    layout
        .write_journal(&artifact_journal)
        .expect("write downloaded artifact evidence");

    let mut executor = DownloadEffectExecutor::new(None, false);
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect("exact downloaded evidence rebuilds the execution receipt");
    let persisted = layout
        .read_execution_journal()
        .expect("read reconciled execution journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        persisted.operations[download.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(
        persisted
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == download.sequence)
            .count(),
        1
    );
    assert!(executor.commands.is_empty());
    assert_eq!(
        fs::read(temp_path.join("snapshot.bin")).expect("read preserved staged snapshot"),
        COMPLETE_BYTES
    );
    fs::remove_dir_all(root).expect("remove downloaded evidence layout");
}

#[test]
fn pending_downloaded_evidence_rejects_missing_staging_before_receipt() {
    let (root, layout) = prepared_download_operation("missing-downloaded-stage");
    let pending = mark_download_pending(&layout);
    let download = pending
        .next_ready_operation()
        .cloned()
        .expect("pending download operation");
    let temp_path = artifact_temp_path(&layout);
    let mut artifact_journal = layout.read_journal().expect("read artifact journal");
    artifact_journal.artifacts[0].temp_path = Some(temp_path.display().to_string());
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::Downloaded, "unix:30".to_string())
        .expect("advance artifact to downloaded");
    layout
        .write_journal(&artifact_journal)
        .expect("write downloaded artifact evidence");

    let mut executor = DownloadEffectExecutor::new(None, false);
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("downloaded evidence without staging must reject");
    let persisted = layout
        .read_execution_journal()
        .expect("read rejected execution journal");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::ArtifactTempPathMissing {
            sequence,
            target_canister_id,
            path,
        } if sequence == download.sequence
            && target_canister_id == target(&download)
            && path == temp_path.display().to_string()
    );
    assert_eq!(persisted, pending);
    assert!(executor.commands.is_empty());
    fs::remove_dir_all(root).expect("remove missing downloaded staging layout");
}

fn prove_interrupted_download_recovery() {
    assert_case_defined("CANIC-094-B09/download-snapshot/effect-committed-receipt-missing");
    let (root, layout) = prepared_download_operation("download-effect");
    let handshake_root = temp_dir("canic-backup-download-effect-handshake");
    fs::create_dir_all(&handshake_root).expect("create download effect handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::download_effect::interrupted_download_staging_is_replaced_before_resume",
            "--nocapture",
        ])
        .env(DOWNLOAD_EFFECT_CHILD_ROOT_ENV, &root)
        .env(DOWNLOAD_EFFECT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn download effect child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted download journal");
    let download = interrupted
        .next_ready_operation()
        .cloned()
        .expect("pending download operation");
    let artifact_before = layout
        .read_journal()
        .expect("read created artifact journal");
    let temp_path = artifact_temp_path(&layout);

    assert_eq!(download.kind, BackupOperationKind::DownloadSnapshot);
    assert_eq!(download.state, BackupExecutionOperationState::Pending);
    assert_eq!(artifact_before.artifacts[0].state, ArtifactState::Created);
    assert!(artifact_before.artifacts[0].temp_path.is_none());
    assert_eq!(
        fs::read(temp_path.join("snapshot.bin")).expect("read interrupted staged bytes"),
        INTERRUPTED_BYTES
    );

    let mut recovery_executor = DownloadEffectExecutor::new(None, true);
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut recovery_executor,
    )
    .expect("replace uncommitted staging and resume download");
    let recovered = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let artifact_after = layout
        .read_journal()
        .expect("read downloaded artifact journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        recovery_executor.commands,
        vec![format!(
            "download:{}:{}",
            target(&download),
            artifact_after.artifacts[0].snapshot_id
        )]
    );
    assert_eq!(
        recovered.operations[download.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(artifact_after.artifacts[0].state, ArtifactState::Downloaded);
    assert_eq!(
        artifact_after.artifacts[0].temp_path.as_deref(),
        Some(temp_path.to_string_lossy().as_ref())
    );
    assert!(artifact_after.artifacts[0].checksum.is_none());
    assert_eq!(
        fs::read(temp_path.join("snapshot.bin")).expect("read completed staged bytes"),
        COMPLETE_BYTES
    );
    assert!(
        !root
            .join(&artifact_after.artifacts[0].artifact_path)
            .exists()
    );
    assert_eq!(
        recovered
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == download.sequence)
            .count(),
        1
    );

    let mut replay_executor = DownloadEffectExecutor::new(None, false);
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("completed download replay performs no work");
    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove download effect layout");
    fs::remove_dir_all(handshake_root).expect("remove download effect handshake root");
}

pub(super) fn prepared_download_operation(name: &str) -> (PathBuf, BackupLayout) {
    let root = temp_dir(&format!("canic-backup-{name}"));
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let mut executor = FakeBackupRunnerExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(3)), &mut executor)
            .expect("complete lifecycle and snapshot before download");
    let prepared = layout
        .read_execution_journal()
        .expect("read prepared download journal");

    assert_eq!(response.executed_operation_count, 3);
    assert_eq!(
        prepared
            .next_ready_operation()
            .map(|operation| &operation.kind),
        Some(&BackupOperationKind::DownloadSnapshot)
    );
    (root, layout)
}

pub(super) fn mark_download_pending(layout: &BackupLayout) -> BackupExecutionJournal {
    let mut journal = layout
        .read_execution_journal()
        .expect("read prepared execution journal");
    let download = journal
        .next_ready_operation()
        .cloned()
        .expect("ready download operation");
    journal
        .mark_operation_pending_at(download.sequence, Some("unix:20".to_string()))
        .expect("mark download pending");
    layout
        .write_execution_journal(&journal)
        .expect("write pending download journal");
    journal
}

pub(super) fn artifact_temp_path(layout: &BackupLayout) -> PathBuf {
    let journal = layout.read_journal().expect("read artifact journal");
    layout
        .root()
        .join(format!("{}.tmp", journal.artifacts[0].artifact_path))
}

struct DownloadEffectExecutor {
    delegate: FakeBackupRunnerExecutor,
    crash_after_download: Option<PathBuf>,
    require_clean_stage: bool,
    commands: Vec<String>,
}

impl DownloadEffectExecutor {
    fn new(crash_after_download: Option<PathBuf>, require_clean_stage: bool) -> Self {
        Self {
            delegate: FakeBackupRunnerExecutor::default(),
            crash_after_download,
            require_clean_stage,
            commands: Vec::new(),
        }
    }
}

impl BackupRunnerExecutor for DownloadEffectExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        self.delegate
            .preflight_receipts(plan, preflight_id, validated_at, expires_at)
    }

    fn canister_status(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError> {
        self.delegate.canister_status(canister_id)
    }

    fn snapshot_inventory(
        &mut self,
        canister_id: &str,
    ) -> Result<Vec<BackupRunnerSnapshot>, BackupRunnerCommandError> {
        self.delegate.snapshot_inventory(canister_id)
    }

    fn stop_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.delegate.stop_canister(canister_id, command_lifetime)
    }

    fn start_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.delegate.start_canister(canister_id, command_lifetime)
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshot, BackupRunnerCommandError> {
        self.delegate.create_snapshot(canister_id, command_lifetime)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands
            .push(format!("download:{canister_id}:{snapshot_id}"));
        if self.require_clean_stage
            && fs::read_dir(artifact_path)
                .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?
                .next()
                .is_some()
        {
            return Err(BackupRunnerCommandError::failed(
                "stale-stage",
                "download staging was not cleared before retry",
            ));
        }
        let bytes = if self.crash_after_download.is_some() {
            INTERRUPTED_BYTES
        } else {
            COMPLETE_BYTES
        };
        let path = artifact_path.join("snapshot.bin");
        let mut file = File::create(&path)
            .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?;
        File::open(artifact_path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?;
        if let Some(handshake_root) = &self.crash_after_download {
            hold_at_acknowledged_barrier(handshake_root);
        }
        Ok(())
    }
}

pub(super) fn target(operation: &crate::execution::BackupExecutionJournalOperation) -> &str {
    operation
        .target_canister_id
        .as_deref()
        .expect("download operation target")
}

pub(super) fn runner_config(out: PathBuf, max_steps: Option<usize>) -> BackupRunnerConfig {
    BackupRunnerConfig {
        out,
        max_steps,
        updated_at: Some("unix:10".to_string()),
        tool_name: "canic".to_string(),
        tool_version: "test".to_string(),
    }
}
