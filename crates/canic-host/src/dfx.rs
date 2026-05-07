use crate::default_network;
use std::{error::Error, fmt, path::Path, process::Command};

///
/// DfxRawOutput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfxRawOutput {
    pub success: bool,
    pub status: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

///
/// DfxCommandError
///

#[derive(Debug)]
pub enum DfxCommandError {
    Io(std::io::Error),
    Failed { command: String, stderr: String },
    SnapshotIdUnavailable { output: String },
}

impl fmt::Display for DfxCommandError {
    // Render dfx command failures with the command line and captured diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Failed { command, stderr } => {
                write!(formatter, "dfx command failed: {command}\n{stderr}")
            }
            Self::SnapshotIdUnavailable { output } => {
                write!(
                    formatter,
                    "could not parse snapshot id from dfx output: {output}"
                )
            }
        }
    }
}

impl Error for DfxCommandError {
    // Preserve the underlying I/O error as the source when command execution fails locally.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Failed { .. } | Self::SnapshotIdUnavailable { .. } => None,
        }
    }
}

impl From<std::io::Error> for DfxCommandError {
    // Convert process-spawn failures into the shared dfx command error type.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

///
/// Dfx
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dfx {
    executable: String,
    network: Option<String>,
}

impl Dfx {
    /// Build a dfx command context from an executable path and optional network.
    #[must_use]
    pub fn new(executable: impl Into<String>, network: Option<String>) -> Self {
        Self {
            executable: executable.into(),
            network,
        }
    }

    /// Return the optional network name carried by this command context.
    #[must_use]
    pub fn network(&self) -> Option<&str> {
        self.network.as_deref()
    }

    /// Build a `dfx canister ...` command with optional network args applied.
    #[must_use]
    pub fn canister_command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command.arg("canister");
        add_network_args(&mut command, self.network());
        command
    }

    /// Ping the selected dfx network.
    pub fn ping(&self) -> Result<(), DfxCommandError> {
        let mut command = Command::new(&self.executable);
        command.arg("ping");
        let network = self.network().map_or_else(default_network, str::to_string);
        command.arg(network);
        run_status(&mut command)
    }

    /// Resolve one project canister id, returning `None` when the id is absent.
    pub fn canister_id_optional(&self, name: &str) -> Result<Option<String>, DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["id", name]);
        match run_output(&mut command) {
            Ok(output) => Ok(Some(output)),
            Err(DfxCommandError::Failed { command, stderr }) if canister_id_missing(&stderr) => {
                let _ = command;
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    /// Resolve one project canister id.
    pub fn canister_id(&self, name: &str) -> Result<String, DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["id", name]);
        run_output(&mut command)
    }

    /// Call one canister method with optional dfx JSON output.
    pub fn canister_call_output(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        if let Some(output) = output {
            command.args(["--output", output]);
        }
        run_output(&mut command)
    }

    /// List snapshot ids for one canister.
    pub fn snapshot_list(&self, canister: &str) -> Result<String, DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "list", canister]);
        run_output(&mut command)
    }

    /// Create one canister snapshot and return combined stdout/stderr text.
    pub fn snapshot_create(&self, canister: &str) -> Result<String, DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
        run_output_with_stderr(&mut command)
    }

    /// Create one canister snapshot and resolve the resulting snapshot id.
    pub fn snapshot_create_id(&self, canister: &str) -> Result<String, DfxCommandError> {
        let before = self.snapshot_list_ids(canister)?;
        let output = self.snapshot_create(canister)?;
        if let Some(snapshot_id) = parse_snapshot_id(&output) {
            return Ok(snapshot_id);
        }

        let before = before
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut new_ids = self
            .snapshot_list_ids(canister)?
            .into_iter()
            .filter(|snapshot_id| !before.contains(snapshot_id))
            .collect::<Vec<_>>();
        if new_ids.len() == 1 {
            Ok(new_ids.remove(0))
        } else {
            Err(DfxCommandError::SnapshotIdUnavailable { output })
        }
    }

    /// List snapshot ids for one canister as parsed dfx identifiers.
    pub fn snapshot_list_ids(&self, canister: &str) -> Result<Vec<String>, DfxCommandError> {
        let output = self.snapshot_list(canister)?;
        Ok(parse_snapshot_list_ids(&output))
    }

    /// Stop one canister.
    pub fn stop_canister(&self, canister: &str) -> Result<(), DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        run_status(&mut command)
    }

    /// Start one canister.
    pub fn start_canister(&self, canister: &str) -> Result<(), DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        run_status(&mut command)
    }

    /// Download one canister snapshot into an artifact directory.
    pub fn snapshot_download(
        &self,
        canister: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), DfxCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "download", canister, snapshot_id, "--dir"]);
        command.arg(artifact_path);
        run_status(&mut command)
    }

    /// Render a dry-run snapshot-create command.
    #[must_use]
    pub fn snapshot_create_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
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
        command.args(["snapshot", "download", canister, snapshot_id, "--dir"]);
        command.arg(artifact_path);
        command_display(&command)
    }

    /// Render a dry-run stop command.
    #[must_use]
    pub fn stop_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        command_display(&command)
    }

    /// Render a dry-run start command.
    #[must_use]
    pub fn start_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        command_display(&command)
    }
}

/// Add optional `--network` arguments after `dfx canister`.
pub fn add_network_args(command: &mut Command, network: Option<&str>) {
    if let Some(network) = network {
        command.args(["--network", network]);
    }
}

/// Execute a command and capture trimmed stdout.
pub fn run_output(command: &mut Command) -> Result<String, DfxCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(DfxCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and capture stdout plus stderr on success.
pub fn run_output_with_stderr(command: &mut Command) -> Result<String, DfxCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(text.trim().to_string())
    } else {
        Err(DfxCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and require a successful status.
pub fn run_status(command: &mut Command) -> Result<(), DfxCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(DfxCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a rendered dfx-compatible command and return raw process output.
pub fn run_raw_output(program: &str, args: &[String]) -> Result<DfxRawOutput, std::io::Error> {
    let output = Command::new(program).args(args).output()?;
    Ok(DfxRawOutput {
        success: output.status.success(),
        status: exit_status_label(output.status),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Render a command for diagnostics and dry-run previews.
#[must_use]
pub fn command_display(command: &Command) -> String {
    let mut parts = vec![command.get_program().to_string_lossy().to_string()];
    parts.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string()),
    );
    parts.join(" ")
}

/// Detect dfx's missing-canister-id diagnostic.
#[must_use]
pub fn canister_id_missing(stderr: &str) -> bool {
    stderr.contains("Cannot find canister id")
}

/// Parse a likely snapshot id from `dfx canister snapshot create` output.
#[must_use]
pub fn parse_snapshot_id(output: &str) -> Option<String> {
    output
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ':' | ','))
        .filter(|part| !part.is_empty())
        .rev()
        .find(|part| {
            part.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
        .map(str::to_string)
}

/// Parse `dfx canister snapshot list` output into snapshot ids.
#[must_use]
pub fn parse_snapshot_list_ids(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            line.split_once(':')
                .map(|(snapshot_id, _)| snapshot_id.trim())
        })
        .filter(|snapshot_id| !snapshot_id.is_empty())
        .map(str::to_string)
        .collect()
}

// Prefer stderr, but keep stdout diagnostics for dfx commands that report there.
fn command_stderr(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        stderr.to_string()
    }
}

// Render process exit status without relying on platform-specific internals.
fn exit_status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure snapshot ids can be extracted from common dfx create output.
    #[test]
    fn parses_snapshot_id_from_output() {
        let snapshot_id = parse_snapshot_id("Created snapshot: snap_abc-123\n");

        assert_eq!(snapshot_id.as_deref(), Some("snap_abc-123"));
    }

    // Ensure snapshot list output is reduced to ordered snapshot ids.
    #[test]
    fn parses_snapshot_ids_from_list_output() {
        let snapshot_ids = parse_snapshot_list_ids(
            "0000000000000000ffffffffff9000050101: size 10\n\
             0000000000000000ffffffffff9000050102: size 12\n",
        );

        assert_eq!(
            snapshot_ids,
            vec![
                "0000000000000000ffffffffff9000050101",
                "0000000000000000ffffffffff9000050102"
            ]
        );
    }
}
