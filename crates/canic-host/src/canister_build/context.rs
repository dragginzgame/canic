use std::{env, fs, path::PathBuf};

use crate::{
    icp_environment_from_env,
    release_set::{icp_root, workspace_root},
    selected_icp_environment_from_env,
};

use super::{
    CanisterBuildProfile,
    process::{icp_ancestor_process_id, parent_process_id},
};

/// WorkspaceBuildContext
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceBuildContext {
    pub profile: String,
    pub requested_profile: String,
    pub environment: String,
    pub build_network: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
}

impl WorkspaceBuildContext {
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Canic build:".to_string(),
            format!("profile: {}", self.profile),
            format!("environment: {}", self.environment),
            format!("build network: {}", self.build_network),
            format!("workspace: {}", self.workspace_root.display()),
        ];

        if self.requested_profile != "unset" {
            lines.push(format!("requested profile: {}", self.requested_profile));
        }
        if self.icp_root != self.workspace_root {
            lines.push(format!("icp root: {}", self.icp_root.display()));
        }

        lines
    }
}

// Print the current build context once per caller session so caller builds
// stay readable without repeating root/profile diagnostics for every canister.
pub fn print_current_workspace_build_context_once(
    profile: CanisterBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(context) = current_workspace_build_context_once(profile)? {
        eprintln!("{}", context.lines().join("\n"));
    }

    Ok(())
}

// Return the current build context once per caller session.
pub fn current_workspace_build_context_once(
    profile: CanisterBuildProfile,
) -> Result<Option<WorkspaceBuildContext>, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    let marker_dir = icp_root.join(".icp");
    fs::create_dir_all(&marker_dir)?;

    let requested_profile = env::var("CANIC_WASM_PROFILE").unwrap_or_else(|_| "unset".to_string());
    let environment = selected_icp_environment_from_env();
    let build_network = icp_environment_from_env();
    let marker_key = icp_ancestor_process_id()
        .or_else(parent_process_id)
        .unwrap_or_else(std::process::id)
        .to_string();
    let marker_file = marker_dir.join(format!(".canic-build-context-{marker_key}"));

    if marker_file.exists() {
        return Ok(None);
    }

    fs::write(&marker_file, [])?;
    Ok(Some(WorkspaceBuildContext {
        profile: profile.target_dir_name().to_string(),
        requested_profile,
        environment,
        build_network,
        workspace_root,
        icp_root,
    }))
}
