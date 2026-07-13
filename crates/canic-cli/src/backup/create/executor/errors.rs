//! Module: backup::create::executor::errors
//!
//! Responsibility: map ICP and preflight errors for backup execution.
//! Does not own: command execution or receipt construction.
//! Boundary: error translation into backup command and runner errors.

use canic_backup::runner::BackupRunnerCommandError;
use canic_host::icp::IcpCommandError;

pub(super) fn preflight_error(error: impl std::error::Error) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("preflight", error.to_string())
}

pub(super) fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}
