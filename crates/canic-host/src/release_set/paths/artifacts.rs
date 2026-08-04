use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

use super::super::ROOT_RELEASE_SET_MANIFEST_FILE;

/// Failure to locate the exact artifact root for a selected artifact environment.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum ArtifactRootError {
    #[error("missing built ICP artifacts under {artifact_root}")]
    Missing { artifact_root: PathBuf },

    #[error("built ICP artifact root escapes the canonical ICP project root: {artifact_root}")]
    OutsideProject { artifact_root: PathBuf },
}

/// Resolve the built artifact directory for the selected artifact environment.
pub fn resolve_artifact_root(
    icp_root: &Path,
    artifact_environment: &str,
) -> Result<PathBuf, ArtifactRootError> {
    let artifact_root = artifact_root_path(icp_root, artifact_environment);
    resolve_artifact_root_path(icp_root, &artifact_root)
}

/// Resolve one exact built artifact directory confined to the canonical project root.
pub fn resolve_artifact_root_path(
    icp_root: &Path,
    artifact_root: &Path,
) -> Result<PathBuf, ArtifactRootError> {
    if !artifact_root.is_dir() {
        return Err(ArtifactRootError::Missing {
            artifact_root: artifact_root.to_path_buf(),
        });
    }

    let canonical_project = icp_root
        .canonicalize()
        .map_err(|_| ArtifactRootError::Missing {
            artifact_root: icp_root.to_path_buf(),
        })?;
    let canonical_artifact =
        artifact_root
            .canonicalize()
            .map_err(|_| ArtifactRootError::Missing {
                artifact_root: artifact_root.to_path_buf(),
            })?;
    if !canonical_artifact.starts_with(&canonical_project) {
        return Err(ArtifactRootError::OutsideProject {
            artifact_root: artifact_root.to_path_buf(),
        });
    }

    Ok(canonical_artifact)
}

/// Return the canonical artifact directory for one artifact environment.
#[must_use]
pub fn artifact_root_path(icp_root: &Path, artifact_environment: &str) -> PathBuf {
    icp_root
        .join(".icp")
        .join(artifact_environment)
        .join("canisters")
}

/// Return the canonical manifest path for the staged root release set.
#[must_use]
pub fn root_release_set_manifest_path(artifact_root: &Path) -> PathBuf {
    artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE)
}
