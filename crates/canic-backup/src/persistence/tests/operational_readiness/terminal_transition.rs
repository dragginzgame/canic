//! Module: persistence::tests::operational_readiness::terminal_transition
//!
//! Responsibility: prove terminal operation state and receipt publication is atomic.
//! Does not own: operation-specific effect reconciliation or final response recovery.
//! Boundary: composes each proven effect owner with the shared execution journal write.

use super::{download_effect::runner_config, kill_child_at_acknowledged_barrier};
use crate::{
    execution::{
        BackupExecutionJournal, BackupExecutionOperationReceiptOutcome,
        BackupExecutionOperationState,
    },
    operational_readiness::manifest::{
        assert_case_defined, backup_operation_label, backup_post_preflight_operations,
    },
    persistence::{BackupLayout, DurableWriteBarrier},
    plan::BackupOperationKind,
    runner::{
        BackupRunnerCanisterStatus, backup_run_execute_with_executor,
        backup_run_execute_with_terminal_barriers,
    },
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const TERMINAL_TRANSITION_CHILD_ROOT_ENV: &str = "CANIC_TEST_TERMINAL_TRANSITION_ROOT";
const TERMINAL_TRANSITION_CHILD_OPERATION_ENV: &str = "CANIC_TEST_TERMINAL_TRANSITION_OPERATION";
const TERMINAL_TRANSITION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_TERMINAL_TRANSITION_BARRIER";
const TERMINAL_TRANSITION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_TERMINAL_TRANSITION_HANDSHAKE";

#[test]
fn terminal_operation_transition_survives_process_death_for_every_variant() {
    let Some(root) = std::env::var_os(TERMINAL_TRANSITION_CHILD_ROOT_ENV) else {
        for operation in backup_post_preflight_operations() {
            for barrier_name in ["before-rename", "after-directory-sync"] {
                prove_terminal_transition_side(&operation, barrier_name);
            }
        }
        return;
    };

    let root = PathBuf::from(root);
    let operation_label = std::env::var(TERMINAL_TRANSITION_CHILD_OPERATION_ENV)
        .expect("terminal transition operation");
    let operation = backup_post_preflight_operations()
        .into_iter()
        .find(|operation| backup_operation_label(operation) == operation_label)
        .expect("supported terminal transition operation");
    let barrier_name =
        std::env::var(TERMINAL_TRANSITION_CHILD_BARRIER_ENV).expect("terminal transition barrier");
    let target = match barrier_name.as_str() {
        "before-rename" => DurableWriteBarrier::BeforeRename,
        "after-directory-sync" => DurableWriteBarrier::AfterDirectorySync,
        _ => panic!("unsupported terminal transition barrier: {barrier_name}"),
    };
    let handshake_root = PathBuf::from(
        std::env::var_os(TERMINAL_TRANSITION_CHILD_HANDSHAKE_ENV)
            .expect("terminal transition handshake root"),
    );
    let mut executor = FakeBackupRunnerExecutor::default();

    backup_run_execute_with_terminal_barriers(
        &runner_config(root, Some(1)),
        &mut executor,
        |barrier| {
            if barrier == target {
                super::hold_at_acknowledged_barrier(&handshake_root);
            }
        },
    )
    .unwrap_or_else(|error| panic!("execute {operation:?} in crash child: {error}"));
    panic!("terminal-transition child passed its armed barrier");
}

fn prove_terminal_transition_side(operation: &BackupOperationKind, barrier_name: &str) {
    let operation_label = backup_operation_label(operation);
    let side = if barrier_name == "before-rename" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!("CANIC-094-B16/{operation_label}/{side}"));
    let (root, layout, target_sequence) = prepare_target_operation(operation);
    let handshake_root = temp_dir(&format!(
        "canic-backup-terminal-transition-{operation_label}-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create terminal transition handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::terminal_transition::terminal_operation_transition_survives_process_death_for_every_variant",
            "--nocapture",
        ])
        .env(TERMINAL_TRANSITION_CHILD_ROOT_ENV, &root)
        .env(TERMINAL_TRANSITION_CHILD_OPERATION_ENV, operation_label)
        .env(TERMINAL_TRANSITION_CHILD_BARRIER_ENV, barrier_name)
        .env(TERMINAL_TRANSITION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn terminal-transition child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted execution journal");
    if barrier_name == "before-rename" {
        assert_terminal_pair(&interrupted, target_sequence, false);
    } else {
        assert_terminal_pair(&interrupted, target_sequence, true);
    }

    assert_terminal_transition_recovery(operation, barrier_name, &root, &layout, target_sequence);

    fs::remove_dir_all(root).expect("remove terminal transition layout");
    fs::remove_dir_all(handshake_root).expect("remove terminal transition handshake root");
}

fn prepare_target_operation(operation: &BackupOperationKind) -> (PathBuf, BackupLayout, usize) {
    let operations = backup_post_preflight_operations();
    let preceding_steps = operations
        .iter()
        .position(|candidate| candidate == operation)
        .expect("post-preflight operation index");
    let root = temp_dir(&format!(
        "canic-backup-terminal-transition-{}",
        backup_operation_label(operation)
    ));
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(preceding_steps)),
        &mut executor,
    )
    .expect("complete operations before terminal transition target");
    assert_eq!(response.executed_operation_count, preceding_steps);
    let prepared = layout
        .read_execution_journal()
        .expect("read target-ready execution journal");
    let target = prepared
        .next_ready_operation()
        .expect("ready terminal transition target");
    assert_eq!(&target.kind, operation);
    (root, layout, target.sequence)
}

fn assert_terminal_transition_recovery(
    operation: &BackupOperationKind,
    barrier_name: &str,
    root: &Path,
    layout: &BackupLayout,
    target_sequence: usize,
) {
    let after_durable_write = barrier_name == "after-directory-sync";
    let mut executor = FakeBackupRunnerExecutor::default();
    if operation == &BackupOperationKind::Stop {
        let target = layout
            .read_execution_journal()
            .expect("read stop target")
            .operations[target_sequence]
            .target_canister_id
            .clone()
            .expect("stop target canister");
        executor
            .canister_statuses
            .insert(target, BackupRunnerCanisterStatus::Stopped);
    }
    let max_steps = usize::from(!after_durable_write);
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(max_steps)),
        &mut executor,
    )
    .expect("recover terminal operation transition");
    let recovered = layout
        .read_execution_journal()
        .expect("read recovered execution journal");

    assert_eq!(response.executed_operation_count, max_steps);
    assert_terminal_pair(&recovered, target_sequence, true);
    assert!(
        executor
            .commands
            .iter()
            .all(|command| !is_mutating_command(command)),
        "restart repeated a mutating command for {operation:?}: {:?}",
        executor.commands
    );
}

fn assert_terminal_pair(
    journal: &BackupExecutionJournal,
    sequence: usize,
    expected_terminal: bool,
) {
    let operation = &journal.operations[sequence];
    let receipts = journal
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == sequence)
        .collect::<Vec<_>>();
    if expected_terminal {
        assert_eq!(operation.state, BackupExecutionOperationState::Completed);
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].outcome,
            BackupExecutionOperationReceiptOutcome::Completed
        );
        assert_eq!(
            operation.state_updated_at.as_deref(),
            receipts[0].updated_at.as_deref()
        );
    } else {
        assert_eq!(operation.state, BackupExecutionOperationState::Pending);
        assert!(receipts.is_empty());
    }
}

fn is_mutating_command(command: &str) -> bool {
    ["stop:", "snapshot:", "start:", "download:"]
        .iter()
        .any(|prefix| command.starts_with(prefix))
}
