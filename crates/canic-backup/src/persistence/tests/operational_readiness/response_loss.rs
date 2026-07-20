//! Module: persistence::tests::operational_readiness::response_loss
//!
//! Responsibility: prove completed backup replay after the final response is lost.
//! Does not own: terminal journal publication or in-flight command recovery.
//! Boundary: kills the completed runner before its response leaves the test child.

use super::{
    download_effect::runner_config, kill_child_at_acknowledged_barrier, snapshot_layout_tree,
};
use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
    operational_readiness::manifest::{assert_case_defined, backup_post_preflight_operations},
    persistence::BackupLayout,
    runner::backup_run_execute_with_executor,
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{fs, path::PathBuf, process::Command};

const RESPONSE_LOSS_CHILD_ROOT_ENV: &str = "CANIC_TEST_RESPONSE_LOSS_ROOT";
const RESPONSE_LOSS_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESPONSE_LOSS_HANDSHAKE";

#[test]
fn completed_backup_replays_after_final_response_is_lost() {
    let Some(root) = std::env::var_os(RESPONSE_LOSS_CHILD_ROOT_ENV) else {
        prove_completed_backup_replay_after_response_loss();
        return;
    };

    let root = PathBuf::from(root);
    let handshake_root = PathBuf::from(
        std::env::var_os(RESPONSE_LOSS_CHILD_HANDSHAKE_ENV).expect("response-loss handshake root"),
    );
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(&runner_config(root, None), &mut executor)
        .expect("complete backup before losing response");

    assert!(response.complete);
    assert!(!response.max_steps_reached);
    assert_eq!(response.executed_operation_count, 6);
    assert_eq!(response.executed_operations.len(), 6);
    super::hold_at_acknowledged_barrier(&handshake_root);
}

fn prove_completed_backup_replay_after_response_loss() {
    assert_case_defined("CANIC-094-B17/final-successful-response/response-lost-after-persistence");
    let root = temp_dir("canic-backup-final-response-loss");
    let handshake_root = temp_dir("canic-backup-final-response-loss-handshake");
    let layout = BackupLayout::new(root.clone());
    let plan = super::valid_backup_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    fs::create_dir_all(&handshake_root).expect("create response-loss handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::response_loss::completed_backup_replays_after_final_response_is_lost",
            "--nocapture",
        ])
        .env(RESPONSE_LOSS_CHILD_ROOT_ENV, &root)
        .env(RESPONSE_LOSS_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn final-response-loss child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    layout
        .verify_integrity()
        .expect("verify completed backup after response loss");
    let before_replay = layout
        .read_execution_journal()
        .expect("read completed journal after response loss");
    assert!(
        before_replay
            .operations
            .iter()
            .all(|operation| operation.state == BackupExecutionOperationState::Completed)
    );
    assert_eq!(
        before_replay.operation_receipts.len(),
        backup_post_preflight_operations().len()
    );
    let expected_execution = before_replay.resume_summary();
    let before_tree = snapshot_layout_tree(&root);

    let mut executor = FakeBackupRunnerExecutor::default();
    let replay =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
            .expect("replay completed backup after response loss");
    let after_replay = layout
        .read_execution_journal()
        .expect("read journal after completed replay");
    layout
        .verify_integrity()
        .expect("verify replayed completed backup");

    assert_eq!(replay.run_id, plan.run_id);
    assert_eq!(replay.plan_id, plan.plan_id);
    assert_eq!(replay.backup_id, plan.run_id);
    assert!(replay.complete);
    assert!(!replay.max_steps_reached);
    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay.executed_operations.is_empty());
    assert_eq!(replay.execution, expected_execution);
    assert!(executor.commands.is_empty());
    assert_eq!(after_replay, before_replay);
    assert_eq!(snapshot_layout_tree(&root), before_tree);

    fs::remove_dir_all(root).expect("remove response-loss layout");
    fs::remove_dir_all(handshake_root).expect("remove response-loss handshake root");
}
