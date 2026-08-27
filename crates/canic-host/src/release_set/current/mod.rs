//! Module: release_set::current
//!
//! Responsibility: bind the application union and infrastructure manifest into one release authority.
//! Does not own: artifact builds, topology, Fleet identities, or installation.
//! Boundary: finalization hashes one canonical manifest whose children were independently validated.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    release_build::{ReleaseBuildPlanError, ReleaseBuildPlanState, load_release_build_plan},
    release_set::{
        PersistedApplicationArtifactUnion, PersistedCanicInfrastructureArtifactManifest,
    },
};
use canic_core::ids::ReleaseBuildId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const CURRENT_RELEASE_SET_MANIFEST_FILE: &str = "current-release-set-manifest.json";

/// Canonical complete current release authority consumed by Fleet generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentReleaseSetManifest {
    pub application_artifact_union_sha256: [u8; 32],
    pub infrastructure_artifact_manifest_sha256: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub schema_version: u16,
}

impl CurrentReleaseSetManifest {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn validate(
        &self,
        release_build_id: ReleaseBuildId,
    ) -> Result<(), CurrentReleaseSetManifestError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(CurrentReleaseSetManifestError::Invalid(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if self.release_build_id != release_build_id {
            return Err(CurrentReleaseSetManifestError::Invalid(
                "path and document release-build identities differ".to_string(),
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CurrentReleaseSetManifestError> {
        self.validate(self.release_build_id)?;
        serde_json::to_vec(self).map_err(CurrentReleaseSetManifestError::Serialize)
    }
}

/// One persisted complete current release authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedCurrentReleaseSetManifest {
    pub digest: [u8; 32],
    pub manifest: CurrentReleaseSetManifest,
    pub path: PathBuf,
}

/// Typed complete release-set persistence failure.
#[derive(Debug, ThisError)]
pub enum CurrentReleaseSetManifestError {
    #[error("current release-set manifest already exists with different contents: {0}")]
    Conflict(PathBuf),

    #[error("invalid current release-set manifest: {0}")]
    Invalid(String),

    #[error("current release-set manifest is missing: {0}")]
    Missing(PathBuf),

    #[error("current release-set manifest is not a regular no-follow file: {0}")]
    Unsafe(PathBuf),

    #[error("failed to access current release-set manifest {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to encode current release-set manifest: {0}")]
    Serialize(serde_json::Error),

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),
}

/// Bind two exact child manifests before the release build becomes immutable.
pub fn compile_and_persist_current_release_set_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
    application: &PersistedApplicationArtifactUnion,
    infrastructure: &PersistedCanicInfrastructureArtifactManifest,
) -> Result<PersistedCurrentReleaseSetManifest, CurrentReleaseSetManifestError> {
    let release_build = load_release_build_plan(root, release_build_id)?;
    if application.union.release_build_id != release_build_id
        || infrastructure.manifest.release_build_id != release_build_id
    {
        return Err(CurrentReleaseSetManifestError::Invalid(
            "child manifest release-build identities differ".to_string(),
        ));
    }
    let expected = CurrentReleaseSetManifest {
        application_artifact_union_sha256: application.digest,
        infrastructure_artifact_manifest_sha256: infrastructure.digest,
        release_build_id,
        schema_version: CurrentReleaseSetManifest::SCHEMA_VERSION,
    };
    let path = current_release_set_manifest_path(root, release_build_id);
    let existing = load_optional(&path, release_build_id)?;
    if matches!(release_build.state, ReleaseBuildPlanState::Finalized { .. }) {
        return match existing {
            Some(existing) if existing.manifest == expected => Ok(existing),
            _ => Err(CurrentReleaseSetManifestError::Conflict(path)),
        };
    }
    if let Some(existing) = existing {
        return if existing.manifest == expected {
            Ok(existing)
        } else {
            Err(CurrentReleaseSetManifestError::Conflict(path))
        };
    }
    let bytes = expected.canonical_bytes()?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        if let Ok(Some(existing)) = load_optional(&path, release_build_id)
            && existing.manifest == expected
        {
            return Ok(existing);
        }
        return Err(CurrentReleaseSetManifestError::Io { path, source });
    }
    load_persisted_current_release_set_manifest(root, release_build_id)
}

/// Load one exact complete current release authority.
pub fn load_persisted_current_release_set_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<PersistedCurrentReleaseSetManifest, CurrentReleaseSetManifestError> {
    let path = current_release_set_manifest_path(root, release_build_id);
    load_optional(&path, release_build_id)?.ok_or(CurrentReleaseSetManifestError::Missing(path))
}

fn load_optional(
    path: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<Option<PersistedCurrentReleaseSetManifest>, CurrentReleaseSetManifestError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(value) => value,
        Err(RegularFileReadError::NotRegular) => {
            return Err(CurrentReleaseSetManifestError::Unsafe(path.to_path_buf()));
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(CurrentReleaseSetManifestError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(CurrentReleaseSetManifestError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "safe file reads are unavailable",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let manifest: CurrentReleaseSetManifest = serde_json::from_slice(&bytes)
        .map_err(|error| CurrentReleaseSetManifestError::Invalid(error.to_string()))?;
    manifest.validate(release_build_id)?;
    if manifest.canonical_bytes()? != bytes {
        return Err(CurrentReleaseSetManifestError::Invalid(
            "document bytes are not canonical".to_string(),
        ));
    }
    Ok(Some(PersistedCurrentReleaseSetManifest {
        digest: Sha256::digest(&bytes).into(),
        manifest,
        path: path.to_path_buf(),
    }))
}

fn current_release_set_manifest_path(root: &Path, release_build_id: ReleaseBuildId) -> PathBuf {
    root.join(".canic")
        .join("release-builds")
        .join(release_build_id.to_string())
        .join(CURRENT_RELEASE_SET_MANIFEST_FILE)
}
