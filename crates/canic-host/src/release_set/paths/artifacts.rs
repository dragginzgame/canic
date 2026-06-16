use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::ROOT_RELEASE_SET_MANIFEST_FILE;

// Resolve the built artifact directory for the selected ICP environment.
pub fn resolve_artifact_root(
    icp_root: &Path,
    network: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let preferred = icp_root.join(".icp").join(network).join("canisters");
    if preferred.is_dir() {
        return Ok(preferred);
    }

    let local_artifact_root = icp_root.join(".icp/local/canisters");
    if local_artifact_root.is_dir() {
        return Ok(local_artifact_root);
    }

    Err(format!(
        "missing built ICP artifacts under {} or {}",
        preferred.display(),
        local_artifact_root.display()
    )
    .into())
}

// Return the canonical manifest path for the staged root release set.
pub fn root_release_set_manifest_path(
    artifact_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(manifest_path)
}
