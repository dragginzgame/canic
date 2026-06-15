use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    error::IcpCommandError,
    model::{CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV, IcpCli, LOCAL_NETWORK},
    version::compatible_version_output,
};

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
            cwd: None,
        }
    }

    /// Return a copy of this ICP CLI context rooted at one project directory.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
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
        let mut command = Command::new(&self.executable);
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
            add_project_root_override_arg(&mut command, cwd);
        }
        command
    }

    /// Build a base ICP CLI command rooted at one workspace directory.
    #[must_use]
    pub fn command_in(&self, cwd: &Path) -> Command {
        let mut command = Command::new(&self.executable);
        command.current_dir(cwd);
        add_project_root_override_arg(&mut command, cwd);
        command
    }

    /// Build an `icp canister ...` command with optional environment args applied.
    #[must_use]
    pub fn canister_command(&self) -> Command {
        let mut command = self.command();
        command.arg("canister");
        command
    }

    pub(super) fn add_target_args(&self, command: &mut Command) {
        add_target_args(command, self.environment(), self.network());
    }

    pub(super) fn add_local_network_target(&self, command: &mut Command) {
        if let Some(environment) = self.environment() {
            command.args(["-e", environment]);
        } else if let Some(network) = self.network() {
            command.arg(network);
        } else {
            command.arg(LOCAL_NETWORK);
        }
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
        if environment == LOCAL_NETWORK
            && let Some(url) = env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV)
        {
            command.env_remove("ICP_ENVIRONMENT");
            command.arg("-n").arg(url);
            if let Some(root_key) = env::var_os(CANIC_ICP_LOCAL_ROOT_KEY_ENV) {
                command.arg("-k").arg(root_key);
            }
            return;
        }
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

/// Add an ICP CLI local Candid interface path when one is available.
pub fn add_candid_arg(command: &mut Command, candid_path: Option<&Path>) {
    if let Some(candid_path) = candid_path {
        command.arg("--candid").arg(candid_path);
    }
}

/// Return Canic's local ICP CLI Candid sidecar path for one role.
#[must_use]
pub fn local_canister_candid_path(icp_root: &Path, environment: &str, role: &str) -> PathBuf {
    icp_root
        .join(".icp")
        .join(environment)
        .join("canisters")
        .join(role)
        .join(format!("{role}.did"))
}

/// Return the local Candid sidecar path only when it exists on disk.
#[must_use]
pub fn existing_local_canister_candid_path(
    icp_root: &Path,
    environment: &str,
    role: &str,
) -> Option<PathBuf> {
    let path = local_canister_candid_path(icp_root, environment, role);
    path.is_file().then_some(path)
}

/// Add ICP CLI debug logging when requested.
pub fn add_debug_arg(command: &mut Command, debug: bool) {
    if debug {
        command.arg("--debug");
    }
}

/// Ensure a command points at a supported ICP CLI executable before spawning it.
pub fn ensure_command_compatible(command: &Command) -> Result<(), IcpCommandError> {
    let executable = command.get_program().to_string_lossy();
    compatible_version_output(executable.as_ref(), command.get_current_dir()).map(|_| ())
}

fn add_project_root_override_arg(command: &mut Command, cwd: &Path) {
    command.arg("--project-root-override").arg(cwd);
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
