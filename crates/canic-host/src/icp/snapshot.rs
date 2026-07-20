use std::path::Path;

use serde::Deserialize;

use super::{
    command::command_display,
    error::IcpCommandError,
    model::{IcpCli, IcpSnapshot},
    run::{run_json, run_status},
};

impl IcpCli {
    /// Create one canister snapshot and return its typed metadata.
    pub fn snapshot_create(&self, canister: &str) -> Result<IcpSnapshot, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
        command.arg("--json");
        self.add_target_args(&mut command);
        run_json(&mut command)
    }

    /// Create one canister snapshot and resolve the resulting snapshot id.
    pub fn snapshot_create_id(&self, canister: &str) -> Result<String, IcpCommandError> {
        Ok(self.snapshot_create(canister)?.snapshot_id)
    }

    /// List the authoritative snapshots currently retained for one canister.
    pub fn snapshot_inventory(&self, canister: &str) -> Result<Vec<IcpSnapshot>, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "list", canister]);
        command.arg("--json");
        self.add_target_args(&mut command);
        run_json::<IcpSnapshotInventory>(&mut command).map(|inventory| inventory.snapshots)
    }

    /// Download one canister snapshot into an artifact directory.
    pub fn snapshot_download(
        &self,
        canister: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "download", canister, snapshot_id, "--output"]);
        command.arg(artifact_path);
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Render a dry-run snapshot-create command.
    #[must_use]
    pub fn snapshot_create_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
        command.arg("--json");
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run snapshot-download command.
    #[must_use]
    pub fn snapshot_download_display(
        &self,
        canister: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String {
        let mut command = self.canister_command();
        command.args(["snapshot", "download", canister, snapshot_id, "--output"]);
        command.arg(artifact_path);
        self.add_target_args(&mut command);
        command_display(&command)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IcpSnapshotInventory {
    pub(super) snapshots: Vec<IcpSnapshot>,
}
