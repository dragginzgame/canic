//! Module: persistence::tests::operational_readiness::snapshot_create
//!
//! Responsibility: prove exact snapshot-create reconciliation across lost local receipts.
//! Does not own: later download, publication, or restore recovery.
//! Boundary: binds durable pre-effect inventory and artifact evidence to one snapshot identity.

use super::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier};
use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics},
    operational_readiness::manifest::assert_case_defined,
    persistence::{BackupLayout, CommandLifetimeHandle},
    plan::{BackupExecutionPreflightReceipts, BackupPlan},
    runner::{
        BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerConfig,
        BackupRunnerExecutor, BackupRunnerSnapshot, backup_run_execute_with_executor,
    },
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const SNAPSHOT_ID: &str = "0000000000000000ffffffffffc000020101";
const SNAPSHOT_TAKEN_AT: u64 = 1_778_709_681_897_818_005;
const SNAPSHOT_SIZE: u64 = 272_586_987;
const SNAPSHOT_EFFECT_CHILD_ROOT_ENV: &str = "CANIC_TEST_SNAPSHOT_EFFECT_ROOT";
const SNAPSHOT_EFFECT_CHILD_STATE_ENV: &str = "CANIC_TEST_SNAPSHOT_EFFECT_STATE";
const SNAPSHOT_EFFECT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_SNAPSHOT_EFFECT_HANDSHAKE";

#[test]
fn committed_snapshot_without_artifact_or_receipt_is_reconciled_from_exact_inventory() {
    let Some(root) = std::env::var_os(SNAPSHOT_EFFECT_CHILD_ROOT_ENV) else {
        prove_committed_snapshot_recovery();
        return;
    };

    let root = PathBuf::from(root);
    let state_path = PathBuf::from(
        std::env::var_os(SNAPSHOT_EFFECT_CHILD_STATE_ENV).expect("snapshot effect state path"),
    );
    let handshake_root = PathBuf::from(
        std::env::var_os(SNAPSHOT_EFFECT_CHILD_HANDSHAKE_ENV)
            .expect("snapshot effect handshake root"),
    );
    let mut executor = SnapshotEffectExecutor::new(state_path, Some(handshake_root), false, false);
    backup_run_execute_with_executor(&runner_config(root, Some(1)), &mut executor)
        .expect("snapshot effect child remains at armed barrier");
    panic!("snapshot effect child passed its armed barrier");
}

#[test]
fn exact_created_artifact_rebuilds_the_complete_execution_receipt() {
    assert_case_defined("CANIC-094-B07/created-artifact-journal-publication/after-durable-write");
    let root = prepared_snapshot_operation("canic-backup-created-artifact-recovery");
    let layout = BackupLayout::new(root.clone());
    let plan = layout.read_backup_plan().expect("read backup plan");
    let mut journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let operation = journal
        .next_ready_operation()
        .cloned()
        .expect("ready snapshot operation");
    journal
        .mark_snapshot_create_pending_at(
            operation.sequence,
            Some("unix:20".to_string()),
            vec!["preexisting-snapshot".to_string()],
        )
        .expect("mark snapshot pending");
    layout
        .write_execution_journal(&journal)
        .expect("write pending execution journal");
    layout
        .write_journal(&created_artifact_journal(&plan, &target(&operation)))
        .expect("write created artifact journal");

    let mut executor = SnapshotEffectExecutor::new(root.join("unused-state"), None, true, false);
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect("rebuild execution receipt from artifact evidence");
    let recovered = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let receipt = recovered
        .operation_receipts
        .iter()
        .find(|receipt| receipt.sequence == operation.sequence)
        .expect("reconstructed snapshot receipt");

    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());
    assert_eq!(receipt.snapshot_id.as_deref(), Some(SNAPSHOT_ID));
    assert_eq!(receipt.snapshot_taken_at_timestamp, Some(SNAPSHOT_TAKEN_AT));
    assert_eq!(receipt.snapshot_total_size_bytes, Some(SNAPSHOT_SIZE));
    fs::remove_dir_all(root).expect("remove artifact recovery layout");
}

#[test]
fn pending_snapshot_rejects_multiple_new_inventory_identities_without_creating() {
    let root = prepared_snapshot_operation("canic-backup-snapshot-ambiguous");
    let layout = BackupLayout::new(root.clone());
    let (pending, operation, canister_id) =
        mark_snapshot_pending(&layout, vec!["preexisting-snapshot".to_string()]);
    let mut executor = FakeBackupRunnerExecutor::default();
    executor.snapshots.insert(
        canister_id.clone(),
        vec![
            snapshot("preexisting-snapshot".to_string()),
            snapshot("new-snapshot-a".to_string()),
            snapshot("new-snapshot-b".to_string()),
        ],
    );

    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("ambiguous inventory delta must reject");
    let persisted = layout
        .read_execution_journal()
        .expect("read rejected execution journal");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::SnapshotIdentityAmbiguous {
            sequence,
            operation_id,
            snapshot_ids,
        } if sequence == operation.sequence
            && operation_id == operation.operation_id
            && snapshot_ids == vec!["new-snapshot-a".to_string(), "new-snapshot-b".to_string()]
    );
    assert_eq!(persisted, pending);
    assert_eq!(
        executor.commands,
        vec![format!("snapshot-list:{canister_id}")]
    );
    assert!(!layout.journal_path().exists());
    fs::remove_dir_all(root).expect("remove ambiguous inventory layout");
}

#[test]
fn pending_snapshot_rejects_lost_baseline_identity_without_creating() {
    let root = prepared_snapshot_operation("canic-backup-snapshot-baseline-loss");
    let layout = BackupLayout::new(root.clone());
    let (pending, operation, canister_id) =
        mark_snapshot_pending(&layout, vec!["preexisting-snapshot".to_string()]);
    let mut executor = FakeBackupRunnerExecutor::default();
    executor.snapshots.insert(
        canister_id.clone(),
        vec![snapshot("unrelated-new-snapshot".to_string())],
    );

    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("lost baseline identity must reject");
    let persisted = layout
        .read_execution_journal()
        .expect("read rejected execution journal");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::SnapshotInventoryLostBaseline {
            sequence,
            operation_id,
            snapshot_ids,
        } if sequence == operation.sequence
            && operation_id == operation.operation_id
            && snapshot_ids == vec!["preexisting-snapshot".to_string()]
    );
    assert_eq!(persisted, pending);
    assert_eq!(
        executor.commands,
        vec![format!("snapshot-list:{canister_id}")]
    );
    assert!(!layout.journal_path().exists());
    fs::remove_dir_all(root).expect("remove lost baseline layout");
}

#[test]
fn explicit_retry_after_failed_snapshot_command_reconciles_original_inventory() {
    let root = prepared_snapshot_operation("canic-backup-snapshot-failed-command");
    let layout = BackupLayout::new(root.clone());
    let state_path = root.join("test-snapshot-inventory");
    let mut failing_executor = SnapshotEffectExecutor::new(state_path.clone(), None, false, true);

    let error = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut failing_executor,
    )
    .expect_err("simulated post-effect command failure");
    let failed = layout
        .read_execution_journal()
        .expect("read failed execution journal");
    let operation = failed
        .next_ready_operation()
        .cloned()
        .expect("failed snapshot operation");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::CommandFailed { status, .. }
            if status == "snapshot-output"
    );
    assert_eq!(operation.state, BackupExecutionOperationState::Failed);
    assert_eq!(operation.snapshot_ids_before, Some(Vec::new()));
    assert_eq!(read_snapshot_id(&state_path).as_deref(), Some(SNAPSHOT_ID));

    let mut retrying = failed;
    retrying
        .retry_failed_operation_at(operation.sequence, Some("unix:30".to_string()))
        .expect("mark failed snapshot ready for explicit retry");
    assert_eq!(
        retrying
            .next_ready_operation()
            .expect("ready snapshot operation")
            .snapshot_ids_before,
        Some(Vec::new())
    );
    layout
        .write_execution_journal(&retrying)
        .expect("write explicit retry transition");

    let mut recovery_executor = SnapshotEffectExecutor::new(state_path, None, true, false);
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut recovery_executor,
    )
    .expect("reconcile original failed-attempt inventory");
    let recovered = layout
        .read_execution_journal()
        .expect("read recovered execution journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        recovery_executor.commands,
        vec![format!("snapshot-list:{}", target(&operation))]
    );
    assert_eq!(
        recovered.operations[operation.sequence].state,
        BackupExecutionOperationState::Completed
    );
    fs::remove_dir_all(root).expect("remove failed command recovery layout");
}

fn prove_committed_snapshot_recovery() {
    assert_case_defined("CANIC-094-B06/create-snapshot/effect-committed-receipt-missing");
    assert_case_defined("CANIC-094-B07/created-artifact-journal-publication/before-durable-write");
    let root = prepared_snapshot_operation("canic-backup-snapshot-effect");
    let handshake_root = temp_dir("canic-backup-snapshot-effect-handshake");
    fs::create_dir_all(&handshake_root).expect("create snapshot effect handshake root");
    let state_path = root.join("test-snapshot-inventory");
    let layout = BackupLayout::new(root.clone());

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::snapshot_create::committed_snapshot_without_artifact_or_receipt_is_reconciled_from_exact_inventory",
            "--nocapture",
        ])
        .env(SNAPSHOT_EFFECT_CHILD_ROOT_ENV, &root)
        .env(SNAPSHOT_EFFECT_CHILD_STATE_ENV, &state_path)
        .env(SNAPSHOT_EFFECT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn snapshot effect child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted snapshot journal");
    let operation = interrupted
        .next_ready_operation()
        .cloned()
        .expect("pending snapshot operation");

    assert_eq!(read_snapshot_id(&state_path).as_deref(), Some(SNAPSHOT_ID));
    assert_eq!(operation.state, BackupExecutionOperationState::Pending);
    assert_eq!(operation.snapshot_ids_before, Some(Vec::new()));
    assert!(!layout.journal_path().exists());
    assert!(
        interrupted
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != operation.sequence)
    );

    let mut recovery_executor = SnapshotEffectExecutor::new(state_path.clone(), None, true, false);
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut recovery_executor,
    )
    .expect("reconcile committed snapshot from exact inventory delta");
    let reconciled = layout
        .read_execution_journal()
        .expect("read reconciled snapshot journal");
    let artifact_journal = layout
        .read_journal()
        .expect("read created artifact journal");
    let receipt = reconciled
        .operation_receipts
        .iter()
        .find(|receipt| receipt.sequence == operation.sequence)
        .expect("snapshot execution receipt");
    let artifact = artifact_journal
        .artifacts
        .first()
        .expect("snapshot artifact");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        recovery_executor.commands,
        vec![format!("snapshot-list:{}", target(&operation))]
    );
    assert_eq!(receipt.snapshot_id.as_deref(), Some(SNAPSHOT_ID));
    assert_eq!(receipt.snapshot_taken_at_timestamp, Some(SNAPSHOT_TAKEN_AT));
    assert_eq!(receipt.snapshot_total_size_bytes, Some(SNAPSHOT_SIZE));
    assert_eq!(artifact.snapshot_id, SNAPSHOT_ID);
    assert_eq!(
        artifact.snapshot_taken_at_timestamp,
        Some(SNAPSHOT_TAKEN_AT)
    );
    assert_eq!(artifact.snapshot_total_size_bytes, Some(SNAPSHOT_SIZE));

    let mut replay_executor = SnapshotEffectExecutor::new(state_path, None, true, false);
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("replay skips reconciled snapshot");
    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove snapshot effect layout");
    fs::remove_dir_all(handshake_root).expect("remove snapshot effect handshake root");
}

fn mark_snapshot_pending(
    layout: &BackupLayout,
    snapshot_ids_before: Vec<String>,
) -> (
    BackupExecutionJournal,
    crate::execution::BackupExecutionJournalOperation,
    String,
) {
    let mut journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let operation = journal
        .next_ready_operation()
        .cloned()
        .expect("ready snapshot operation");
    let canister_id = target(&operation);
    journal
        .mark_snapshot_create_pending_at(
            operation.sequence,
            Some("unix:20".to_string()),
            snapshot_ids_before,
        )
        .expect("mark snapshot pending");
    layout
        .write_execution_journal(&journal)
        .expect("write pending execution journal");
    (journal, operation, canister_id)
}

fn prepared_snapshot_operation(name: &str) -> PathBuf {
    let root = temp_dir(name);
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    let mut executor = FakeBackupRunnerExecutor::default();
    backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
        .expect("complete stop before snapshot operation");
    root
}

fn created_artifact_journal(plan: &BackupPlan, canister_id: &str) -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: plan.run_id.clone(),
        discovery_topology_hash: plan.topology_hash_before_quiesce.clone(),
        pre_snapshot_topology_hash: plan.topology_hash_before_quiesce.clone(),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: canister_id.to_string(),
            snapshot_id: SNAPSHOT_ID.to_string(),
            snapshot_taken_at_timestamp: Some(SNAPSHOT_TAKEN_AT),
            snapshot_total_size_bytes: Some(SNAPSHOT_SIZE),
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: canister_id.to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: "unix:21".to_string(),
        }],
    }
}

struct SnapshotEffectExecutor {
    delegate: FakeBackupRunnerExecutor,
    state_path: PathBuf,
    crash_after_create: Option<PathBuf>,
    reject_create: bool,
    fail_after_create: bool,
    commands: Vec<String>,
}

impl SnapshotEffectExecutor {
    fn new(
        state_path: PathBuf,
        crash_after_create: Option<PathBuf>,
        reject_create: bool,
        fail_after_create: bool,
    ) -> Self {
        Self {
            delegate: FakeBackupRunnerExecutor::default(),
            state_path,
            crash_after_create,
            reject_create,
            fail_after_create,
            commands: Vec::new(),
        }
    }
}

impl BackupRunnerExecutor for SnapshotEffectExecutor {
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
        self.commands.push(format!("snapshot-list:{canister_id}"));
        Ok(read_snapshot_id(&self.state_path)
            .map(|snapshot_id| vec![snapshot(snapshot_id)])
            .unwrap_or_default())
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
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshot, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot:{canister_id}"));
        if self.reject_create {
            return Err(BackupRunnerCommandError::failed(
                "duplicate-create",
                "snapshot creation was not expected during reconciliation",
            ));
        }
        write_snapshot_id(&self.state_path, SNAPSHOT_ID);
        if let Some(handshake_root) = &self.crash_after_create {
            hold_at_acknowledged_barrier(handshake_root);
        }
        if self.fail_after_create {
            return Err(BackupRunnerCommandError::failed(
                "snapshot-output",
                "snapshot committed before command output failed",
            ));
        }
        Ok(snapshot(SNAPSHOT_ID.to_string()))
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.delegate
            .download_snapshot(canister_id, snapshot_id, artifact_path, command_lifetime)
    }
}

fn snapshot(snapshot_id: String) -> BackupRunnerSnapshot {
    BackupRunnerSnapshot {
        snapshot_id,
        taken_at_timestamp: Some(SNAPSHOT_TAKEN_AT),
        total_size_bytes: Some(SNAPSHOT_SIZE),
    }
}

fn write_snapshot_id(path: &Path, snapshot_id: &str) {
    let mut file = File::create(path).expect("create test snapshot inventory");
    file.write_all(snapshot_id.as_bytes())
        .expect("write test snapshot inventory");
    file.sync_all().expect("sync test snapshot inventory");
}

fn read_snapshot_id(path: &Path) -> Option<String> {
    path.is_file()
        .then(|| fs::read_to_string(path).expect("read test snapshot inventory"))
}

fn target(operation: &crate::execution::BackupExecutionJournalOperation) -> String {
    operation
        .target_canister_id
        .clone()
        .expect("snapshot operation target")
}

fn runner_config(out: PathBuf, max_steps: Option<usize>) -> BackupRunnerConfig {
    BackupRunnerConfig {
        out,
        max_steps,
        updated_at: Some("unix:10".to_string()),
        tool_name: "canic".to_string(),
        tool_version: "test".to_string(),
    }
}
