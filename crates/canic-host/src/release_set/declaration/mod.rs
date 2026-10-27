//! Resolve an offline declaration through exact finalized release authority.
//!
//! This owner verifies retained manifest and declaration bytes; it does not select
//! the latest build, require current application source, or contact a canister.

#[cfg(test)]
mod tests;

use crate::{
    MAX_DOCUMENT_READ_BYTES,
    release_build::{ReleaseBuildPlanError, validate_finalized_release_build_manifest},
    release_set::{
        ApplicationArtifactUnionPersistenceError, CanicInfrastructureArtifactPersistenceError,
        CurrentReleaseSetManifestError, application::load_retained_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use canic_contracts::ids::ReleaseBuildId;
use ic_host_artifacts::artifact::ArtifactError;
use ic_host_fs::read::read_optional_file_no_follow;
use sha2_host::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Exact native schema and module identity from the selected immutable release.
#[derive(Debug)]
pub struct BuiltWireSchema {
    pub release_build_id: ReleaseBuildId,
    pub path: PathBuf,
    pub schema: Vec<u8>,
    pub schema_sha256: [u8; 32],
    pub wasm_sha256: String,
}

///
/// BuiltDeclarationError
///
/// Failure to bind an offline declaration to a finalized managed build.
///

#[derive(Debug, Error)]
pub enum BuiltDeclarationError {
    #[error("selected build contains ambiguous role {0}")]
    AmbiguousRole(String),

    #[error(transparent)]
    Application(#[from] ApplicationArtifactUnionPersistenceError),

    #[error("selected build declaration hash differs: {}", .0.display())]
    DigestMismatch(PathBuf),

    #[error(transparent)]
    Infrastructure(#[from] CanicInfrastructureArtifactPersistenceError),

    #[error("selected build child manifest digest differs from finalized authority")]
    ManifestDigestMismatch,

    #[error("selected build has no role {0}")]
    MissingRole(String),

    #[error("failed to read selected build declaration {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),

    #[error(transparent)]
    ReleaseSet(#[from] CurrentReleaseSetManifestError),

    #[error("selected build declaration is missing or not a regular contained file: {}", .0.display())]
    UnsafeDeclaration(PathBuf),
}

/// Load exact Wire schema bytes bound by a finalized release.
pub fn load_built_wire_schema(
    root: &Path,
    release_build_id: ReleaseBuildId,
    role: &str,
) -> Result<BuiltWireSchema, BuiltDeclarationError> {
    let release = load_persisted_current_release_set_manifest(root, release_build_id)?;
    validate_finalized_release_build_manifest(root, release_build_id, &release.path)?;
    let application = load_retained_application_artifact_union(root, release_build_id)?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)?;
    if application.digest != release.manifest.application_artifact_union_sha256
        || infrastructure.digest != release.manifest.infrastructure_artifact_manifest_sha256
    {
        return Err(BuiltDeclarationError::ManifestDigestMismatch);
    }
    let mut matches = application
        .union
        .entries
        .iter()
        .filter(|entry| entry.role.as_str() == role)
        .map(|entry| {
            (
                &entry.wasm_relative_path,
                entry.wire_schema_sha256,
                entry.wasm_sha256_hex.clone(),
            )
        })
        .chain(
            infrastructure
                .manifest
                .entries
                .iter()
                .filter(|entry| entry.protocol_role.as_str() == role)
                .map(|entry| {
                    (
                        &entry.wasm_relative_path,
                        entry.wire_schema_sha256,
                        entry.wasm_sha256_hex.clone(),
                    )
                }),
        );
    let (wasm, digest, wasm_sha256) = matches
        .next()
        .ok_or_else(|| BuiltDeclarationError::MissingRole(role.into()))?;
    if matches.next().is_some() {
        return Err(BuiltDeclarationError::AmbiguousRole(role.into()));
    }
    let path = root.join(wasm).with_extension("wire.json");
    let bytes = read_contained_declaration(root, &path)?;
    let observed: [u8; 32] = Sha256::digest(&bytes).into();
    if observed != digest {
        return Err(BuiltDeclarationError::DigestMismatch(path));
    }
    Ok(BuiltWireSchema {
        release_build_id,
        path,
        schema: bytes,
        schema_sha256: digest,
        wasm_sha256,
    })
}

fn read_contained_declaration(root: &Path, path: &Path) -> Result<Vec<u8>, BuiltDeclarationError> {
    let unsafe_path = || BuiltDeclarationError::UnsafeDeclaration(path.to_path_buf());
    let canonical_root = root.canonicalize().map_err(|_| unsafe_path())?;
    let parent = path.parent().ok_or_else(unsafe_path)?;
    if !parent
        .canonicalize()
        .map_err(|_| unsafe_path())?
        .starts_with(canonical_root)
    {
        return Err(unsafe_path());
    }
    match read_optional_file_no_follow(path, MAX_DOCUMENT_READ_BYTES) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) | Err(ArtifactError::NotRegularFile) => Err(unsafe_path()),
        Err(source) => Err(BuiltDeclarationError::Read {
            path: path.to_path_buf(),
            source: source.into(),
        }),
    }
}
