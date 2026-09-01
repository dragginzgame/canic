//! Module: backup::create::executor
//!
//! Responsibility: execute backup create runner operations through the ICP CLI.
//! Does not own: backup planning, layout persistence, or command option parsing.
//! Boundary: maps runner preflight and snapshot operations onto host ICP commands.

#[cfg(test)]
mod tests;

use super::super::options::BackupCreateOptions;
use canic_backup::{
    persistence::CommandLifetimeHandle,
    plan::{BackupExecutionPreflightReceipts, BackupPlan},
    runner::{
        BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerExecutor,
        BackupRunnerSnapshot,
    },
};
use canic_host::icp::{IcpCanisterStatusReport, IcpCli, IcpCommandError};
use std::path::{Path, PathBuf};

///
/// BackupIcpRunnerExecutor
///

pub(super) struct BackupIcpRunnerExecutor {
    icp: IcpCli,
}

impl BackupIcpRunnerExecutor {
    pub(super) fn new(options: &BackupCreateOptions, icp_root: PathBuf) -> Self {
        Self {
            icp: IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root),
        }
    }

    fn command_icp(&self, command_lifetime: CommandLifetimeHandle) -> IcpCli {
        self.icp
            .clone()
            .with_inherited_fd(Some(command_lifetime.raw_fd()))
    }
}

impl BackupRunnerExecutor for BackupIcpRunnerExecutor {
    fn preflight_receipts(
        &mut self,
        _plan: &BackupPlan,
        _preflight_id: &str,
        _validated_at: &str,
        _expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        Err(component_topology_preflight_unavailable())
    }

    fn canister_status(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError> {
        let report = self
            .icp
            .canister_status_report(canister_id)
            .map_err(runner_icp_error)?;
        runner_canister_status(canister_id, &report)
    }

    fn snapshot_inventory(
        &mut self,
        canister_id: &str,
    ) -> Result<Vec<BackupRunnerSnapshot>, BackupRunnerCommandError> {
        self.icp
            .snapshot_inventory(canister_id)
            .map(|snapshots| {
                snapshots
                    .into_iter()
                    .map(|snapshot| BackupRunnerSnapshot {
                        snapshot_id: snapshot.snapshot_id,
                        taken_at_timestamp: snapshot.taken_at_timestamp,
                        total_size_bytes: snapshot.total_size_bytes,
                    })
                    .collect()
            })
            .map_err(runner_icp_error)
    }

    fn stop_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.command_icp(command_lifetime)
            .stop_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn start_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.command_icp(command_lifetime)
            .start_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshot, BackupRunnerCommandError> {
        self.command_icp(command_lifetime)
            .snapshot_create(canister_id)
            .map(|snapshot| BackupRunnerSnapshot {
                snapshot_id: snapshot.snapshot_id,
                taken_at_timestamp: snapshot.taken_at_timestamp,
                total_size_bytes: snapshot.total_size_bytes,
            })
            .map_err(runner_icp_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.command_icp(command_lifetime)
            .snapshot_download(canister_id, snapshot_id, artifact_path)
            .map_err(runner_icp_error)
    }
}

fn component_topology_preflight_unavailable() -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed(
        "preflight",
        "Coordinator-backed Component Registry preflight is not implemented",
    )
}

fn runner_icp_error(error: IcpCommandError) -> BackupRunnerCommandError {
    BackupRunnerCommandError::failed("icp", error.to_string())
}

fn runner_canister_status(
    expected_canister_id: &str,
    report: &IcpCanisterStatusReport,
) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError> {
    if report.id != expected_canister_id {
        return Err(BackupRunnerCommandError::failed(
            "icp-status",
            format!(
                "icp canister status returned id {} for expected canister {expected_canister_id}",
                report.id
            ),
        ));
    }
    match report.status.as_deref() {
        Some("Running") => Ok(BackupRunnerCanisterStatus::Running),
        Some("Stopped") => Ok(BackupRunnerCanisterStatus::Stopped),
        Some("Stopping") => Ok(BackupRunnerCanisterStatus::Stopping),
        Some(status) => Err(BackupRunnerCommandError::failed(
            "icp-status",
            format!("unsupported canister status {status}"),
        )),
        None => Err(BackupRunnerCommandError::failed(
            "icp-status",
            "canister lifecycle status is unavailable",
        )),
    }
}
