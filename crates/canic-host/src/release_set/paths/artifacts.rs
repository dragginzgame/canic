use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

use super::super::ROOT_RELEASE_SET_MANIFEST_FILE;

/// Failure to locate the exact artifact root for a selected artifact network.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum ArtifactRootError {
    #[error("missing built ICP artifacts under {artifact_root}")]
    Missing { artifact_root: PathBuf },
}

/// Resolve the built artifact directory for the selected artifact network.
pub fn resolve_artifact_root(icp_root: &Path, network: &str) -> Result<PathBuf, ArtifactRootError> {
    let artifact_root = artifact_root_path(icp_root, network);
    if artifact_root.is_dir() {
        return Ok(artifact_root);
    }

    Err(ArtifactRootError::Missing { artifact_root })
}

/// Return the canonical artifact directory for one artifact network.
pub fn artifact_root_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root.join(".icp").join(network).join("canisters")
}

/// Return the canonical manifest path for the staged root release set.
#[must_use]
pub fn root_release_set_manifest_path(artifact_root: &Path) -> PathBuf {
    artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE)
}
