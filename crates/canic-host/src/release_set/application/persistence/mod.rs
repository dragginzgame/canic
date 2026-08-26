//! Module: release_set::application::persistence
//!
//! Responsibility: compile and immutably persist one qualified application artifact union.
//! Does not own: Cargo execution, root projection, Store publication, or installation.
//! Boundary: exact current-build files populate one topology-bound union before finalization.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    release_build::{ReleaseBuildPlanError, ReleaseBuildPlanState, load_release_build_plan},
    release_set::artifact::{
        ReleaseArtifactMaterializationError, contains_release_build_identity,
        materialize_qualified_release_artifact,
    },
};
use std::{
    io,
    path::{Path, PathBuf},
};

use canic_core::{
    bootstrap::compiled::ComponentTopology,
    ids::{CanisterRole, ReleaseBuildId},
    role_contract::ProtocolProfileDigest,
};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

use super::{
    ApplicationArtifactBuildOutput, ApplicationArtifactBuildTarget, ApplicationArtifactUnion,
    ApplicationReleaseSetError,
};

pub const APPLICATION_ARTIFACT_UNION_FILE: &str = "application-artifact-union.json";

///
/// ApplicationArtifactFileBuildOutput
///
/// Exact current-build files for one topology role under one release-build identity.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationArtifactFileBuildOutput {
    pub role: CanisterRole,
    pub package: String,
    pub release_build_id: ReleaseBuildId,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// PersistedApplicationArtifactUnion
///
/// Canonical application union admitted under one durable release-build identity.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedApplicationArtifactUnion {
    pub union: ApplicationArtifactUnion,
    pub digest: [u8; 32],
    pub path: PathBuf,
}

///
/// ApplicationArtifactUnionPersistenceError
///
/// Typed rejection while materializing or admitting durable application evidence.
///

#[derive(Debug, ThisError)]
pub enum ApplicationArtifactUnionPersistenceError {
    #[error("application artifact {role} {kind} path is outside the ICP root: {path}")]
    ArtifactOutsideRoot {
        role: CanisterRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error("failed to read application artifact {role} {kind} at {path}: {source}")]
    ArtifactRead {
        role: CanisterRole,
        kind: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("application artifact union already exists with different canonical bytes: {path}")]
    ConflictingUnion { path: PathBuf },

    #[error("finalized release build {release_build_id} has no exact application artifact union")]
    FinalizedWithoutExactUnion { release_build_id: ReleaseBuildId },

    #[error("application artifact {role} has an invalid {kind} path: {path}")]
    InvalidArtifactPath {
        role: CanisterRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error("invalid application artifact union {path}: {reason}")]
    InvalidUnionDocument { path: PathBuf, reason: String },

    #[error("application artifact union is missing: {path}")]
    MissingUnion { path: PathBuf },

    #[error(
        "application artifact {role} raw Wasm at {path} does not embed release build {release_build_id}"
    )]
    MissingReleaseBuildIdentity {
        role: CanisterRole,
        release_build_id: ReleaseBuildId,
        path: PathBuf,
    },

    #[error("application artifact {role} has a non-UTF-8 {kind} path: {path}")]
    NonUtf8ArtifactPath {
        role: CanisterRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),

    #[error(transparent)]
    ReleaseSet(#[from] ApplicationReleaseSetError),

    #[error("application artifact {role} {kind} is not a regular no-follow file: {path}")]
    UnsafeArtifact {
        role: CanisterRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error("application artifact union is not a regular no-follow file: {path}")]
    UnsafeUnion { path: PathBuf },

    #[error("failed to access application artifact union {path}: {source}")]
    UnionIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Compile one union from qualified files and durably publish its canonical bytes.
pub fn compile_and_persist_application_artifact_union(
    root: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
    targets: &[ApplicationArtifactBuildTarget],
    outputs: &[ApplicationArtifactFileBuildOutput],
) -> Result<PersistedApplicationArtifactUnion, ApplicationArtifactUnionPersistenceError> {
    let release_build = load_release_build_plan(root, release_build_id)?;
    ApplicationArtifactUnion::validate_build_targets(topology, targets)?;
    let mut output_roles = outputs
        .iter()
        .map(|output| output.role.clone())
        .collect::<Vec<_>>();
    ApplicationArtifactUnion::validate_build_output_roles(topology, &mut output_roles)?;
    let materialized = outputs
        .iter()
        .map(|output| materialize_build_output(root, release_build_id, output))
        .collect::<Result<Vec<_>, _>>()?;
    let union =
        ApplicationArtifactUnion::compile(topology, release_build_id, targets, &materialized)?;
    let expected = persisted_union(root, topology, union)?;
    let existing = load_optional_persisted_union(&expected.path, topology, release_build_id)?;

    if matches!(release_build.state, ReleaseBuildPlanState::Finalized { .. }) {
        return match existing {
            Some(observed) if observed.union == expected.union => Ok(observed),
            _ => Err(
                ApplicationArtifactUnionPersistenceError::FinalizedWithoutExactUnion {
                    release_build_id,
                },
            ),
        };
    }

    if let Some(observed) = existing {
        return if observed.union == expected.union {
            Ok(observed)
        } else {
            Err(ApplicationArtifactUnionPersistenceError::ConflictingUnion {
                path: expected.path,
            })
        };
    }

    let canonical_bytes = expected.union.canonical_bytes(topology)?;
    if let Err(source) = create_new_bytes_with_parents(&expected.path, &canonical_bytes) {
        match load_optional_persisted_union(&expected.path, topology, release_build_id) {
            Ok(Some(observed)) if observed.union == expected.union => return Ok(observed),
            Ok(Some(_)) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(ApplicationArtifactUnionPersistenceError::ConflictingUnion {
                    path: expected.path,
                });
            }
            _ => {
                return Err(ApplicationArtifactUnionPersistenceError::UnionIo {
                    path: expected.path,
                    source,
                });
            }
        }
    }

    load_persisted_application_artifact_union(root, topology, release_build_id)
}

/// Load and validate the exact canonical application union for one release build.
pub fn load_persisted_application_artifact_union(
    root: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
) -> Result<PersistedApplicationArtifactUnion, ApplicationArtifactUnionPersistenceError> {
    let path = application_artifact_union_path(root, release_build_id);
    load_optional_persisted_union(&path, topology, release_build_id)?
        .ok_or(ApplicationArtifactUnionPersistenceError::MissingUnion { path })
}

fn materialize_build_output(
    root: &Path,
    release_build_id: ReleaseBuildId,
    output: &ApplicationArtifactFileBuildOutput,
) -> Result<ApplicationArtifactBuildOutput, ApplicationArtifactUnionPersistenceError> {
    if output.release_build_id != release_build_id {
        return Err(ApplicationReleaseSetError::ReleaseBuildMismatch {
            role: output.role.clone(),
            expected: release_build_id,
            actual: output.release_build_id,
        }
        .into());
    }

    let wasm = materialize_artifact(root, &output.role, "raw Wasm", &output.wasm_path)?;
    if !contains_release_build_identity(&wasm.bytes, release_build_id) {
        return Err(
            ApplicationArtifactUnionPersistenceError::MissingReleaseBuildIdentity {
                role: output.role.clone(),
                release_build_id,
                path: output.wasm_path.clone(),
            },
        );
    }
    let wasm_gz = materialize_artifact(root, &output.role, "gzip Wasm", &output.wasm_gz_path)?;
    Ok(ApplicationArtifactBuildOutput {
        role: output.role.clone(),
        package: output.package.clone(),
        release_build_id: output.release_build_id,
        wasm_relative_path: wasm.relative_path,
        wasm: wasm.bytes,
        wasm_gz_relative_path: wasm_gz.relative_path,
        wasm_gz: wasm_gz.bytes,
        candid_sha256: output.candid_sha256,
        protocol_profile_digest: output.protocol_profile_digest,
    })
}

fn materialize_artifact(
    root: &Path,
    role: &CanisterRole,
    kind: &'static str,
    path: &Path,
) -> Result<
    crate::release_set::artifact::MaterializedReleaseArtifact,
    ApplicationArtifactUnionPersistenceError,
> {
    materialize_qualified_release_artifact(root, path).map_err(|error| match error {
        ReleaseArtifactMaterializationError::InvalidPath => {
            ApplicationArtifactUnionPersistenceError::InvalidArtifactPath {
                role: role.clone(),
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::NonUtf8Path => {
            ApplicationArtifactUnionPersistenceError::NonUtf8ArtifactPath {
                role: role.clone(),
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::OutsideRoot => {
            ApplicationArtifactUnionPersistenceError::ArtifactOutsideRoot {
                role: role.clone(),
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::Read(source) => {
            ApplicationArtifactUnionPersistenceError::ArtifactRead {
                role: role.clone(),
                kind,
                path: path.to_path_buf(),
                source,
            }
        }
        ReleaseArtifactMaterializationError::UnsafeFile => {
            ApplicationArtifactUnionPersistenceError::UnsafeArtifact {
                role: role.clone(),
                kind,
                path: path.to_path_buf(),
            }
        }
    })
}

fn persisted_union(
    root: &Path,
    topology: &ComponentTopology,
    union: ApplicationArtifactUnion,
) -> Result<PersistedApplicationArtifactUnion, ApplicationArtifactUnionPersistenceError> {
    let digest = union.digest(topology)?;
    let path = application_artifact_union_path(root, union.release_build_id);
    Ok(PersistedApplicationArtifactUnion {
        union,
        digest,
        path,
    })
}

fn load_optional_persisted_union(
    path: &Path,
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
) -> Result<Option<PersistedApplicationArtifactUnion>, ApplicationArtifactUnionPersistenceError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(ApplicationArtifactUnionPersistenceError::UnsafeUnion {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(ApplicationArtifactUnionPersistenceError::UnionIo {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(ApplicationArtifactUnionPersistenceError::UnionIo {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "regular no-follow union reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };

    let union: ApplicationArtifactUnion = serde_json::from_slice(&bytes).map_err(|error| {
        ApplicationArtifactUnionPersistenceError::InvalidUnionDocument {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }
    })?;
    if union.release_build_id != release_build_id {
        return Err(
            ApplicationArtifactUnionPersistenceError::InvalidUnionDocument {
                path: path.to_path_buf(),
                reason: format!(
                    "union release build {} does not match path release build {release_build_id}",
                    union.release_build_id
                ),
            },
        );
    }
    let canonical = union.canonical_bytes(topology)?;
    if canonical != bytes {
        return Err(
            ApplicationArtifactUnionPersistenceError::InvalidUnionDocument {
                path: path.to_path_buf(),
                reason: "union bytes are not canonical".to_string(),
            },
        );
    }

    Ok(Some(PersistedApplicationArtifactUnion {
        digest: Sha256::digest(&bytes).into(),
        union,
        path: path.to_path_buf(),
    }))
}

fn application_artifact_union_path(root: &Path, release_build_id: ReleaseBuildId) -> PathBuf {
    root.join(".canic")
        .join("release-builds")
        .join(release_build_id.to_string())
        .join(APPLICATION_ARTIFACT_UNION_FILE)
}
