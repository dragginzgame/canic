//! Module: install_root::icp_context
//!
//! Responsibility: retain one fresh install's exact ICP command authority.
//! Does not own: command execution, durable journals, or environment discovery.
//! Boundary: every install subprocess derives from this complete immutable context.

use crate::icp::{IcpCli, LocalReplicaTarget};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

///
/// InstallIcpContext
///
/// Frozen executable, credential selection, project root and network target for one install.
///

pub(super) struct InstallIcpContext {
    cli: IcpCli,
    root: PathBuf,
    environment: String,
}

impl InstallIcpContext {
    /// Freeze command authority before discovery or readiness subprocesses run.
    #[must_use]
    pub(super) fn new(executable: &str, root: &Path, environment: &str) -> Self {
        let root = root.to_path_buf();
        let environment = environment.to_string();
        let cli = IcpCli::new(executable, Some(environment.clone())).with_cwd(root.clone());
        Self {
            cli,
            root,
            environment,
        }
    }

    /// Resolve the optional direct local-replica target without changing earlier authority.
    #[must_use]
    pub(super) fn with_local_replica(mut self, target: Option<LocalReplicaTarget>) -> Self {
        self.cli = self.cli.with_local_replica(target);
        self
    }

    /// Borrow the complete command authority for one subprocess operation.
    #[must_use]
    pub(super) const fn cli(&self) -> &IcpCli {
        &self.cli
    }

    /// Apply the frozen named-environment or direct-replica target.
    pub(super) fn add_target_args(&self, command: &mut Command) {
        self.cli.add_target_args(command);
    }

    /// Return the frozen ICP environment name.
    #[must_use]
    pub(super) fn environment(&self) -> &str {
        &self.environment
    }

    /// Return the frozen ICP project root.
    #[must_use]
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
}
