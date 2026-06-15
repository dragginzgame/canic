use std::path::Path;

use super::{
    command::command_display,
    error::IcpCommandError,
    model::{IcpCli, IcpSnapshotCreateReceipt, IcpSnapshotUploadReceipt},
    run::{run_json, run_status},
};

impl IcpCli {
    /// Create one canister snapshot and return the ICP CLI JSON receipt.
    pub fn snapshot_create_receipt(
        &self,
        canister: &str,
    ) -> Result<IcpSnapshotCreateReceipt, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
        command.arg("--json");
        self.add_target_args(&mut command);
        run_json(&mut command)
    }

    /// Create one canister snapshot and resolve the resulting snapshot id.
    pub fn snapshot_create_id(&self, canister: &str) -> Result<String, IcpCommandError> {
        Ok(self.snapshot_create_receipt(canister)?.snapshot_id)
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

    /// Upload one snapshot artifact and return the uploaded snapshot id.
    pub fn snapshot_upload(
        &self,
        canister: &str,
        artifact_path: &Path,
    ) -> Result<String, IcpCommandError> {
        Ok(self
            .snapshot_upload_receipt(canister, artifact_path)?
            .snapshot_id)
    }

    /// Upload one snapshot artifact and return the ICP CLI JSON receipt.
    pub fn snapshot_upload_receipt(
        &self,
        canister: &str,
        artifact_path: &Path,
    ) -> Result<IcpSnapshotUploadReceipt, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "upload", canister, "--input"]);
        command.arg(artifact_path);
        command.arg("--resume");
        command.arg("--json");
        self.add_target_args(&mut command);
        run_json(&mut command)
    }

    /// Restore one uploaded snapshot onto a canister.
    pub fn snapshot_restore(
        &self,
        canister: &str,
        snapshot_id: &str,
    ) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "restore", canister, snapshot_id]);
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

    /// Render a dry-run snapshot-upload command.
    #[must_use]
    pub fn snapshot_upload_display(&self, canister: &str, artifact_path: &Path) -> String {
        let mut command = self.canister_command();
        command.args(["snapshot", "upload", canister, "--input"]);
        command.arg(artifact_path);
        command.arg("--resume");
        command.arg("--json");
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run snapshot-restore command.
    #[must_use]
    pub fn snapshot_restore_display(&self, canister: &str, snapshot_id: &str) -> String {
        let mut command = self.canister_command();
        command.args(["snapshot", "restore", canister, snapshot_id]);
        self.add_target_args(&mut command);
        command_display(&command)
    }
}

/// Parse a likely snapshot id from `icp canister snapshot create` output.
#[must_use]
pub fn parse_snapshot_id(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if is_snapshot_id_token(trimmed) {
        return Some(trimmed.to_string());
    }

    output
        .lines()
        .flat_map(|line| {
            line.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ':' | ','))
        })
        .find(|part| is_snapshot_id_token(part))
        .map(str::to_string)
}

// ICP snapshot ids are rendered as even-length hexadecimal blobs.
fn is_snapshot_id_token(value: &str) -> bool {
    !value.is_empty()
        && value.len().is_multiple_of(2)
        && value.chars().all(|c| c.is_ascii_hexdigit())
}
