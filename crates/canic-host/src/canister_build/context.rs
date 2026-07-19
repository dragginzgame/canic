use std::{fs, path::PathBuf, process::Command};

use canic_core::ids::BuildNetwork;

use crate::icp::LocalReplicaTarget;

use super::{
    CanisterBuildProfile,
    process::{icp_ancestor_process_id, parent_process_id},
};

/// Exact authority for one canister artifact build.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceBuildContext {
    pub role: String,
    pub profile: CanisterBuildProfile,
    pub environment: String,
    pub build_network: BuildNetwork,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: PathBuf,
    pub local_replica: Option<LocalReplicaTarget>,
    pub refresh_canonical_wasm_store_did: bool,
}

impl WorkspaceBuildContext {
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Canic build:".to_string(),
            format!("role: {}", self.role),
            format!("profile: {}", self.profile.target_dir_name()),
            format!("environment: {}", self.environment),
            format!("build network: {}", self.build_network),
            format!("workspace: {}", self.workspace_root.display()),
        ];

        if self.icp_root != self.workspace_root {
            lines.push(format!("icp root: {}", self.icp_root.display()));
        }

        lines
    }

    /// Return a copy using a different Cargo profile for one child build.
    #[must_use]
    pub fn with_profile(&self, profile: CanisterBuildProfile) -> Self {
        let mut context = self.clone();
        context.profile = profile;
        context
    }

    /// Return a copy selecting another role in the same workspace authority.
    #[must_use]
    pub fn with_role(&self, role: impl Into<String>) -> Self {
        let mut context = self.clone();
        context.role = role.into();
        context
    }

    /// Apply the exact Canic build authority to one child command.
    pub fn apply_to_command(&self, command: &mut Command) {
        command
            .env("ICP_ENVIRONMENT", self.build_network.as_str())
            .env(
                canic_core::role_contract::CANONICAL_BUILD_ICP_ROOT_ENV,
                &self.icp_root,
            )
            .env(
                canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                &self.config_path,
            );
    }
}

// Print the current build context once per caller session so caller builds
// stay readable without repeating root/profile diagnostics for every canister.
pub fn print_workspace_build_context_once(
    context: &WorkspaceBuildContext,
) -> Result<(), Box<dyn std::error::Error>> {
    if workspace_build_context_once(context)? {
        eprintln!("{}", context.lines().join("\n"));
    }

    Ok(())
}

// Return whether this caller should print its explicit build context.
pub fn workspace_build_context_once(
    context: &WorkspaceBuildContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    let marker_dir = context.icp_root.join(".icp");
    fs::create_dir_all(&marker_dir)?;

    let marker_key = icp_ancestor_process_id()
        .or_else(parent_process_id)
        .unwrap_or_else(std::process::id)
        .to_string();
    let marker_file = marker_dir.join(format!(".canic-build-context-{marker_key}"));

    if marker_file.exists() {
        return Ok(false);
    }

    fs::write(&marker_file, [])?;
    Ok(true)
}
