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
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

use super::*;

const SOURCE_BYTES: &[u8] = b"authoritative snapshot bytes";

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
