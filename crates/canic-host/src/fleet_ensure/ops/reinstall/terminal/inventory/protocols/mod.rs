//! Module: fleet_ensure::ops::reinstall::terminal::inventory::protocols
//!
//! Responsibility: bind every installed source interface to immutable completed-release evidence.
//! Does not own: current release admission, predecessor execution or live module verification.
//! Boundary: the maintained manifest binds reinstall-only release authority and exact artifacts.

mod application;
#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{CompletedEstateInventoryView, model::DesiredCanisterKind, ops::authority_seal},
    protocol_binding::{
        ReleaseProtocolBindingError, ResolvedProtocolBinding,
        resolve_infrastructure_registry_protocol_binding,
    },
    registry::RegistryEntry,
    release_build::{ReleaseBuildPlanError, validate_finalized_release_build_manifest},
    release_set::{
        CanicInfrastructureArtifactPersistenceError, CanicInfrastructureRole,
        CurrentReleaseSetManifest, load_persisted_canic_infrastructure_artifact_manifest,
    },
};
use canic_core::ids::ReleaseBuildId;
use std::{collections::BTreeMap, io, path::Path};
use thiserror::Error;

/// Source evidence or interface binding could not be established without changing its contract.
#[derive(Debug, Error)]
pub enum CompletedSourceProtocolError {
    #[error(
        "completed authority {name} does not expose the maintained seal command/status contract"
    )]
    AuthoritySealContract { name: String },
    #[error(
        "source release manifest is not exact canonical evidence for the selected finalized build"
    )]
    ManifestIdentity,
    #[error("cannot read completed source manifest: {0}")]
    Read(#[from] io::Error),
    #[error("cannot decode completed source manifest evidence: {0}")]
    Decode(#[from] serde_json::Error),
    #[error(transparent)]
    Finalization(#[from] ReleaseBuildPlanError),
    #[error(transparent)]
    Infrastructure(#[from] CanicInfrastructureArtifactPersistenceError),
    #[error(transparent)]
    Application(#[from] crate::release_set::ApplicationArtifactUnionPersistenceError),
    #[error(transparent)]
    Binding(#[from] ReleaseProtocolBindingError),
}

pub(in crate::fleet_ensure) fn inspect(
    workspace: &Path,
    inventory: &CompletedEstateInventoryView,
) -> Result<BTreeMap<String, ResolvedProtocolBinding>, CompletedSourceProtocolError> {
    let release = inventory.release_build_id;
    let manifest_path = workspace
        .join(".canic/release-builds")
        .join(release.to_string())
        .join("current-release-set-manifest.json");
    let bytes = read_regular_bytes(&manifest_path, 64 * 1024)?;
    let evidence = manifest(&bytes, release)?;
    let finalized = validate_finalized_release_build_manifest(workspace, release, &manifest_path)?;
    // Finalization re-reads the exact regular file. Bind that read to this projection too.
    if read_regular_bytes(&manifest_path, 64 * 1024)? != bytes
        || evidence.build_network != finalized.record.build_network
    {
        return Err(CompletedSourceProtocolError::ManifestIdentity);
    }
    let infrastructure = load_persisted_canic_infrastructure_artifact_manifest(workspace, release)?;
    if infrastructure.digest != evidence.infrastructure_artifact_manifest_sha256 {
        return Err(CompletedSourceProtocolError::ManifestIdentity);
    }
    let mut protocols = application::inspect(
        workspace,
        inventory,
        &evidence,
        &finalized.record.builder_version,
    )?;
    for (name, canister) in &inventory.canisters {
        let role = match canister.kind {
            DesiredCanisterKind::Coordinator => CanicInfrastructureRole::FleetCoordinator,
            DesiredCanisterKind::Root => CanicInfrastructureRole::FleetSubnetRoot,
            DesiredCanisterKind::Store => CanicInfrastructureRole::WasmStore,
            _ => continue,
        };
        let artifact = infrastructure
            .manifest
            .entries
            .iter()
            .find(|artifact| artifact.role == role)
            .ok_or(CompletedSourceProtocolError::ManifestIdentity)?;
        if artifact.protocol_release_identity != finalized.record.builder_version {
            return Err(CompletedSourceProtocolError::ManifestIdentity);
        }
        let entry = RegistryEntry {
            pid: canister.principal.to_text(),
            role: canister
                .protocol_binding
                .as_ref()
                .map(|binding| binding.role.to_string()),
            parent_pid: canister
                .parent
                .as_ref()
                .map(|parent| inventory.canisters[parent].principal.to_text()),
            module_hash: canister.module_sha256.clone(),
            protocol_binding: canister.protocol_binding.clone(),
        };
        let protocol =
            resolve_infrastructure_registry_protocol_binding(workspace, artifact, &entry)?;
        if matches!(
            canister.kind,
            DesiredCanisterKind::Root | DesiredCanisterKind::Coordinator
        ) {
            let bytes = read_regular_bytes(protocol.candid_path(), 1024 * 1024)?;
            let matches = canic_core::cdk::utils::hash::sha256_bytes(&bytes).as_slice()
                == protocol.binding().candid_sha256
                && std::str::from_utf8(&bytes).is_ok_and(|text| {
                    authority_seal::contract::verify(text, canister.kind).is_some()
                });
            if !matches {
                return Err(CompletedSourceProtocolError::AuthoritySealContract {
                    name: name.clone(),
                });
            }
        }
        protocols.insert(name.clone(), protocol);
    }
    Ok(protocols)
}

fn manifest(
    bytes: &[u8],
    release: ReleaseBuildId,
) -> Result<CurrentReleaseSetManifest, CompletedSourceProtocolError> {
    let evidence: CurrentReleaseSetManifest = serde_json::from_slice(bytes)?;
    if evidence.schema_version != 1
        || evidence.release_build_id != release
        || serde_json::to_vec(&evidence)? != bytes
    {
        return Err(CompletedSourceProtocolError::ManifestIdentity);
    }
    Ok(evidence)
}
