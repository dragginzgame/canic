use std::{error::Error, fmt, path::Path, process::Command};

///
/// DfxCommandError
///

#[derive(Debug)]
pub enum DfxCommandError {
    Io(std::io::Error),
    Failed { command: String, stderr: String },
}

impl fmt::Display for DfxCommandError {
    // Render dfx command failures with the command line and captured diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Failed { command, stderr } => {
                write!(formatter, "dfx command failed: {command}\n{stderr}")
            }
        }
    }
}

impl Error for DfxCommandError {
    // Preserve the underlying I/O error as the source when command execution fails locally.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Failed { .. } => None,
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

// Prefer stderr, but keep stdout diagnostics for dfx commands that report there.
fn command_stderr(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        stderr.to_string()
    }
}
