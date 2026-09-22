//! Resolve an offline declaration through exact finalized release authority.
//!
//! This owner verifies retained manifest and declaration bytes; it does not select
//! the latest build, require current application source, or contact a canister.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    release_build::{ReleaseBuildPlanError, validate_finalized_release_build_manifest},
    release_set::{
        ApplicationArtifactUnionPersistenceError, CanicInfrastructureArtifactPersistenceError,
        CurrentReleaseSetManifestError, application::load_retained_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use std::path::{Path, PathBuf};

use canic_core::ids::ReleaseBuildId;
use sha2_host::{Digest, Sha256};
use thiserror::Error;

///
/// BuiltCandid
///
/// Exact declaration bytes verified by the managed artifact owner for offline inspection.
///

#[derive(Debug)]
pub struct BuiltCandid {
    pub release_build_id: ReleaseBuildId,
    pub path: PathBuf,
    pub candid: String,
}

///
/// BuiltCandidError
///
/// Failure to bind an offline declaration to a finalized managed build.
///

#[derive(Debug, Error)]
pub enum BuiltCandidError {
    #[error("selected build contains ambiguous role {0}")]
    AmbiguousRole(String),

    #[error(transparent)]
    Application(#[from] ApplicationArtifactUnionPersistenceError),

    #[error("selected build declaration hash differs: {}", .0.display())]
    CandidDigestMismatch(PathBuf),

    #[error(transparent)]
    Infrastructure(#[from] CanicInfrastructureArtifactPersistenceError),

    #[error("selected build declaration is not UTF-8: {}", .0.display())]
    InvalidUtf8(PathBuf),

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
    UnsafeCandid(PathBuf),
}

/// Load one role's exact declaration without live lookup or current-source substitution.
pub fn load_built_candid(
    root: &Path,
    release_build_id: ReleaseBuildId,
    role: &str,
) -> Result<BuiltCandid, BuiltCandidError> {
    let release = load_persisted_current_release_set_manifest(root, release_build_id)?;
    validate_finalized_release_build_manifest(root, release_build_id, &release.path)?;
    let application = load_retained_application_artifact_union(root, release_build_id)?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)?;
    if application.digest != release.manifest.application_artifact_union_sha256
        || infrastructure.digest != release.manifest.infrastructure_artifact_manifest_sha256
    {
        return Err(BuiltCandidError::ManifestDigestMismatch);
    }
    let mut matches = application
        .union
        .entries
        .iter()
        .filter(|entry| entry.role.as_str() == role)
        .map(|entry| (&entry.wasm_relative_path, entry.candid_sha256))
        .chain(
            infrastructure
                .manifest
                .entries
                .iter()
                .filter(|entry| entry.protocol_role.as_str() == role)
                .map(|entry| (&entry.wasm_relative_path, entry.candid_sha256)),
        );
    let (wasm, digest) = matches
        .next()
        .ok_or_else(|| BuiltCandidError::MissingRole(role.into()))?;
    if matches.next().is_some() {
        return Err(BuiltCandidError::AmbiguousRole(role.into()));
    }
    let path = root.join(wasm).with_extension("did");
    let bytes = read_contained_candid(root, &path)?;
    let observed: [u8; 32] = Sha256::digest(&bytes).into();
    if observed != digest {
        return Err(BuiltCandidError::CandidDigestMismatch(path));
    }
    let candid =
        String::from_utf8(bytes).map_err(|_| BuiltCandidError::InvalidUtf8(path.clone()))?;
    Ok(BuiltCandid {
        release_build_id,
        path,
        candid,
    })
}

fn read_contained_candid(root: &Path, path: &Path) -> Result<Vec<u8>, BuiltCandidError> {
    let unsafe_path = || BuiltCandidError::UnsafeCandid(path.to_path_buf());
    let canonical_root = root.canonicalize().map_err(|_| unsafe_path())?;
    let parent = path.parent().ok_or_else(unsafe_path)?;
    if !parent
        .canonicalize()
        .map_err(|_| unsafe_path())?
        .starts_with(canonical_root)
    {
        return Err(unsafe_path());
    }
    match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => Ok(bytes),
        Err(RegularFileReadError::Io(source)) => Err(BuiltCandidError::Read {
            path: path.to_path_buf(),
            source,
        }),
        _ => Err(unsafe_path()),
    }
}
