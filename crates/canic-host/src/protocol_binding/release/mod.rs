//! Module: protocol_binding::release
//!
//! Responsibility: select infrastructure Candid from a terminal Fleet's selected release.
//! Does not own: Fleet discovery, live admission or release production.
//! Boundary: finalized manifest authority and terminal protocol identity must agree.

#[cfg(test)]
mod tests;

use crate::{
    protocol_binding::{
        ProtocolBindingError, RegistryProtocolBinding, ResolvedProtocolBinding,
        resolve_protocol_binding,
    },
    registry::RegistryEntry,
    release_build::{ReleaseBuildPlanError, validate_finalized_release_build_manifest},
    release_set::{
        CanicInfrastructureArtifactPersistenceError, CurrentReleaseSetManifestError,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use canic_core::ids::ReleaseBuildId;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Failures binding a retained infrastructure participant to its immutable release.
#[derive(Debug, Error)]
pub enum ReleaseProtocolBindingError {
    #[error("Canister {canister} does not match its selected release infrastructure artifact")]
    ArtifactBindingMismatch { canister: String },

    #[error(transparent)]
    Binding(#[from] ProtocolBindingError),

    #[error(transparent)]
    Infrastructure(#[from] CanicInfrastructureArtifactPersistenceError),

    #[error("selected release infrastructure manifest digest does not match finalization")]
    ManifestDigestMismatch,

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),

    #[error(transparent)]
    ReleaseSet(#[from] CurrentReleaseSetManifestError),

    #[error("selected release Candid sidecar is not a regular contained file: {}", path.display())]
    UnsafeCandid { path: PathBuf },
}

/// Resolve an infrastructure participant using its exact retained release, before transport.
pub fn resolve_release_registry_protocol_binding(
    root: &Path,
    release_build_id: ReleaseBuildId,
    entry: &RegistryEntry,
) -> Result<ResolvedProtocolBinding, ReleaseProtocolBindingError> {
    let release = load_persisted_current_release_set_manifest(root, release_build_id)?;
    validate_finalized_release_build_manifest(root, release_build_id, &release.path)?;
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)?;
    if infrastructure.digest != release.manifest.infrastructure_artifact_manifest_sha256 {
        return Err(ReleaseProtocolBindingError::ManifestDigestMismatch);
    }
    let mismatch = || ReleaseProtocolBindingError::ArtifactBindingMismatch {
        canister: entry.pid.clone(),
    };
    let binding = entry.protocol_binding.as_ref().ok_or_else(mismatch)?;
    let artifact = infrastructure
        .manifest
        .entries
        .iter()
        .find(|artifact| entry.role.as_deref() == Some(artifact.protocol_role.as_str()))
        .ok_or_else(mismatch)?;
    let expected = RegistryProtocolBinding {
        release_identity: artifact.protocol_release_identity.clone(),
        role: artifact.protocol_role.clone(),
        capabilities: artifact.protocol_capabilities.clone(),
        candid_sha256: artifact.candid_sha256,
        protocol_profile_digest: artifact.protocol_profile_digest,
    };
    if binding != &expected || entry.module_hash.as_deref() != Some(&artifact.wasm_sha256_hex) {
        return Err(mismatch());
    }
    let candid_path = root
        .join(&artifact.wasm_relative_path)
        .with_extension("did");
    require_contained_sidecar(root, &candid_path)?;
    Ok(resolve_protocol_binding(&entry.pid, expected, candid_path)?)
}

fn require_contained_sidecar(root: &Path, path: &Path) -> Result<(), ReleaseProtocolBindingError> {
    let unsafe_path = || ReleaseProtocolBindingError::UnsafeCandid {
        path: path.to_path_buf(),
    };
    let canonical_root = root.canonicalize().map_err(|_| unsafe_path())?;
    let parent = path.parent().ok_or_else(unsafe_path)?;
    let canonical_parent = parent.canonicalize().map_err(|_| unsafe_path())?;
    if !canonical_parent.starts_with(canonical_root) {
        return Err(unsafe_path());
    }
    Ok(())
}
