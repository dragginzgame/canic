use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

use super::super::ROOT_RELEASE_SET_MANIFEST_FILE;

/// Failure to locate the exact artifact root for a selected ICP environment.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum ArtifactRootError {
    #[error("missing built ICP artifacts under {artifact_root}")]
    Missing { artifact_root: PathBuf },
}

/// Resolve the built artifact directory for the selected ICP environment.
pub fn resolve_artifact_root(icp_root: &Path, network: &str) -> Result<PathBuf, ArtifactRootError> {
    let artifact_root = icp_root.join(".icp").join(network).join("canisters");
    if artifact_root.is_dir() {
        return Ok(artifact_root);
    }

    Err(ArtifactRootError::Missing { artifact_root })
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
