//! Module: persistence::tests::operational_readiness::lifecycle_effect
//!
//! Responsibility: prove recovery after a committed lifecycle effect loses its receipt.
//! Does not own: general canister lifecycle policy or snapshot recovery.
//! Boundary: binds command quiescence and exact status to stop/start reconciliation.

use super::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier};
use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
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

const STOP_EFFECT_CHILD_ROOT_ENV: &str = "CANIC_TEST_STOP_EFFECT_ROOT";
const STOP_EFFECT_CHILD_STATE_ENV: &str = "CANIC_TEST_STOP_EFFECT_STATE";
const STOP_EFFECT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_STOP_EFFECT_HANDSHAKE";
const START_EFFECT_CHILD_ROOT_ENV: &str = "CANIC_TEST_START_EFFECT_ROOT";
const START_EFFECT_CHILD_STATE_ENV: &str = "CANIC_TEST_START_EFFECT_STATE";
const START_EFFECT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_START_EFFECT_HANDSHAKE";

#[test]
fn committed_stop_without_receipt_is_reconciled_from_exact_status() {
    let Some(root) = std::env::var_os(STOP_EFFECT_CHILD_ROOT_ENV) else {
        prove_committed_stop_recovery();
        return;
    };

    let root = PathBuf::from(root);
    let state_path = PathBuf::from(
        std::env::var_os(STOP_EFFECT_CHILD_STATE_ENV).expect("stop effect state path"),
    );
    let handshake_root = PathBuf::from(
        std::env::var_os(STOP_EFFECT_CHILD_HANDSHAKE_ENV).expect("stop effect handshake root"),
    );
    let mut executor = LifecycleEffectExecutor::new(state_path).crash_after_stop(handshake_root);
    backup_run_execute_with_executor(&runner_config(root, Some(1)), &mut executor)
        .expect("stop effect child remains at armed barrier");
    panic!("stop effect child passed its armed barrier");
}

#[test]
fn committed_start_without_receipt_is_reconciled_from_exact_status() {
    let Some(root) = std::env::var_os(START_EFFECT_CHILD_ROOT_ENV) else {
        prove_committed_start_recovery();
        return;
    };

    let root = PathBuf::from(root);
    let state_path = PathBuf::from(
        std::env::var_os(START_EFFECT_CHILD_STATE_ENV).expect("start effect state path"),
    );
    let handshake_root = PathBuf::from(
        std::env::var_os(START_EFFECT_CHILD_HANDSHAKE_ENV).expect("start effect handshake root"),
    );
    let mut executor = LifecycleEffectExecutor::new(state_path).crash_after_start(handshake_root);
    backup_run_execute_with_executor(&runner_config(root, Some(1)), &mut executor)
        .expect("start effect child remains at armed barrier");
    panic!("start effect child passed its armed barrier");
}

#[test]
fn committed_start_command_failure_is_reconciled_during_containment() {
    let (root, state_path, layout) = prepared_start_operation("failed-start-command");
    let mut failing_executor = LifecycleEffectExecutor::new(state_path.clone()).fail_after_start();

    let error = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut failing_executor,
    )
    .expect_err("simulated post-effect start command failure");
    let failed = layout
        .read_execution_journal()
        .expect("read failed start execution journal");
    let start = failed
        .operations
        .iter()
        .find(|operation| operation.kind == crate::plan::BackupOperationKind::Start)
        .cloned()
        .expect("contained start operation");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::CommandFailed { status, .. }
            if status == "start-output"
    );
    assert_eq!(start.state, BackupExecutionOperationState::Completed);
    assert_eq!(read_state(&state_path), "Running");
    assert_eq!(
        failing_executor.commands,
        vec![
            format!("start:{}", target(&start)),
            format!("status:{}", target(&start)),
        ]
    );
    assert_eq!(
        failed
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == start.sequence)
            .count(),
        2
    );

    let mut replay_executor = LifecycleEffectExecutor::new(state_path).reject_start();
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("completed containment is idempotently observable");
    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());
    fs::remove_dir_all(root).expect("remove failed start recovery layout");
}

#[test]
fn pending_stop_rejects_unsettled_status_without_mutation() {
    let (root, state_path, layout, pending) = prepared_pending_stop("stopping", "Stopping");
    let mut executor = LifecycleEffectExecutor::new(state_path);
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("unsettled stop status must halt");
    let persisted = layout
        .read_execution_journal()
        .expect("read unsettled stop journal");
    let stop = pending
        .next_ready_operation()
        .expect("pending stop operation");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::CanisterStatusUnsettled {
            sequence,
            operation_id,
            status: "Stopping",
        } if sequence == stop.sequence && operation_id == stop.operation_id
    );
    assert_eq!(persisted, pending);
    assert_eq!(executor.commands, vec![format!("status:{}", target(stop))]);
    fs::remove_dir_all(root).expect("remove unsettled stop layout");
}

#[test]
fn pending_stop_preserves_typed_status_observation_failure() {
    let (root, state_path, layout, pending) = prepared_pending_stop("invalid-status", "Deleted");
    let mut executor = LifecycleEffectExecutor::new(state_path);
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("invalid stop status must halt");
    let persisted = layout
        .read_execution_journal()
        .expect("read failed status journal");
    let stop = pending
        .next_ready_operation()
        .expect("pending stop operation");

    std::assert_matches!(
        error,
        crate::runner::BackupRunnerError::CanisterStatusFailed {
            sequence,
            status,
            ..
        } if sequence == stop.sequence && status == "test-status"
    );
    assert_eq!(persisted, pending);
    assert_eq!(executor.commands, vec![format!("status:{}", target(stop))]);
    fs::remove_dir_all(root).expect("remove failed status layout");
}

fn prove_committed_stop_recovery() {
    let case_id = "CANIC-094-B05/stop/effect-committed-receipt-missing";
    assert_case_defined(case_id);
    let root = temp_dir("canic-backup-stop-effect");
    let handshake_root = temp_dir("canic-backup-stop-effect-handshake");
    fs::create_dir_all(&handshake_root).expect("create stop effect handshake root");
    let state_path = root.join("test-canister-state");
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    let mut preflight_executor = FakeBackupRunnerExecutor::default();
    backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut preflight_executor,
    )
    .expect("accept backup preflight");
    write_state(&state_path, "Running");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::lifecycle_effect::committed_stop_without_receipt_is_reconciled_from_exact_status",
            "--nocapture",
        ])
        .env(STOP_EFFECT_CHILD_ROOT_ENV, &root)
        .env(STOP_EFFECT_CHILD_STATE_ENV, &state_path)
        .env(STOP_EFFECT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn stop effect child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted stop journal");
    let stop = interrupted
        .next_ready_operation()
        .cloned()
        .expect("pending stop operation");

    assert_eq!(read_state(&state_path), "Stopped");
    assert_eq!(stop.state, BackupExecutionOperationState::Pending);
    assert!(
        interrupted
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != stop.sequence)
    );

    let mut recovery_executor = LifecycleEffectExecutor::new(state_path.clone());
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut recovery_executor,
    )
    .expect("reconcile committed stop from status");
    let reconciled = layout
        .read_execution_journal()
        .expect("read reconciled stop journal");
    let receipts = reconciled
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == stop.sequence)
        .count();

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        recovery_executor.commands,
        vec![format!("status:{}", target(&stop))]
    );
    assert_eq!(
        reconciled.operations[stop.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(receipts, 1);

    let mut replay_executor = LifecycleEffectExecutor::new(state_path);
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("replay skips reconciled stop");
    let replayed = layout
        .read_execution_journal()
        .expect("read replayed stop journal");

    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());
    assert_eq!(replayed.operation_receipts, reconciled.operation_receipts);

    fs::remove_dir_all(root).expect("remove stop effect layout");
    fs::remove_dir_all(handshake_root).expect("remove stop effect handshake root");
}

fn prove_committed_start_recovery() {
    let case_id = "CANIC-094-B08/start/effect-committed-receipt-missing";
    assert_case_defined(case_id);
    let (root, state_path, layout) = prepared_start_operation("start-effect");
    let handshake_root = temp_dir("canic-backup-start-effect-handshake");
    fs::create_dir_all(&handshake_root).expect("create start effect handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::lifecycle_effect::committed_start_without_receipt_is_reconciled_from_exact_status",
            "--nocapture",
        ])
        .env(START_EFFECT_CHILD_ROOT_ENV, &root)
        .env(START_EFFECT_CHILD_STATE_ENV, &state_path)
        .env(START_EFFECT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn start effect child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted start journal");
    let start = interrupted
        .next_ready_operation()
        .cloned()
        .expect("pending start operation");

    assert_eq!(read_state(&state_path), "Running");
    assert_eq!(start.state, BackupExecutionOperationState::Pending);
    assert!(
        interrupted
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != start.sequence)
    );

    let mut recovery_executor = LifecycleEffectExecutor::new(state_path.clone()).reject_start();
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(1)),
        &mut recovery_executor,
    )
    .expect("reconcile committed start from status");
    let reconciled = layout
        .read_execution_journal()
        .expect("read reconciled start journal");
    let receipts = reconciled
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == start.sequence)
        .count();

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        recovery_executor.commands,
        vec![format!("status:{}", target(&start))]
    );
    assert_eq!(
        reconciled.operations[start.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(receipts, 1);

    let mut replay_executor = LifecycleEffectExecutor::new(state_path);
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("replay skips reconciled start");
    let replayed = layout
        .read_execution_journal()
        .expect("read replayed start journal");

    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());
    assert_eq!(replayed.operation_receipts, reconciled.operation_receipts);

    fs::remove_dir_all(root).expect("remove start effect layout");
    fs::remove_dir_all(handshake_root).expect("remove start effect handshake root");
}

fn prepared_start_operation(name: &str) -> (PathBuf, PathBuf, BackupLayout) {
    let root = temp_dir(&format!("canic-backup-{name}"));
    let state_path = root.join("test-canister-state");
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    write_state(&state_path, "Running");

    let mut executor = LifecycleEffectExecutor::new(state_path.clone());
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(2)), &mut executor)
            .expect("complete stop and snapshot before start");
    let prepared = layout
        .read_execution_journal()
        .expect("read prepared start journal");

    assert_eq!(response.executed_operation_count, 2);
    assert_eq!(read_state(&state_path), "Stopped");
    assert_eq!(
        prepared
            .next_ready_operation()
            .map(|operation| &operation.kind),
        Some(&crate::plan::BackupOperationKind::Start)
    );
    (root, state_path, layout)
}

fn prepared_pending_stop(
    name: &str,
    state: &str,
) -> (PathBuf, PathBuf, BackupLayout, BackupExecutionJournal) {
    let root = temp_dir(&format!("canic-backup-pending-stop-{name}"));
    let state_path = root.join("test-canister-state");
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    let mut executor = FakeBackupRunnerExecutor::default();
    backup_run_execute_with_executor(&runner_config(root.clone(), Some(0)), &mut executor)
        .expect("accept backup preflight");
    let mut pending = layout
        .read_execution_journal()
        .expect("read accepted execution journal");
    let stop = pending
        .next_ready_operation()
        .cloned()
        .expect("ready stop operation");
    pending
        .mark_operation_pending_at(stop.sequence, Some("unix:20".to_string()))
        .expect("mark stop pending");
    layout
        .write_execution_journal(&pending)
        .expect("write pending stop journal");
    write_state(&state_path, state);
    (root, state_path, layout, pending)
}

struct LifecycleEffectExecutor {
    delegate: FakeBackupRunnerExecutor,
    state_path: PathBuf,
    crash_after_stop: Option<PathBuf>,
    crash_after_start: Option<PathBuf>,
    fail_after_start: bool,
    reject_start: bool,
    commands: Vec<String>,
}

impl LifecycleEffectExecutor {
    fn new(state_path: PathBuf) -> Self {
        Self {
            delegate: FakeBackupRunnerExecutor::default(),
            state_path,
            crash_after_stop: None,
            crash_after_start: None,
            fail_after_start: false,
            reject_start: false,
            commands: Vec::new(),
        }
    }

    fn crash_after_stop(mut self, handshake_root: PathBuf) -> Self {
        self.crash_after_stop = Some(handshake_root);
        self
    }

    fn crash_after_start(mut self, handshake_root: PathBuf) -> Self {
        self.crash_after_start = Some(handshake_root);
        self
    }

    fn fail_after_start(mut self) -> Self {
        self.fail_after_start = true;
        self
    }

    fn reject_start(mut self) -> Self {
        self.reject_start = true;
        self
    }
}

impl BackupRunnerExecutor for LifecycleEffectExecutor {
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
        self.commands.push(format!("status:{canister_id}"));
        match read_state(&self.state_path).as_str() {
            "Running" => Ok(BackupRunnerCanisterStatus::Running),
            "Stopped" => Ok(BackupRunnerCanisterStatus::Stopped),
            "Stopping" => Ok(BackupRunnerCanisterStatus::Stopping),
            state => Err(BackupRunnerCommandError::failed(
                "test-status",
                format!("unsupported state {state}"),
            )),
        }
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
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("stop:{canister_id}"));
        write_state(&self.state_path, "Stopped");
        if let Some(handshake_root) = &self.crash_after_stop {
            hold_at_acknowledged_barrier(handshake_root);
        }
        Ok(())
    }

    fn start_canister(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("start:{canister_id}"));
        if self.reject_start {
            return Err(BackupRunnerCommandError::failed(
                "duplicate-start",
                "start was not expected during reconciliation",
            ));
        }
        write_state(&self.state_path, "Running");
        if let Some(handshake_root) = &self.crash_after_start {
            hold_at_acknowledged_barrier(handshake_root);
        }
        if self.fail_after_start {
            return Err(BackupRunnerCommandError::failed(
                "start-output",
                "start committed before command output failed",
            ));
        }
        Ok(())
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
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.delegate
            .download_snapshot(canister_id, snapshot_id, artifact_path, command_lifetime)
    }
}

fn write_state(path: &Path, state: &str) {
    let mut file = File::create(path).expect("create test canister state");
    file.write_all(state.as_bytes())
        .expect("write test canister state");
    file.sync_all().expect("sync test canister state");
}

fn read_state(path: &Path) -> String {
    fs::read_to_string(path).expect("read test canister state")
}

fn target(operation: &crate::execution::BackupExecutionJournalOperation) -> &str {
    operation
        .target_canister_id
        .as_deref()
        .expect("lifecycle operation target")
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
