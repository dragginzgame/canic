//! Module: persistence::tests::operational_readiness::durable_transition
//!
//! Responsibility: prove both durable sides of the Durable artifact-journal transition.
//! Does not own: canonical publication, manifest publication, or terminal receipt recovery.
//! Boundary: trusts Durable state only while exact canonical bytes match its checksum.

use super::{
    artifact_publication::prepared_pending_finalize, download_effect::runner_config,
    kill_child_at_acknowledged_barrier, write_document_at_barrier,
};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::{BackupLayout, PersistenceError, commit_artifact_directory},
    runner::{BackupRunnerError, backup_run_execute_with_executor},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
    timestamp::current_timestamp_marker,
};

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

const DURABLE_TRANSITION_CHILD_ROOT_ENV: &str = "CANIC_TEST_DURABLE_TRANSITION_ROOT";
const DURABLE_TRANSITION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_DURABLE_TRANSITION_BARRIER";
const DURABLE_TRANSITION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_DURABLE_TRANSITION_HANDSHAKE";

#[test]
fn durable_artifact_transition_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(DURABLE_TRANSITION_CHILD_ROOT_ENV) else {
        for barrier_name in ["before-rename", "after-directory-sync"] {
            prove_durable_transition_side(barrier_name);
        }
        return;
    };

    let root = PathBuf::from(root);
    let barrier_name =
        std::env::var(DURABLE_TRANSITION_CHILD_BARRIER_ENV).expect("durable transition barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(DURABLE_TRANSITION_CHILD_HANDSHAKE_ENV)
            .expect("durable transition handshake root"),
    );
    let layout = BackupLayout::new(root);
    let mut artifact_journal = layout
        .read_journal()
        .expect("read checksum-verified journal");
    artifact_journal.artifacts[0].temp_path = None;
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::Durable, current_timestamp_marker())
        .expect("advance artifact to durable");
    write_document_at_barrier(
        &layout.journal_path(),
        &artifact_journal,
        &barrier_name,
        &handshake_root,
    );
}

#[test]
fn pending_finalize_rejects_missing_or_changed_durable_artifact() {
    for corruption in [DurableCorruption::Missing, DurableCorruption::Changed] {
        prove_durable_artifact_rejection(corruption);
    }
}

#[derive(Clone, Copy)]
enum DurableCorruption {
    Missing,
    Changed,
}

impl DurableCorruption {
    const fn label(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Changed => "changed",
        }
    }
}

fn prove_durable_transition_side(barrier_name: &str) {
    let side = if barrier_name == "before-rename" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!("CANIC-094-B14/durable-artifact-transition/{side}"));
    let (root, layout, finalize, temporary, canonical, expected) = prepared_pending_finalize(
        &format!("durable-transition-{}", barrier_name.replace('-', "_")),
    );
    commit_artifact_directory(&temporary, &canonical, &expected.hash)
        .expect("publish canonical artifact before durable transition");
    let canonical_inode = canonical
        .metadata()
        .expect("read canonical metadata before interruption")
        .ino();
    let handshake_root = temp_dir(&format!(
        "canic-backup-durable-transition-handshake-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create durable transition handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::durable_transition::durable_artifact_transition_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(DURABLE_TRANSITION_CHILD_ROOT_ENV, &root)
        .env(DURABLE_TRANSITION_CHILD_BARRIER_ENV, barrier_name)
        .env(DURABLE_TRANSITION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn durable-transition child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let interrupted_execution = layout
        .read_execution_journal()
        .expect("read interrupted execution journal");
    let interrupted_artifact = layout
        .read_journal()
        .expect("read interrupted artifact journal");
    let expected_state = if barrier_name == "before-rename" {
        ArtifactState::ChecksumVerified
    } else {
        ArtifactState::Durable
    };
    assert_eq!(
        interrupted_execution.operations[finalize.sequence].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(interrupted_artifact.artifacts[0].state, expected_state);
    assert!(!temporary.exists());
    assert_eq!(
        canonical.metadata().expect("canonical metadata").ino(),
        canonical_inode
    );

    assert_durable_transition_recovery(
        &root,
        &layout,
        &finalize,
        &canonical,
        canonical_inode,
        &expected,
    );

    fs::remove_dir_all(root).expect("remove durable transition layout");
    fs::remove_dir_all(handshake_root).expect("remove durable transition handshake root");
}

fn assert_durable_transition_recovery(
    root: &Path,
    layout: &BackupLayout,
    finalize: &BackupExecutionJournalOperation,
    canonical: &Path,
    canonical_inode: u64,
    expected: &ArtifactChecksum,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume durable artifact transition");
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
    assert_eq!(artifact_journal.artifacts[0].state, ArtifactState::Durable);
    assert_eq!(
        ArtifactChecksum::from_path(canonical)
            .expect("checksum recovered durable artifact")
            .hash,
        expected.hash
    );
    assert_eq!(
        canonical
            .metadata()
            .expect("canonical metadata after recovery")
            .ino(),
        canonical_inode
    );
    assert!(layout.manifest_path().is_file());
}

fn prove_durable_artifact_rejection(corruption: DurableCorruption) {
    let (root, layout, finalize, temporary, canonical, expected) =
        prepared_pending_finalize(&format!("durable-artifact-{}", corruption.label()));
    commit_artifact_directory(&temporary, &canonical, &expected.hash)
        .expect("publish canonical artifact before durable transition");
    persist_durable_artifact(&layout);
    match corruption {
        DurableCorruption::Missing => {
            fs::remove_dir_all(&canonical).expect("remove durable canonical artifact");
        }
        DurableCorruption::Changed => {
            fs::write(canonical.join("snapshot.bin"), b"changed durable artifact")
                .expect("change durable canonical artifact");
        }
    }

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("invalid durable artifact must reject");
    match corruption {
        DurableCorruption::Missing => std::assert_matches!(
            error,
            BackupRunnerError::Persistence(PersistenceError::MissingArtifact(path))
                if path == canonical.display().to_string()
        ),
        DurableCorruption::Changed => std::assert_matches!(
            error,
            BackupRunnerError::Persistence(PersistenceError::Checksum(
                ArtifactChecksumError::ChecksumMismatch { expected: actual_expected, .. }
            )) if actual_expected == expected.hash
        ),
    }
    let execution = layout
        .read_execution_journal()
        .expect("read rejected execution journal");
    let artifact_journal = layout
        .read_journal()
        .expect("read rejected artifact journal");
    assert_eq!(
        execution.operations[finalize.sequence].state,
        BackupExecutionOperationState::Failed
    );
    assert_eq!(artifact_journal.artifacts[0].state, ArtifactState::Durable);
    assert!(!layout.manifest_path().exists());
    assert!(executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove durable artifact rejection layout");
}

fn persist_durable_artifact(layout: &BackupLayout) {
    let mut artifact_journal = layout
        .read_journal()
        .expect("read checksum-verified journal");
    artifact_journal.artifacts[0].temp_path = None;
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::Durable, current_timestamp_marker())
        .expect("advance artifact to durable");
    layout
        .write_journal(&artifact_journal)
        .expect("write durable artifact journal");
}
