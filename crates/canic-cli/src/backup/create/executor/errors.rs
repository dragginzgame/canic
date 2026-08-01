//! Module: backup::create::executor::errors
//!
//! Responsibility: map ICP errors for backup execution.
//! Does not own: command execution or receipt construction.
//! Boundary: error translation into backup command and runner errors.

use canic_backup::runner::BackupRunnerCommandError;
use canic_host::icp::IcpCommandError;

pub(super) fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}
