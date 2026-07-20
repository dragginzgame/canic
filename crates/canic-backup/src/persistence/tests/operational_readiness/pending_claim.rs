//! Module: persistence::tests::operational_readiness::pending_claim
//!
//! Responsibility: execute every backup pending-claim crash case.
//! Does not own: operation-specific effect reconciliation or terminal receipts.
//! Boundary: proves claim publication precedes effects and selects restart policy.

use super::{
    durable_write_barrier, hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
};
use crate::{
    execution::{
        BackupExecutionJournal, BackupExecutionJournalOperation, BackupExecutionOperationState,
    },
    operational_readiness::manifest::{
        assert_case_defined, backup_operation_label, backup_post_preflight_operations,
    },
    persistence::{BackupLayout, write_json_durable_at_barriers},
    plan::BackupOperationKind,
    runner::{BackupRunnerConfig, BackupRunnerError, backup_run_execute_with_executor},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{fs, path::Path, process::Command};

const PENDING_CLAIM_CHILD_ROOT_ENV: &str = "CANIC_TEST_PENDING_CLAIM_ROOT";
const PENDING_CLAIM_CHILD_OPERATION_ENV: &str = "CANIC_TEST_PENDING_CLAIM_OPERATION";
const PENDING_CLAIM_CHILD_BARRIER_ENV: &str = "CANIC_TEST_PENDING_CLAIM_BARRIER";
const PENDING_CLAIM_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_PENDING_CLAIM_HANDSHAKE";

#[test]
fn pending_claim_publication_selects_the_exact_restart_policy() {
    let Some(root) = std::env::var_os(PENDING_CLAIM_CHILD_ROOT_ENV) else {
        for (prior_steps, operation) in backup_post_preflight_operations().into_iter().enumerate() {
            let label = backup_operation_label(&operation);
            for barrier_name in ["before-rename", "after-directory-sync"] {
                let side = if barrier_name == "before-rename" {
                    "before-durable-write"
                } else {
                    "after-durable-write"
                };
                assert_case_defined(&format!("CANIC-094-B04/{label}/{side}"));
                prove_pending_claim_barrier(operation.clone(), label, prior_steps, barrier_name);
            }
        }
        return;
    };

    let root = std::path::PathBuf::from(root);
    let operation_label =
        std::env::var(PENDING_CLAIM_CHILD_OPERATION_ENV).expect("pending operation label");
    let barrier_name =
        std::env::var(PENDING_CLAIM_CHILD_BARRIER_ENV).expect("pending claim barrier");
    let handshake_root = std::path::PathBuf::from(
        std::env::var_os(PENDING_CLAIM_CHILD_HANDSHAKE_ENV).expect("pending claim handshake root"),
    );
    let layout = BackupLayout::new(root);
    let mut journal = layout
        .read_execution_journal()
        .expect("read pre-claim execution journal");
    let operation = journal
        .next_ready_operation()
        .cloned()
        .expect("next pending-claim operation");
    assert_eq!(backup_operation_label(&operation.kind), operation_label);
    if operation.kind == BackupOperationKind::CreateSnapshot {
        journal
            .mark_snapshot_create_pending_at(
                operation.sequence,
                Some("unix:20".to_string()),
                Vec::new(),
            )
            .expect("mark snapshot operation pending in crash child");
    } else {
        journal
            .mark_operation_pending_at(operation.sequence, Some("unix:20".to_string()))
            .expect("mark operation pending in crash child");
    }
    let barrier = durable_write_barrier(&barrier_name);
    write_json_durable_at_barriers(&layout.execution_journal_path(), &journal, |observed| {
        if observed == barrier {
            hold_at_acknowledged_barrier(&handshake_root);
        }
    })
    .expect("write pending claim in crash child");
}

fn prove_pending_claim_barrier(
    expected_kind: BackupOperationKind,
    operation_label: &str,
    prior_steps: usize,
    barrier_name: &str,
) {
    let root = prepare_operation(operation_label, prior_steps, &expected_kind);
    let handshake_root = temp_dir(&format!(
        "canic-backup-pending-claim-handshake-{operation_label}-{barrier_name}"
    ));
    fs::create_dir_all(&handshake_root).expect("create pending claim handshake root");
    let layout = BackupLayout::new(root.clone());
    let before_claim = layout
        .read_execution_journal()
        .expect("read execution journal before claim");
    let expected_operation = before_claim
        .next_ready_operation()
        .cloned()
        .expect("next operation before claim");
    let receipt_count = before_claim.operation_receipts.len();
    let expected_command = expected_command(&before_claim, &expected_operation);
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::pending_claim::pending_claim_publication_selects_the_exact_restart_policy",
            "--nocapture",
        ])
        .env(PENDING_CLAIM_CHILD_ROOT_ENV, &root)
        .env(PENDING_CLAIM_CHILD_OPERATION_ENV, operation_label)
        .env(PENDING_CLAIM_CHILD_BARRIER_ENV, barrier_name)
        .env(PENDING_CLAIM_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn pending claim child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let observed = layout
        .read_execution_journal()
        .expect("read execution journal after claim crash");
    let observed_operation = operation_at(&observed, expected_operation.sequence);

    if barrier_name == "before-rename" {
        assert_eq!(observed, before_claim);
        prove_ready_operation_resumes(&root, &expected_operation, receipt_count, expected_command);
    } else {
        assert_eq!(
            observed_operation.state,
            BackupExecutionOperationState::Pending
        );
        assert_eq!(observed.operation_receipts.len(), receipt_count);
        if expected_kind == BackupOperationKind::Stop {
            prove_pending_stop_observes_then_executes(
                &root,
                &expected_operation,
                receipt_count,
                expected_command.expect("stop command"),
            );
        } else if expected_kind == BackupOperationKind::CreateSnapshot {
            prove_pending_snapshot_without_effect_executes_once(
                &root,
                &expected_operation,
                receipt_count,
                expected_command.expect("snapshot command"),
            );
        } else if expected_kind == BackupOperationKind::Start {
            prove_pending_start_observes_then_executes(
                &root,
                &expected_operation,
                receipt_count,
                expected_command.expect("start command"),
            );
        } else if expected_kind == BackupOperationKind::DownloadSnapshot {
            prove_pending_download_resumes_private_staging(
                &root,
                &expected_operation,
                receipt_count,
                expected_command.expect("download command"),
            );
        } else if expected_command.is_some() {
            prove_unknown_external_operation_halts(&root, &expected_operation, &observed);
        } else {
            prove_replay_safe_operation_resumes(&root, &expected_operation, receipt_count);
        }
    }

    fs::remove_dir_all(root).expect("remove pending claim layout");
    fs::remove_dir_all(handshake_root).expect("remove pending claim handshake root");
}

fn prove_pending_snapshot_without_effect_executes_once(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
    snapshot_command: String,
) {
    let target = expected_operation
        .target_canister_id
        .as_deref()
        .expect("snapshot operation target");
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("empty exact inventory proves pending create effect absent");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read reconciled snapshot journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        executor.commands,
        vec![format!("snapshot-list:{target}"), snapshot_command]
    );
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
}

fn prove_pending_stop_observes_then_executes(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
    stop_command: String,
) {
    let target = expected_operation
        .target_canister_id
        .as_deref()
        .expect("stop operation target");
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("observe running target and execute pending stop");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read reconciled stop journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        executor.commands,
        vec![format!("status:{target}"), stop_command]
    );
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
}

fn prove_pending_start_observes_then_executes(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
    start_command: String,
) {
    let target = expected_operation
        .target_canister_id
        .as_deref()
        .expect("start operation target");
    let mut executor = FakeBackupRunnerExecutor::default();
    executor.canister_statuses.insert(
        target.to_string(),
        crate::runner::BackupRunnerCanisterStatus::Stopped,
    );
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("observe stopped target and execute pending start");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read reconciled start journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(
        executor.commands,
        vec![format!("status:{target}"), start_command]
    );
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
}

fn prove_pending_download_resumes_private_staging(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
    download_command: String,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume pending download into private staging");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read resumed download journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_eq!(executor.commands, vec![download_command]);
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
}

fn prepare_operation(
    operation_label: &str,
    prior_steps: usize,
    expected_kind: &BackupOperationKind,
) -> std::path::PathBuf {
    let root = temp_dir(&format!("canic-backup-pending-claim-{operation_label}"));
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(prior_steps)),
        &mut executor,
    )
    .expect("prepare pending-claim operation");
    let prepared = layout
        .read_execution_journal()
        .expect("read prepared execution journal");

    assert_eq!(response.executed_operation_count, prior_steps);
    assert_eq!(
        prepared
            .next_ready_operation()
            .map(|operation| &operation.kind),
        Some(expected_kind)
    );
    root
}

fn prove_ready_operation_resumes(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
    expected_command: Option<String>,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume operation whose claim did not persist");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read resumed execution journal");

    assert_eq!(response.executed_operation_count, 1);
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
    let mut expected_commands = Vec::new();
    if expected_operation.kind == BackupOperationKind::CreateSnapshot {
        expected_commands.push(format!(
            "snapshot-list:{}",
            expected_operation
                .target_canister_id
                .as_deref()
                .expect("snapshot operation target")
        ));
    }
    expected_commands.extend(expected_command);
    assert_eq!(executor.commands, expected_commands);
}

fn prove_unknown_external_operation_halts(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    observed: &BackupExecutionJournal,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let error = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect_err("pending external mutation must halt with unknown outcome");
    let persisted = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read halted execution journal");

    std::assert_matches!(
        error,
        BackupRunnerError::CommandOutcomeUnknown {
            sequence,
            operation_id,
            ..
        } if sequence == expected_operation.sequence
            && operation_id == expected_operation.operation_id
    );
    assert!(executor.commands.is_empty());
    assert_eq!(&persisted, observed);
}

fn prove_replay_safe_operation_resumes(
    root: &Path,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume replay-safe pending operation");
    let journal = BackupLayout::new(root.to_path_buf())
        .read_execution_journal()
        .expect("read replay-safe execution journal");

    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());
    assert_operation_completed_once(&journal, expected_operation, receipt_count);
}

fn assert_operation_completed_once(
    journal: &BackupExecutionJournal,
    expected_operation: &BackupExecutionJournalOperation,
    receipt_count: usize,
) {
    assert_eq!(
        operation_at(journal, expected_operation.sequence).state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(journal.operation_receipts.len(), receipt_count + 1);
    assert_eq!(
        journal
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == expected_operation.sequence)
            .count(),
        1
    );
}

fn operation_at(
    journal: &BackupExecutionJournal,
    sequence: usize,
) -> &BackupExecutionJournalOperation {
    journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation by sequence")
}

fn expected_command(
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Option<String> {
    let target = operation.target_canister_id.as_deref()?;
    match operation.kind {
        BackupOperationKind::Stop => Some(format!("stop:{target}")),
        BackupOperationKind::CreateSnapshot => Some(format!("snapshot:{target}")),
        BackupOperationKind::Start => Some(format!("start:{target}")),
        BackupOperationKind::DownloadSnapshot => {
            let snapshot_id = journal
                .operation_receipts
                .iter()
                .rev()
                .find_map(|receipt| receipt.snapshot_id.as_deref())
                .expect("snapshot receipt before download");
            Some(format!("download:{target}:{snapshot_id}"))
        }
        BackupOperationKind::VerifyArtifact | BackupOperationKind::FinalizeManifest => None,
        BackupOperationKind::ValidateTopology
        | BackupOperationKind::ValidateControlAuthority
        | BackupOperationKind::ValidateSnapshotReadAuthority
        | BackupOperationKind::ValidateQuiescencePolicy => {
            panic!("preflight operation is outside B04")
        }
    }
}

fn runner_config(out: std::path::PathBuf, max_steps: Option<usize>) -> BackupRunnerConfig {
    BackupRunnerConfig {
        out,
        max_steps,
        updated_at: Some("unix:10".to_string()),
        tool_name: "canic".to_string(),
        tool_version: "test".to_string(),
    }
}
