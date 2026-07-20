//! Module: backup::create::executor
//!
//! Responsibility: execute backup create runner operations through the ICP CLI.
//! Does not own: backup planning, layout persistence, or command option parsing.
//! Boundary: maps runner preflight and snapshot operations onto host ICP commands.

mod errors;
mod preflight;
mod registry;
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
use canic_host::icp::{IcpCanisterStatusReport, IcpCli};
use std::path::{Path, PathBuf};

use errors::runner_icp_error;
use preflight::build_preflight_receipts;

///
/// BackupIcpRunnerExecutor
///

pub(super) struct BackupIcpRunnerExecutor {
    options: BackupCreateOptions,
    icp_root: PathBuf,
    icp: IcpCli,
}

impl BackupIcpRunnerExecutor {
    pub(super) fn new(options: &BackupCreateOptions, icp_root: PathBuf) -> Self {
        Self {
            options: options.clone(),
            icp: IcpCli::new(&options.icp, Some(options.environment.clone())).with_cwd(&icp_root),
            icp_root,
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
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        build_preflight_receipts(
            &self.icp,
            &self.options,
            &self.icp_root,
            plan,
            preflight_id,
            validated_at,
            expires_at,
        )
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
    match report.status.as_str() {
        "Running" => Ok(BackupRunnerCanisterStatus::Running),
        "Stopped" => Ok(BackupRunnerCanisterStatus::Stopped),
        "Stopping" => Ok(BackupRunnerCanisterStatus::Stopping),
        status => Err(BackupRunnerCommandError::failed(
            "icp-status",
            format!("unsupported canister status {status}"),
        )),
    }
}
