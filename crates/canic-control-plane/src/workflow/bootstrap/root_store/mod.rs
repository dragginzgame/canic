//! Module: workflow::bootstrap::root_store
//!
//! Responsibility: verify and publish one root's exact initial application release set.
//! Does not own: host staging, Fleet Registry registration, or root runtime activation.
//! Boundary: no Store effect begins until the staged canonical manifest matches protected root
//! authority and every admitted application artifact is complete.

#[cfg(test)]
mod tests;

use crate::{
    dto::template::{TemplateManifestResponse, WasmStoreCatalogEntryResponse},
    ids::{CanisterRole, TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion},
    ops::{
        component_registry::ComponentRegistryOps,
        storage::{
            state::root_wasm_store::RootWasmStoreStateOps,
            template::{TemplateChunkedOps, TemplateManifestOps},
        },
    },
    workflow::{
        root_authority::validated_root_authority,
        runtime::template::{WASM_STORE_BOOTSTRAP_BINDING, WasmStorePublicationWorkflow},
    },
};
use canic_core::{
    cdk::utils::hash::wasm_hash,
    control_plane_support::{
        error::InternalError, ops::config::ConfigOps, workflow::topology::guard::TopologyGuard,
    },
    dto::root_store::{
        ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
        ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX, RootStoreBootstrapRequest,
        RootStoreBootstrapResponse, RootStoreCatalogEntry, RootStoreReleaseSetEntry,
        RootStoreReleaseSetEntryKind, RootStoreReleaseSetManifest,
    },
    ids::{ComponentSpecId, ReleaseBuildId},
};
use std::collections::{BTreeMap, BTreeSet};

/// Manifest authority the root can reproduce from its embedded Component topology.
///
/// The configured role package is a workspace-relative package selector, while the artifact
/// package is its canonical Cargo package name. Host provenance binds those two identities before
/// the manifest digest is protected; the root must not compare the distinct representations.
#[derive(Debug, Eq, PartialEq)]
struct RootReleaseSetEntryAuthority<'a> {
    component_spec: &'a ComponentSpecId,
    kind: RootStoreReleaseSetEntryKind,
    role: &'a CanisterRole,
    release_build_id: &'a ReleaseBuildId,
}

impl<'a> RootReleaseSetEntryAuthority<'a> {
    const fn from_entry(entry: &'a RootStoreReleaseSetEntry) -> Self {
        Self {
            component_spec: &entry.component_spec,
            kind: entry.kind,
            role: &entry.artifact.role,
            release_build_id: &entry.artifact.release_build_id,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StagedArtifactAuthority<'a> {
    template_id: &'a TemplateId,
    role: &'a CanisterRole,
    version: &'a TemplateVersion,
    payload_hash: &'a [u8],
    payload_size_bytes: u64,
    chunking_mode: TemplateChunkingMode,
    manifest_state: TemplateManifestState,
}

impl<'a> StagedArtifactAuthority<'a> {
    const fn from_manifest(manifest: &'a TemplateManifestResponse) -> Self {
        Self {
            template_id: &manifest.template_id,
            role: &manifest.role,
            version: &manifest.version,
            payload_hash: manifest.payload_hash.as_slice(),
            payload_size_bytes: manifest.payload_size_bytes,
            chunking_mode: manifest.chunking_mode,
            manifest_state: manifest.manifest_state,
        }
    }
}

/// Bootstrap and verify the exact local Store for one still-Prepared root.
pub async fn bootstrap(
    request: RootStoreBootstrapRequest,
) -> Result<RootStoreBootstrapResponse, InternalError> {
    let _guard = TopologyGuard::try_enter()?;
    ComponentRegistryOps::require_root_store_admin_open()?;
    let (authority, root) = validated_root_authority()?;

    let manifest = load_and_validate_manifest(&authority, request)?;
    let module_hashes = artifact_module_hashes(&manifest)?;
    let staged = exact_staged_manifests(&manifest)?;

    super::root::ensure_required_wasm_store_canister()?;
    let (wasm_store, live_catalog) =
        WasmStorePublicationWorkflow::bootstrap_exact_staged_release_set(staged.clone()).await?;
    let catalog = verify_live_catalog(&staged, live_catalog, &module_hashes)?;

    Ok(RootStoreBootstrapResponse {
        fleet_subnet_root: root,
        wasm_store,
        release_set: authority.initial_release_set,
        catalog,
    })
}

/// Verify the exact live Store catalog without changing root or Store state.
pub async fn status(
    request: RootStoreBootstrapRequest,
) -> Result<RootStoreBootstrapResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    let manifest = load_and_validate_manifest(&authority, request)?;
    let module_hashes = artifact_module_hashes(&manifest)?;
    // Bootstrap already verified every staged chunk before publishing the release set. Status is
    // a bounded verification of the protected manifest metadata against the live Store catalog;
    // re-reading and hashing every staged payload here makes its query cost scale with Wasm bytes.
    let staged = exact_staged_manifest_metadata(&manifest)?;
    let (wasm_store, live_catalog) = WasmStorePublicationWorkflow::single_store_catalog().await?;
    let catalog = verify_live_catalog(&staged, live_catalog, &module_hashes)?;

    Ok(RootStoreBootstrapResponse {
        fleet_subnet_root: root,
        wasm_store,
        release_set: authority.initial_release_set,
        catalog,
    })
}

fn load_and_validate_manifest(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    request: RootStoreBootstrapRequest,
) -> Result<RootStoreReleaseSetManifest, InternalError> {
    if request.manifest_payload_size_bytes == 0
        || request.manifest_payload_size_bytes > ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES
    {
        return Err(InternalError::invalid_input());
    }

    let release_set = authority.initial_release_set;
    let template_id = release_set_template_id(release_set.manifest_digest);
    let version = TemplateVersion::owned(release_set.release_build_id.to_string());
    let bytes = TemplateChunkedOps::staged_payload_bytes(
        &template_id,
        &version,
        release_set.manifest_digest.as_bytes(),
        request.manifest_payload_size_bytes,
    )?;
    let manifest = serde_json::from_slice::<RootStoreReleaseSetManifest>(&bytes)
        .map_err(|_error| InternalError::invalid_input())?;
    let canonical =
        serde_json::to_vec(&manifest).map_err(|_error| InternalError::invalid_input())?;
    if canonical != bytes {
        return Err(InternalError::invalid_input());
    }
    if wasm_hash(&canonical) != release_set.manifest_digest.as_bytes() {
        return Err(InternalError::invalid_input());
    }

    validate_manifest_projection(authority, &manifest)?;
    Ok(manifest)
}

fn validate_manifest_projection(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    manifest: &RootStoreReleaseSetManifest,
) -> Result<(), InternalError> {
    if manifest.release_build_id != authority.initial_release_set.release_build_id {
        return Err(InternalError::invalid_input());
    }
    if manifest.component_topology_digest != authority.binding.component_topology_digest {
        return Err(InternalError::invalid_input());
    }

    let topology = ConfigOps::component_topology()?;
    let projected = topology
        .project_for_admissions(&authority.binding.component_admissions)
        .map_err(|_error| InternalError::invalid_input())?;
    let projected_digest = projected
        .digest()
        .map_err(|_error| InternalError::invalid_input())?;
    if projected_digest != authority.binding.component_topology_digest {
        return Err(InternalError::invalid_input());
    }

    let mut expected = Vec::new();
    for spec in projected.component_specs {
        expected.push((
            spec.component_spec.clone(),
            RootStoreReleaseSetEntryKind::Component,
            spec.component_role,
        ));
        expected.extend(spec.children.into_iter().map(|child| {
            (
                spec.component_spec.clone(),
                RootStoreReleaseSetEntryKind::ComponentChild,
                child.role,
            )
        }));
    }
    if manifest.entries.len() != expected.len() {
        return Err(InternalError::invalid_input());
    }

    let mut unique_artifacts = BTreeMap::<CanisterRole, ([u8; 32], u64)>::new();
    let mut unique_payloads = BTreeSet::new();
    let mut total_bytes = 0_u64;
    for (entry, (component_spec, kind, role)) in manifest.entries.iter().zip(expected) {
        let expected_authority = RootReleaseSetEntryAuthority {
            component_spec: &component_spec,
            kind,
            role: &role,
            release_build_id: &manifest.release_build_id,
        };
        if RootReleaseSetEntryAuthority::from_entry(entry) != expected_authority {
            return Err(InternalError::invalid_input());
        }
        validate_artifact_shape(entry)?;
        let payload_hash = decode_sha256(&entry.artifact.wasm_gz_sha256_hex)?;
        let payload = (payload_hash, entry.artifact.wasm_gz_size_bytes);
        if let Some(existing) = unique_artifacts.insert(role, payload)
            && existing != payload
        {
            return Err(InternalError::invalid_input());
        }
        if unique_payloads.insert(payload) {
            total_bytes = total_bytes
                .checked_add(payload.1)
                .ok_or_else(|| InternalError::invalid_input())?;
        }
    }
    if total_bytes > authority.binding.limits.maximum_wasm_store_bytes {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}

fn validate_artifact_shape(
    entry: &canic_core::dto::root_store::RootStoreReleaseSetEntry,
) -> Result<(), InternalError> {
    let artifact = &entry.artifact;
    let paths_are_complete = [
        artifact.package.as_str(),
        artifact.wasm_relative_path.as_str(),
        artifact.wasm_gz_relative_path.as_str(),
    ]
    .into_iter()
    .all(|value| !value.is_empty());
    let sizes_are_positive = artifact.wasm_size_bytes > 0 && artifact.wasm_gz_size_bytes > 0;
    if !paths_are_complete || !sizes_are_positive {
        return Err(InternalError::invalid_input());
    }
    let _ = decode_sha256(&artifact.wasm_sha256_hex)?;
    let _ = decode_sha256(&artifact.wasm_gz_sha256_hex)?;
    Ok(())
}

fn exact_staged_manifests(
    manifest: &RootStoreReleaseSetManifest,
) -> Result<Vec<TemplateManifestResponse>, InternalError> {
    let staged = exact_staged_manifest_metadata(manifest)?;
    for observed in &staged {
        TemplateChunkedOps::validate_staged_release(observed)?;
    }
    Ok(staged)
}

fn exact_staged_manifest_metadata(
    manifest: &RootStoreReleaseSetManifest,
) -> Result<Vec<TemplateManifestResponse>, InternalError> {
    let mut artifacts = BTreeMap::new();
    for entry in &manifest.entries {
        let payload_hash = decode_sha256(&entry.artifact.wasm_gz_sha256_hex)?;
        let payload = (payload_hash, entry.artifact.wasm_gz_size_bytes);
        if let Some(existing) = artifacts.insert(entry.artifact.role.clone(), payload)
            && existing != payload
        {
            return Err(InternalError::invalid_input());
        }
    }

    let version = TemplateVersion::owned(manifest.release_build_id.to_string());
    artifacts
        .into_iter()
        .map(|(role, (payload_hash, payload_size_bytes))| {
            let observed = TemplateManifestOps::approved_for_role_response(&role)?;
            let expected_template_id = artifact_template_id(&role);
            let expected = StagedArtifactAuthority {
                template_id: &expected_template_id,
                role: &role,
                version: &version,
                payload_hash: payload_hash.as_slice(),
                payload_size_bytes,
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: TemplateManifestState::Approved,
            };
            if StagedArtifactAuthority::from_manifest(&observed) != expected
                || !is_exact_bootstrap_source(&observed.store_binding)
            {
                return Err(InternalError::invalid_input());
            }
            Ok(observed)
        })
        .collect()
}

fn is_exact_bootstrap_source(binding: &crate::ids::WasmStoreBinding) -> bool {
    binding == &WASM_STORE_BOOTSTRAP_BINDING
        || RootWasmStoreStateOps::wasm_store_pid(binding).is_some()
}

fn artifact_module_hashes(
    manifest: &RootStoreReleaseSetManifest,
) -> Result<BTreeMap<CanisterRole, [u8; 32]>, InternalError> {
    let mut module_hashes = BTreeMap::new();
    for entry in &manifest.entries {
        let module_hash = decode_sha256(&entry.artifact.wasm_sha256_hex)?;
        if let Some(existing) = module_hashes.insert(entry.artifact.role.clone(), module_hash)
            && existing != module_hash
        {
            return Err(InternalError::invalid_input());
        }
    }
    Ok(module_hashes)
}

fn verify_live_catalog(
    expected: &[TemplateManifestResponse],
    observed: Vec<WasmStoreCatalogEntryResponse>,
    module_hashes: &BTreeMap<CanisterRole, [u8; 32]>,
) -> Result<Vec<RootStoreCatalogEntry>, InternalError> {
    let expected = expected
        .iter()
        .map(|manifest| {
            (
                manifest.role.clone(),
                manifest.template_id.clone(),
                manifest.version.clone(),
                manifest.payload_hash.clone(),
                manifest.payload_size_bytes,
            )
        })
        .collect::<Vec<_>>();
    let actual = observed
        .iter()
        .map(|entry| {
            (
                entry.role.clone(),
                entry.template_id.clone(),
                entry.version.clone(),
                entry.payload_hash.clone(),
                entry.payload_size_bytes,
            )
        })
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(InternalError::conflict());
    }

    observed
        .into_iter()
        .map(|entry| {
            let raw_module_hash = module_hashes
                .get(&entry.role)
                .copied()
                .ok_or_else(|| InternalError::invalid_input())?;
            Ok(RootStoreCatalogEntry {
                role: entry.role,
                raw_module_hash,
                payload_hash: entry
                    .payload_hash
                    .try_into()
                    .map_err(|_| InternalError::invalid_input())?,
                payload_size_bytes: entry.payload_size_bytes,
            })
        })
        .collect()
}

fn release_set_template_id(digest: canic_core::ids::ReleaseSetDigest) -> TemplateId {
    TemplateId::owned(format!("{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{digest}"))
}

fn artifact_template_id(role: &CanisterRole) -> TemplateId {
    TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], InternalError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(InternalError::invalid_input());
    }

    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
    }
    Ok(bytes)
}

const fn decode_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!(),
    }
}
