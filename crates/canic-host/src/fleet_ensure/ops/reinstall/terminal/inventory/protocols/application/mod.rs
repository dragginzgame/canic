//! Bind passive application queries to the exact finalized source artifact union.
//!
//! No current configuration, executable predecessor authority or default is consulted.

use crate::{
    fleet_ensure::{
        CompletedEstateInventoryView, model::DesiredCanisterKind,
        ops::reinstall::terminal::inventory::protocols::CompletedSourceProtocolError,
        view::terminal_source::inventory::evidence::CompletedReleaseManifestEvidence,
    },
    protocol_binding::{
        ReleaseProtocolBindingError, ResolvedProtocolBinding, require_contained_sidecar,
        resolve_protocol_binding,
    },
    release_set::load_retained_application_artifact_union,
};
use std::{collections::BTreeMap, path::Path};

pub(super) fn inspect(
    workspace: &Path,
    inventory: &CompletedEstateInventoryView,
    manifest: &CompletedReleaseManifestEvidence,
    builder: &str,
) -> Result<BTreeMap<String, ResolvedProtocolBinding>, CompletedSourceProtocolError> {
    let application =
        load_retained_application_artifact_union(workspace, inventory.release_build_id)?;
    if application.digest != manifest.application_artifact_union_sha256 {
        return Err(CompletedSourceProtocolError::ManifestIdentity);
    }
    let mut protocols = BTreeMap::new();
    for (name, canister) in &inventory.canisters {
        if canister.kind != DesiredCanisterKind::Component {
            continue;
        }
        let invalid = || ReleaseProtocolBindingError::ArtifactBindingMismatch {
            canister: canister.principal.to_text(),
        };
        let binding = canister.protocol_binding.as_ref().ok_or_else(invalid)?;
        let artifact = application
            .union
            .entries
            .iter()
            .find(|artifact| artifact.role == binding.role)
            .ok_or_else(invalid)?;
        let exact = binding.release_identity == builder
            && binding.candid_sha256 == artifact.candid_sha256
            && binding.protocol_profile_digest == artifact.protocol_profile_digest
            && canister.module_sha256.as_deref() == Some(&artifact.wasm_gz_sha256_hex);
        if !exact {
            return Err(invalid().into());
        }
        let path = workspace
            .join(&artifact.wasm_relative_path)
            .with_extension("did");
        require_contained_sidecar(workspace, &path)?;
        let protocol = resolve_protocol_binding(name, binding.clone(), path)
            .map_err(ReleaseProtocolBindingError::from)?;
        protocols.insert(name.clone(), protocol);
    }
    Ok(protocols)
}
