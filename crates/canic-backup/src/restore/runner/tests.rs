//! Module: restore::runner::tests
//!
//! Responsibility: prove restore execution consumes private checksum-bound artifact bytes.
//! Does not own: artifact traversal implementation or ICP command behavior.
//! Boundary: exercises journal claim, staging, command execution, receipt, and cleanup together.

use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    persistence::CommandLifetimeLock,
    restore::{
        RestoreApplyCommandConfig, RestoreApplyJournalOperation, RestoreApplyOperationKind,
        RestoreApplyOperationKindCounts, RestoreApplyOperationState, write_restore_apply_journal,
    },
    test_support::temp_dir,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::*;

const SOURCE_BYTES: &[u8] = b"authoritative snapshot bytes";

#[test]
fn execute_upload_uses_private_verified_copy_and_records_checksum() {
    let fixture = upload_fixture("canic-restore-private-stage");
    let source_path = fixture.root.join("artifacts/root");
    let mut executor = InspectingExecutor {
        original_source: source_path.clone(),
        observed_input: None,
        calls: 0,
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
    journal.ready_operations = 0;
    journal.pending_operations = 1;
    write_restore_apply_journal(&fixture.config.journal, &journal).expect("write pending journal");
    let command_lock =
        CommandLifetimeLock::acquire(&fixture.config.journal, 0).expect("hold prior command lock");
    let mut executor = InspectingExecutor {
        original_source: fixture.root.join("artifacts/root"),
        observed_input: None,
        calls: 0,
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
    let error = restore_run_execute_with_executor(&fixture.config, &mut executor)
        .expect_err("quiescent unknown outcome must stop resume");
    let persisted: RestoreApplyJournal =
        serde_json::from_slice(&fs::read(&fixture.config.journal).expect("read pending journal"))
            .expect("decode pending journal");

    std::assert_matches!(
        error,
        RestoreRunnerError::CommandOutcomeUnknown {
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
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn execute_repeats_pending_verification_without_a_mutating_command_lock() {
    let root = temp_dir("canic-restore-pending-verification");
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
    let mut executor = SuccessfulExecutor { calls: 0 };

    let response = restore_run_execute_with_executor(&config, &mut executor)
        .expect("repeat read-only verification");

    assert!(response.complete);
    assert_eq!(executor.calls, 1);
    fs::remove_dir_all(root).expect("remove fixture");
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
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

impl RestoreRunnerCommandExecutor for InspectingExecutor {
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
        _command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
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
