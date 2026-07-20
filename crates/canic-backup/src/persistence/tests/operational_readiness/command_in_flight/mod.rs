//! Module: persistence::tests::operational_readiness::command_in_flight
//!
//! Responsibility: prove restart exclusion while an orphaned backup command lives.
//! Does not own: effect-specific reconciliation policy or command construction.
//! Boundary: exercises the runner/executor descriptor contract with a real process tree.

use super::{
    kill_child_at_acknowledged_barrier, snapshot_layout_tree,
    terminal_transition::prepare_target_operation,
};
use crate::{
    execution::{BackupExecutionOperationReceiptOutcome, BackupExecutionOperationState},
    operational_readiness::manifest::{assert_case_defined, backup_operation_label},
    persistence::{
        BackupLayout, CommandLifetimeHandle, CommandLifetimeLock, CommandLifetimeLockError,
    },
    plan::{BackupExecutionPreflightReceipts, BackupOperationKind, BackupPlan},
    runner::{
        BackupRunResponse, BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerError,
        BackupRunnerExecutor, BackupRunnerSnapshot, backup_run_execute_with_executor,
    },
    test_support::{FakeBackupRunnerExecutor, temp_dir},
};

use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
use std::{
    fs::{self, File},
    io::{self, Write},
    os::{
        fd::{BorrowedFd, FromRawFd},
        unix::process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

const ROLE_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_ROLE";
const ROOT_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_ROOT";
const OPERATION_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_OPERATION";
const HANDSHAKE_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_HANDSHAKE";
const COMMAND_FD_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_FD";
const ARTIFACT_PATH_ENV: &str = "CANIC_TEST_COMMAND_IN_FLIGHT_ARTIFACT";

const SNAPSHOT_ID: &str = "snap-app";
const SNAPSHOT_TAKEN_AT: u64 = 1_778_709_681_897_818_005;
const SNAPSHOT_SIZE: u64 = 272_586_987;

#[test]
fn orphaned_command_tree_blocks_restart_for_every_mutating_backup_operation() {
    match std::env::var(ROLE_ENV).ok().as_deref() {
        Some("runner") => run_owner_role(),
        Some("external") => run_external_role(),
        Some("descendant") => run_descendant_role(),
        Some(role) => panic!("unsupported command-in-flight test role: {role}"),
        None => {
            for operation in [
                BackupOperationKind::Stop,
                BackupOperationKind::CreateSnapshot,
                BackupOperationKind::Start,
                BackupOperationKind::DownloadSnapshot,
            ] {
                prove_orphaned_command_tree_blocks_restart(&operation);
            }
        }
    }
}

fn prove_orphaned_command_tree_blocks_restart(operation: &BackupOperationKind) {
    let operation_label = backup_operation_label(operation);
    assert_case_defined(&format!(
        "CANIC-094-B18/{operation_label}/owner-dead-command-in-flight"
    ));
    let (root, layout, target_sequence) = prepare_target_operation(operation);
    let target_operation = layout
        .read_execution_journal()
        .expect("read target-ready journal")
        .operations[target_sequence]
        .clone();
    let handshake_root = temp_dir(&format!("canic-backup-command-in-flight-{operation_label}"));
    fs::create_dir_all(&handshake_root).expect("create command-in-flight handshake root");

    let mut owner = spawn_owner(&root, operation_label, &handshake_root);
    kill_child_at_acknowledged_barrier(&mut owner, &handshake_root);

    let pending = layout
        .read_execution_journal()
        .expect("read pending journal after owner death");
    assert_eq!(
        pending.operations[target_sequence].state,
        BackupExecutionOperationState::Pending
    );
    assert!(
        pending
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != target_sequence)
    );
    assert!(!effect_marker(&handshake_root).exists());

    fs::write(
        direct_release_marker(&handshake_root),
        b"release direct owner\n",
    )
    .expect("release direct command owner descriptor");
    super::wait_for_path(
        &direct_closed_marker(&handshake_root),
        "direct command descriptor close",
    );

    let before_blocked_restart = snapshot_layout_tree(&root);
    let mut blocked_executor = FakeBackupRunnerExecutor::default();
    let error = backup_run_execute_with_executor(
        &super::download_effect::runner_config(root.clone(), Some(1)),
        &mut blocked_executor,
    )
    .expect_err("live descendant must block backup restart");
    let after_blocked_restart = layout
        .read_execution_journal()
        .expect("read journal after blocked restart");

    std::assert_matches!(
        error,
        BackupRunnerError::CommandInFlight {
            sequence,
            operation_id,
            ..
        } if sequence == target_sequence && operation_id == target_operation.operation_id
    );
    assert!(blocked_executor.commands.is_empty());
    assert_eq!(after_blocked_restart, pending);
    assert_eq!(snapshot_layout_tree(&root), before_blocked_restart);
    assert!(!effect_marker(&handshake_root).exists());

    recover_and_complete(
        operation,
        &root,
        &layout,
        target_sequence,
        &target_operation,
        &handshake_root,
    );

    fs::remove_dir_all(root).expect("remove command-in-flight layout");
    fs::remove_dir_all(handshake_root).expect("remove command-in-flight handshake root");
}

fn recover_and_complete(
    operation: &BackupOperationKind,
    root: &Path,
    layout: &BackupLayout,
    target_sequence: usize,
    target_operation: &crate::execution::BackupExecutionJournalOperation,
    handshake_root: &Path,
) {
    fs::write(effect_release_marker(handshake_root), b"commit effect\n")
        .expect("release external effect descendant");
    super::wait_for_path(&effect_marker(handshake_root), "committed external effect");
    super::wait_for_path(
        &external_complete_marker(handshake_root),
        "external command completion",
    );
    wait_for_command_quiescence(layout, target_sequence);

    let mut recovery_executor = recovery_executor(operation, target_operation);
    let recovery = backup_run_execute_with_executor(
        &super::download_effect::runner_config(root.to_path_buf(), Some(1)),
        &mut recovery_executor,
    )
    .expect("recover after orphaned command becomes quiescent");
    assert_target_recovery(
        operation,
        target_sequence,
        layout,
        &recovery,
        &recovery_executor.commands,
    );

    let completion = backup_run_execute_with_executor(
        &super::download_effect::runner_config(root.to_path_buf(), None),
        &mut recovery_executor,
    )
    .expect("complete backup after command-in-flight recovery");
    assert!(completion.complete);
    layout
        .verify_integrity()
        .expect("verify completed command-in-flight backup");
    let complete_journal = layout
        .read_execution_journal()
        .expect("read completed command-in-flight journal");
    assert_eq!(
        complete_journal
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == target_sequence)
            .count(),
        1
    );
}

fn spawn_owner(root: &Path, operation_label: &str, handshake_root: &Path) -> Child {
    Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::command_in_flight::orphaned_command_tree_blocks_restart_for_every_mutating_backup_operation",
            "--nocapture",
        ])
        .env(ROLE_ENV, "runner")
        .env(ROOT_ENV, root)
        .env(OPERATION_ENV, operation_label)
        .env(HANDSHAKE_ENV, handshake_root)
        .spawn()
        .expect("spawn backup runner owner")
}

fn run_owner_role() {
    let root = required_path(ROOT_ENV);
    let operation = required_operation();
    let handshake_root = required_path(HANDSHAKE_ENV);
    let mut executor = InFlightCommandExecutor {
        operation: operation.clone(),
        handshake_root,
        delegate: FakeBackupRunnerExecutor::default(),
    };
    backup_run_execute_with_executor(
        &super::download_effect::runner_config(root, Some(1)),
        &mut executor,
    )
    .unwrap_or_else(|error| panic!("execute {operation:?} in owner process: {error}"));
    panic!("runner owner returned before process-death barrier");
}

fn run_external_role() {
    let handshake_root = required_path(HANDSHAKE_ENV);
    let descendant_ready = handshake_root.join("descendant-ready");
    let mut descendant = Command::new(std::env::current_exe().expect("resolve test executable"));
    descendant
        .args([
            "--exact",
            "persistence::tests::operational_readiness::command_in_flight::orphaned_command_tree_blocks_restart_for_every_mutating_backup_operation",
            "--nocapture",
        ])
        .env(ROLE_ENV, "descendant");
    let mut descendant = descendant.spawn().expect("spawn effect descendant");
    super::wait_for_child_path(
        &mut descendant,
        &descendant_ready,
        "effect descendant readiness",
    );

    fs::write(handshake_root.join("barrier-ready"), b"ready\n").expect("signal in-flight barrier");
    super::wait_for_path(
        &handshake_root.join("barrier-acknowledged"),
        "in-flight barrier acknowledgement",
    );
    fs::write(handshake_root.join("barrier-armed"), b"armed\n").expect("arm owner-death barrier");
    super::wait_for_path(
        &direct_release_marker(&handshake_root),
        "direct descriptor release",
    );

    let raw_fd = std::env::var(COMMAND_FD_ENV)
        .expect("inherited command descriptor")
        .parse::<i32>()
        .expect("numeric inherited command descriptor");
    // SAFETY: this exec'd test process inherited this descriptor without a
    // Rust owner. The descendant was spawned first and retains its own copy.
    unsafe {
        drop(File::from_raw_fd(raw_fd));
    }
    fs::write(direct_closed_marker(&handshake_root), b"closed\n")
        .expect("signal direct descriptor close");

    let status = descendant.wait().expect("wait for effect descendant");
    assert!(status.success(), "effect descendant failed: {status}");
    fs::write(external_complete_marker(&handshake_root), b"complete\n")
        .expect("signal external command completion");
}

fn run_descendant_role() {
    let operation = required_operation();
    let handshake_root = required_path(HANDSHAKE_ENV);
    fs::write(handshake_root.join("descendant-ready"), b"ready\n")
        .expect("signal effect descendant readiness");
    super::wait_for_path(
        &effect_release_marker(&handshake_root),
        "external effect release",
    );
    commit_external_effect(&operation, &handshake_root);
}

struct InFlightCommandExecutor {
    operation: BackupOperationKind,
    handshake_root: PathBuf,
    delegate: FakeBackupRunnerExecutor,
}

impl InFlightCommandExecutor {
    fn execute_command(
        &self,
        operation: BackupOperationKind,
        command_lifetime: CommandLifetimeHandle,
        artifact_path: Option<&Path>,
    ) -> Result<(), BackupRunnerCommandError> {
        assert_eq!(self.operation, operation);
        let mut command = Command::new(std::env::current_exe().map_err(|error| {
            BackupRunnerCommandError::failed("test-command", error.to_string())
        })?);
        command
            .args([
                "--exact",
                "persistence::tests::operational_readiness::command_in_flight::orphaned_command_tree_blocks_restart_for_every_mutating_backup_operation",
                "--nocapture",
            ])
            .env(ROLE_ENV, "external")
            .env(OPERATION_ENV, backup_operation_label(&operation))
            .env(HANDSHAKE_ENV, &self.handshake_root)
            .env(COMMAND_FD_ENV, command_lifetime.raw_fd().to_string());
        if let Some(artifact_path) = artifact_path {
            command.env(ARTIFACT_PATH_ENV, artifact_path);
        }
        inherit_command_descriptor(&mut command, command_lifetime);
        let status = command
            .status()
            .map_err(|error| BackupRunnerCommandError::failed("test-command", error.to_string()))?;
        if !status.success() {
            return Err(BackupRunnerCommandError::failed(
                "test-command",
                format!("external command failed: {status}"),
            ));
        }
        Ok(())
    }
}

impl BackupRunnerExecutor for InFlightCommandExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        self.delegate
            .preflight_receipts(plan, preflight_id, validated_at, expires_at)
    }

    fn canister_status(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError> {
        self.delegate.canister_status(canister_id)
    }

    fn snapshot_inventory(
        &mut self,
        canister_id: &str,
    ) -> Result<Vec<BackupRunnerSnapshot>, BackupRunnerCommandError> {
        self.delegate.snapshot_inventory(canister_id)
    }

    fn stop_canister(
        &mut self,
        _canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.execute_command(BackupOperationKind::Stop, command_lifetime, None)
    }

    fn start_canister(
        &mut self,
        _canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.execute_command(BackupOperationKind::Start, command_lifetime, None)
    }

    fn create_snapshot(
        &mut self,
        _canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshot, BackupRunnerCommandError> {
        self.execute_command(BackupOperationKind::CreateSnapshot, command_lifetime, None)?;
        Ok(snapshot())
    }

    fn download_snapshot(
        &mut self,
        _canister_id: &str,
        _snapshot_id: &str,
        artifact_path: &Path,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.execute_command(
            BackupOperationKind::DownloadSnapshot,
            command_lifetime,
            Some(artifact_path),
        )
    }
}

fn commit_external_effect(operation: &BackupOperationKind, handshake_root: &Path) {
    let marker_value = match operation {
        BackupOperationKind::Stop => "Stopped",
        BackupOperationKind::CreateSnapshot => SNAPSHOT_ID,
        BackupOperationKind::Start => "Running",
        BackupOperationKind::DownloadSnapshot => {
            let artifact_path = required_path(ARTIFACT_PATH_ENV);
            fs::create_dir_all(&artifact_path).expect("create descendant artifact directory");
            write_synchronized(
                &artifact_path.join("snapshot.bin"),
                b"orphaned command bytes",
            );
            File::open(&artifact_path)
                .and_then(|directory| directory.sync_all())
                .expect("sync descendant artifact directory");
            "Downloaded"
        }
        operation => panic!("unsupported in-flight effect: {operation:?}"),
    };
    write_synchronized(&effect_marker(handshake_root), marker_value.as_bytes());
}

fn recovery_executor(
    operation: &BackupOperationKind,
    target: &crate::execution::BackupExecutionJournalOperation,
) -> FakeBackupRunnerExecutor {
    let mut executor = FakeBackupRunnerExecutor::default();
    let target_canister = target
        .target_canister_id
        .clone()
        .expect("mutating operation target");
    match operation {
        BackupOperationKind::Stop => {
            executor
                .canister_statuses
                .insert(target_canister, BackupRunnerCanisterStatus::Stopped);
        }
        BackupOperationKind::CreateSnapshot => {
            executor.snapshots.insert(target_canister, vec![snapshot()]);
        }
        BackupOperationKind::Start => {
            executor
                .canister_statuses
                .insert(target_canister, BackupRunnerCanisterStatus::Running);
        }
        BackupOperationKind::DownloadSnapshot => {}
        operation => panic!("unsupported recovery operation: {operation:?}"),
    }
    executor
}

fn assert_target_recovery(
    operation: &BackupOperationKind,
    target_sequence: usize,
    layout: &BackupLayout,
    response: &BackupRunResponse,
    commands: &[String],
) {
    assert_eq!(response.executed_operation_count, 1);
    let journal = layout
        .read_execution_journal()
        .expect("read recovered command-in-flight journal");
    assert_eq!(
        journal.operations[target_sequence].state,
        BackupExecutionOperationState::Completed
    );
    let receipts = journal
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == target_sequence)
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 1);
    assert_eq!(
        receipts[0].outcome,
        BackupExecutionOperationReceiptOutcome::Completed
    );

    match operation {
        BackupOperationKind::Stop => {
            assert!(
                commands
                    .iter()
                    .any(|command| command.starts_with("status:"))
            );
            assert!(commands.iter().all(|command| !command.starts_with("stop:")));
        }
        BackupOperationKind::CreateSnapshot => {
            assert!(
                commands
                    .iter()
                    .any(|command| command.starts_with("snapshot-list:"))
            );
            assert!(
                commands
                    .iter()
                    .all(|command| !command.starts_with("snapshot:"))
            );
        }
        BackupOperationKind::Start => {
            assert!(
                commands
                    .iter()
                    .any(|command| command.starts_with("status:"))
            );
            assert!(
                commands
                    .iter()
                    .all(|command| !command.starts_with("start:"))
            );
        }
        BackupOperationKind::DownloadSnapshot => {
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.starts_with("download:"))
                    .count(),
                1
            );
        }
        operation => panic!("unsupported recovered operation: {operation:?}"),
    }
}

fn wait_for_command_quiescence(layout: &BackupLayout, sequence: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match CommandLifetimeLock::acquire(&layout.execution_journal_path(), sequence) {
            Ok(lock) => {
                lock.finish().expect("finish quiescence probe");
                return;
            }
            Err(CommandLifetimeLockError::InFlight { .. }) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("command tree did not become quiescent: {error:?}"),
        }
    }
}

fn inherit_command_descriptor(command: &mut Command, handle: CommandLifetimeHandle) {
    let raw_fd = handle.raw_fd();
    // SAFETY: the runner-owned command lock keeps the descriptor valid through
    // spawn, and this child setup performs only fcntl on that descriptor.
    unsafe {
        command.pre_exec(move || {
            // SAFETY: the runner retains the descriptor until command return.
            let fd = BorrowedFd::borrow_raw(raw_fd);
            let mut flags = fcntl_getfd(fd).map_err(errno_to_io)?;
            flags.remove(FdFlags::CLOEXEC);
            fcntl_setfd(fd, flags).map_err(errno_to_io)
        });
    }
}

fn errno_to_io(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}

fn snapshot() -> BackupRunnerSnapshot {
    BackupRunnerSnapshot {
        snapshot_id: SNAPSHOT_ID.to_string(),
        taken_at_timestamp: Some(SNAPSHOT_TAKEN_AT),
        total_size_bytes: Some(SNAPSHOT_SIZE),
    }
}

fn required_operation() -> BackupOperationKind {
    let label = std::env::var(OPERATION_ENV).expect("command-in-flight operation");
    [
        BackupOperationKind::Stop,
        BackupOperationKind::CreateSnapshot,
        BackupOperationKind::Start,
        BackupOperationKind::DownloadSnapshot,
    ]
    .into_iter()
    .find(|operation| backup_operation_label(operation) == label)
    .expect("supported command-in-flight operation")
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("required environment {name}")))
}

fn write_synchronized(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).expect("create synchronized test file");
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .expect("write synchronized test file");
}

fn direct_release_marker(root: &Path) -> PathBuf {
    root.join("direct-release")
}

fn direct_closed_marker(root: &Path) -> PathBuf {
    root.join("direct-closed")
}

fn effect_release_marker(root: &Path) -> PathBuf {
    root.join("effect-release")
}

fn effect_marker(root: &Path) -> PathBuf {
    root.join("effect-committed")
}

fn external_complete_marker(root: &Path) -> PathBuf {
    root.join("external-complete")
}
