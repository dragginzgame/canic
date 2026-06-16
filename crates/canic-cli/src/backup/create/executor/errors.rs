//! Module: backup::create::executor::errors
//!
//! Responsibility: map ICP and preflight errors for backup execution.
//! Does not own: command execution or receipt construction.
//! Boundary: error translation into backup command and runner errors.

use crate::backup::BackupCommandError;
use canic_backup::runner::BackupRunnerCommandError;
use canic_host::icp::IcpCommandError;

pub(super) fn preflight_error(error: impl std::error::Error) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("preflight", error.to_string())
}

pub(super) fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}

pub(super) fn backup_icp_error(error: IcpCommandError) -> BackupCommandError {
    match error {
        IcpCommandError::Io(err) => BackupCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            BackupCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => BackupCommandError::IcpFailed {
            command,
            stderr: output,
        },
        error @ (IcpCommandError::MissingCli { .. }
        | IcpCommandError::IncompatibleCliVersion { .. }) => BackupCommandError::IcpFailed {
            command: "icp --version".to_string(),
            stderr: error.to_string(),
        },
        IcpCommandError::SnapshotIdUnavailable { output } => BackupCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}
