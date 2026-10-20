//! Module: restore::tests::operational_readiness
//!
//! Responsibility: prove a real upload effect can be reconciled after process death.
//! Does not own: production failpoints, restore planning, or disposable network setup.
//! Boundary: wraps the maintained host command executor only in this ignored environment test.

use super::*;
use canic_backup::{
    persistence::CommandLifetimeLock,
    restore::{
        RestoreApplyCommandConfig, RestoreApplyOperationState, RestoreApplyRunnerCommand,
        RestoreRunnerCommandExecutor, RestoreRunnerCommandOutput, RestoreRunnerConfig,
        restore_run_execute_with_executor,
    },
};
use ic_host_process::testing::crash_barrier::{
    hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const JOURNAL_ENV: &str = "CANIC_TEST_REAL_RESTORE_JOURNAL";
const ENVIRONMENT_ENV: &str = "CANIC_TEST_REAL_RESTORE_ENVIRONMENT";
const CHILD_ENV: &str = "CANIC_TEST_REAL_RESTORE_UPLOAD_CHILD";
const TEST_NAME: &str =
    "restore::tests::operational_readiness::real_upload_effect_survives_process_death";

#[test]
#[ignore = "requires an explicitly disposable ICP environment and prepared restore journal"]
fn real_upload_effect_survives_process_death() {
    let journal_path = PathBuf::from(
        std::env::var_os(JOURNAL_ENV).expect("prepared restore journal path is required"),
    );
    let environment = std::env::var(ENVIRONMENT_ENV).unwrap_or_else(|_| "local".to_string());
    let handshake = journal_path.with_extension("real-upload-crash");

    if std::env::var_os(CHILD_ENV).is_some() {
        let mut executor = RealRestoreExecutor::with_barrier(handshake);
        restore_run_execute_with_executor(
            &runner_config(journal_path, environment, Some(1)),
            &mut executor,
        )
        .expect("real upload crash child must remain at its armed barrier");
        panic!("real upload crash child passed its armed barrier");
    }

    let initial = read_journal(&journal_path);
    assert_eq!(
        initial.operations[0].state,
        RestoreApplyOperationState::Ready
    );
    fs::create_dir_all(&handshake).expect("create real upload handshake directory");
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args(["--exact", TEST_NAME, "--ignored", "--nocapture"])
        .env(JOURNAL_ENV, &journal_path)
        .env(ENVIRONMENT_ENV, &environment)
        .env(CHILD_ENV, "1")
        .spawn()
        .expect("spawn real upload crash child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake, std::time::Duration::from_secs(30))
        .expect("complete crash barrier handshake");
    let interrupted = read_journal(&journal_path);
    assert_eq!(
        interrupted.operations[0].state,
        RestoreApplyOperationState::Pending
    );
    assert!(interrupted.operations[0].snapshot_ids_before.is_some());
    assert!(
        interrupted
            .operation_receipts
            .iter()
            .all(|receipt| receipt.sequence != 0)
    );

    let mut executor = RealRestoreExecutor::default();
    let response = restore_run_execute_with_executor(
        &runner_config(journal_path.clone(), environment, Some(1)),
        &mut executor,
    )
    .expect("reconcile the committed upload from authoritative inventory");
    let recovered = read_journal(&journal_path);

    assert_eq!(response.executed_operation_count, Some(1));
    assert_eq!(executor.upload_commands, 0);
    assert_eq!(executor.inventory_commands, 1);
    assert_eq!(
        recovered.operations[0].state,
        RestoreApplyOperationState::Completed
    );
    assert_eq!(
        recovered
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == 0)
            .count(),
        1
    );
    fs::remove_dir_all(handshake).expect("remove real upload handshake directory");
}

#[derive(Default)]
struct RealRestoreExecutor {
    barrier: Option<PathBuf>,
    upload_commands: usize,
    inventory_commands: usize,
}

impl RealRestoreExecutor {
    fn with_barrier(barrier: PathBuf) -> Self {
        Self {
            barrier: Some(barrier),
            ..Self::default()
        }
    }
}

impl RestoreRunnerCommandExecutor for RealRestoreExecutor {
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
        mut command_lifetime: Option<&mut CommandLifetimeLock>,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        let is_upload = command.args.get(1).map(String::as_str) == Some("snapshot")
            && command.args.get(2).map(String::as_str) == Some("upload");
        let is_inventory = command.args.get(1).map(String::as_str) == Some("snapshot")
            && command.args.get(2).map(String::as_str) == Some("list");
        self.upload_commands += usize::from(is_upload);
        self.inventory_commands += usize::from(is_inventory);
        let output = icp::run_raw_output(
            &command.program,
            &command.args,
            restore_output_limits(),
            |command| spawn_restore_command(&mut command_lifetime, command),
        )?;
        if is_upload
            && output.success
            && let Some(barrier) = &self.barrier
        {
            hold_at_acknowledged_barrier(barrier, std::time::Duration::from_secs(30));
        }
        Ok(RestoreRunnerCommandOutput {
            success: output.success,
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

fn runner_config(
    journal: PathBuf,
    environment: String,
    max_steps: Option<usize>,
) -> RestoreRunnerConfig {
    RestoreRunnerConfig {
        journal,
        command: RestoreApplyCommandConfig {
            program: "icp".to_string(),
            environment: Some(environment),
        },
        max_steps,
        updated_at: None,
    }
}

fn read_journal(path: &Path) -> RestoreApplyJournal {
    serde_json::from_slice(&fs::read(path).expect("read restore apply journal"))
        .expect("decode restore apply journal")
}
