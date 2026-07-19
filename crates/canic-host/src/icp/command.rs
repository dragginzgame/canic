use std::{
    io,
    os::fd::BorrowedFd,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};

use crate::release_set::artifact_root_path;

use super::{
    error::IcpCommandError,
    model::{IcpCli, LOCAL_ICP_TARGET, LocalReplicaTarget},
    version::compatible_version_output,
};

impl IcpCli {
    /// Build an ICP CLI command context from an executable path and optional ICP environment.
    #[must_use]
    pub fn new(executable: impl Into<String>, environment: Option<String>) -> Self {
        Self {
            executable: executable.into(),
            environment,
            cwd: None,
            local_replica: None,
            inherited_fd: None,
        }
    }

    /// Return a copy of this ICP CLI context rooted at one project directory.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Return a copy using an explicit direct local replica target.
    #[must_use]
    pub fn with_local_replica(mut self, target: Option<LocalReplicaTarget>) -> Self {
        self.local_replica = target;
        self
    }

    /// Keep one caller-owned descriptor open in commands spawned by this context.
    #[must_use]
    pub const fn with_inherited_fd(mut self, inherited_fd: Option<i32>) -> Self {
        self.inherited_fd = inherited_fd;
        self
    }

    /// Return the optional ICP environment carried by this command context.
    #[must_use]
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    /// Build a base ICP CLI command from this context.
    #[must_use]
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
            add_project_root_override_arg(&mut command, cwd);
        }
        configure_inherited_fd(&mut command, self.inherited_fd);
        command
    }

    /// Build a base ICP CLI command rooted at one workspace directory.
    #[must_use]
    pub fn command_in(&self, cwd: &Path) -> Command {
        let mut command = Command::new(&self.executable);
        command.current_dir(cwd);
        add_project_root_override_arg(&mut command, cwd);
        configure_inherited_fd(&mut command, self.inherited_fd);
        command
    }

    /// Build an `icp canister ...` command with optional environment selection applied.
    #[must_use]
    pub fn canister_command(&self) -> Command {
        let mut command = self.command();
        command.arg("canister");
        command
    }

    pub(super) fn add_target_args(&self, command: &mut Command) {
        add_target_args(command, self.environment(), self.local_replica.as_ref());
    }
}

pub(super) fn configure_inherited_fd(command: &mut Command, inherited_fd: Option<i32>) {
    let Some(inherited_fd) = inherited_fd else {
        return;
    };

    #[cfg(unix)]
    // SAFETY: the closure performs only fcntl operations on the caller-owned
    // descriptor between fork and exec. The caller keeps that descriptor open
    // until the synchronous command completes.
    unsafe {
        command.pre_exec(move || {
            // SAFETY: the runner guarantees the raw descriptor remains valid
            // for the duration of this child setup callback.
            let fd = BorrowedFd::borrow_raw(inherited_fd);
            let mut flags = fcntl_getfd(fd).map_err(errno_to_io)?;
            flags.remove(FdFlags::CLOEXEC);
            fcntl_setfd(fd, flags).map_err(errno_to_io)
        });
    }

    #[cfg(not(unix))]
    let _ = (command, inherited_fd);
}

#[cfg(unix)]
fn errno_to_io(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use rustix::fs::{FlockOperation, flock};
    use std::{fs, time::Duration};

    #[test]
    fn configured_descriptor_lives_through_command_descendants_only() {
        let root = temp_dir("canic-icp-command-inherited-fd");
        fs::create_dir_all(&root).expect("create temp root");
        let lock_path = root.join("command.lock");
        let owner = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .expect("open owner lock");
        flock(&owner, FlockOperation::NonBlockingLockExclusive).expect("lock owner file");

        let mut command = Command::new("sh");
        command.args(["-c", "sleep 0.2 & wait"]);
        configure_inherited_fd(&mut command, Some(std::os::fd::AsRawFd::as_raw_fd(&owner)));
        let mut child = command.spawn().expect("spawn inherited command tree");
        drop(owner);

        let contender = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("open contender lock");
        assert_eq!(
            flock(&contender, FlockOperation::NonBlockingLockExclusive),
            Err(rustix::io::Errno::WOULDBLOCK)
        );

        child.wait().expect("wait for command tree");
        for _ in 0..50 {
            match flock(&contender, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => {
                    fs::remove_dir_all(root).expect("remove temp root");
                    return;
                }
                Err(rustix::io::Errno::WOULDBLOCK) => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("unexpected contender lock error: {error}"),
            }
        }
        panic!("inherited descriptor survived the command tree");
    }
}

pub(super) fn add_local_network_target(command: &mut Command) {
    command.arg(LOCAL_ICP_TARGET);
}

/// Build a base `icp` command with the default executable.
#[must_use]
pub fn default_command() -> Command {
    IcpCli::new("icp", None).command()
}

/// Build a base `icp` command rooted at one workspace directory.
#[must_use]
pub fn default_command_in(cwd: &Path) -> Command {
    IcpCli::new("icp", None).command_in(cwd)
}

/// Add the selected ICP environment through ICP CLI's named-environment selector.
pub fn add_target_args(
    command: &mut Command,
    environment: Option<&str>,
    local_replica: Option<&LocalReplicaTarget>,
) {
    if let Some(environment) = environment {
        if environment == LOCAL_ICP_TARGET
            && let Some(local_replica) = local_replica
        {
            command.env_remove("ICP_ENVIRONMENT");
            command
                .arg("-n")
                .arg(&local_replica.url)
                .arg("-k")
                .arg(&local_replica.root_key);
            return;
        }
        command.args(["-e", environment]);
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
pub fn local_canister_candid_path(
    icp_root: &Path,
    artifact_environment: &str,
    role: &str,
) -> PathBuf {
    artifact_root_path(icp_root, artifact_environment)
        .join(role)
        .join(format!("{role}.did"))
}

/// Return the local Candid sidecar path only when it exists on disk.
#[must_use]
pub fn existing_local_canister_candid_path(
    icp_root: &Path,
    artifact_environment: &str,
    role: &str,
) -> Option<PathBuf> {
    let path = local_canister_candid_path(icp_root, artifact_environment, role);
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
