//! Module: persistence::tests::operational_readiness::checksum_effect
//!
//! Responsibility: prove checksum recomputation after process death.
//! Does not own: checksum-journal publication or canonical artifact publication.
//! Boundary: reads exact Downloaded staging without mutating authoritative bytes.

use super::{
    download_effect::{artifact_temp_path, prepared_download_operation, runner_config},
    hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::BackupLayout,
    plan::BackupOperationKind,
    runner::{BackupRunnerError, backup_run_execute_with_executor},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

const CHECKSUM_EFFECT_CHILD_ROOT_ENV: &str = "CANIC_TEST_CHECKSUM_EFFECT_ROOT";
const CHECKSUM_EFFECT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_CHECKSUM_EFFECT_HANDSHAKE";
const COMPUTED_CHECKSUM_FILE: &str = "computed-checksum";

#[test]
fn completed_in_memory_checksum_is_recomputed_after_process_death() {
    let Some(root) = std::env::var_os(CHECKSUM_EFFECT_CHILD_ROOT_ENV) else {
        prove_checksum_recomputation();
        return;
    };

    let layout = BackupLayout::new(PathBuf::from(root));
    let handshake_root = PathBuf::from(
        std::env::var_os(CHECKSUM_EFFECT_CHILD_HANDSHAKE_ENV)
            .expect("checksum effect handshake root"),
    );
    let checksum = ArtifactChecksum::from_path(&artifact_temp_path(&layout))
        .expect("compute staged artifact checksum");
    persist_computed_checksum(&handshake_root, &checksum.hash);
    hold_at_acknowledged_barrier(&handshake_root);
}

#[test]
fn pending_checksum_rejects_missing_staging_without_verified_state() {
    let (root, layout, verify, temp_path) = prepared_pending_checksum("checksum-missing-stage");
    fs::remove_dir_all(&temp_path).expect("remove staged artifact");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("missing checksum input must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::Checksum(ArtifactChecksumError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound
    );
    assert_failed_checksum_state(&layout, &verify);
    assert!(executor.commands.is_empty());
    fs::remove_dir_all(root).expect("remove missing-stage checksum layout");
}

#[test]
fn pending_checksum_rejects_unsafe_staging_without_following_it() {
    let (root, layout, verify, temp_path) = prepared_pending_checksum("checksum-unsafe-stage");
    let outside = temp_dir("canic-backup-checksum-unsafe-outside");
    fs::create_dir_all(&outside).expect("create checksum outside directory");
    let sentinel = outside.join("sentinel");
    fs::write(&sentinel, b"must survive").expect("write checksum outside sentinel");
    symlink(&sentinel, temp_path.join("linked-snapshot")).expect("create unsafe staged symlink");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("unsafe checksum input must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::Checksum(ArtifactChecksumError::UnsupportedEntry { .. })
    );
    assert_failed_checksum_state(&layout, &verify);
    assert!(executor.commands.is_empty());
    assert_eq!(
        fs::read(&sentinel).expect("read checksum outside sentinel"),
        b"must survive"
    );
    fs::remove_dir_all(root).expect("remove unsafe-stage checksum layout");
    fs::remove_dir_all(outside).expect("remove checksum outside directory");
}

fn prove_checksum_recomputation() {
    assert_case_defined("CANIC-094-B11/verify-artifact/effect-committed-receipt-missing");
    let (root, layout, verify, temp_path) = prepared_pending_checksum("checksum-effect");
    let expected = ArtifactChecksum::from_path(&temp_path).expect("checksum before interruption");
    let bytes_before = fs::read(temp_path.join("snapshot.bin")).expect("read staged bytes");
    let handshake_root = temp_dir("canic-backup-checksum-effect-handshake");
    fs::create_dir_all(&handshake_root).expect("create checksum handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::checksum_effect::completed_in_memory_checksum_is_recomputed_after_process_death",
            "--nocapture",
        ])
        .env(CHECKSUM_EFFECT_CHILD_ROOT_ENV, &root)
        .env(CHECKSUM_EFFECT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn checksum effect child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let child_checksum = fs::read_to_string(handshake_root.join(COMPUTED_CHECKSUM_FILE))
        .expect("read child checksum evidence");
    let interrupted_execution = layout
        .read_execution_journal()
        .expect("read interrupted execution journal");
    let interrupted_artifact = layout
        .read_journal()
        .expect("read interrupted artifact journal");

    assert_eq!(child_checksum, expected.hash);
    assert_eq!(
        interrupted_execution.operations[verify.sequence].state,
        BackupExecutionOperationState::Pending
    );
    assert!(
        interrupted_execution
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != verify.sequence)
    );
    assert_eq!(
        interrupted_artifact.artifacts[0].state,
        ArtifactState::Downloaded
    );
    assert!(interrupted_artifact.artifacts[0].checksum.is_none());
    assert_eq!(
        ArtifactChecksum::from_path(&temp_path)
            .expect("checksum after interruption")
            .hash,
        expected.hash
    );

    let mut executor = FakeBackupRunnerExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect("recompute checksum after process death");
    let recovered_execution = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let recovered_artifact = layout
        .read_journal()
        .expect("read recovered artifact journal");
    let receipt = recovered_execution
        .operation_receipts
        .iter()
        .find(|receipt| receipt.sequence == verify.sequence)
        .expect("checksum execution receipt");

    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());
    assert_eq!(
        recovered_execution.operations[verify.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(receipt.checksum.as_deref(), Some(expected.hash.as_str()));
    assert_eq!(
        recovered_artifact.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );
    assert_eq!(
        recovered_artifact.artifacts[0].checksum.as_deref(),
        Some(expected.hash.as_str())
    );
    assert_eq!(
        fs::read(temp_path.join("snapshot.bin")).expect("read recovered staged bytes"),
        bytes_before
    );

    let mut replay_executor = FakeBackupRunnerExecutor::default();
    let replay = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(0)),
        &mut replay_executor,
    )
    .expect("completed checksum replay performs no work");
    assert_eq!(replay.executed_operation_count, 0);
    assert!(replay_executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove checksum effect layout");
    fs::remove_dir_all(handshake_root).expect("remove checksum handshake root");
}

pub(super) fn prepared_pending_checksum(
    name: &str,
) -> (
    PathBuf,
    BackupLayout,
    BackupExecutionJournalOperation,
    PathBuf,
) {
    let (root, layout) = prepared_download_operation(name);
    let mut executor = FakeBackupRunnerExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect("complete download before checksum");
    assert_eq!(response.executed_operation_count, 1);
    let mut execution = layout
        .read_execution_journal()
        .expect("read checksum-ready execution journal");
    let verify = execution
        .next_ready_operation()
        .cloned()
        .expect("ready checksum operation");
    assert_eq!(verify.kind, BackupOperationKind::VerifyArtifact);
    execution
        .mark_operation_pending_at(verify.sequence, Some("unix:40".to_string()))
        .expect("mark checksum pending");
    layout
        .write_execution_journal(&execution)
        .expect("write pending checksum journal");
    let temp_path = artifact_temp_path(&layout);
    (root, layout, verify, temp_path)
}

fn assert_failed_checksum_state(layout: &BackupLayout, verify: &BackupExecutionJournalOperation) {
    let execution = layout
        .read_execution_journal()
        .expect("read failed checksum execution journal");
    let artifact = layout
        .read_journal()
        .expect("read failed checksum artifact journal");
    assert_eq!(
        execution.operations[verify.sequence].state,
        BackupExecutionOperationState::Failed
    );
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == verify.sequence)
            .count(),
        1
    );
    assert_eq!(artifact.artifacts[0].state, ArtifactState::Downloaded);
    assert!(artifact.artifacts[0].checksum.is_none());
}

fn persist_computed_checksum(root: &Path, checksum: &str) {
    let mut file =
        File::create(root.join(COMPUTED_CHECKSUM_FILE)).expect("create computed checksum evidence");
    file.write_all(checksum.as_bytes())
        .expect("write computed checksum evidence");
    file.sync_all().expect("sync computed checksum evidence");
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .expect("sync computed checksum directory");
}
