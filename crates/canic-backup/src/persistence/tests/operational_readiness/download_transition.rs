//! Module: persistence::tests::operational_readiness::download_transition
//!
//! Responsibility: prove both durable sides of the Downloaded artifact transition.
//! Does not own: download-effect, checksum-interruption, or artifact-publication recovery.
//! Boundary: binds exact staged bytes and journal identity to execution-receipt recovery.

use super::{
    download_effect::{
        COMPLETE_BYTES, artifact_temp_path, mark_download_pending, prepared_download_operation,
        runner_config, target,
    },
    kill_child_at_acknowledged_barrier, write_document_at_barrier,
};
use crate::{
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    operational_readiness::manifest::assert_case_defined,
    persistence::BackupLayout,
    plan::BackupOperationKind,
    runner::backup_run_execute_with_executor,
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const DOWNLOAD_TRANSITION_CHILD_ROOT_ENV: &str = "CANIC_TEST_DOWNLOAD_TRANSITION_ROOT";
const DOWNLOAD_TRANSITION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_DOWNLOAD_TRANSITION_BARRIER";
const DOWNLOAD_TRANSITION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_DOWNLOAD_TRANSITION_HANDSHAKE";

#[test]
fn downloaded_artifact_transition_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(DOWNLOAD_TRANSITION_CHILD_ROOT_ENV) else {
        for barrier_name in ["before-rename", "after-directory-sync"] {
            prove_downloaded_transition_side(barrier_name);
        }
        return;
    };

    let root = PathBuf::from(root);
    let barrier_name =
        std::env::var(DOWNLOAD_TRANSITION_CHILD_BARRIER_ENV).expect("download transition barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(DOWNLOAD_TRANSITION_CHILD_HANDSHAKE_ENV)
            .expect("download transition handshake root"),
    );
    let layout = BackupLayout::new(root);
    let mut artifact_journal = layout
        .read_journal()
        .expect("read created artifact journal");
    let temp_path = artifact_temp_path(&layout);
    artifact_journal.artifacts[0].temp_path = Some(temp_path.display().to_string());
    artifact_journal.artifacts[0]
        .advance_to(ArtifactState::Downloaded, "unix:30".to_string())
        .expect("advance artifact to downloaded");
    write_document_at_barrier(
        &layout.journal_path(),
        &artifact_journal,
        &barrier_name,
        &handshake_root,
    );
}

#[test]
fn downloaded_artifact_transition_rejects_mismatched_identity_and_paths() {
    for mismatch in [
        DownloadedMismatch::Snapshot,
        DownloadedMismatch::ArtifactPath,
        DownloadedMismatch::TempPath,
    ] {
        let (root, layout) = prepared_download_operation(mismatch.layout_name());
        let pending = mark_download_pending(&layout);
        let download = pending
            .next_ready_operation()
            .cloned()
            .expect("pending download operation");
        let temp_path = artifact_temp_path(&layout);
        persist_staged_bytes(&temp_path, COMPLETE_BYTES);
        let mut artifact_journal = layout.read_journal().expect("read artifact journal");
        artifact_journal.artifacts[0].temp_path = Some(temp_path.display().to_string());
        artifact_journal.artifacts[0]
            .advance_to(ArtifactState::Downloaded, "unix:30".to_string())
            .expect("advance artifact to downloaded");
        match mismatch {
            DownloadedMismatch::Snapshot => {
                artifact_journal.artifacts[0].snapshot_id = "other-snapshot".to_string();
            }
            DownloadedMismatch::ArtifactPath => {
                artifact_journal.artifacts[0].artifact_path = "other-artifact".to_string();
            }
            DownloadedMismatch::TempPath => {
                artifact_journal.artifacts[0].temp_path =
                    Some(root.join("other.tmp").display().to_string());
            }
        }
        layout
            .write_journal(&artifact_journal)
            .expect("write mismatched downloaded evidence");

        let mut executor = FakeBackupRunnerExecutor::default();
        let error =
            backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
                .expect_err("mismatched downloaded evidence must reject");
        match mismatch {
            DownloadedMismatch::Snapshot => std::assert_matches!(
                error,
                crate::runner::BackupRunnerError::ArtifactDownloadSnapshotMismatch {
                    sequence,
                    target_canister_id,
                    ..
                } if sequence == download.sequence
                    && target_canister_id == target(&download)
            ),
            DownloadedMismatch::ArtifactPath => std::assert_matches!(
                error,
                crate::runner::BackupRunnerError::ArtifactPathMismatch {
                    sequence,
                    target_canister_id,
                    ..
                } if sequence == download.sequence
                    && target_canister_id == target(&download)
            ),
            DownloadedMismatch::TempPath => std::assert_matches!(
                error,
                crate::runner::BackupRunnerError::ArtifactTempPathMismatch {
                    sequence,
                    target_canister_id,
                    ..
                } if sequence == download.sequence
                    && target_canister_id == target(&download)
            ),
        }
        let persisted = layout
            .read_execution_journal()
            .expect("read rejected execution journal");
        assert_eq!(persisted, pending);
        assert!(executor.commands.is_empty());
        fs::remove_dir_all(root).expect("remove mismatched download layout");
    }
}

#[derive(Clone, Copy)]
enum DownloadedMismatch {
    Snapshot,
    ArtifactPath,
    TempPath,
}

impl DownloadedMismatch {
    const fn layout_name(self) -> &'static str {
        match self {
            Self::Snapshot => "downloaded-snapshot-mismatch",
            Self::ArtifactPath => "downloaded-artifact-path-mismatch",
            Self::TempPath => "downloaded-temp-path-mismatch",
        }
    }
}

fn prove_downloaded_transition_side(barrier_name: &str) {
    let side = if barrier_name == "before-rename" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!(
        "CANIC-094-B10/downloaded-artifact-transition/{side}"
    ));
    let (root, layout) = prepared_download_operation(&format!(
        "downloaded-transition-{}",
        barrier_name.replace('-', "_")
    ));
    let pending = mark_download_pending(&layout);
    let download = pending
        .next_ready_operation()
        .cloned()
        .expect("pending download operation");
    assert_eq!(download.kind, BackupOperationKind::DownloadSnapshot);
    let temp_path = artifact_temp_path(&layout);
    persist_staged_bytes(&temp_path, COMPLETE_BYTES);
    let handshake_root = temp_dir(&format!(
        "canic-backup-downloaded-transition-handshake-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create transition handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::download_transition::downloaded_artifact_transition_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(DOWNLOAD_TRANSITION_CHILD_ROOT_ENV, &root)
        .env(DOWNLOAD_TRANSITION_CHILD_BARRIER_ENV, barrier_name)
        .env(DOWNLOAD_TRANSITION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn downloaded-transition child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let observed = layout
        .read_journal()
        .expect("read artifact journal after transition crash");
    let expected_observed_state = if barrier_name == "before-rename" {
        ArtifactState::Created
    } else {
        ArtifactState::Downloaded
    };
    assert_eq!(observed.artifacts[0].state, expected_observed_state);

    assert_downloaded_transition_recovery(barrier_name, &root, &layout, &download, &temp_path);

    fs::remove_dir_all(root).expect("remove downloaded-transition layout");
    fs::remove_dir_all(handshake_root).expect("remove transition handshake root");
}

fn assert_downloaded_transition_recovery(
    barrier_name: &str,
    root: &Path,
    layout: &BackupLayout,
    download: &BackupExecutionJournalOperation,
    temp_path: &Path,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(2)),
        &mut executor,
    )
    .expect("resume download transition and verify checksum");
    let execution = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let recovered = layout
        .read_journal()
        .expect("read checksum-verified artifact journal");
    let verify = execution
        .operations
        .iter()
        .find(|operation| operation.kind == BackupOperationKind::VerifyArtifact)
        .expect("verify operation");

    assert_eq!(response.executed_operation_count, 2);
    if barrier_name == "before-rename" {
        assert_eq!(
            executor.commands,
            vec![format!(
                "download:{}:{}",
                target(download),
                recovered.artifacts[0].snapshot_id
            )]
        );
        assert_eq!(
            fs::read(temp_path.join("snapshot.bin")).expect("read repeated download"),
            b"app snapshot"
        );
    } else {
        assert!(executor.commands.is_empty());
        assert_eq!(
            fs::read(temp_path.join("snapshot.bin")).expect("read adopted download"),
            COMPLETE_BYTES
        );
    }
    assert_eq!(
        execution.operations[download.sequence].state,
        BackupExecutionOperationState::Completed
    );
    assert_eq!(verify.state, BackupExecutionOperationState::Completed);
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == download.sequence)
            .count(),
        1
    );
    assert_eq!(
        execution
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == verify.sequence)
            .count(),
        1
    );
    assert_eq!(
        recovered.artifacts[0].state,
        ArtifactState::ChecksumVerified
    );
    assert_eq!(
        recovered.artifacts[0].temp_path.as_deref(),
        Some(temp_path.to_string_lossy().as_ref())
    );
    assert!(recovered.artifacts[0].checksum.is_some());
    assert!(!root.join(&recovered.artifacts[0].artifact_path).exists());
}

fn persist_staged_bytes(temp_path: &Path, bytes: &[u8]) {
    fs::create_dir_all(temp_path).expect("create staged download directory");
    let mut file = File::create(temp_path.join("snapshot.bin")).expect("create staged snapshot");
    file.write_all(bytes).expect("write staged snapshot");
    file.sync_all().expect("sync staged snapshot");
    File::open(temp_path)
        .and_then(|directory| directory.sync_all())
        .expect("sync staged download directory");
}
