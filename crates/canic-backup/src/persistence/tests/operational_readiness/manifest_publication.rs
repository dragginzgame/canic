//! Module: persistence::tests::operational_readiness::manifest_publication
//!
//! Responsibility: prove both durable sides of final manifest publication.
//! Does not own: durable artifact reconciliation, terminal receipts, or final responses.
//! Boundary: publishes or adopts only the manifest derived from current durable authority.

use super::{
    artifact_publication::prepared_pending_finalize,
    download_effect::{prepared_download_operation, runner_config},
    hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
};
use crate::{
    artifacts::ArtifactChecksum,
    execution::{BackupExecutionJournalOperation, BackupExecutionOperationState},
    journal::ArtifactState,
    manifest::DeploymentBackupManifest,
    operational_readiness::manifest::assert_case_defined,
    persistence::{BackupLayout, PersistenceError, commit_artifact_directory},
    runner::{BackupRunnerError, backup_run_execute_with_executor, build_manifest_for_test},
    test_support::{FakeBackupRunnerExecutor, temp_dir},
    timestamp::current_timestamp_marker,
};

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

const MANIFEST_PUBLICATION_CHILD_ROOT_ENV: &str = "CANIC_TEST_MANIFEST_PUBLICATION_ROOT";
const MANIFEST_PUBLICATION_CHILD_BARRIER_ENV: &str = "CANIC_TEST_MANIFEST_PUBLICATION_BARRIER";
const MANIFEST_PUBLICATION_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_MANIFEST_PUBLICATION_HANDSHAKE";

#[test]
fn manifest_publication_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(MANIFEST_PUBLICATION_CHILD_ROOT_ENV) else {
        for barrier_name in ["before-publication", "after-directory-sync"] {
            prove_manifest_publication_side(barrier_name);
        }
        return;
    };

    let root = PathBuf::from(root);
    let barrier_name = std::env::var(MANIFEST_PUBLICATION_CHILD_BARRIER_ENV)
        .expect("manifest publication barrier");
    let handshake_root = PathBuf::from(
        std::env::var_os(MANIFEST_PUBLICATION_CHILD_HANDSHAKE_ENV)
            .expect("manifest publication handshake root"),
    );
    let layout = BackupLayout::new(root.clone());
    let manifest = expected_manifest(&root, &layout);
    layout
        .publish_manifest_at_barriers(
            &manifest,
            || {
                if barrier_name == "before-publication" {
                    hold_at_acknowledged_barrier(&handshake_root);
                }
            },
            || {
                if barrier_name == "after-directory-sync" {
                    hold_at_acknowledged_barrier(&handshake_root);
                }
            },
        )
        .expect("publish manifest in crash child");
    panic!("manifest-publication child passed its armed barrier");
}

#[test]
fn pending_finalize_rejects_conflicting_manifest_without_replacing_it() {
    assert_case_defined("CANIC-094-C06/publication-journal-disagreement/rejection");
    let (root, layout, finalize, _canonical, mut expected) =
        prepared_pending_manifest("manifest-publication-conflict");
    expected.created_at = "unix:999".to_string();
    layout
        .publish_manifest(&expected)
        .expect("publish conflicting valid manifest");
    let bytes_before = fs::read(layout.manifest_path()).expect("read conflicting manifest");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("conflicting manifest must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::Persistence(PersistenceError::ManifestConflict { path })
            if path == layout.manifest_path().display().to_string()
    );
    assert_eq!(
        fs::read(layout.manifest_path()).expect("read preserved conflicting manifest"),
        bytes_before
    );
    assert_failed_finalize(&layout, &finalize);
    assert!(executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove conflicting manifest layout");
}

#[test]
fn manifest_before_pending_finalization_rejects_before_any_operation() {
    let (root, layout) = prepared_download_operation("premature-manifest");
    fs::write(layout.manifest_path(), b"premature manifest")
        .expect("write premature manifest marker");
    let execution_before = layout
        .read_execution_journal()
        .expect("read execution before premature manifest rejection");
    let finalize = execution_before
        .operations
        .iter()
        .find(|operation| operation.kind == crate::plan::BackupOperationKind::FinalizeManifest)
        .expect("finalize operation");

    let mut executor = FakeBackupRunnerExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), Some(1)), &mut executor)
            .expect_err("premature manifest must reject");

    std::assert_matches!(
        error,
        BackupRunnerError::PrematureManifest { sequence, state }
            if sequence == finalize.sequence && state == BackupExecutionOperationState::Ready
    );
    assert_eq!(
        layout
            .read_execution_journal()
            .expect("read execution after premature manifest rejection"),
        execution_before
    );
    assert!(executor.commands.is_empty());

    fs::remove_dir_all(root).expect("remove premature manifest layout");
}

fn prove_manifest_publication_side(barrier_name: &str) {
    let side = if barrier_name == "before-publication" {
        "before-durable-write"
    } else {
        "after-durable-write"
    };
    assert_case_defined(&format!("CANIC-094-B15/manifest-publication/{side}"));
    let (root, layout, finalize, canonical, expected) = prepared_pending_manifest(&format!(
        "manifest-publication-{}",
        barrier_name.replace('-', "_")
    ));
    let handshake_root = temp_dir(&format!(
        "canic-backup-manifest-publication-handshake-{}",
        barrier_name.replace('-', "_")
    ));
    fs::create_dir_all(&handshake_root).expect("create manifest publication handshake root");

    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::manifest_publication::manifest_publication_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(MANIFEST_PUBLICATION_CHILD_ROOT_ENV, &root)
        .env(MANIFEST_PUBLICATION_CHILD_BARRIER_ENV, barrier_name)
        .env(MANIFEST_PUBLICATION_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn manifest-publication child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let manifest_inode = if barrier_name == "before-publication" {
        assert!(!layout.manifest_path().exists());
        None
    } else {
        let observed = layout.read_manifest().expect("read published manifest");
        assert_manifest_exact(&observed, &expected);
        Some(
            layout
                .manifest_path()
                .metadata()
                .expect("manifest metadata")
                .ino(),
        )
    };
    let interrupted = layout
        .read_execution_journal()
        .expect("read interrupted execution journal");
    assert_eq!(
        interrupted.operations[finalize.sequence].state,
        BackupExecutionOperationState::Pending
    );

    assert_manifest_publication_recovery(
        &root,
        &layout,
        &finalize,
        &canonical,
        &expected,
        manifest_inode,
    );

    fs::remove_dir_all(root).expect("remove manifest publication layout");
    fs::remove_dir_all(handshake_root).expect("remove manifest publication handshake root");
}

fn prepared_pending_manifest(
    name: &str,
) -> (
    PathBuf,
    BackupLayout,
    BackupExecutionJournalOperation,
    PathBuf,
    DeploymentBackupManifest,
) {
    let (root, layout, finalize, temporary, canonical, expected_checksum) =
        prepared_pending_finalize(name);
    commit_artifact_directory(&temporary, &canonical, &expected_checksum.hash)
        .expect("publish canonical artifact before manifest");
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
    let expected = expected_manifest(&root, &layout);
    (root, layout, finalize, canonical, expected)
}

fn expected_manifest(root: &Path, layout: &BackupLayout) -> DeploymentBackupManifest {
    let plan = layout.read_backup_plan().expect("read backup plan");
    let artifact_journal = layout.read_journal().expect("read artifact journal");
    build_manifest_for_test(
        &runner_config(root.to_path_buf(), Some(1)),
        &plan,
        &artifact_journal,
    )
    .expect("build expected manifest")
}

fn assert_manifest_publication_recovery(
    root: &Path,
    layout: &BackupLayout,
    finalize: &BackupExecutionJournalOperation,
    canonical: &Path,
    expected: &DeploymentBackupManifest,
    manifest_inode: Option<u64>,
) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &runner_config(root.to_path_buf(), Some(1)),
        &mut executor,
    )
    .expect("resume manifest publication");
    let execution = layout
        .read_execution_journal()
        .expect("read recovered execution journal");
    let recovered = layout.read_manifest().expect("read recovered manifest");

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
    assert_manifest_exact(&recovered, expected);
    if let Some(inode) = manifest_inode {
        assert_eq!(
            layout
                .manifest_path()
                .metadata()
                .expect("recovered manifest metadata")
                .ino(),
            inode
        );
    }
    assert_eq!(
        ArtifactChecksum::from_path(canonical)
            .expect("checksum canonical artifact after manifest recovery")
            .hash,
        recovered.deployment.members[0]
            .source_snapshot
            .checksum
            .clone()
            .expect("manifest checksum")
    );
}

fn assert_manifest_exact(actual: &DeploymentBackupManifest, expected: &DeploymentBackupManifest) {
    assert_eq!(
        serde_json::to_value(actual).expect("serialize actual manifest"),
        serde_json::to_value(expected).expect("serialize expected manifest")
    );
}

fn assert_failed_finalize(layout: &BackupLayout, finalize: &BackupExecutionJournalOperation) {
    let execution = layout
        .read_execution_journal()
        .expect("read failed finalize execution journal");
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
}
