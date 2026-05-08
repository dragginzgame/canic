use std::{error::Error, fmt, path::Path, process::Command};

///
/// IcpRawOutput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRawOutput {
    pub success: bool,
    pub status: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

///
/// IcpCommandError
///

#[derive(Debug)]
pub enum IcpCommandError {
    Io(std::io::Error),
    Failed { command: String, stderr: String },
    SnapshotIdUnavailable { output: String },
}

impl fmt::Display for IcpCommandError {
    // Render ICP CLI command failures with the command line and captured diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Failed { command, stderr } => {
                write!(formatter, "icp command failed: {command}\n{stderr}")
            }
            Self::SnapshotIdUnavailable { output } => {
                write!(
                    formatter,
                    "could not parse snapshot id from icp output: {output}"
                )
            }
        }
    }
}

impl Error for IcpCommandError {
    // Preserve the underlying I/O error as the source when command execution fails locally.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Failed { .. } | Self::SnapshotIdUnavailable { .. } => None,
        }
    }
}

impl From<std::io::Error> for IcpCommandError {
    // Convert process-spawn failures into the shared ICP CLI command error type.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

///
/// IcpCli
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpCli {
    executable: String,
    environment: Option<String>,
    network: Option<String>,
}

impl IcpCli {
    /// Build an ICP CLI command context from an executable path and optional target.
    #[must_use]
    pub fn new(
        executable: impl Into<String>,
        environment: Option<String>,
        network: Option<String>,
    ) -> Self {
        Self {
            executable: executable.into(),
            environment,
            network,
        }
    }

    /// Return the optional ICP environment name carried by this command context.
    #[must_use]
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    /// Return the optional direct network name carried by this command context.
    #[must_use]
    pub fn network(&self) -> Option<&str> {
        self.network.as_deref()
    }

    /// Build a base ICP CLI command from this context.
    #[must_use]
    pub fn command(&self) -> Command {
        Command::new(&self.executable)
    }

    /// Build a base ICP CLI command rooted at one workspace directory.
    #[must_use]
    pub fn command_in(&self, cwd: &Path) -> Command {
        let mut command = self.command();
        command.current_dir(cwd);
        command
    }

    /// Build an `icp canister ...` command with optional environment args applied.
    #[must_use]
    pub fn canister_command(&self) -> Command {
        let mut command = self.command();
        command.arg("canister");
        command
    }

    /// Resolve the installed ICP CLI version.
    pub fn version(&self) -> Result<String, IcpCommandError> {
        let mut command = self.command();
        command.arg("--version");
        run_output(&mut command)
    }

    /// Call one canister method with optional JSON output.
    pub fn canister_call_output(
        &self,
        canister: &str,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["call", canister, method]);
        command.arg("()");
        if let Some(output) = output {
            add_output_arg(&mut command, output);
        }
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Return one canister status report.
    pub fn canister_status(&self, canister: &str) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["status", canister]);
        self.add_target_args(&mut command);
        run_output(&mut command)
    }

    /// Create one canister snapshot and return combined stdout/stderr text.
    pub fn snapshot_create(&self, canister: &str) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "create", canister]);
        self.add_target_args(&mut command);
        run_output_with_stderr(&mut command)
    }

    /// Create one canister snapshot and resolve the resulting snapshot id.
    pub fn snapshot_create_id(&self, canister: &str) -> Result<String, IcpCommandError> {
        let output = self.snapshot_create(canister)?;
        parse_snapshot_id(&output).ok_or(IcpCommandError::SnapshotIdUnavailable { output })
    }

    /// Stop one canister.
    pub fn stop_canister(&self, canister: &str) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Start one canister.
    pub fn start_canister(&self, canister: &str) -> Result<(), IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        self.add_target_args(&mut command);
        run_status(&mut command)
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
        command.arg("--resume");
        self.add_target_args(&mut command);
        run_status(&mut command)
    }

    /// Upload one snapshot artifact and return the uploaded snapshot id.
    pub fn snapshot_upload(
        &self,
        canister: &str,
        artifact_path: &Path,
    ) -> Result<String, IcpCommandError> {
        let mut command = self.canister_command();
        command.args(["snapshot", "upload", canister, "--input"]);
        command.arg(artifact_path);
        command.arg("--resume");
        self.add_target_args(&mut command);
        run_output_with_stderr(&mut command)
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
        command.arg("--resume");
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

    /// Render a dry-run stop command.
    #[must_use]
    pub fn stop_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["stop", canister]);
        self.add_target_args(&mut command);
        command_display(&command)
    }

    /// Render a dry-run start command.
    #[must_use]
    pub fn start_canister_display(&self, canister: &str) -> String {
        let mut command = self.canister_command();
        command.args(["start", canister]);
        self.add_target_args(&mut command);
        command_display(&command)
    }

    fn add_target_args(&self, command: &mut Command) {
        add_target_args(command, self.environment(), self.network());
    }
}

/// Build a base `icp` command with the default executable.
#[must_use]
pub fn default_command() -> Command {
    IcpCli::new("icp", None, None).command()
}

/// Build a base `icp` command rooted at one workspace directory.
#[must_use]
pub fn default_command_in(cwd: &Path) -> Command {
    IcpCli::new("icp", None, None).command_in(cwd)
}

/// Add optional ICP CLI target arguments, preferring named environments.
pub fn add_target_args(command: &mut Command, environment: Option<&str>, network: Option<&str>) {
    if let Some(environment) = environment {
        command.args(["-e", environment]);
    } else if let Some(network) = network {
        command.args(["-n", network]);
    }
}

/// Add ICP CLI output formatting, handling JSON as its own flag.
pub fn add_output_arg(command: &mut Command, output: &str) {
    if output == "json" {
        command.arg("--json");
    } else {
        command.args(["--output", output]);
    }
}

/// Execute a command and capture trimmed stdout.
pub fn run_output(command: &mut Command) -> Result<String, IcpCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and capture stdout plus stderr on success.
pub fn run_output_with_stderr(command: &mut Command) -> Result<String, IcpCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(text.trim().to_string())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and require a successful status.
pub fn run_status(command: &mut Command) -> Result<(), IcpCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a rendered ICP CLI-compatible command and return raw process output.
pub fn run_raw_output(program: &str, args: &[String]) -> Result<IcpRawOutput, std::io::Error> {
    let output = Command::new(program).args(args).output()?;
    Ok(IcpRawOutput {
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

/// Parse a likely snapshot id from `icp canister snapshot create` output.
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

// Prefer stderr, but keep stdout diagnostics for CLI commands that report there.
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

    // Keep generated commands tied to ICP CLI environments when one is selected.
    #[test]
    fn renders_environment_target() {
        let icp = IcpCli::new("icp", Some("staging".to_string()), Some("ic".to_string()));

        assert_eq!(
            icp.snapshot_download_display("root", "snap-1", Path::new("backups/root")),
            "icp canister snapshot download root snap-1 --output backups/root --resume -e staging"
        );
    }

    // Keep direct network targeting available for local and ad hoc command contexts.
    #[test]
    fn renders_network_target() {
        let icp = IcpCli::new("icp", None, Some("ic".to_string()));

        assert_eq!(
            icp.snapshot_create_display("aaaaa-aa"),
            "icp canister snapshot create aaaaa-aa -n ic"
        );
    }

    // Ensure restore planning uses the ICP CLI upload/restore flow.
    #[test]
    fn renders_snapshot_restore_flow() {
        let icp = IcpCli::new("icp", Some("prod".to_string()), None);

        assert_eq!(
            icp.snapshot_upload_display("root", Path::new("artifact")),
            "icp canister snapshot upload root --input artifact --resume -e prod"
        );
        assert_eq!(
            icp.snapshot_restore_display("root", "uploaded-1"),
            "icp canister snapshot restore root uploaded-1 -e prod"
        );
    }

    // Ensure snapshot ids can be extracted from common create output.
    #[test]
    fn parses_snapshot_id_from_output() {
        let snapshot_id = parse_snapshot_id("Created snapshot: snap_abc-123\n");

        assert_eq!(snapshot_id.as_deref(), Some("snap_abc-123"));
    }
}
