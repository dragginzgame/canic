//! Module: restore::runner::tests
//!
//! Responsibility: prove restore execution consumes private checksum-bound artifact bytes.
//! Does not own: artifact traversal implementation or ICP command behavior.
//! Boundary: exercises journal claim, staging, command execution, receipt, and cleanup together.

use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    operational_readiness::manifest::{
        assert_case_defined, restore_operation_label, restore_operations,
    },
    persistence::{CommandLifetimeLock, DurableWriteBarrier, write_json_durable_at_barriers},
    restore::{
        RestoreApplyCommandConfig, RestoreApplyJournalOperation, RestoreApplyOperationKind,
        RestoreApplyOperationKindCounts, RestoreApplyOperationState, write_restore_apply_journal,
    },
    test_support::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier, temp_dir},
};

use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::process::Command;

use super::*;

const SOURCE_BYTES: &[u8] = b"authoritative snapshot bytes";

#[cfg(unix)]
const STAGING_CHILD_ROOT_ENV: &str = "CANIC_TEST_RESTORE_STAGING_ROOT";
#[cfg(unix)]
const STAGING_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESTORE_STAGING_HANDSHAKE";
#[cfg(unix)]
const CLAIM_CHILD_ROOT_ENV: &str = "CANIC_TEST_RESTORE_CLAIM_ROOT";
#[cfg(unix)]
const CLAIM_CHILD_OPERATION_ENV: &str = "CANIC_TEST_RESTORE_CLAIM_OPERATION";
#[cfg(unix)]
const CLAIM_CHILD_BARRIER_ENV: &str = "CANIC_TEST_RESTORE_CLAIM_BARRIER";
#[cfg(unix)]
const CLAIM_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESTORE_CLAIM_HANDSHAKE";
#[cfg(unix)]
const TERMINAL_CHILD_ROOT_ENV: &str = "CANIC_TEST_RESTORE_TERMINAL_ROOT";
#[cfg(unix)]
const TERMINAL_CHILD_OPERATION_ENV: &str = "CANIC_TEST_RESTORE_TERMINAL_OPERATION";
#[cfg(unix)]
const TERMINAL_CHILD_BARRIER_ENV: &str = "CANIC_TEST_RESTORE_TERMINAL_BARRIER";
#[cfg(unix)]
const TERMINAL_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESTORE_TERMINAL_HANDSHAKE";
#[cfg(unix)]
const RESPONSE_CHILD_ROOT_ENV: &str = "CANIC_TEST_RESTORE_RESPONSE_ROOT";
#[cfg(unix)]
const RESPONSE_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_RESTORE_RESPONSE_HANDSHAKE";

#[cfg(unix)]
#[test]
fn interrupted_private_upload_staging_is_replaced_before_claim() {
    let Some(root) = std::env::var_os(STAGING_CHILD_ROOT_ENV) else {
        assert_case_defined("CANIC-094-R03/private-upload-staging/interrupted");
        prove_interrupted_private_upload_staging();
        return;
    };

    let root = PathBuf::from(root);
    let handshake_root = PathBuf::from(
        std::env::var_os(STAGING_CHILD_HANDSHAKE_ENV).expect("restore staging handshake root"),
    );
    let config = runner_test_config(&root);
    let journal: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&config.journal).expect("read staging journal"))
            .expect("decode staging journal");
    let operation = journal.operations.first().expect("upload operation");

    super::artifact::stage_upload_artifact_at_test_barrier(&config, &journal, operation, || {
        hold_at_acknowledged_barrier(&handshake_root)
    })
    .expect("stage upload artifact in crash child");
    panic!("restore staging child passed its armed barrier");
}

#[cfg(unix)]
fn prove_interrupted_private_upload_staging() {
    let fixture = upload_fixture("canic-restore-interrupted-private-stage");
    let handshake_root = temp_dir("canic-restore-interrupted-private-stage-handshake");
    fs::create_dir_all(&handshake_root).expect("create restore staging handshake root");
    let journal_before = fs::read(&fixture.config.journal).expect("read pristine journal");
    let stage_root =
        super::artifact::restore_upload_stage_root(&fixture.config.journal).expect("stage root");
    let operation_root = stage_root.join("operation-0");
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "restore::runner::tests::interrupted_private_upload_staging_is_replaced_before_claim",
            "--nocapture",
        ])
        .env(STAGING_CHILD_ROOT_ENV, &fixture.root)
        .env(STAGING_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn restore staging child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);

    assert!(stage_root.is_dir());
    assert!(operation_root.is_dir());
    assert!(!operation_root.join("artifact").exists());
    assert_eq!(
        fs::read(&fixture.config.journal).expect("read journal after staging crash"),
        journal_before
    );
    let interrupted: RestoreApplyJournal =
        serde_json::from_slice(&journal_before).expect("decode journal after staging crash");
    assert_eq!(
        interrupted.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    assert!(interrupted.operation_receipts.is_empty());

    let mut executor = InspectingExecutor {
        original_source: fixture.root.join("artifacts/root"),
        observed_input: None,
        calls: 0,
        snapshot_ids: Vec::new(),
    };
    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("replace stale staging and execute exact upload");
    let recovered: RestoreApplyJournal = serde_json::from_slice(
        &fs::read(&fixture.config.journal).expect("read recovered staging journal"),
    )
    .expect("decode recovered staging journal");

    assert!(response.complete);
    assert_eq!(executor.calls, 1);
    assert!(!stage_root.exists());
    assert_eq!(
        recovered.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(recovered.operation_receipts.len(), 1);
    assert_eq!(
        recovered.operation_receipts[0].artifact_checksum,
        recovered.operations[0].artifact_checksum
    );

    fs::remove_dir_all(fixture.root).expect("remove restore staging fixture");
    fs::remove_dir_all(handshake_root).expect("remove restore staging handshake root");
}

#[cfg(unix)]
#[test]
fn unsafe_private_upload_staging_is_rejected_without_following_it() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    for unsafe_entry in ["stage-root", "operation-root"] {
        let fixture = upload_fixture(&format!("canic-restore-unsafe-{unsafe_entry}"));
        let journal_before = fs::read(&fixture.config.journal).expect("read pristine journal");
        let stage_root = super::artifact::restore_upload_stage_root(&fixture.config.journal)
            .expect("stage root");
        let operation_root = stage_root.join("operation-0");
        let outside = fixture.root.join("outside");
        fs::create_dir_all(&outside).expect("create outside directory");
        fs::write(outside.join("marker"), b"must survive").expect("write outside marker");

        let conflict_path = if unsafe_entry == "stage-root" {
            symlink(&outside, &stage_root).expect("create stage-root symlink");
            stage_root.clone()
        } else {
            fs::create_dir_all(&stage_root).expect("create private stage root");
            fs::set_permissions(&stage_root, fs::Permissions::from_mode(0o700))
                .expect("set private stage-root permissions");
            symlink(&outside, &operation_root).expect("create operation-root symlink");
            operation_root.clone()
        };
        let mut executor = InspectingExecutor {
            original_source: fixture.root.join("artifacts/root"),
            observed_input: None,
            calls: 0,
            snapshot_ids: Vec::new(),
        };

        let error = restore_run_execute_with_executor(&fixture.config, &mut executor)
            .expect_err("unsafe staging entry must reject before execution");

        std::assert_matches!(
            error,
            RestoreRunnerError::ArtifactStagePathConflict { path }
                if path == conflict_path
        );
        assert_eq!(executor.calls, 0);
        assert_eq!(
            fs::read(&fixture.config.journal).expect("read unchanged journal"),
            journal_before
        );
        assert_eq!(
            fs::read(outside.join("marker")).expect("read outside marker"),
            b"must survive"
        );

        if unsafe_entry == "stage-root" {
            fs::remove_file(&stage_root).expect("remove stage-root symlink");
        } else {
            fs::remove_file(&operation_root).expect("remove operation-root symlink");
        }
        fs::remove_dir_all(fixture.root).expect("remove unsafe staging fixture");
    }
}

#[cfg(unix)]
#[test]
fn pending_claim_publication_selects_each_restore_recovery_policy() {
    let Some(root) = std::env::var_os(CLAIM_CHILD_ROOT_ENV) else {
        for operation in restore_operations() {
            let operation_label = restore_operation_label(&operation);
            for barrier in ["before-rename", "after-directory-sync"] {
                let side = if barrier == "before-rename" {
                    "before-durable-write"
                } else {
                    "after-durable-write"
                };
                assert_case_defined(&format!("CANIC-094-R04/{operation_label}/{side}"));
                prove_restore_pending_claim(operation.clone(), operation_label, barrier);
            }
        }
        return;
    };

    let root = PathBuf::from(root);
    let operation_label =
        std::env::var(CLAIM_CHILD_OPERATION_ENV).expect("restore claim operation");
    let barrier = std::env::var(CLAIM_CHILD_BARRIER_ENV).expect("restore claim barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(CLAIM_CHILD_HANDSHAKE_ENV).expect("restore claim handshake root"),
    );
    let config = runner_test_config(&root);
    let mut journal: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&config.journal).expect("read restore claim journal"))
            .expect("decode restore claim journal");
    let operation = journal
        .next_transition_operation()
        .cloned()
        .expect("next restore claim operation");
    assert_eq!(
        restore_operation_label(&operation.operation),
        operation_label
    );
    if operation.operation == RestoreApplyOperationKind::UploadSnapshot {
        journal
            .mark_upload_snapshot_pending_at(
                operation.sequence,
                Some("unix:20".to_string()),
                Vec::new(),
            )
            .expect("mark upload pending in crash child");
    } else {
        journal
            .mark_operation_pending_at(operation.sequence, Some("unix:20".to_string()))
            .expect("mark restore operation pending in crash child");
    }
    let target = match barrier.as_str() {
        "before-rename" => DurableWriteBarrier::BeforeRename,
        "after-directory-sync" => DurableWriteBarrier::AfterDirectorySync,
        _ => panic!("unsupported restore claim barrier: {barrier}"),
    };
    write_json_durable_at_barriers(&config.journal, &journal, |observed| {
        if observed == target {
            hold_at_acknowledged_barrier(&handshake_root);
        }
    })
    .expect("write restore pending claim in crash child");
    panic!("restore claim child passed its armed barrier");
}

#[cfg(unix)]
fn prove_restore_pending_claim(
    operation_kind: RestoreApplyOperationKind,
    operation_label: &str,
    barrier: &str,
) {
    let fixture = ready_restore_operation_fixture(
        &format!("canic-restore-claim-{operation_label}-{barrier}"),
        operation_kind.clone(),
    );
    let handshake_root = temp_dir(&format!(
        "canic-restore-claim-handshake-{operation_label}-{barrier}"
    ));
    fs::create_dir_all(&handshake_root).expect("create restore claim handshake root");
    let before_bytes = fs::read(&fixture.config.journal).expect("read pre-claim journal");
    let before: RestoreApplyJournal =
        serde_json::from_slice(&before_bytes).expect("decode pre-claim journal");
    let target_sequence = before
        .next_transition_operation()
        .expect("target restore operation")
        .sequence;
    let receipt_count = before.operation_receipts.len();
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "restore::runner::tests::pending_claim_publication_selects_each_restore_recovery_policy",
            "--nocapture",
        ])
        .env(CLAIM_CHILD_ROOT_ENV, &fixture.root)
        .env(CLAIM_CHILD_OPERATION_ENV, operation_label)
        .env(CLAIM_CHILD_BARRIER_ENV, barrier)
        .env(CLAIM_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn restore claim child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted_bytes =
        fs::read(&fixture.config.journal).expect("read interrupted restore claim journal");
    let interrupted: RestoreApplyJournal =
        serde_json::from_slice(&interrupted_bytes).expect("decode interrupted claim journal");
    let interrupted_operation = interrupted
        .operations
        .iter()
        .find(|operation| operation.sequence == target_sequence)
        .expect("interrupted target operation");

    if barrier == "before-rename" {
        assert_eq!(interrupted_bytes, before_bytes);
        assert_eq!(
            interrupted_operation.state,
            RestoreApplyOperationState::Ready
        );
    } else {
        assert_eq!(
            interrupted_operation.state,
            RestoreApplyOperationState::Pending
        );
        assert_eq!(interrupted.operation_receipts.len(), receipt_count);
        if operation_kind == RestoreApplyOperationKind::UploadSnapshot {
            assert_eq!(interrupted_operation.snapshot_ids_before, Some(Vec::new()));
        }
    }

    let recovering = barrier == "after-directory-sync";
    let outputs = restore_claim_outputs(&operation_kind, recovering);
    let mut executor = ScriptedExecutor::new(outputs);
    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("resume restore operation from exact claim side");
    let recovered: RestoreApplyJournal = serde_json::from_slice(
        &fs::read(&fixture.config.journal).expect("read recovered restore claim journal"),
    )
    .expect("decode recovered restore claim journal");
    let recovered_operation = recovered
        .operations
        .iter()
        .find(|operation| operation.sequence == target_sequence)
        .expect("recovered target operation");

    assert!(response.complete);
    assert_eq!(
        restore_effect_command_count(&executor.commands, &operation_kind),
        1
    );
    assert_eq!(
        recovered_operation.state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(recovered.operation_receipts.len(), receipt_count + 1);
    assert_eq!(
        recovered
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == target_sequence)
            .count(),
        1
    );

    fs::remove_dir_all(fixture.root).expect("remove restore claim fixture");
    fs::remove_dir_all(handshake_root).expect("remove restore claim handshake root");
}

#[cfg(unix)]
#[test]
fn terminal_state_and_receipt_publish_atomically_for_each_restore_operation() {
    let Some(root) = std::env::var_os(TERMINAL_CHILD_ROOT_ENV) else {
        for operation in restore_operations() {
            let operation_label = restore_operation_label(&operation);
            for barrier in ["before-rename", "after-directory-sync"] {
                let side = if barrier == "before-rename" {
                    "before-durable-write"
                } else {
                    "after-durable-write"
                };
                assert_case_defined(&format!("CANIC-094-R12/{operation_label}/{side}"));
                prove_restore_terminal_publication(operation.clone(), operation_label, barrier);
            }
        }
        return;
    };

    let root = PathBuf::from(root);
    let operation_label =
        std::env::var(TERMINAL_CHILD_OPERATION_ENV).expect("restore terminal operation");
    let operation = restore_operations()
        .into_iter()
        .find(|operation| restore_operation_label(operation) == operation_label)
        .expect("supported restore terminal operation");
    let barrier = std::env::var(TERMINAL_CHILD_BARRIER_ENV).expect("restore terminal barrier");
    let target = match barrier.as_str() {
        "before-rename" => DurableWriteBarrier::BeforeRename,
        "after-directory-sync" => DurableWriteBarrier::AfterDirectorySync,
        _ => panic!("unsupported restore terminal barrier: {barrier}"),
    };
    let handshake_root = PathBuf::from(
        std::env::var_os(TERMINAL_CHILD_HANDSHAKE_ENV).expect("restore terminal handshake root"),
    );
    let config = runner_test_config(&root);
    let mut executor = ScriptedExecutor::new(restore_claim_outputs(&operation, false));

    super::execute::restore_run_execute_with_terminal_barriers(
        &config,
        &mut executor,
        |observed| {
            if observed == target {
                hold_at_acknowledged_barrier(&handshake_root);
            }
        },
    )
    .expect("execute restore operation in terminal crash child");
    panic!("restore terminal child passed its armed barrier");
}

#[cfg(unix)]
fn prove_restore_terminal_publication(
    operation_kind: RestoreApplyOperationKind,
    operation_label: &str,
    barrier: &str,
) {
    let fixture = ready_restore_operation_fixture(
        &format!("canic-restore-terminal-{operation_label}-{barrier}"),
        operation_kind.clone(),
    );
    let handshake_root = temp_dir(&format!(
        "canic-restore-terminal-handshake-{operation_label}-{barrier}"
    ));
    fs::create_dir_all(&handshake_root).expect("create restore terminal handshake root");
    let before: RestoreApplyJournal = serde_json::from_slice(
        &fs::read(&fixture.config.journal).expect("read pre-terminal journal"),
    )
    .expect("decode pre-terminal journal");
    let target_sequence = before
        .next_transition_operation()
        .expect("target terminal operation")
        .sequence;
    let receipt_count = before.operation_receipts.len();
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "restore::runner::tests::terminal_state_and_receipt_publish_atomically_for_each_restore_operation",
            "--nocapture",
        ])
        .env(TERMINAL_CHILD_ROOT_ENV, &fixture.root)
        .env(TERMINAL_CHILD_OPERATION_ENV, operation_label)
        .env(TERMINAL_CHILD_BARRIER_ENV, barrier)
        .env(TERMINAL_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn restore terminal child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted: RestoreApplyJournal = serde_json::from_slice(
        &fs::read(&fixture.config.journal).expect("read interrupted terminal journal"),
    )
    .expect("decode interrupted terminal journal");
    assert_restore_terminal_pair(
        &interrupted,
        target_sequence,
        barrier == "after-directory-sync",
    );
    assert_eq!(
        interrupted.operation_receipts.len(),
        receipt_count + usize::from(barrier == "after-directory-sync")
    );

    let outputs = if barrier == "before-rename" {
        restore_terminal_recovery_outputs(&operation_kind)
    } else {
        Vec::new()
    };
    let mut executor = ScriptedExecutor::new(outputs);
    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("recover restore terminal publication");
    let recovered: RestoreApplyJournal = serde_json::from_slice(
        &fs::read(&fixture.config.journal).expect("read recovered terminal journal"),
    )
    .expect("decode recovered terminal journal");

    assert!(response.complete);
    assert_restore_terminal_pair(&recovered, target_sequence, true);
    assert_eq!(recovered.operation_receipts.len(), receipt_count + 1);
    if barrier == "after-directory-sync" {
        assert!(executor.commands.is_empty());
    } else {
        assert_eq!(
            restore_mutating_command_count(&executor.commands),
            usize::from(operation_kind == RestoreApplyOperationKind::LoadSnapshot)
        );
    }
    if operation_kind == RestoreApplyOperationKind::UploadSnapshot {
        let stage_root = super::artifact::restore_upload_stage_root(&fixture.config.journal)
            .expect("restore upload stage root");
        assert!(!stage_root.exists());
    }

    fs::remove_dir_all(fixture.root).expect("remove restore terminal fixture");
    fs::remove_dir_all(handshake_root).expect("remove restore terminal handshake root");
}

fn assert_restore_terminal_pair(
    journal: &RestoreApplyJournal,
    sequence: usize,
    expected_terminal: bool,
) {
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("terminal operation");
    let receipts = journal
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == sequence)
        .collect::<Vec<_>>();
    if expected_terminal {
        assert_eq!(operation.state, RestoreApplyOperationState::Completed);
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            operation.state_updated_at.as_deref(),
            receipts[0].updated_at.as_deref()
        );
    } else {
        assert_eq!(operation.state, RestoreApplyOperationState::Pending);
        assert!(receipts.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn completed_restore_replays_after_final_response_loss() {
    let Some(root) = std::env::var_os(RESPONSE_CHILD_ROOT_ENV) else {
        assert_case_defined(
            "CANIC-094-R13/final-successful-response/response-lost-after-persistence",
        );
        prove_completed_restore_response_loss();
        return;
    };

    let root = PathBuf::from(root);
    let handshake_root = PathBuf::from(
        std::env::var_os(RESPONSE_CHILD_HANDSHAKE_ENV).expect("restore response handshake root"),
    );
    let config = runner_test_config(&root);
    let mut executor = ScriptedExecutor::new(restore_claim_outputs(
        &RestoreApplyOperationKind::VerifyDeployment,
        false,
    ));
    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("complete restore before response loss");
    assert!(response.complete);
    hold_at_acknowledged_barrier(&handshake_root);
}

#[cfg(unix)]
fn prove_completed_restore_response_loss() {
    let fixture = ready_restore_operation_fixture(
        "canic-restore-final-response-loss",
        RestoreApplyOperationKind::VerifyDeployment,
    );
    let handshake_root = temp_dir("canic-restore-final-response-loss-handshake");
    fs::create_dir_all(&handshake_root).expect("create restore response handshake root");
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "restore::runner::tests::completed_restore_replays_after_final_response_loss",
            "--nocapture",
        ])
        .env(RESPONSE_CHILD_ROOT_ENV, &fixture.root)
        .env(RESPONSE_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn restore response-loss child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let completed_bytes =
        fs::read(&fixture.config.journal).expect("read completed response-loss journal");
    let completed: RestoreApplyJournal =
        serde_json::from_slice(&completed_bytes).expect("decode completed response-loss journal");
    assert_restore_terminal_pair(&completed, 0, true);

    let mut executor = ScriptedExecutor::new([]);
    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("replay completed restore after response loss");

    assert!(response.complete);
    assert_eq!(response.executed_operation_count, Some(0));
    assert!(executor.commands.is_empty());
    assert_eq!(
        fs::read(&fixture.config.journal).expect("read replayed response-loss journal"),
        completed_bytes
    );

    fs::remove_dir_all(fixture.root).expect("remove restore response-loss fixture");
    fs::remove_dir_all(handshake_root).expect("remove restore response-loss handshake root");
}

#[test]
fn execute_reconciles_pending_stop_from_authoritative_status() {
    let (root, config) = pending_lifecycle_fixture(
        "canic-restore-pending-stop",
        RestoreApplyOperationKind::StopCanister,
    );
    let mut executor = ScriptedExecutor::new([status_output("Stopped")]);

    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("reconcile committed stop");

    assert!(response.complete);
    assert_eq!(executor.commands.len(), 1);
    assert_eq!(executor.commands[0].args[1], "status");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn execute_reconciles_pending_start_from_authoritative_status() {
    let (root, config) = pending_lifecycle_fixture(
        "canic-restore-pending-start",
        RestoreApplyOperationKind::StartCanister,
    );
    let mut executor = ScriptedExecutor::new([status_output("Running")]);

    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("reconcile committed start");

    assert!(response.complete);
    assert_eq!(executor.commands.len(), 1);
    assert_eq!(executor.commands[0].args[1], "status");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn execute_replays_exact_pending_load_while_target_is_stopped() {
    let (root, config) = pending_load_fixture();
    let mut executor = ScriptedExecutor::new([
        status_output("Stopped"),
        RestoreRunnerCommandOutput {
            success: true,
            status: "0".to_string(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        },
    ]);

    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("replay exact pending load");

    assert!(response.complete);
    assert_eq!(executor.commands.len(), 2);
    assert_eq!(executor.commands[0].args[1], "status");
    assert_eq!(executor.commands[1].args[1..3], ["snapshot", "restore"]);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn execute_upload_uses_private_verified_copy_and_records_checksum() {
    let fixture = upload_fixture("canic-restore-private-stage");
    let source_path = fixture.root.join("artifacts/root");
    let mut executor = InspectingExecutor {
        original_source: source_path.clone(),
        observed_input: None,
        calls: 0,
        snapshot_ids: Vec::new(),
    };

    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("execute verified upload");

    let staged_input = executor.observed_input.expect("staged input path");
    assert_ne!(staged_input, source_path);
    assert!(!staged_input.exists());
    assert_eq!(executor.calls, 1);
    assert!(response.complete);
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read completed journal"))
            .expect("decode completed journal");
    assert_eq!(
        persisted.operation_receipts[0].artifact_checksum,
        persisted.operations[0].artifact_checksum
    );

    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn execute_upload_rejects_source_replacement_before_claim() {
    let fixture = upload_fixture("canic-restore-source-replacement");
    fs::write(fixture.root.join("artifacts/root"), b"replacement").expect("replace source bytes");
    let mut executor = InspectingExecutor {
        original_source: fixture.root.join("artifacts/root"),
        observed_input: None,
        calls: 0,
        snapshot_ids: Vec::new(),
    };

    let error = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect_err("replacement must reject before command execution");

    std::assert_matches!(
        error,
        RestoreRunnerError::ArtifactStageChecksum {
            source: ArtifactChecksumError::ChecksumMismatch { .. },
            ..
        }
    );
    assert_eq!(executor.calls, 0);
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read unchanged journal"))
            .expect("decode unchanged journal");
    assert_eq!(
        persisted.operations[0].state,
        RestoreApplyOperationState::Ready
    );

    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn execute_upload_rejects_post_validation_symlink_replacement() {
    let fixture = upload_fixture("canic-restore-symlink-replacement");
    let source = fixture.root.join("artifacts/root");
    let outside = fixture.root.join("outside");
    fs::write(&outside, SOURCE_BYTES).expect("write outside bytes");
    fs::remove_file(&source).expect("remove original source");
    std::os::unix::fs::symlink(&outside, &source).expect("replace source with symlink");
    let mut executor = InspectingExecutor {
        original_source: source,
        observed_input: None,
        calls: 0,
        snapshot_ids: Vec::new(),
    };

    let error = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect_err("symlink replacement must reject before execution");

    std::assert_matches!(error, RestoreRunnerError::ArtifactStageChecksum { .. });
    assert_eq!(executor.calls, 0);
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn execute_upload_stages_complete_snapshot_directory() {
    let fixture = upload_directory_fixture("canic-restore-directory-stage");
    let source_path = fixture.root.join("artifacts/root");
    let expected = ArtifactChecksum::from_directory(&source_path).expect("checksum source tree");
    let mut executor = DirectoryInspectingExecutor {
        original_source: source_path,
        expected,
        observed_input: None,
    };

    restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("execute directory upload");

    let staged_input = executor.observed_input.expect("staged directory path");
    assert!(!staged_input.exists());
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn execute_preserves_pending_operation_while_command_is_in_flight() {
    let fixture = upload_fixture("canic-restore-command-in-flight");
    let mut journal: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read ready journal"))
            .expect("decode ready journal");
    journal.operations[0].state = RestoreApplyOperationState::Pending;
    journal.operations[0].snapshot_ids_before = Some(Vec::new());
    journal.ready_operations = 0;
    journal.pending_operations = 1;
    write_restore_apply_journal(&fixture.config.journal, &journal).expect("write pending journal");
    let command_lock =
        CommandLifetimeLock::acquire(&fixture.config.journal, 0).expect("hold prior command lock");
    let mut executor = InspectingExecutor {
        original_source: fixture.root.join("artifacts/root"),
        observed_input: None,
        calls: 0,
        snapshot_ids: vec!["recovered-upload".to_string()],
    };

    let error = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect_err("in-flight command must stop resume");
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read pending journal"))
            .expect("decode pending journal");

    std::assert_matches!(
        error,
        RestoreRunnerError::CommandInFlight {
            sequence: 0,
            operation: RestoreApplyOperationKind::UploadSnapshot,
            ..
        }
    );
    assert_eq!(executor.calls, 0);
    assert_eq!(
        persisted.operations[0].state,
        RestoreApplyOperationState::Pending
    );
    assert!(persisted.operation_receipts.is_empty());

    command_lock.finish().expect("release prior command lock");
    let response = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect("quiescent committed upload must reconcile");
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read pending journal"))
            .expect("decode pending journal");

    assert!(response.complete);
    assert_eq!(executor.calls, 0);
    assert_eq!(
        persisted.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(
        persisted.operation_receipts[0]
            .uploaded_snapshot_id
            .as_deref(),
        Some("recovered-upload")
    );
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn execute_repeats_pending_verification_without_a_mutating_command_lock() {
    let (root, config) = pending_verification_fixture("canic-restore-pending-verification");
    let mut executor = SuccessfulExecutor { calls: 0 };

    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("repeat read-only verification");

    assert!(response.complete);
    assert_eq!(executor.calls, 1);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn execute_verification_mismatch_persists_failed_evidence() {
    let (root, config) = pending_verification_fixture("canic-restore-verification-mismatch");
    let mut executor = ScriptedExecutor::new([RestoreRunnerCommandOutput {
        success: true,
        status: "0".to_string(),
        stdout: br#"{"status":"Running","module_hash":"0xDEAD"}"#.to_vec(),
        stderr: Vec::new(),
    }]);

    let error = restore_run_execute_with_executor(&config, &mut executor)
        .expect_err("mismatched restored module must fail closed");
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&config.journal).expect("read failed journal"))
            .expect("decode failed journal");

    std::assert_matches!(
        error,
        RestoreRunnerError::CommandFailed { ref status, .. }
            if status == "verification-evidence-mismatch"
    );
    assert_eq!(persisted.failed_operations, 1);
    assert_eq!(
        persisted.operations[0].state,
        RestoreApplyOperationState::Failed
    );
    assert_eq!(
        persisted.operation_receipts[0].failure_reason.as_deref(),
        Some("runner-command-exit-verification-evidence-mismatch")
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

fn pending_verification_fixture(prefix: &str) -> (PathBuf, RestoreRunnerConfig) {
    let root = temp_dir(prefix);
    fs::create_dir_all(&root).expect("create temp root");
    let operation = RestoreApplyJournalOperation {
        sequence: 0,
        operation: RestoreApplyOperationKind::VerifyMember,
        state: RestoreApplyOperationState::Pending,
        state_updated_at: Some("2026-07-18T12:00:00Z".to_string()),
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: "aaaaa-aa".to_string(),
        target_canister: "rno2w-sqaaa-aaaaa-aaacq-cai".to_string(),
        role: "root".to_string(),
        snapshot_id: None,
        artifact_path: None,
        artifact_checksum: None,
        snapshot_ids_before: None,
        expected_module_hash: Some("abcd".to_string()),
        verification_kind: Some("status".to_string()),
    };
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "backup-pending-verification".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some(root.to_string_lossy().to_string()),
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::from_operations(std::slice::from_ref(
            &operation,
        )),
        pending_operations: 1,
        ready_operations: 0,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![operation],
        operation_receipts: Vec::new(),
    };
    let config = RestoreRunnerConfig {
        journal: root.join("restore-apply.json"),
        command: RestoreApplyCommandConfig::default(),
        max_steps: None,
        updated_at: Some("2026-07-18T12:01:00Z".to_string()),
    };
    write_restore_apply_journal(&config.journal, &journal).expect("write pending journal");
    (root, config)
}

struct UploadFixture {
    root: PathBuf,
    config: RestoreRunnerConfig,
}

fn upload_fixture(prefix: &str) -> UploadFixture {
    let root = temp_dir(prefix);
    fs::create_dir_all(root.join("artifacts")).expect("create artifact root");
    fs::write(root.join("artifacts/root"), SOURCE_BYTES).expect("write source artifact");
    let checksum = ArtifactChecksum::from_bytes(SOURCE_BYTES);
    finish_upload_fixture(root, checksum)
}

fn upload_directory_fixture(prefix: &str) -> UploadFixture {
    let root = temp_dir(prefix);
    let artifact = root.join("artifacts/root");
    fs::create_dir_all(artifact.join("nested")).expect("create artifact tree");
    fs::write(artifact.join("snapshot.bin"), SOURCE_BYTES).expect("write snapshot bytes");
    fs::write(artifact.join("nested/metadata.json"), b"{}").expect("write snapshot metadata");
    let checksum = ArtifactChecksum::from_directory(&artifact).expect("checksum artifact tree");
    finish_upload_fixture(root, checksum)
}

fn finish_upload_fixture(root: PathBuf, checksum: ArtifactChecksum) -> UploadFixture {
    let operation = RestoreApplyJournalOperation {
        sequence: 0,
        operation: RestoreApplyOperationKind::UploadSnapshot,
        state: RestoreApplyOperationState::Ready,
        state_updated_at: None,
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: "aaaaa-aa".to_string(),
        target_canister: "rno2w-sqaaa-aaaaa-aaacq-cai".to_string(),
        role: "root".to_string(),
        snapshot_id: Some("source-snapshot".to_string()),
        artifact_path: Some("artifacts/root".to_string()),
        artifact_checksum: Some(checksum),
        snapshot_ids_before: None,
        expected_module_hash: None,
        verification_kind: None,
    };
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "backup-private-stage".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some(root.to_string_lossy().to_string()),
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::from_operations(std::slice::from_ref(
            &operation,
        )),
        pending_operations: 0,
        ready_operations: 1,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![operation],
        operation_receipts: Vec::new(),
    };
    let journal_path = root.join("restore-apply.json");
    write_restore_apply_journal(&journal_path, &journal).expect("write apply journal");
    UploadFixture {
        root,
        config: RestoreRunnerConfig {
            journal: journal_path,
            command: RestoreApplyCommandConfig::default(),
            max_steps: None,
            updated_at: Some("2026-07-18T12:00:00Z".to_string()),
        },
    }
}

fn ready_restore_operation_fixture(
    prefix: &str,
    operation_kind: RestoreApplyOperationKind,
) -> UploadFixture {
    let root = temp_dir(prefix);
    fs::create_dir_all(root.join("artifacts")).expect("create restore claim artifacts");
    fs::write(root.join("artifacts/root"), SOURCE_BYTES).expect("write restore claim artifact");
    let checksum = ArtifactChecksum::from_bytes(SOURCE_BYTES);
    let target_sequence = usize::from(operation_kind == RestoreApplyOperationKind::LoadSnapshot);
    let target_operation = restore_operation(
        target_sequence,
        operation_kind,
        RestoreApplyOperationState::Ready,
        None,
        &checksum,
    );
    let (operations, operation_receipts) = if target_sequence == 0 {
        (vec![target_operation], Vec::new())
    } else {
        let upload = restore_operation(
            0,
            RestoreApplyOperationKind::UploadSnapshot,
            RestoreApplyOperationState::Completed,
            Some("unix:1".to_string()),
            &checksum,
        );
        let upload_command = RestoreApplyRunnerCommand {
            program: "icp".to_string(),
            args: vec![
                "canister".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
            ],
            mutates: true,
            requires_stopped_canister: false,
            note: "uploads the exact staged snapshot".to_string(),
        };
        let upload_receipt = RestoreApplyOperationReceipt::command_completed(
            &upload,
            upload_command,
            "0".to_string(),
            Some("unix:1".to_string()),
            RestoreApplyCommandOutputPair::from_bytes(b"", b"", 1024),
            1,
            Some("uploaded-snapshot".to_string()),
        );
        (vec![upload, target_operation], vec![upload_receipt])
    };
    let state_counts = operations
        .iter()
        .fold((0, 0), |(ready, completed), operation| {
            match operation.state {
                RestoreApplyOperationState::Ready => (ready + 1, completed),
                RestoreApplyOperationState::Completed => (ready, completed + 1),
                _ => (ready, completed),
            }
        });
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "backup-restore-claim".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some(root.to_string_lossy().to_string()),
        operation_count: operations.len(),
        operation_counts: RestoreApplyOperationKindCounts::from_operations(&operations),
        pending_operations: 0,
        ready_operations: state_counts.0,
        blocked_operations: 0,
        completed_operations: state_counts.1,
        failed_operations: 0,
        operations,
        operation_receipts,
    };
    let config = runner_test_config(&root);
    write_restore_apply_journal(&config.journal, &journal).expect("write restore claim journal");
    UploadFixture { root, config }
}

fn restore_operation(
    sequence: usize,
    operation: RestoreApplyOperationKind,
    state: RestoreApplyOperationState,
    state_updated_at: Option<String>,
    checksum: &ArtifactChecksum,
) -> RestoreApplyJournalOperation {
    let has_artifact = matches!(
        operation,
        RestoreApplyOperationKind::UploadSnapshot | RestoreApplyOperationKind::LoadSnapshot
    );
    let verifies = matches!(
        operation,
        RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyDeployment
    );
    let snapshot_ids_before = (operation == RestoreApplyOperationKind::UploadSnapshot
        && state == RestoreApplyOperationState::Completed)
        .then(Vec::new);

    RestoreApplyJournalOperation {
        sequence,
        operation,
        state,
        state_updated_at,
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: "aaaaa-aa".to_string(),
        target_canister: "rno2w-sqaaa-aaaaa-aaacq-cai".to_string(),
        role: "root".to_string(),
        snapshot_id: has_artifact.then(|| "source-snapshot".to_string()),
        artifact_path: has_artifact.then(|| "artifacts/root".to_string()),
        artifact_checksum: has_artifact.then(|| checksum.clone()),
        snapshot_ids_before,
        expected_module_hash: None,
        verification_kind: verifies.then(|| "status".to_string()),
    }
}

fn restore_claim_outputs(
    operation: &RestoreApplyOperationKind,
    recovering: bool,
) -> Vec<RestoreRunnerCommandOutput> {
    let command_success = || RestoreRunnerCommandOutput {
        success: true,
        status: "0".to_string(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    match operation {
        RestoreApplyOperationKind::UploadSnapshot => vec![
            snapshot_inventory_output(&[]),
            RestoreRunnerCommandOutput {
                success: true,
                status: "0".to_string(),
                stdout: br#"{"snapshot_id":"uploaded-snapshot"}"#.to_vec(),
                stderr: Vec::new(),
            },
        ],
        RestoreApplyOperationKind::StopCanister if recovering => {
            vec![status_output("Running"), command_success()]
        }
        RestoreApplyOperationKind::StartCanister if recovering => {
            vec![status_output("Stopped"), command_success()]
        }
        RestoreApplyOperationKind::StopCanister | RestoreApplyOperationKind::StartCanister => {
            vec![command_success()]
        }
        RestoreApplyOperationKind::LoadSnapshot => {
            vec![status_output("Stopped"), command_success()]
        }
        RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyDeployment => {
            vec![status_output("Running")]
        }
    }
}

fn restore_terminal_recovery_outputs(
    operation: &RestoreApplyOperationKind,
) -> Vec<RestoreRunnerCommandOutput> {
    let command_success = || RestoreRunnerCommandOutput {
        success: true,
        status: "0".to_string(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    match operation {
        RestoreApplyOperationKind::UploadSnapshot => {
            vec![snapshot_inventory_output(
                &["uploaded-snapshot".to_string()],
            )]
        }
        RestoreApplyOperationKind::StopCanister => vec![status_output("Stopped")],
        RestoreApplyOperationKind::StartCanister => vec![status_output("Running")],
        RestoreApplyOperationKind::LoadSnapshot => {
            vec![status_output("Stopped"), command_success()]
        }
        RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyDeployment => {
            vec![status_output("Running")]
        }
    }
}

fn restore_effect_command_count(
    commands: &[RestoreApplyRunnerCommand],
    operation: &RestoreApplyOperationKind,
) -> usize {
    commands
        .iter()
        .filter(|command| match operation {
            RestoreApplyOperationKind::UploadSnapshot => {
                command.args.get(1).map(String::as_str) == Some("snapshot")
                    && command.args.get(2).map(String::as_str) == Some("upload")
            }
            RestoreApplyOperationKind::StopCanister => {
                command.args.get(1).map(String::as_str) == Some("stop")
            }
            RestoreApplyOperationKind::LoadSnapshot => {
                command.args.get(1).map(String::as_str) == Some("snapshot")
                    && command.args.get(2).map(String::as_str) == Some("restore")
            }
            RestoreApplyOperationKind::StartCanister => {
                command.args.get(1).map(String::as_str) == Some("start")
            }
            RestoreApplyOperationKind::VerifyMember
            | RestoreApplyOperationKind::VerifyDeployment => {
                command.args.get(1).map(String::as_str) == Some("status")
            }
        })
        .count()
}

fn restore_mutating_command_count(commands: &[RestoreApplyRunnerCommand]) -> usize {
    commands
        .iter()
        .filter(|command| {
            matches!(
                command.args.get(1).map(String::as_str),
                Some("stop" | "start")
            ) || (command.args.get(1).map(String::as_str) == Some("snapshot")
                && matches!(
                    command.args.get(2).map(String::as_str),
                    Some("upload" | "restore")
                ))
        })
        .count()
}

fn pending_lifecycle_fixture(
    prefix: &str,
    operation_kind: RestoreApplyOperationKind,
) -> (PathBuf, RestoreRunnerConfig) {
    let root = temp_dir(prefix);
    fs::create_dir_all(&root).expect("create fixture root");
    let operation = RestoreApplyJournalOperation {
        sequence: 0,
        operation: operation_kind,
        state: RestoreApplyOperationState::Pending,
        state_updated_at: Some("unix:1".to_string()),
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: "aaaaa-aa".to_string(),
        target_canister: "rno2w-sqaaa-aaaaa-aaacq-cai".to_string(),
        role: "root".to_string(),
        snapshot_id: None,
        artifact_path: None,
        artifact_checksum: None,
        snapshot_ids_before: None,
        expected_module_hash: None,
        verification_kind: None,
    };
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "backup-pending-lifecycle".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some(root.to_string_lossy().to_string()),
        operation_count: 1,
        operation_counts: RestoreApplyOperationKindCounts::from_operations(std::slice::from_ref(
            &operation,
        )),
        pending_operations: 1,
        ready_operations: 0,
        blocked_operations: 0,
        completed_operations: 0,
        failed_operations: 0,
        operations: vec![operation],
        operation_receipts: Vec::new(),
    };
    let config = runner_test_config(&root);
    write_restore_apply_journal(&config.journal, &journal).expect("write pending journal");
    (root, config)
}

fn pending_load_fixture() -> (PathBuf, RestoreRunnerConfig) {
    let root = temp_dir("canic-restore-pending-load");
    fs::create_dir_all(&root).expect("create fixture root");
    let checksum = ArtifactChecksum::from_bytes(SOURCE_BYTES);
    let upload = RestoreApplyJournalOperation {
        sequence: 0,
        operation: RestoreApplyOperationKind::UploadSnapshot,
        state: RestoreApplyOperationState::Completed,
        state_updated_at: Some("unix:1".to_string()),
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: "aaaaa-aa".to_string(),
        target_canister: "rno2w-sqaaa-aaaaa-aaacq-cai".to_string(),
        role: "root".to_string(),
        snapshot_id: Some("source-snapshot".to_string()),
        artifact_path: Some("artifacts/root".to_string()),
        artifact_checksum: Some(checksum.clone()),
        snapshot_ids_before: Some(Vec::new()),
        expected_module_hash: None,
        verification_kind: None,
    };
    let load = RestoreApplyJournalOperation {
        sequence: 1,
        operation: RestoreApplyOperationKind::LoadSnapshot,
        state: RestoreApplyOperationState::Pending,
        state_updated_at: Some("unix:2".to_string()),
        blocking_reasons: Vec::new(),
        member_order: 0,
        source_canister: upload.source_canister.clone(),
        target_canister: upload.target_canister.clone(),
        role: upload.role.clone(),
        snapshot_id: upload.snapshot_id.clone(),
        artifact_path: upload.artifact_path.clone(),
        artifact_checksum: Some(checksum),
        snapshot_ids_before: None,
        expected_module_hash: None,
        verification_kind: None,
    };
    let upload_command = RestoreApplyRunnerCommand {
        program: "icp".to_string(),
        args: vec![
            "canister".to_string(),
            "snapshot".to_string(),
            "upload".to_string(),
        ],
        mutates: true,
        requires_stopped_canister: false,
        note: "uploads the exact staged snapshot".to_string(),
    };
    let upload_receipt = RestoreApplyOperationReceipt::command_completed(
        &upload,
        upload_command,
        "0".to_string(),
        Some("unix:1".to_string()),
        RestoreApplyCommandOutputPair::from_bytes(b"", b"", 1024),
        1,
        Some("uploaded-snapshot".to_string()),
    );
    let operations = vec![upload, load];
    let journal = RestoreApplyJournal {
        journal_version: 1,
        backup_id: "backup-pending-load".to_string(),
        ready: true,
        blocked_reasons: Vec::new(),
        backup_root: Some(root.to_string_lossy().to_string()),
        operation_count: operations.len(),
        operation_counts: RestoreApplyOperationKindCounts::from_operations(&operations),
        pending_operations: 1,
        ready_operations: 0,
        blocked_operations: 0,
        completed_operations: 1,
        failed_operations: 0,
        operations,
        operation_receipts: vec![upload_receipt],
    };
    let config = runner_test_config(&root);
    write_restore_apply_journal(&config.journal, &journal).expect("write pending load journal");
    (root, config)
}

fn runner_test_config(root: &Path) -> RestoreRunnerConfig {
    RestoreRunnerConfig {
        journal: root.join("restore-apply.json"),
        command: RestoreApplyCommandConfig::default(),
        max_steps: None,
        updated_at: Some("unix:3".to_string()),
    }
}

struct ScriptedExecutor {
    outputs: VecDeque<RestoreRunnerCommandOutput>,
    commands: Vec<RestoreApplyRunnerCommand>,
}

impl ScriptedExecutor {
    fn new(outputs: impl IntoIterator<Item = RestoreRunnerCommandOutput>) -> Self {
        Self {
            outputs: outputs.into_iter().collect(),
            commands: Vec::new(),
        }
    }
}

impl RestoreRunnerCommandExecutor for ScriptedExecutor {
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
        _command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        self.commands.push(command.clone());
        self.outputs
            .pop_front()
            .ok_or_else(|| std::io::Error::other("unexpected restore command"))
    }
}

fn status_output(status: &str) -> RestoreRunnerCommandOutput {
    RestoreRunnerCommandOutput {
        success: true,
        status: "0".to_string(),
        stdout: serde_json::to_vec(&serde_json::json!({ "status": status }))
            .expect("serialize status"),
        stderr: Vec::new(),
    }
}

struct DirectoryInspectingExecutor {
    original_source: PathBuf,
    expected: ArtifactChecksum,
    observed_input: Option<PathBuf>,
}

impl RestoreRunnerCommandExecutor for DirectoryInspectingExecutor {
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
        _command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        if is_snapshot_inventory_command(command) {
            return Ok(snapshot_inventory_output(&[]));
        }
        let input = command
            .args
            .windows(2)
            .find(|args| args[0] == "--input")
            .map(|args| Path::new(&args[1]).to_path_buf())
            .ok_or_else(|| std::io::Error::other("missing staged --input"))?;
        assert!(input.is_dir());
        assert_eq!(
            ArtifactChecksum::from_directory(&input)
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            self.expected
        );
        fs::write(
            self.original_source.join("snapshot.bin"),
            b"changed original tree",
        )?;
        assert_eq!(
            ArtifactChecksum::from_directory(&input)
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            self.expected
        );
        self.observed_input = Some(input);
        Ok(RestoreRunnerCommandOutput {
            success: true,
            status: "0".to_string(),
            stdout: br#"{"snapshot_id":"uploaded-directory"}"#.to_vec(),
            stderr: Vec::new(),
        })
    }
}

struct InspectingExecutor {
    original_source: PathBuf,
    observed_input: Option<PathBuf>,
    calls: usize,
    snapshot_ids: Vec<String>,
}

struct SuccessfulExecutor {
    calls: usize,
}

impl RestoreRunnerCommandExecutor for SuccessfulExecutor {
    fn execute(
        &mut self,
        _command: &RestoreApplyRunnerCommand,
        _command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        self.calls += 1;
        Ok(RestoreRunnerCommandOutput {
            success: true,
            status: "0".to_string(),
            stdout: br#"{"status":"Running","module_hash":"0xABCD"}"#.to_vec(),
            stderr: Vec::new(),
        })
    }
}

fn is_snapshot_inventory_command(command: &RestoreApplyRunnerCommand) -> bool {
    command.args.get(1).map(String::as_str) == Some("snapshot")
        && command.args.get(2).map(String::as_str) == Some("list")
}

fn snapshot_inventory_output(snapshot_ids: &[String]) -> RestoreRunnerCommandOutput {
    let snapshots = snapshot_ids
        .iter()
        .map(|snapshot_id| serde_json::json!({ "snapshot_id": snapshot_id }))
        .collect::<Vec<_>>();
    RestoreRunnerCommandOutput {
        success: true,
        status: "0".to_string(),
        stdout: serde_json::to_vec(&serde_json::json!({ "snapshots": snapshots }))
            .expect("serialize inventory"),
        stderr: Vec::new(),
    }
}

impl RestoreRunnerCommandExecutor for InspectingExecutor {
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
        _command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        if is_snapshot_inventory_command(command) {
            return Ok(snapshot_inventory_output(&self.snapshot_ids));
        }
        self.calls += 1;
        let input = command
            .args
            .windows(2)
            .find(|args| args[0] == "--input")
            .map(|args| Path::new(&args[1]).to_path_buf())
            .ok_or_else(|| std::io::Error::other("missing staged --input"))?;
        assert_eq!(fs::read(&input)?, SOURCE_BYTES);
        fs::write(&self.original_source, b"changed after private staging")?;
        assert_eq!(fs::read(&input)?, SOURCE_BYTES);
        self.observed_input = Some(input);
        Ok(RestoreRunnerCommandOutput {
            success: true,
            status: "0".to_string(),
            stdout: br#"{"snapshot_id":"uploaded-snapshot"}"#.to_vec(),
            stderr: Vec::new(),
        })
    }
}
