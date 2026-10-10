//! Module: release_set::artifact
//!
//! Responsibility: resolve, validate, and describe release-set Wasm artifacts.
//! Does not own: manifest persistence or build orchestration.
//! Boundary: keeps every artifact read within the canonical ICP project root.

mod representation;

use crate::MAX_ARTIFACT_READ_BYTES;
use canic_contracts::ids::ReleaseBuildId;
use ic_host_artifacts::artifact::ArtifactError;
use ic_host_fs::read::read_optional_file_no_follow;
use std::{
    fs, io,
    path::{Component, Path},
};

pub(in crate::release_set) use representation::{RepresentationError, qualify_representation};

pub(in crate::release_set) struct MaterializedReleaseArtifact {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

pub(in crate::release_set) enum ReleaseArtifactMaterializationError {
    InvalidPath,
    NonUtf8Path,
    OutsideRoot,
    Read(io::Error),
    UnsafeFile,
}

/// Prove that raw Wasm carries the exact release-build identity supplied to its build.
pub(in crate::release_set) fn contains_release_build_identity(
    wasm: &[u8],
    release_build_id: ReleaseBuildId,
) -> bool {
    let identity = release_build_id.to_string();
    wasm.windows(identity.len())
        .any(|window| window == identity.as_bytes())
}

/// Validate the lexical path contract shared by manifest admission and
/// filesystem resolution.
pub fn validate_release_artifact_relative_path(
    artifact_relative_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let relative_path = Path::new(artifact_relative_path);
    if relative_path.as_os_str().is_empty()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "release artifact path must be relative to the ICP root: {artifact_relative_path}"
        )
        .into());
    }

    Ok(())
}

/// Read one exact regular no-follow artifact beneath the canonical project root.
pub(in crate::release_set) fn materialize_qualified_release_artifact(
    root: &Path,
    path: &Path,
) -> Result<MaterializedReleaseArtifact, ReleaseArtifactMaterializationError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ReleaseArtifactMaterializationError::OutsideRoot)?;
    let relative = relative
        .to_str()
        .ok_or(ReleaseArtifactMaterializationError::NonUtf8Path)?;
    validate_release_artifact_relative_path(relative)
        .map_err(|_| ReleaseArtifactMaterializationError::InvalidPath)?;

    let canonical_root =
        fs::canonicalize(root).map_err(ReleaseArtifactMaterializationError::Read)?;
    let parent = path
        .parent()
        .ok_or(ReleaseArtifactMaterializationError::InvalidPath)?;
    let canonical_parent =
        fs::canonicalize(parent).map_err(ReleaseArtifactMaterializationError::Read)?;
    if !canonical_parent.starts_with(canonical_root) {
        return Err(ReleaseArtifactMaterializationError::OutsideRoot);
    }

    let bytes = match read_optional_file_no_follow(path, MAX_ARTIFACT_READ_BYTES) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(ReleaseArtifactMaterializationError::Read(io::Error::new(
                io::ErrorKind::NotFound,
                "artifact is missing",
            )));
        }
        Err(ArtifactError::NotRegularFile) => {
            return Err(ReleaseArtifactMaterializationError::UnsafeFile);
        }
        Err(source) => {
            return Err(ReleaseArtifactMaterializationError::Read(source.into()));
        }
    };

    Ok(MaterializedReleaseArtifact {
        relative_path: relative.to_string(),
        bytes,
    })
}
