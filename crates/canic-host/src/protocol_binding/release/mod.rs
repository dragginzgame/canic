//! Module: protocol_binding::release
//!
//! Responsibility: select native artifact authority from a terminal Fleet's selected release.
//! Does not own: Fleet discovery, live admission or release production.
//! Boundary: finalized manifest authority and terminal protocol identity must agree.

#[cfg(test)]
mod tests;

use crate::{
    protocol_binding::RegistryProtocolBinding,
    registry::RegistryEntry,
    release_set::{
        CanicInfrastructureArtifactPersistenceError,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use canic_contracts::ids::ReleaseBuildId;
use std::path::Path;
use thiserror::Error;

/// Failures binding a retained infrastructure participant to its immutable release.
#[derive(Debug, Error)]
pub enum ReleaseProtocolBindingError {
    #[error("terminal Fleet plan has no selected release")]
    MissingSelectedRelease,

    #[error(transparent)]
    Application(#[from] crate::release_set::ApplicationArtifactUnionPersistenceError),

    #[error("Canister {canister} does not match its selected release artifact")]
    ArtifactBindingMismatch { canister: String },

    #[error(transparent)]
    Infrastructure(#[from] CanicInfrastructureArtifactPersistenceError),

    #[error(transparent)]
    Declaration(#[from] crate::release_set::BuiltDeclarationError),
}

/// Bind bootstrap metadata to the finalized release's native artifact authority.
pub fn resolve_infrastructure_wire_profile(
    root: &Path,
    artifact: &crate::release_set::CanicInfrastructureArtifactEntry,
) -> Result<RegistryProtocolBinding, ReleaseProtocolBindingError> {
    let binding = infrastructure_profile(artifact);
    let mismatch = || ReleaseProtocolBindingError::ArtifactBindingMismatch {
        canister: artifact.role.as_str().to_string(),
    };
    let digest = canic_core::role_contract::derive_protocol_profile_digest(
        &binding.release_identity,
        &binding.role,
        &binding.capabilities,
        binding.wire_schema_sha256,
    );
    if binding.role.as_str() != artifact.role.protocol_role_name()
        || binding.release_identity.trim().is_empty()
        || digest != binding.protocol_profile_digest
    {
        return Err(mismatch());
    }
    let entry = RegistryEntry {
        pid: artifact.role.as_str().to_string(),
        role: Some(binding.role.to_string()),
        parent_pid: None,
        module_hash: Some(artifact.wasm_sha256_hex.clone()),
        protocol_binding: Some(binding.clone()),
    };
    let schema = resolve_release_registry_wire_schema(root, artifact.release_build_id, &entry)?;
    if schema.schema_sha256 != artifact.wire_schema_sha256 {
        return Err(mismatch());
    }
    Ok(binding)
}

fn infrastructure_profile(
    artifact: &crate::release_set::CanicInfrastructureArtifactEntry,
) -> RegistryProtocolBinding {
    RegistryProtocolBinding {
        release_identity: artifact.protocol_release_identity.clone(),
        role: artifact.protocol_role.clone(),
        capabilities: artifact.protocol_capabilities.clone(),
        wire_schema_sha256: artifact.wire_schema_sha256,
        protocol_profile_digest: artifact.protocol_profile_digest,
    }
}

fn require_registry_identity(
    artifact: &crate::release_set::CanicInfrastructureArtifactEntry,
    entry: &RegistryEntry,
) -> Result<RegistryProtocolBinding, ReleaseProtocolBindingError> {
    let mismatch = || ReleaseProtocolBindingError::ArtifactBindingMismatch {
        canister: entry.pid.clone(),
    };
    let binding = entry.protocol_binding.as_ref().ok_or_else(mismatch)?;
    let expected = infrastructure_profile(artifact);
    let digest = canic_core::role_contract::derive_protocol_profile_digest(
        &expected.release_identity,
        &expected.role,
        &expected.capabilities,
        expected.wire_schema_sha256,
    );
    if expected.protocol_profile_digest != digest
        || binding != &expected
        || entry.role.as_deref() != Some(artifact.protocol_role.as_str())
        || entry.module_hash.as_deref() != Some(&artifact.wasm_sha256_hex)
    {
        return Err(mismatch());
    }
    Ok(expected)
}

/// Resolve exact native schema authority without reading a Candid sidecar.
pub fn resolve_release_registry_wire_schema(
    root: &Path,
    release_build_id: ReleaseBuildId,
    entry: &RegistryEntry,
) -> Result<crate::release_set::BuiltWireSchema, ReleaseProtocolBindingError> {
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)?;
    let mismatch = || ReleaseProtocolBindingError::ArtifactBindingMismatch {
        canister: entry.pid.clone(),
    };
    let role = entry.role.as_deref().ok_or_else(mismatch)?;
    // Loading the schema validates finalized digests for both artifact manifests.
    let schema = crate::release_set::load_built_wire_schema(root, release_build_id, role)?;
    if let Some(artifact) = infrastructure
        .manifest
        .entries
        .iter()
        .find(|artifact| role == artifact.protocol_role.as_str())
    {
        require_registry_identity(artifact, entry)?;
        if schema.schema_sha256 != artifact.wire_schema_sha256
            || schema.wasm_sha256 != artifact.wasm_sha256_hex
        {
            return Err(mismatch());
        }
    } else {
        let applications =
            crate::release_set::load_retained_application_artifact_union(root, release_build_id)?;
        let artifact = applications
            .union
            .entries
            .iter()
            .find(|artifact| artifact.role.as_str() == role)
            .ok_or_else(mismatch)?;
        let binding = entry.protocol_binding.as_ref().ok_or_else(mismatch)?;
        let selected_profile = canic_core::role_contract::derive_protocol_profile_digest(
            &binding.release_identity,
            &binding.role,
            &binding.capabilities,
            binding.wire_schema_sha256,
        );
        let profile_matches = (
            binding.role.as_str(),
            binding.wire_schema_sha256,
            binding.protocol_profile_digest,
            selected_profile,
        ) == (
            role,
            artifact.wire_schema_sha256,
            artifact.protocol_profile_digest,
            artifact.protocol_profile_digest,
        );
        let artifact_matches = (
            schema.schema_sha256,
            schema.wasm_sha256.as_str(),
            entry.module_hash.as_deref(),
        ) == (
            artifact.wire_schema_sha256,
            artifact.wasm_sha256_hex.as_str(),
            Some(artifact.wasm_gz_sha256_hex.as_str()),
        );
        if binding.release_identity.trim().is_empty() || !profile_matches || !artifact_matches {
            return Err(mismatch());
        }
    }
    Ok(schema)
}

/// Selected release authority from the terminal current Fleet plan.
fn current_release(
    current: &crate::fleet_ensure::CurrentFleetResolution,
) -> Result<ReleaseBuildId, ReleaseProtocolBindingError> {
    current
        .plan
        .reviewed_desired
        .as_ref()
        .and_then(|reviewed| reviewed.desired().bootstrap.as_ref())
        .map(|bootstrap| bootstrap.release_build_id)
        .ok_or(ReleaseProtocolBindingError::MissingSelectedRelease)
}
/// Resolve the exact retained native schema for a current Fleet participant.
pub fn resolve_current_registry_wire_schema(
    root: &Path,
    current: &crate::fleet_ensure::CurrentFleetResolution,
    entry: &RegistryEntry,
) -> Result<crate::release_set::BuiltWireSchema, ReleaseProtocolBindingError> {
    resolve_release_registry_wire_schema(root, current_release(current)?, entry)
}
