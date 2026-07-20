//! Module: persistence::tests::operational_readiness::checksum_transition
//!
//! Responsibility: prove both durable sides of the ChecksumVerified artifact transition.
//! Does not own: checksum-effect, artifact-directory publication, or manifest recovery.
//! Boundary: adopts only checksum-bound staged bytes when rebuilding an execution receipt.

use super::{
    checksum_effect::prepared_pending_checksum, download_effect::runner_config,
    kill_child_at_acknowledged_barrier, write_document_at_barrier,
};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::BackupLayout,
    runner::{BackupRunnerError, backup_run_execute_with_executor},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CHECKSUM_TRANSITION_CHILD_ROOT_ENV: &str = "CANIC_TEST_CHECKSUM_TRANSITION_ROOT";
const CHECKSUM_TRANSITION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_CHECKSUM_TRANSITION_BARRIER";
const CHECKSUM_TRANSITION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_CHECKSUM_TRANSITION_HANDSHAKE";

#[test]
fn checksum_verified_transition_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(CHECKSUM_TRANSITION_CHILD_ROOT_ENV) else {
        for barrier_name in ["before-rename", "after-directory-sync"] {
            prove_checksum_verified_transition_side(barrier_name);
        }
        return;
    };

    let root = PathBuf::from(root);
    let barrier_name =
        std::env::var(CHECKSUM_TRANSITION_CHILD_BARRIER_ENV).expect("checksum transition barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(CHECKSUM_TRANSITION_CHILD_HANDSHAKE_ENV)
            .expect("checksum transition handshake root"),
    );
    let layout = BackupLayout::new(root);
    let mut artifact_journal = layout.read_journal().expect("read downloaded journal");
    let temp_path = PathBuf::from(
        artifact_journal.artifacts[0]
            .temp_path
            .as_deref()
            .expect("downloaded artifact temp path"),
    );
    let checksum = ArtifactChecksum::from_path(&temp_path).expect("checksum staged artifact");
    artifact_journal.artifacts[0].checksum = Some(checksum.hash);
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::ChecksumVerified, "unix:50".to_string())
        .expect("advance artifact to checksum verified");
    write_document_at_barrier(
        &layout.journal_path(),
        &artifact_journal,
        &barrier_name,
        &handshake_root,
    );
}

#[test]
fn pending_checksum_verified_evidence_rejects_changed_staging() {
    let (root, layout, _verify, temp_path) =
        prepared_pending_checksum("checksum-verified-changed-stage");
    let expected = ArtifactChecksum::from_path(&temp_path).expect("checksum trusted staging");
    persist_checksum_verified(&layout, &expected.hash);
    fs::write(
        temp_path.join("snapshot.bin"),
        b"changed after verified transition",
    )
    .expect("replace verified staged bytes");
    let actual = ArtifactChecksum::from_path(&temp_path).expect("checksum changed staging");
    let pending_execution = layout
        .read_execution_journal()
        .expect("read pending execution journal");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("changed checksum-verified staging must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::Checksum(ArtifactChecksumError::ChecksumMismatch {
            expected: error_expected,
            actual: error_actual,
        }) if error_expected == expected.hash && error_actual == actual.hash
    );
    assert_eq!(
        layout
            .read_execution_journal()
            .expect("read rejected execution journal"),
        pending_execution
    );
    let artifact_journal = layout
        .read_journal()
        .expect("read preserved artifact journal");
    assert_eq!(
        artifact_journal.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );
    assert_eq!(
        artifact_journal.artifacts[0].checksum.as_deref(),
        Some(expected.hash.as_str())
    );
    assert!(executor.commands.is_empty());
    assert_no_canonical_artifact(&root, &artifact_journal.artifacts[0].artifact_path);

    fs::remove_dir_all(root).expect("remove changed staging layout");
}

fn prove_checksum_verified_transition_side(barrier_name: &str) {
    let side = if barrier_name == "before-rename" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!(
        "CANIC-094-B12/checksum-verified-artifact-transition/{side}"
    ));
    let (root, layout, verify, temp_path) = prepared_pending_checksum(&format!(
        "checksum-verified-transition-{}",
        barrier_name.replace('-', "_")
    ));
    let expected = ArtifactChecksum::from_path(&temp_path).expect("checksum before interruption");
    let bytes_before = fs::read(temp_path.join("snapshot.bin")).expect("read staged bytes");
    let handshake_root = temp_dir(&format!(
        "canic-backup-checksum-transition-handshake-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create checksum transition handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::checksum_transition::checksum_verified_transition_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(CHECKSUM_TRANSITION_CHILD_ROOT_ENV, &root)
        .env(CHECKSUM_TRANSITION_CHILD_BARRIER_ENV, barrier_name)
        .env(CHECKSUM_TRANSITION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn checksum-transition child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let observed = layout
        .read_journal()
        .expect("read artifact journal after checksum transition crash");
    let expected_observed_state = if barrier_name == "before-rename" {
        ArtifactState::Downloaded
    } else {
        ArtifactState::ChecksumVerified
    };
    assert_eq!(observed.artifacts[0].state, expected_observed_state);

    assert_checksum_transition_recovery(
        &root,
        &layout,
        &verify,
        &temp_path,
        &bytes_before,
        &expected,
    );

    fs::remove_dir_all(root).expect("remove checksum transition layout");
    fs::remove_dir_all(handshake_root).expect("remove checksum transition handshake root");
}

fn assert_checksum_transition_recovery(
    root: &Path,
    layout: &BackupLayout,
    verify: &BackupExecutionJournalOperation,
    temp_path: &Path,
    bytes_before: &[u8],
    expected: &ArtifactChecksum,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume checksum-verified transition");
    let execution = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let recovered = layout
        .read_journal()
        .expect("read checksum-verified artifact journal");
    let receipt = execution
        .operation_receipts
        .iter()
        .find(|receipt| receipt.sequence == verify.sequence)
        .expect("verification receipt");

    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());
    assert_eq!(
        execution.operations[verify.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == verify.sequence)
            .count(),
        1
    );
    assert_eq!(receipt.checksum.as_deref(), Some(expected.hash.as_str()));
    assert_eq!(
        recovered.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );
    assert_eq!(
        recovered.artifacts[0].checksum.as_deref(),
        Some(expected.hash.as_str())
    );
    assert_eq!(
        fs::read(temp_path.join("snapshot.bin")).expect("read recovered staged bytes"),
        bytes_before
    );
    assert_no_canonical_artifact(root, &recovered.artifacts[0].artifact_path);
}

fn persist_checksum_verified(layout: &BackupLayout, checksum: &str) {
    let mut artifact_journal = layout.read_journal().expect("read downloaded journal");
    artifact_journal.artifacts[0].checksum = Some(checksum.to_string());
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::ChecksumVerified, "unix:50".to_string())
        .expect("advance artifact to checksum verified");
    layout
        .write_journal(&artifact_journal)
        .expect("write checksum-verified journal");
}

fn assert_no_canonical_artifact(root: &Path, artifact_path: &str) {
    assert!(!root.join(artifact_path).exists());
}
