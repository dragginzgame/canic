//! Module: release_set::infrastructure::persistence
//!
//! Responsibility: compile and immutably persist infrastructure evidence from one build.
//! Does not own: Cargo execution, release-build finalization, placement, or installation.
//! Boundary: only exact current-build outputs may populate the manifest under their durable ID.

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
    ids::{CanisterRole, ReleaseBuildId},
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

use super::{
    CanicInfrastructureArtifactInput, CanicInfrastructureArtifactManifest,
    CanicInfrastructureArtifactManifestError, CanicInfrastructureRole,
};

pub const INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE: &str = "infrastructure-artifact-manifest.json";

///
/// PersistedCanicInfrastructureArtifactManifest
///
/// Exact canonical infrastructure manifest admitted under one durable release-build identity.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedCanicInfrastructureArtifactManifest {
    pub manifest: CanicInfrastructureArtifactManifest,
    pub digest: [u8; 32],
    pub path: PathBuf,
}

///
/// CanicInfrastructureArtifactBuildOutput
///
/// Qualified package, build identity, and exact artifact paths from one complete build.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanicInfrastructureArtifactBuildOutput {
    pub role: CanicInfrastructureRole,
    pub package: String,
    pub protocol_release_identity: String,
    pub protocol_role: CanisterRole,
    pub protocol_capabilities: BTreeSet<RoleCapabilityKey>,
    pub release_build_id: ReleaseBuildId,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// CanicInfrastructureArtifactPersistenceError
///
/// Typed rejection while materializing or admitting durable infrastructure evidence.
///

#[derive(Debug, ThisError)]
pub enum CanicInfrastructureArtifactPersistenceError {
    #[error("infrastructure artifact {role:?} {kind} path is outside the ICP root: {path}")]
    ArtifactOutsideRoot {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error("failed to read infrastructure artifact {role:?} {kind} at {path}: {source}")]
    ArtifactRead {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "infrastructure artifact manifest already exists with different canonical bytes: {path}"
    )]
    ConflictingManifest { path: PathBuf },

    #[error(
        "finalized release build {release_build_id} has no exact infrastructure artifact manifest"
    )]
    FinalizedWithoutExactManifest { release_build_id: ReleaseBuildId },

    #[error("infrastructure artifact {role:?} has an invalid {kind} path: {path}")]
    InvalidArtifactPath {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error(
        "infrastructure artifact {role:?} raw Wasm at {path} does not embed release build {release_build_id}"
    )]
    MissingReleaseBuildIdentity {
        role: CanicInfrastructureRole,
        release_build_id: ReleaseBuildId,
        path: PathBuf,
    },

    #[error("invalid infrastructure artifact manifest {path}: {reason}")]
    InvalidManifestDocument { path: PathBuf, reason: String },

    #[error(transparent)]
    Manifest(#[from] CanicInfrastructureArtifactManifestError),

    #[error("failed to access infrastructure artifact manifest {path}: {source}")]
    ManifestIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("infrastructure artifact manifest is missing: {path}")]
    MissingManifest { path: PathBuf },

    #[error("infrastructure artifact {role:?} has a non-UTF-8 {kind} path: {path}")]
    NonUtf8ArtifactPath {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),

    #[error("infrastructure artifact {role:?} {kind} is not a regular no-follow file: {path}")]
    UnsafeArtifact {
        role: CanicInfrastructureRole,
        kind: &'static str,
        path: PathBuf,
    },

    #[error("infrastructure artifact manifest is not a regular no-follow file: {path}")]
    UnsafeManifest { path: PathBuf },
}

struct MaterializedBuildOutput {
    role: CanicInfrastructureRole,
    package: String,
    protocol_release_identity: String,
    protocol_role: CanisterRole,
    protocol_capabilities: BTreeSet<RoleCapabilityKey>,
    release_build_id: ReleaseBuildId,
    wasm_relative_path: String,
    wasm: Vec<u8>,
    wasm_gz_relative_path: String,
    wasm_gz: Vec<u8>,
    candid_sha256: [u8; 32],
    protocol_profile_digest: ProtocolProfileDigest,
}

impl MaterializedBuildOutput {
    fn as_input(&self) -> CanicInfrastructureArtifactInput<'_> {
        CanicInfrastructureArtifactInput {
            role: self.role,
            package: &self.package,
            protocol_release_identity: &self.protocol_release_identity,
            protocol_role: &self.protocol_role,
            protocol_capabilities: &self.protocol_capabilities,
            release_build_id: self.release_build_id,
            wasm_relative_path: &self.wasm_relative_path,
            wasm: &self.wasm,
            wasm_gz_relative_path: &self.wasm_gz_relative_path,
            wasm_gz: &self.wasm_gz,
            candid_sha256: self.candid_sha256,
            protocol_profile_digest: self.protocol_profile_digest,
        }
    }
}

/// Compile one manifest from qualified outputs and durably publish its canonical bytes.
pub fn compile_and_persist_canic_infrastructure_artifact_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
    outputs: &[CanicInfrastructureArtifactBuildOutput],
) -> Result<PersistedCanicInfrastructureArtifactManifest, CanicInfrastructureArtifactPersistenceError>
{
    let release_build = load_release_build_plan(root, release_build_id)?;
    let materialized = outputs
        .iter()
        .map(|output| materialize_build_output(root, release_build_id, output))
        .collect::<Result<Vec<_>, _>>()?;
    let inputs = materialized
        .iter()
        .map(MaterializedBuildOutput::as_input)
        .collect::<Vec<_>>();
    let manifest = CanicInfrastructureArtifactManifest::compile(release_build_id, &inputs)?;
    let expected = persisted_manifest(root, manifest)?;
    let existing = load_optional_persisted_manifest(&expected.path, release_build_id)?;

    if matches!(release_build.state, ReleaseBuildPlanState::Finalized { .. }) {
        return match existing {
            Some(observed) if observed.manifest == expected.manifest => Ok(observed),
            _ => Err(
                CanicInfrastructureArtifactPersistenceError::FinalizedWithoutExactManifest {
                    release_build_id,
                },
            ),
        };
    }

    if let Some(observed) = existing {
        return if observed.manifest == expected.manifest {
            Ok(observed)
        } else {
            Err(
                CanicInfrastructureArtifactPersistenceError::ConflictingManifest {
                    path: expected.path,
                },
            )
        };
    }

    let canonical_bytes = expected.manifest.canonical_bytes()?;
    if let Err(source) = create_new_bytes_with_parents(&expected.path, &canonical_bytes) {
        match load_optional_persisted_manifest(&expected.path, release_build_id) {
            Ok(Some(observed)) if observed.manifest == expected.manifest => return Ok(observed),
            Ok(Some(_)) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(
                    CanicInfrastructureArtifactPersistenceError::ConflictingManifest {
                        path: expected.path,
                    },
                );
            }
            _ => {
                return Err(CanicInfrastructureArtifactPersistenceError::ManifestIo {
                    path: expected.path,
                    source,
                });
            }
        }
    }

    load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)
}

/// Load and validate the exact canonical infrastructure manifest for one release build.
pub fn load_persisted_canic_infrastructure_artifact_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<PersistedCanicInfrastructureArtifactManifest, CanicInfrastructureArtifactPersistenceError>
{
    let path = infrastructure_artifact_manifest_path(root, release_build_id);
    load_optional_persisted_manifest(&path, release_build_id)?
        .ok_or(CanicInfrastructureArtifactPersistenceError::MissingManifest { path })
}

fn materialize_build_output(
    root: &Path,
    release_build_id: ReleaseBuildId,
    output: &CanicInfrastructureArtifactBuildOutput,
) -> Result<MaterializedBuildOutput, CanicInfrastructureArtifactPersistenceError> {
    let role = output.role;
    if output.release_build_id != release_build_id {
        return Err(
            CanicInfrastructureArtifactManifestError::ReleaseBuildMismatch {
                role,
                expected: release_build_id,
                actual: output.release_build_id,
            }
            .into(),
        );
    }

    let wasm = materialize_artifact(root, role, "raw Wasm", &output.wasm_path)?;
    if !contains_release_build_identity(&wasm.bytes, release_build_id) {
        return Err(
            CanicInfrastructureArtifactPersistenceError::MissingReleaseBuildIdentity {
                role,
                release_build_id,
                path: output.wasm_path.clone(),
            },
        );
    }
    let wasm_gz = materialize_artifact(root, role, "gzip Wasm", &output.wasm_gz_path)?;

    Ok(MaterializedBuildOutput {
        role,
        package: output.package.clone(),
        protocol_release_identity: output.protocol_release_identity.clone(),
        protocol_role: output.protocol_role.clone(),
        protocol_capabilities: output.protocol_capabilities.clone(),
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
    role: CanicInfrastructureRole,
    kind: &'static str,
    path: &Path,
) -> Result<
    crate::release_set::artifact::MaterializedReleaseArtifact,
    CanicInfrastructureArtifactPersistenceError,
> {
    materialize_qualified_release_artifact(root, path).map_err(|error| match error {
        ReleaseArtifactMaterializationError::InvalidPath => {
            CanicInfrastructureArtifactPersistenceError::InvalidArtifactPath {
                role,
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::NonUtf8Path => {
            CanicInfrastructureArtifactPersistenceError::NonUtf8ArtifactPath {
                role,
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::OutsideRoot => {
            CanicInfrastructureArtifactPersistenceError::ArtifactOutsideRoot {
                role,
                kind,
                path: path.to_path_buf(),
            }
        }
        ReleaseArtifactMaterializationError::Read(source) => {
            CanicInfrastructureArtifactPersistenceError::ArtifactRead {
                role,
                kind,
                path: path.to_path_buf(),
                source,
            }
        }
        ReleaseArtifactMaterializationError::UnsafeFile => {
            CanicInfrastructureArtifactPersistenceError::UnsafeArtifact {
                role,
                kind,
                path: path.to_path_buf(),
            }
        }
    })
}

fn persisted_manifest(
    root: &Path,
    manifest: CanicInfrastructureArtifactManifest,
) -> Result<PersistedCanicInfrastructureArtifactManifest, CanicInfrastructureArtifactPersistenceError>
{
    let digest = manifest.digest()?;
    let path = infrastructure_artifact_manifest_path(root, manifest.release_build_id);
    Ok(PersistedCanicInfrastructureArtifactManifest {
        manifest,
        digest,
        path,
    })
}

fn load_optional_persisted_manifest(
    path: &Path,
    release_build_id: ReleaseBuildId,
) -> Result<
    Option<PersistedCanicInfrastructureArtifactManifest>,
    CanicInfrastructureArtifactPersistenceError,
> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(
                CanicInfrastructureArtifactPersistenceError::UnsafeManifest {
                    path: path.to_path_buf(),
                },
            );
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(CanicInfrastructureArtifactPersistenceError::ManifestIo {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(CanicInfrastructureArtifactPersistenceError::ManifestIo {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "regular no-follow manifest reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };

    let manifest: CanicInfrastructureArtifactManifest =
        serde_json::from_slice(&bytes).map_err(|error| {
            CanicInfrastructureArtifactPersistenceError::InvalidManifestDocument {
                path: path.to_path_buf(),
                reason: error.to_string(),
            }
        })?;
    if manifest.release_build_id != release_build_id {
        return Err(
            CanicInfrastructureArtifactPersistenceError::InvalidManifestDocument {
                path: path.to_path_buf(),
                reason: format!(
                    "manifest release build {} does not match path release build {release_build_id}",
                    manifest.release_build_id
                ),
            },
        );
    }
    let canonical = manifest.canonical_bytes()?;
    if canonical != bytes {
        return Err(
            CanicInfrastructureArtifactPersistenceError::InvalidManifestDocument {
                path: path.to_path_buf(),
                reason: "manifest bytes are not canonical".to_string(),
            },
        );
    }

    Ok(Some(PersistedCanicInfrastructureArtifactManifest {
        digest: Sha256::digest(&bytes).into(),
        manifest,
        path: path.to_path_buf(),
    }))
}

fn infrastructure_artifact_manifest_path(root: &Path, release_build_id: ReleaseBuildId) -> PathBuf {
    root.join(".canic")
        .join("release-builds")
        .join(release_build_id.to_string())
        .join(INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE)
}
