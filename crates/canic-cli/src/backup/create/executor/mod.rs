//! Module: backup::create::executor
//!
//! Responsibility: execute backup create runner operations through the ICP CLI.
//! Does not own: backup planning, layout persistence, or command option parsing.
//! Boundary: maps runner preflight and snapshot operations onto host ICP commands.

mod errors;
mod preflight;
mod registry;

use super::super::options::BackupCreateOptions;
use canic_backup::{
    plan::{BackupExecutionPreflightReceipts, BackupPlan},
    runner::{BackupRunnerCommandError, BackupRunnerExecutor, BackupRunnerSnapshotReceipt},
};
use canic_host::icp::IcpCli;
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
            icp: IcpCli::new(&options.icp, None, Some(options.network.clone())).with_cwd(&icp_root),
            icp_root,
        }
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

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .stop_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .start_canister(canister_id)
            .map_err(runner_icp_error)
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError> {
        self.icp
            .snapshot_create_receipt(canister_id)
            .map(|receipt| BackupRunnerSnapshotReceipt {
                snapshot_id: receipt.snapshot_id,
                taken_at_timestamp: receipt.taken_at_timestamp,
                total_size_bytes: receipt.total_size_bytes,
            })
            .map_err(runner_icp_error)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError> {
        self.icp
            .snapshot_download(canister_id, snapshot_id, artifact_path)
            .map_err(runner_icp_error)
    }
}
