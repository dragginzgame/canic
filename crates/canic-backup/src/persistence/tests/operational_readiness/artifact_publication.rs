//! Module: persistence::tests::operational_readiness::artifact_publication
//!
//! Responsibility: prove both durable sides of canonical artifact-directory publication.
//! Does not own: checksum verification, the Durable journal transition, or manifest recovery.
//! Boundary: publishes or adopts only the exact directory bound to the journal checksum.

use super::{
    checksum_effect::prepared_pending_checksum, download_effect::runner_config,
    hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::{
        ArtifactCommitBarrier, BackupLayout, PersistenceError,
        commit_artifact_directory_at_barriers,
    },
    plan::BackupOperationKind,
    runner::{BackupRunnerError, backup_run_execute_with_executor},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const ARTIFACT_PUBLICATION_CHILD_ROOT_ENV: &str = "CANIC_TEST_ARTIFACT_PUBLICATION_ROOT";
const ARTIFACT_PUBLICATION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_ARTIFACT_PUBLICATION_BARRIER";
const ARTIFACT_PUBLICATION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_ARTIFACT_PUBLICATION_HANDSHAKE";

#[test]
fn canonical_artifact_publication_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(ARTIFACT_PUBLICATION_CHILD_ROOT_ENV) else {
        for barrier_name in ["before-publication", "after-publication-sync"] {
            prove_artifact_publication_side(barrier_name);
        }
        return;
    };

    let root = PathBuf::from(root);
    let barrier_name = std::env::var(ARTIFACT_PUBLICATION_CHILD_BARRIER_ENV)
        .expect("artifact publication barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(ARTIFACT_PUBLICATION_CHILD_HANDSHAKE_ENV)
            .expect("artifact publication handshake root"),
    );
    let layout = BackupLayout::new(root);
    let artifact_journal = layout
        .read_journal()
        .expect("read checksum-verified journal");
    let entry = &artifact_journal.artifacts[0];
    let temporary = PathBuf::from(
        entry
            .temp_path
            .as_deref()
            .expect("checksum-verified temp path"),
    );
    let canonical = layout.root().join(&entry.artifact_path);
    let checksum = entry
        .checksum
        .as_deref()
        .expect("checksum-verified checksum");
    let target = match barrier_name.as_str() {
        "before-publication" => ArtifactCommitBarrier::BeforePublication,
        "after-publication-sync" => ArtifactCommitBarrier::AfterPublicationSync,
        _ => panic!("unsupported artifact publication barrier: {barrier_name}"),
    };

    commit_artifact_directory_at_barriers(&temporary, &canonical, checksum, |barrier| {
        if barrier == target {
            hold_at_acknowledged_barrier(&handshake_root);
        }
    })
    .expect("publish artifact in crash child");
    panic!("artifact-publication child passed its armed barrier");
}

#[test]
fn pending_finalize_rejects_changed_checksum_verified_staging() {
    let (root, layout, finalize, temporary, canonical, expected) =
        prepared_pending_finalize("artifact-publication-changed-staging");
    fs::write(
        temporary.join("snapshot.bin"),
        b"changed before publication",
    )
    .expect("change checksum-verified staging");
    let actual = ArtifactChecksum::from_path(&temporary).expect("checksum changed staging");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("changed checksum-verified staging must not publish");

    std::assert_matches!(
        error,
        BackupRunnerError::Persistence(PersistenceError::Checksum(
            ArtifactChecksumError::ChecksumMismatch {
                expected: error_expected,
                actual: error_actual,
            }
        )) if error_expected == expected.hash && error_actual == actual.hash
    );
    assert_failed_finalize_preserves_artifact(&layout, &finalize, &expected.hash);
    assert!(temporary.is_dir());
    assert!(!canonical.exists());
    assert!(executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove changed staging layout");
}

#[test]
fn pending_finalize_rejects_conflicting_canonical_directory() {
    let (root, layout, finalize, temporary, canonical, expected) =
        prepared_pending_finalize("artifact-publication-conflicting-destination");
    fs::create_dir_all(&canonical).expect("create conflicting canonical directory");
    fs::write(canonical.join("other.bin"), b"unrelated artifact")
        .expect("write conflicting canonical artifact");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("conflicting canonical directory must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::Persistence(PersistenceError::ArtifactCommitPathConflict { .. })
    );
    assert_failed_finalize_preserves_artifact(&layout, &finalize, &expected.hash);
    assert!(temporary.is_dir());
    assert_eq!(
        fs::read(canonical.join("other.bin")).expect("read conflicting artifact"),
        b"unrelated artifact"
    );
    assert!(executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove conflicting destination layout");
}

fn prove_artifact_publication_side(barrier_name: &str) {
    let side = if barrier_name == "before-publication" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!(
        "CANIC-094-B13/canonical-artifact-publication/{side}"
    ));
    let (root, layout, finalize, temporary, canonical, expected) = prepared_pending_finalize(
        &format!("artifact-publication-{}", barrier_name.replace('-', "_")),
    );
    let handshake_root = temp_dir(&format!(
        "canic-backup-artifact-publication-handshake-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create artifact publication handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::artifact_publication::canonical_artifact_publication_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(ARTIFACT_PUBLICATION_CHILD_ROOT_ENV, &root)
        .env(ARTIFACT_PUBLICATION_CHILD_BARRIER_ENV, barrier_name)
        .env(ARTIFACT_PUBLICATION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn artifact-publication child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    if barrier_name == "before-publication" {
        assert!(temporary.is_dir());
        assert!(!canonical.exists());
    } else {
        assert!(!temporary.exists());
        assert_eq!(
            ArtifactChecksum::from_path(&canonical)
                .expect("checksum published artifact")
                .hash,
            expected.hash
        );
    }
    let interrupted_execution = layout
        .read_execution_journal()
        .expect("read interrupted execution journal");
    let interrupted_artifact = layout
        .read_journal()
        .expect("read interrupted artifact journal");
    assert_eq!(
        interrupted_execution.operations[finalize.sequence].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(
        interrupted_artifact.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );

    assert_artifact_publication_recovery(&root, &layout, &finalize, &canonical, &expected);

    fs::remove_dir_all(root).expect("remove artifact publication layout");
    fs::remove_dir_all(handshake_root).expect("remove artifact publication handshake root");
}

pub(super) fn prepared_pending_finalize(
    name: &str,
) -> (
    PathBuf,
    BackupLayout,
    BackupExecutionJournalOperation,
    PathBuf,
    PathBuf,
    ArtifactChecksum,
) {
    let (root, layout, _verify, temporary) = prepared_pending_checksum(name);
    let mut executor = FakeBackupRunnerExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect("complete artifact verification before publication");
    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());

    let mut execution = layout
        .read_execution_journal()
        .expect("read finalize-ready execution journal");
    let finalize = execution
        .next_ready_operation()
        .cloned()
        .expect("ready finalize operation");
    assert_eq!(finalize.kind, BackupOperationKind::FinalizeManifest);
    execution
        .mark_operation_pending_at(finalize.sequence, Some("unix:60".to_string()))
        .expect("mark finalize pending");
    layout
        .write_execution_journal(&execution)
        .expect("write pending finalize journal");

    let artifact_journal = layout
        .read_journal()
        .expect("read checksum-verified journal");
    let entry = &artifact_journal.artifacts[0];
    assert_eq!(entry.state, ArtifactState::ChecksumVerified);
    let canonical = root.join(&entry.artifact_path);
    let expected = ArtifactChecksum {
        algorithm: entry.checksum_algorithm.clone(),
        hash: entry.checksum.clone().expect("checksum-verified checksum"),
    };
    (root, layout, finalize, temporary, canonical, expected)
}

fn assert_artifact_publication_recovery(
    root: &Path,
    layout: &BackupLayout,
    finalize: &BackupExecutionJournalOperation,
    canonical: &Path,
    expected: &ArtifactChecksum,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume canonical artifact publication");
    let execution = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let artifact_journal = layout
        .read_journal()
        .expect("read durable artifact journal");

    assert!(response.complete);
    assert_eq!(response.executed_operation_count, 1);
    assert!(executor.commands.is_empty());
    assert_eq!(
        execution.operations[finalize.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == finalize.sequence)
            .count(),
        1
    );
    assert_eq!(artifact_journal.artifacts[0].state, ArtifactState::Durable);
    assert!(artifact_journal.artifacts[0].temp_path.is_none());
    assert_eq!(
        ArtifactChecksum::from_path(canonical)
            .expect("checksum recovered canonical artifact")
            .hash,
        expected.hash
    );
    assert!(layout.manifest_path().is_file());
}

fn assert_failed_finalize_preserves_artifact(
    layout: &BackupLayout,
    finalize: &BackupExecutionJournalOperation,
    expected_checksum: &str,
) {
    let execution = layout
        .read_execution_journal()
        .expect("read failed finalize execution journal");
    let artifact_journal = layout
        .read_journal()
        .expect("read preserved artifact journal");
    assert_eq!(
        execution.operations[finalize.sequence].state,
        BackupExecutionOperationState::Failed
    );
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == finalize.sequence)
            .count(),
        1
    );
    assert_eq!(
        artifact_journal.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );
    assert_eq!(
        artifact_journal.artifacts[0].checksum.as_deref(),
        Some(expected_checksum)
    );
    assert!(!layout.manifest_path().exists());
}
